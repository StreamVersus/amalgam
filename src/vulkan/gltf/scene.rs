use crate::vulkan::func::{Destructible, Vulkan};
use crate::vulkan::gltf::gltf_struct;
use crate::vulkan::gltf::gltf_struct::{Attributes, Gltf};
use crate::vulkan::r#impl::command_buffer::RecordingInfo;
use crate::vulkan::r#impl::descriptors::{BufferDescriptorInfo, DescriptorSetInfo, ImageDescriptorInfo};
use crate::vulkan::r#impl::image::ImageTransition;
use crate::vulkan::r#impl::memory::{AllocationInfo, AllocationTask, MemoryInfo, VkDestroy};
use crate::vulkan::r#impl::sampler::SamplerInfo;
use crate::vulkan::utils::{build_pool_size, BufferUsage, ImageUsage};
use image::ImageReader;
use std::collections::{HashMap, HashSet};
use std::ffi::c_void;
use std::io::Cursor;
use std::ptr::null_mut;
use ultraviolet::{Mat4, Rotor3, Vec3};
use vulkan_raw::{vkCmdDrawIndexedIndirect, VkAccessFlags, VkBorderColor, VkBuffer, VkBufferCopy, VkBufferImageCopy, VkCommandBuffer, VkCommandBufferLevel, VkCommandBufferUsageFlags, VkCommandPoolCreateFlags, VkCompareOp, VkDescriptorBufferInfo, VkDescriptorImageInfo, VkDescriptorSet, VkDescriptorSetLayout, VkDescriptorSetLayoutBinding, VkDescriptorType, VkDeviceMemory, VkDeviceSize, VkExtent3D, VkFence, VkFilter, VkFormat, VkImage, VkImageAspectFlags, VkImageLayout, VkImageSubresourceLayers, VkImageType, VkImageView, VkImageViewType, VkIndexType, VkPipelineBindPoint, VkPipelineLayout, VkPipelineStageFlags, VkSampleCountFlagBits, VkSampler, VkSamplerAddressMode, VkSamplerMipmapMode, VkShaderStageFlags, VK_QUEUE_FAMILY_IGNORED, VK_WHOLE_SIZE};
use crate::engine::vbo::VBO;

#[derive(Default)]
pub struct Scene {
    pub ubo: UniformBuffer,

    pub vbo: VBO,
    pub idx: VkDestroy<VkBuffer>,
    pub indirect_buffer: VkDestroy<VkBuffer>,

    pub idx_size: u64,
    pub parameters_size: u64,

    pub parameter_count: u32,
    parameters: Vec<IndirectParameters>,
    pub descriptor_sets: Vec<VkDescriptorSet>,
    pub descriptor_layouts: Vec<VkDescriptorSetLayout>,

    pub indices: Vec<u16>,

    texture_images: Vec<Image>,
    _samplers: Vec<VkDestroy<VkSampler>>,
    _memory: Vec<VkDestroy<VkDeviceMemory>>,
}

pub struct Node<'a> {
    pub mesh: &'a Mesh,
    pub pos: Vec3,
    pub rot: Rotor3,
}
#[derive(Clone)]
pub struct Mesh {
    pub id: u32,
    pub primitives: Vec<Primitive>,
}

#[derive(Clone)]
pub struct Primitive {
    pub indices: u32,
    pub vertices: u32,
}
const SIZE_TEXCOORDS: usize = size_of::<[f32; 2]>();
#[derive(Debug)]
#[repr(C)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub texcoords: [f32; 2],
}

struct Chunk {
    pub data: Vec<u8>,
}

struct Image {
    pub image: VkDestroy<VkImage>,
    pub image_view: VkDestroy<VkImageView>,

    pub data: Vec<u8>,
    pub size: usize,
    pub extent: VkExtent3D,
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq)]
enum ChunkType{
    JSON = 0x4E4F534A,
    BIN = 0x004E4942,
}

impl TryFrom<u32> for ChunkType {
    type Error = ();

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        if value == ChunkType::JSON as u32 {
            Ok(ChunkType::JSON)
        } else if value == ChunkType::BIN as u32 {
            Ok(ChunkType::BIN)
        } else {
            panic!("Unknown chunk type {}", value)
        }
    }
}

