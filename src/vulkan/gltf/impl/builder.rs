use crate::engine::vbo::VBO;
use crate::vulkan::func::Vulkan;
use crate::vulkan::gltf::gltf_struct;
use crate::vulkan::gltf::gltf_struct::{Attributes, Gltf};
use crate::vulkan::gltf::scene::{check_length, check_magic, raw_to_chunks, SIZE_TEXCOORDS};
use crate::vulkan::gltf::scene::{Image, Mesh, Node, Primitive, Scene};
use crate::vulkan::gltf::ubo::{UniformBuffer, MATRICES_SIZE};
use crate::vulkan::gltf::utils::{read_samplers, resolve_amount, resolve_offset, resolve_size, resolve_vertex, resolve_vertices, ImageFormat, IndirectParameters, StagingBuffer};
use crate::vulkan::r#impl::descriptors::{BufferDescriptorInfo, DescriptorSetInfo, ImageDescriptorInfo};
use crate::vulkan::r#impl::memory::{AllocationInfo, AllocationTask, VkDestroy};
use crate::vulkan::utils::{build_pool_size, BufferUsage, ImageUsage};
use image::ImageReader;
use shaders::{SAMPLER_LIMIT, TEXTURE_LIMIT};
use std::collections::{HashMap, HashSet};
use std::io::Cursor;
use std::ptr::null_mut;
use ultraviolet::{Mat3, Mat4, Rotor3, Vec3, Vec4};
use vulkan_raw::{VkDescriptorBufferInfo, VkDescriptorImageInfo, VkDescriptorSetLayoutBinding, VkDescriptorType, VkExtent3D, VkImageAspectFlags, VkImageLayout, VkImageType, VkImageView, VkImageViewType, VkSampleCountFlagBits, VkSampler, VkShaderStageFlags, VK_WHOLE_SIZE};