#[allow(dead_code)]
#[derive(Debug)]
struct IndirectParameters {
    index_count: u32,
    instance_count: u32,
    first_index: u32,
    vertex_offset: u32,
    first_instance: u32
}

impl Scene {
    pub fn from_glb(bytes: &[u8], vulkan: Vulkan) -> Scene {
        check_magic(bytes);
        check_length(bytes);

        let bytes = &bytes[12..]; // data
        let (json_chunk, bin_chunk) = raw_to_chunks(bytes);
        let gltf: Gltf = unsafe { sonic_rs::from_slice_unchecked(json_chunk.data.as_slice()).expect("broken json") };

        let parameters_size = (gltf.meshes.len() * size_of::<IndirectParameters>()) as u64;
        let mut vbo_size: u64 = 0;
        let mut idx_size: u64 = 0;
        let mut attr_set: HashSet<Attributes> = HashSet::new();
        let mut idx_set: HashSet<u32> = HashSet::new();
        gltf.meshes.iter().for_each(|mesh| {
            mesh.primitives.iter().for_each(|primitive| {
                let attr = primitive.attributes;
                if !attr_set.contains(&attr) {
                    [attr.POSITION, attr.NORMAL].into_iter().for_each(|id| {
                        vbo_size += resolve_size(&gltf, id) as u64;
                    });

                    match attr.TEXCOORD_0 {
                        Some(texcoords_id) => vbo_size += resolve_size(&gltf, texcoords_id) as u64,
                        None => vbo_size += SIZE_TEXCOORDS as u64 * resolve_vertices(&gltf, attr) as u64,
                    }
                    attr_set.insert(attr);
                }
                if !idx_set.contains(&primitive.indices) {
                    idx_size += resolve_size(&gltf, primitive.indices) as u64;
                    idx_set.insert(primitive.indices);
                }
            });
        });

        let idx_buffer = vulkan.create_buffer(idx_size, BufferUsage::preset_index()).unwrap();
        let indirect_buffer = vulkan.create_buffer(parameters_size, BufferUsage::default().transfer_dst(true).indirect_buffer(true)).unwrap();
        let main_buffers_info = AllocationTask::device()
            .add_allocatable(idx_buffer)
            .add_allocatable(indirect_buffer)
            .allocate_all(&vulkan);
        
        let mut vbo = VBO::new(&vulkan, vbo_size);
        let mut indices = Vec::with_capacity(idx_size as usize / size_of::<u16>());
        let mut meshes: HashMap<u32, Mesh> = HashMap::with_capacity(gltf.meshes.len());
        gltf.meshes.iter().enumerate().for_each(|(mesh_id, mesh)| {
            let mut primitives: Vec<Primitive> = Vec::with_capacity(mesh.primitives.len());

            mesh.primitives.iter().for_each(|primitive| {
                let attr = primitive.attributes;
                let vertex_amount = resolve_vertices(&gltf, attr) as usize;
                for i in 0..vertex_amount {
                    resolve_vertex(&gltf, attr, i, &bin_chunk.data, &mut vbo);
                }

                let index_offset = resolve_offset(&gltf, primitive.indices) as usize;
                let index_size = resolve_size(&gltf, primitive.indices) as usize;
                let bytes = &bin_chunk.data[index_offset..index_offset + index_size];
                let u16_slice: &[u16] = bytemuck::cast_slice(bytes);
                indices.extend_from_slice(u16_slice);

                primitives.push(Primitive {
                    indices: resolve_amount(&gltf, primitive.indices),
                    vertices: vertex_amount as u32,
                });
            });

            let mesh = Mesh {
                id: mesh_id as u32,
                primitives,
            };

            meshes.insert(mesh_id as u32, mesh);
        });

        let mut nodes: Vec<Node> = Vec::with_capacity(gltf.nodes.len());
        gltf.nodes.iter().for_each(|node| {
            nodes.push(Node {
                mesh: meshes.get(&node.mesh).expect("Tried to get nonexistent mesh"),
                pos: Vec3::from(node.translation.unwrap_or([0.0, 0.0, 0.0])),
                rot: Rotor3::from_quaternion_array(node.rotation.unwrap_or([1.0, 0.0, 0.0, 0.0])),
            });
        });

        let mut mesh_renders: HashMap<u32, u32> = HashMap::with_capacity(gltf.meshes.len());
        nodes.iter().for_each(|node| {
            let amount = *mesh_renders.get(&node.mesh.id).unwrap_or(&0);
            mesh_renders.insert(node.mesh.id, amount + 1);
        });
        let mut parameters: Vec<IndirectParameters> = Vec::with_capacity(gltf.meshes.len());

        let mut index_offset = 0;
        let mut vertex_offset = 0;
        let mut instance_offset = 0;
        let mut instance_resolve: HashMap<u32, &gltf_struct::Primitive> = HashMap::with_capacity(gltf.meshes.len());
        for i in 0..gltf.meshes.len() as u32 {
            let mesh = meshes.get(&i).unwrap();
            let instances = mesh_renders.remove(&i).unwrap();

            let json_mesh = &gltf.meshes[i as usize];
            mesh.primitives.iter().enumerate().for_each(|(i, primitive)| {
                parameters.push(IndirectParameters {
                    index_count: primitive.indices,
                    instance_count: instances,
                    first_index: index_offset,
                    vertex_offset,
                    first_instance: instance_offset,
                });

                index_offset += primitive.indices;
                vertex_offset += primitive.vertices;
                for _ in 0..instances {
                    instance_resolve.insert(instance_offset, &json_mesh.primitives[i]);
                    instance_offset += 1;
                }
            });
        }

        let samplers: Vec<VkSampler> = gltf.samplers.iter().map(|sampler| {
            let mut sampler_info = SamplerInfo {
                mip_lod_bias: 0.0,
                anisotropy_enable: false,
                max_anisotropy: 0.0,
                comparison_enable: false,
                compare_op: VkCompareOp::NEVER,
                min_lod: 0.0,
                max_lod: 0.0,
                border_color: VkBorderColor::INT_OPAQUE_BLACK,
                unnormalized_coordinates: false,
                ..Default::default()
            };
            resolve_opengl_magfilter(sampler.magFilter, &mut sampler_info);
            resolve_opengl_minfilter(sampler.minFilter, &mut sampler_info);
            resolve_opengl_wraps(sampler.wrapT, sampler.wrapS, &mut sampler_info);

            vulkan.create_sampler(sampler_info)
        }).collect();

        let mut device_task = AllocationTask::device();
        let texture_images = gltf.images.iter().map(|image_info| {
            let format = ImageFormat::from(image_info.mimeType.clone()).into();
            let byte_blob = {
                let buffer_view = &gltf.bufferViews[image_info.bufferView as usize];
                let offset = buffer_view.byteOffset as usize;
                let size = buffer_view.byteLength as usize;
                &bin_chunk.data[offset..offset + size]
            };
            let img = ImageReader::new(Cursor::new(byte_blob)).with_guessed_format().unwrap().decode().unwrap();

            let resolution = VkExtent3D {
                width: img.width(),
                height: img.height(),
                depth: 1,
            };
            let image = vulkan.create_image(format, VkImageType::IT_2D, false, 1, 1, resolution, VkSampleCountFlagBits::SC_1_BIT, ImageUsage::default().sampled(true).transfer_dst(true));
            device_task.add_allocatable_ref(image);

            let rgba = img.to_rgba8().into_raw();
            (image, rgba, format, resolution)
        }).collect::<Vec<_>>();
        let texture_image_info = device_task.allocate_all(&vulkan);

        let texture_images = texture_images.into_iter().map(|(image, data, format, extent)| {
            let image_view = vulkan.create_image_view(&image, VkImageViewType::IVT_2D, format, VkImageAspectFlags::COLOR_BIT);

            let image = VkDestroy::new(image, &vulkan);
            let image_view = VkDestroy::new(image_view, &vulkan);
            let size = data.len();
            Image {
                image,
                image_view,
                data,
                size,
                extent,
            }
        }).collect::<Vec<_>>();

        let indirect_description_bindings = [
            VkDescriptorSetLayoutBinding {
                binding: 0,
                descriptorType: VkDescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptorCount: gltf.textures.len() as u32,
                stageFlags: VkShaderStageFlags::FRAGMENT_BIT,
                pImmutableSamplers: null_mut(),
            },
            VkDescriptorSetLayoutBinding {
                binding: 1,
                descriptorType: VkDescriptorType::UNIFORM_BUFFER,
                descriptorCount: 1,
                stageFlags: VkShaderStageFlags::VERTEX_BIT,
                pImmutableSamplers: null_mut(),
            }
        ];
        let indirect_descriptor_layout = vulkan.create_descriptor_set_layout(&indirect_description_bindings);

        let vp_description_bindings = [
            VkDescriptorSetLayoutBinding {
                binding: 0,
                descriptorType: VkDescriptorType::UNIFORM_BUFFER,
                descriptorCount: 1,
                stageFlags: VkShaderStageFlags::VERTEX_BIT,
                pImmutableSamplers: null_mut(),
            }
        ];
        let vp_descriptor_layout = vulkan.create_descriptor_set_layout(&vp_description_bindings);
        let mut descriptor_bindings = vp_description_bindings.into_iter().collect::<Vec<_>>();
        descriptor_bindings.extend_from_slice(&indirect_description_bindings);

        let descriptor_pool = vulkan.create_descriptor_pool(&build_pool_size(&descriptor_bindings), 2, true);
        let descriptor_sets = vulkan.allocate_descriptor_sets(descriptor_pool, &[vp_descriptor_layout, indirect_descriptor_layout]);
        let descriptor_layouts = vec![vp_descriptor_layout, indirect_descriptor_layout];

        let image_infos = gltf.textures.iter().map(|texture| {
            VkDescriptorImageInfo {
                sampler: samplers[texture.sampler as usize],
                imageView: *texture_images[texture.source as usize].image_view,
                imageLayout: VkImageLayout::SHADER_READ_ONLY_OPTIMAL,
            }
        }).collect::<Vec<_>>();

        let ubo_host_buffer = vulkan.create_buffer(MATRICES_SIZE as u64, BufferUsage::preset_staging()).unwrap();
        let ubo_host_info = AllocationTask::host_cached().add_allocatable(ubo_host_buffer).allocate_all(&vulkan);

        let ubo_device_buffer = vulkan.create_buffer(MATRICES_SIZE as u64, BufferUsage::default().uniform_buffer(true).transfer_dst(true)).unwrap();
        let ubo_device_info = AllocationTask::device().add_allocatable(ubo_device_buffer).allocate_all(&vulkan);

        vulkan.update_descriptor_sets(vec![
            ImageDescriptorInfo {
                target_descriptor: DescriptorSetInfo {
                    descriptor_set: descriptor_sets[1],
                    descriptor_binding: 0,
                    array_element: 0,
                },
                target_descriptor_type: VkDescriptorType::COMBINED_IMAGE_SAMPLER,
                image_infos,
            }], vec![
            BufferDescriptorInfo {
                target_descriptor: DescriptorSetInfo {
                    descriptor_set: descriptor_sets[0],
                    descriptor_binding: 0,
                    array_element: 0,
                },
                target_descriptor_type: VkDescriptorType::UNIFORM_BUFFER,
                buffer_infos: vec![VkDescriptorBufferInfo {
                    buffer: ubo_device_buffer,
                    offset: 0,
                    range: VK_WHOLE_SIZE,
                }],
            },
            // BufferDescriptorInfo {
            //     target_descriptor: DescriptorSetInfo {
            //         descriptor_set: descriptor_sets[1],
            //         descriptor_binding: 1,
            //         array_element: 0,
            //     },
            //     target_descriptor_type: VkDescriptorType::UNIFORM_BUFFER,
            //     buffer_infos: vec![VkDescriptorBufferInfo {
            //         buffer: ubo_device_buffer,
            //         offset: 0,
            //         range: VK_WHOLE_SIZE,
            //     }],
            // },
        ], vec![], vec![]);


        let ubo = UniformBuffer::new(
            Mat4::identity(),
            Mat4::identity(),
            ubo_host_buffer,
            ubo_host_info.pull_buffer_info(&ubo_host_buffer),
            Some(ubo_device_buffer),
            &vulkan
        );

        let parameter_count = parameters.len() as u32;
        let _samplers = samplers.into_iter().map(|sampler| {
            VkDestroy::new(sampler, &vulkan)
        }).collect::<Vec<_>>();

        let combined_info = AllocationInfo::merge_all(vec![texture_image_info, ubo_host_info, ubo_device_info, main_buffers_info]);
        let _memory = combined_info.get_all_memory_objects().into_iter().map(|memory| {
            VkDestroy::new(memory, &vulkan)
        }).collect::<Vec<_>>();
        let mut scene = Scene {
            ubo,
            vbo,
            idx: VkDestroy::new(idx_buffer, &vulkan),
            indirect_buffer: VkDestroy::new(indirect_buffer, &vulkan),
            idx_size,
            parameters_size,
            parameters,
            parameter_count,
            descriptor_sets,
            descriptor_layouts,
            indices,
            texture_images,
            _samplers,
            _memory,
        };
        scene.prepare(&vulkan);

        scene
    }

    pub fn prepare(&mut self, vulkan: &Vulkan) {
        let mut max_staging_size = self.idx_size + self.parameters_size;
        for image in &self.texture_images {
            max_staging_size += image.size as u64;
        }

        let staging_buffer = vulkan.create_buffer(max_staging_size, BufferUsage::preset_staging()).unwrap();
        let info = AllocationTask::host_cached().add_allocatable(staging_buffer).allocate_all(vulkan);
        let staging_info = info.pull_buffer_info(&staging_buffer);
        let staging_ptr = vulkan.map_memory(&staging_info);

        let command_pool = vulkan.create_command_pool(vulkan.get_loaded_device().queue_info[0].family_index, VkCommandPoolCreateFlags::empty());
        let one_time_command_buffer = vulkan.alloc_command_buffers(command_pool, VkCommandBufferLevel::PRIMARY, 1)[0];
        // prepare staging buffer
        let mut image_offsets: Vec<VkDeviceSize> = Vec::with_capacity(self.texture_images.len());
        let mut current_offset = 0u64;

        unsafe {
            // Copy indices
            Vulkan::copy_info(staging_ptr, self.indices.as_ptr(), self.indices.len());
            current_offset += self.idx_size;

            // Copy parameters
            Vulkan::copy_info(staging_ptr.add(current_offset as usize), self.parameters.as_ptr(), self.parameters.len());
            current_offset += self.parameters_size;

            // Copy images
            for image in &self.texture_images {
                Vulkan::copy_info(staging_ptr.add(current_offset as usize), image.data.as_ptr(), image.size);
                image_offsets.push(current_offset);
                current_offset += image.size as u64;
            }
        }
        vulkan.flush_memory(&[staging_info]);

        vulkan.start_recording(one_time_command_buffer, VkCommandBufferUsageFlags::ONE_TIME_SUBMIT_BIT, RecordingInfo {
            renderPass: Default::default(),
            subpass: 0,
            framebuffer: Default::default(),
            occlusionQueryEnable: false,
            queryFlags: Default::default(),
            pipelineStatistics: Default::default(),
        });

        self.vbo.sync_buffer(vulkan, one_time_command_buffer);

        vulkan.buffer_to_buffer(vec![VkBufferCopy {
            srcOffset: 0,
            dstOffset: 0,
            size: (self.indices.len() * size_of::<u16>()) as VkDeviceSize,
        }], one_time_command_buffer, staging_buffer, *self.idx.get());

        vulkan.buffer_to_buffer(vec![VkBufferCopy {
            srcOffset: self.idx_size,
            dstOffset: 0,
            size: self.parameters_size,
        }], one_time_command_buffer, staging_buffer, *self.indirect_buffer.get());

        let transitions = self.texture_images.iter().map(|image| {
            ImageTransition {
                image: *image.image.get(),
                current_access: VkAccessFlags::NONE,
                new_access: VkAccessFlags::NONE,
                current_layout: VkImageLayout::UNDEFINED,
                new_layout: VkImageLayout::TRANSFER_DST_OPTIMAL,
                current_queue_family: VK_QUEUE_FAMILY_IGNORED,
                new_queue_family: VK_QUEUE_FAMILY_IGNORED,
                aspect: VkImageAspectFlags::COLOR_BIT,
            }
        }).collect::<Vec<_>>();

        vulkan.transition_images(transitions, one_time_command_buffer, VkPipelineStageFlags::TOP_OF_PIPE_BIT, VkPipelineStageFlags::TRANSFER_BIT);

        let transitions = self.texture_images.iter().zip(image_offsets.iter()).map(|(image, &buffer_offset)| {
            vulkan.buffer_to_image(vec![
                VkBufferImageCopy {
                    bufferOffset: buffer_offset,
                    bufferRowLength: 0,
                    bufferImageHeight: 0,
                    imageSubresource: VkImageSubresourceLayers {
                        aspectMask: VkImageAspectFlags::COLOR_BIT,
                        mipLevel: 0,
                        baseArrayLayer: 0,
                        layerCount: 1,
                    },
                    imageOffset: Default::default(),
                    imageExtent: image.extent,
                }
            ], one_time_command_buffer, staging_buffer, *image.image.get(), VkImageLayout::TRANSFER_DST_OPTIMAL);

            ImageTransition {
                image: *image.image.get(),
                current_access: VkAccessFlags::NONE,
                new_access: VkAccessFlags::NONE,
                current_layout: VkImageLayout::TRANSFER_DST_OPTIMAL,
                new_layout: VkImageLayout::SHADER_READ_ONLY_OPTIMAL,
                current_queue_family: VK_QUEUE_FAMILY_IGNORED,
                new_queue_family: VK_QUEUE_FAMILY_IGNORED,
                aspect: VkImageAspectFlags::COLOR_BIT,
            }
        }).collect::<Vec<_>>();

        vulkan.transition_images(transitions, one_time_command_buffer, VkPipelineStageFlags::TOP_OF_PIPE_BIT, VkPipelineStageFlags::FRAGMENT_SHADER_BIT);

        vulkan.end_recording(one_time_command_buffer);
        vulkan.submit_buffer(vulkan.get_queues()[0], VkFence::none(), &[one_time_command_buffer], &[], &[]);
        vulkan.wait_for_queue(vulkan.get_queues()[0]);

        staging_buffer.destroy(vulkan);
        info.get_all_memory_objects().into_iter().for_each(|memory_object| { memory_object.destroy(vulkan); });

        vulkan.free_buffers(command_pool, &[one_time_command_buffer]);
        command_pool.destroy(vulkan);

        self.texture_images.iter_mut().for_each(|image| {
            image.data.clear()
        });
    }