impl Scene {
    pub fn from_glb(bytes: &[u8], vulkan: Vulkan, staging: &mut StagingBuffer) -> Scene {
        check_magic(bytes);
        check_length(bytes);

        let bytes = &bytes[12..]; // data without headers
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

        let mut mesh_renders: HashMap<u32, Vec<&Node>> = HashMap::with_capacity(gltf.meshes.len());
        nodes.iter().for_each(|node| {
            let vec = if let Some(vec) = mesh_renders.get_mut(&node.mesh.id) {
                vec
            } else {
                let new_vec = vec![];
                mesh_renders.insert(node.mesh.id, new_vec);
                mesh_renders.get_mut(&node.mesh.id).unwrap()
            };
            vec.push(node);
        });
        let mut parameters: Vec<IndirectParameters> = Vec::with_capacity(gltf.meshes.len());

        let mut index_offset = 0;
        let mut vertex_offset = 0;
        let mut instance_offset = 0;
        let mut instance_resolve: HashMap<u32, &gltf_struct::Primitive> = HashMap::with_capacity(gltf.meshes.len());
        for i in 0..gltf.meshes.len() as u32 {
            let mesh = meshes.get(&i).unwrap();
            let nodes = mesh_renders.remove(&i).unwrap();

            let json_mesh = &gltf.meshes[i as usize];
            mesh.primitives.iter().enumerate().for_each(|(i, primitive)| {
                parameters.push(IndirectParameters {
                    index_count: primitive.indices,
                    instance_count: nodes.len() as u32,
                    first_index: index_offset,
                    vertex_offset,
                    first_instance: instance_offset,
                });

                index_offset += primitive.indices;
                vertex_offset += primitive.vertices;
                for _ in 0..nodes.len() as u32 {
                    instance_resolve.insert(instance_offset, &json_mesh.primitives[i]);
                    instance_offset += 1;
                }
            });
        }

        let samplers: Vec<VkSampler> = read_samplers(&vulkan, &gltf);

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

        assert!(gltf.textures.len() <= TEXTURE_LIMIT);
        assert!(samplers.len() <= SAMPLER_LIMIT);

        let indirect_description_bindings = [
            VkDescriptorSetLayoutBinding {
                binding: 0,
                descriptorType: VkDescriptorType::SAMPLED_IMAGE,
                descriptorCount: gltf.textures.len() as u32,
                stageFlags: VkShaderStageFlags::FRAGMENT_BIT,
                pImmutableSamplers: null_mut(),
            },
            VkDescriptorSetLayoutBinding {
                binding: 1,
                descriptorType: VkDescriptorType::SAMPLER,
                descriptorCount: samplers.len() as u32,
                stageFlags: VkShaderStageFlags::FRAGMENT_BIT,
                pImmutableSamplers: null_mut(),
            },
            VkDescriptorSetLayoutBinding {
                binding: 3,
                descriptorType: VkDescriptorType::STORAGE_BUFFER,
                descriptorCount: 1,
                stageFlags: VkShaderStageFlags::VERTEX_BIT,
                pImmutableSamplers: null_mut(),
            },
            VkDescriptorSetLayoutBinding {
                binding: 4,
                descriptorType: VkDescriptorType::STORAGE_BUFFER,
                descriptorCount: 1,
                stageFlags: VkShaderStageFlags::VERTEX_BIT,
                pImmutableSamplers: null_mut(),
            },
            VkDescriptorSetLayoutBinding {
                binding: 5,
                descriptorType: VkDescriptorType::STORAGE_BUFFER,
                descriptorCount: 1,
                stageFlags: VkShaderStageFlags::VERTEX_BIT,
                pImmutableSamplers: null_mut(),
            },
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
                sampler: VkSampler::none(),
                imageView: *texture_images[texture.source as usize].image_view,
                imageLayout: VkImageLayout::SHADER_READ_ONLY_OPTIMAL,
            }
        }).collect::<Vec<_>>();

        let sampler_infos: Vec<_> = samplers.iter().map(|&sampler| {
            VkDescriptorImageInfo {
                sampler,
                imageView: VkImageView::none(),
                imageLayout: VkImageLayout::UNDEFINED,
            }
        }).collect();

        let ubo_host_buffer = vulkan.create_buffer(MATRICES_SIZE as u64, BufferUsage::preset_staging()).unwrap();
        let ubo_host_info = AllocationTask::host_cached().add_allocatable(ubo_host_buffer).allocate_all(&vulkan);

        let ubo_device_buffer = vulkan.create_buffer(MATRICES_SIZE as u64, BufferUsage::default().uniform_buffer(true).transfer_dst(true)).unwrap();
        let ubo_device_info = AllocationTask::device().add_allocatable(ubo_device_buffer).allocate_all(&vulkan);

        //let model_ssbo =
        vulkan.update_descriptor_sets(vec![
            ImageDescriptorInfo {
                target_descriptor: DescriptorSetInfo {
                    descriptor_set: descriptor_sets[1],
                    descriptor_binding: 0,
                    array_element: 0,
                },
                target_descriptor_type: VkDescriptorType::SAMPLED_IMAGE,
                image_infos,
            },
            ImageDescriptorInfo {
                target_descriptor: DescriptorSetInfo {
                    descriptor_set: descriptor_sets[1],
                    descriptor_binding: 1,
                    array_element: 0,
                },
                target_descriptor_type: VkDescriptorType::SAMPLER,
                image_infos: sampler_infos,
            },
            ], vec![
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
            // // model matrices
            // BufferDescriptorInfo {
            //     target_descriptor: DescriptorSetInfo {
            //         descriptor_set: descriptor_sets[1],
            //         descriptor_binding: 3,
            //         array_element: 0,
            //     },
            //     target_descriptor_type: VkDescriptorType::STORAGE_BUFFER,
            //     buffer_infos: vec![VkDescriptorBufferInfo {
            //         buffer: ubo_device_buffer,
            //         offset: 0,
            //         range: VK_WHOLE_SIZE,
            //     }],
            // },
            // // model ranges
            // BufferDescriptorInfo {
            //     target_descriptor: DescriptorSetInfo {
            //         descriptor_set: descriptor_sets[1],
            //         descriptor_binding: 4,
            //         array_element: 0,
            //     },
            //     target_descriptor_type: VkDescriptorType::STORAGE_BUFFER,
            //     buffer_infos: vec![VkDescriptorBufferInfo {
            //         buffer: ubo_device_buffer,
            //         offset: 0,
            //         range: VK_WHOLE_SIZE,
            //     }],
            // },
            // // texture ranges
            // BufferDescriptorInfo {
            //     target_descriptor: DescriptorSetInfo {
            //         descriptor_set: descriptor_sets[1],
            //         descriptor_binding: 5,
            //         array_element: 0,
            //     },
            //     target_descriptor_type: VkDescriptorType::STORAGE_BUFFER,
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
            &vulkan,
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
        scene.prepare(&vulkan, staging);

        scene
    }
}

fn build_model_matrix(
    translation: Option<[f32; 3]>,
    rotation: Option<[f32; 4]>,
    scale: Option<[f32; 3]>,
) -> Mat4 {
    let t = translation.map(Vec3::from).unwrap_or(Vec3::zero());
    let r = rotation.map(|q| Rotor3::from_quaternion_array(q)).unwrap_or(Rotor3::identity());
    let s = scale.map(Vec3::from).unwrap_or(Vec3::one());

    let rot_mat3: Mat3 = r.into_matrix();
    let rot_mat4 = mat3_to_mat4(rot_mat3);

    Mat4::from_translation(t)
        * rot_mat4
        * Mat4::from_nonuniform_scale(s)
}

fn mat3_to_mat4(m: Mat3) -> Mat4 {
    Mat4::new(
        Vec4::from(m.cols[0]),
        Vec4::from(m.cols[1]),
        Vec4::from(m.cols[2]),
        Vec4::new(0.0, 0.0, 0.0, 1.0), // Translation/homogeneous row
    )
}