    pub fn render_scene(&self, vulkan: &Vulkan, command_buffer: VkCommandBuffer, pipeline_layout: VkPipelineLayout) {
        self.vbo.bind(vulkan, command_buffer);
        
        vulkan.bind_index_buffer(command_buffer, *self.idx.get(), 0, VkIndexType::UINT16);
        vulkan.bind_descriptor_sets(command_buffer, VkPipelineBindPoint::GRAPHICS, pipeline_layout, 0, &self.descriptor_sets, &[]);
        
        unsafe { vkCmdDrawIndexedIndirect(command_buffer, *self.indirect_buffer.get(), 0, self.parameters.len() as u32, size_of::<IndirectParameters>() as u32) };
    }
}

const GLB_MAGIC: &[u8] = b"glTF";
fn check_magic(bytes: &[u8]) {
    if !bytes.starts_with(GLB_MAGIC) {
        panic!("Invalid GLTF magic, corrupt scene file");
    };
}

fn check_length(bytes: &[u8]) {
    if !u32::from_le_bytes(bytes[8..12].try_into().unwrap()) == bytes.len() as u32 {
        panic!("Invalid length, corrupt scene file");
    }
}

fn raw_to_chunks(mut bytes: &[u8]) -> (Chunk, Chunk) {
    let mut json_chunk: Option<Chunk> = None;
    let mut buffer_chunk: Option<Chunk> = None;
    loop {
        if bytes.is_empty() {
            break;
        }
        let chunk_length = u32::from_le_bytes(bytes[..4].try_into().unwrap());
        let last_byte = (chunk_length + 8) as usize;
        let chunk_type = ChunkType::try_from(u32::from_le_bytes(bytes[4..8].try_into().unwrap())).unwrap();
        let mut data: Vec<u8> = Vec::with_capacity(chunk_length as usize);
        data.extend_from_slice(&bytes[8..last_byte]);
        let chunk = Chunk {
            data,
        };

        match chunk_type {
            ChunkType::JSON => {
                json_chunk = Some(chunk);
            }
            ChunkType::BIN => {
                buffer_chunk = Some(chunk);
            }
        }

        bytes = &bytes[last_byte..];
    };
    (json_chunk.expect("Corrupted .gtb, no json section"), buffer_chunk.expect("Corrupted .gtb, no bin section"))
}

fn resolve_size(gltf: &Gltf, accessor_id: u32) -> u32 {
    gltf.bufferViews[gltf.accessors[accessor_id as usize].bufferView as usize].byteLength
}

fn resolve_offset(gltf: &Gltf, accessor_id: u32) -> u32 {
    gltf.bufferViews[gltf.accessors[accessor_id as usize].bufferView as usize].byteOffset
}

fn resolve_amount(gltf: &Gltf, accessor_id: u32) -> u32 {
    gltf.accessors[accessor_id as usize].count
}

fn resolve_vertices(gltf: &Gltf, attr: Attributes) -> u32 {
    let position_amount = resolve_amount(gltf, attr.POSITION);
    let normal_amount = resolve_amount(gltf, attr.NORMAL);

    if position_amount == normal_amount {
        position_amount
    } else {
        panic!("Corrupted json");
    }
}

const GL_NEAREST: u32 = 0x2600;
const GL_LINEAR: u32 = 0x2601;
const GL_NEAREST_MIPMAP_NEAREST: u32 = 0x2700;
const GL_LINEAR_MIPMAP_NEAREST: u32 = 0x2701;
const GL_NEAREST_MIPMAP_LINEAR: u32 = 0x2702;
const GL_LINEAR_MIPMAP_LINEAR: u32 = 0x2703;

fn resolve_opengl_magfilter(constant: u32, info: &mut SamplerInfo) {
    let mag_filter = match constant {
        GL_NEAREST => VkFilter::NEAREST,
        GL_LINEAR => VkFilter::LINEAR,
        _ => panic!("Unknown constants"),
    };
    info.mag_filter = mag_filter;
}

fn resolve_opengl_minfilter(constant: u32, info: &mut SamplerInfo) {
    let (min_filter, mipmap_mode) = match constant {
        GL_NEAREST | GL_NEAREST_MIPMAP_NEAREST => (VkFilter::NEAREST, VkSamplerMipmapMode::NEAREST),
        GL_LINEAR | GL_LINEAR_MIPMAP_NEAREST => (VkFilter::LINEAR, VkSamplerMipmapMode::NEAREST),
        GL_NEAREST_MIPMAP_LINEAR => (VkFilter::NEAREST, VkSamplerMipmapMode::LINEAR),
        GL_LINEAR_MIPMAP_LINEAR => (VkFilter::LINEAR, VkSamplerMipmapMode::LINEAR),
        _ => panic!("Unknown constants"),
    };
    info.min_filter = min_filter;
    info.mipmap_mode = mipmap_mode;
}
const GL_CLAMP_TO_EDGE: u32 = 0x812F;
const GL_MIRRORED_REPEAT: u32 = 0x8370;
const GL_REPEAT: u32 = 0x2901;
fn resolve_opengl_wrap(constant: u32) -> VkSamplerAddressMode {
    match constant {
        GL_CLAMP_TO_EDGE => VkSamplerAddressMode::CLAMP_TO_EDGE,
        GL_MIRRORED_REPEAT => VkSamplerAddressMode::MIRRORED_REPEAT,
        GL_REPEAT => VkSamplerAddressMode::REPEAT,
        _ => panic!("Unknown constants"),
    }
}


fn resolve_opengl_wraps(wrap_s: Option<u32>, wrap_t: Option<u32>, info: &mut SamplerInfo) {
    if let Some(wrap_s) = wrap_s {
        info.address_mode_u = resolve_opengl_wrap(wrap_s);
    }
    if let Some(wrap_t) = wrap_t {
        info.address_mode_v = resolve_opengl_wrap(wrap_t);
    }
    info.address_mode_w = VkSamplerAddressMode::REPEAT; // default for glTF
}

fn resolve_vertex(gltf: &Gltf, attr: Attributes, id: usize, data: &[u8], vbo: &mut VBO) {
    let pos_offset = resolve_offset(gltf, attr.POSITION) as usize;
    let norm_offset = resolve_offset(gltf, attr.NORMAL) as usize;
    let position = unsafe {
        let ptr = data.as_ptr().add(pos_offset + id * size_of::<[f32; 3]>()) as *const [f32; 3];
        *ptr
    };
    let normal = unsafe {
        let ptr = data.as_ptr().add(norm_offset + id * size_of::<[f32; 3]>()) as *const [f32; 3];
        *ptr
    };

    let texcoords = if let Some(tex_id) = attr.TEXCOORD_0 {
        unsafe {
            let texcoord_offset = resolve_offset(gltf, tex_id) as usize;
            let texcoord_ptr = data.as_ptr().add(texcoord_offset).add(id * size_of::<[f32; 2]>()) as *const [f32; 2];
            *texcoord_ptr
        }
    } else {
        [0.0f32, 0.0f32]
    };

    vbo.build_vertex_inplace(position, normal, texcoords);
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ImageFormat {
    Jpeg,
    Png,
    Bmp,
    Gif,
    Tiff,
    WebP,
    Unknown,
}

impl Into<VkFormat> for ImageFormat {
    fn into(self) -> VkFormat {
        match self {
            ImageFormat::Jpeg => VkFormat::R8G8B8_UNORM,
            ImageFormat::Png => VkFormat::R8G8B8A8_UNORM,
            ImageFormat::Bmp => VkFormat::R8G8B8A8_UNORM,
            ImageFormat::Gif => VkFormat::R8G8B8A8_UNORM,
            ImageFormat::Tiff => VkFormat::R8G8B8A8_UNORM,
            ImageFormat::WebP => VkFormat::R8G8B8A8_UNORM,
            ImageFormat::Unknown => VkFormat::R8G8B8A8_UNORM,
        }
    }
}
impl From<String> for ImageFormat {
    fn from(mime_type: String) -> Self {
        match mime_type.as_str() {
            "image/jpeg" => ImageFormat::Jpeg,
            "image/png" => ImageFormat::Png,
            "image/bmp" => ImageFormat::Bmp,
            "image/gif" => ImageFormat::Gif,
            "image/tiff" => ImageFormat::Tiff,
            "image/webp" => ImageFormat::WebP,
            _ => ImageFormat::Unknown,
        }
    }
}

const MATRICES_SIZE: usize = size_of::<Mat4>() * 2;
#[repr(C)]
#[derive(Default, Debug)]
pub struct UniformBuffer {
    view: Mat4,
    proj: Mat4,
    pointer: *mut c_void,
    host_buffer: VkDestroy<VkBuffer>,
    host_buffer_info: MemoryInfo,
    device_buffer: Option<VkDestroy<VkBuffer>>,
    dirty: bool,
}

impl UniformBuffer {
    pub fn new(view: Mat4, proj: Mat4, host_buffer: VkBuffer, host_buffer_info: MemoryInfo, device_buffer: Option<VkBuffer>, vulkan: &Vulkan) -> Self {
        let pointer = vulkan.map_memory(&host_buffer_info);
        let device_buffer = match device_buffer {
            Some(device_buffer) => Some(VkDestroy::new(device_buffer, vulkan)),
            None => None,
        };
        UniformBuffer {
            view,
            proj,
            pointer,
            host_buffer: VkDestroy::new(host_buffer, vulkan),
            host_buffer_info,
            device_buffer,
            dirty: true,
        }
    }

    pub fn view(&self) -> Mat4 {
        self.view
    }

    pub fn proj(&self) -> Mat4 {
        self.proj
    }

    pub fn set_proj(&mut self, proj: Mat4) {
        self.proj = proj;
        self.dirty = true;
    }

    pub fn set_view(&mut self, view: Mat4) {
        self.view = view;
        self.dirty = true;
    }

    pub fn sync_with_buffer(&mut self, command_buffer: VkCommandBuffer, vulkan: &Vulkan) {
        if self.dirty {
            Vulkan::copy_info(self.pointer, self as *const _ as *const u8, MATRICES_SIZE);

            if let Some(buffer) = &self.device_buffer {
                vulkan.buffer_to_buffer(vec![VkBufferCopy {
                    srcOffset: 0,
                    dstOffset: 0,
                    size: self.host_buffer_info.data_size,
                }], command_buffer, *self.host_buffer.get(), *buffer.get());
            }
            self.dirty = false;
        }
    }
}