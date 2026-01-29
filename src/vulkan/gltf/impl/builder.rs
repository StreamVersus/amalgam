use crate::engine::utils::obj_n_size::NSize;
use crate::engine::vbo::VBO;
use crate::vulkan::func::Vulkan;
use crate::vulkan::gltf::gltf_struct::{Attributes, Gltf};
use crate::vulkan::gltf::scene::{check_length, check_magic, raw_to_chunks, MaterialID, SIZE_TEXCOORDS};
use crate::vulkan::gltf::scene::{Image, Mesh, Node, Primitive, Scene};
use crate::vulkan::gltf::ubo::{UniformBuffer, MATRICES_SIZE};
use crate::vulkan::gltf::utils::{read_samplers, resolve_amount, resolve_mesh, resolve_offset, resolve_size, resolve_vertex, resolve_vertices, ImageFormat, IndirectParameters, StagingBuffer};
use crate::vulkan::r#impl::descriptors::{BufferDescriptorInfo, DescriptorSetInfo, ImageDescriptorInfo, PooledDescriptors};
use crate::vulkan::r#impl::memory::{AllocationInfo, AllocationTask, VkDestroy};
use crate::vulkan::utils::{build_pool_size, BufferUsage, ImageUsage};
use fragment::{SAMPLER_LIMIT, TEXTURE_LIMIT};
use png::Decoder;
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

        let mut vbo_size: u64 = 0;
        let mut idx_size: u64 = 0;
        let mut attr_set: HashSet<Attributes> = HashSet::new();
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
                idx_size += resolve_size(&gltf, primitive.indices) as u64;
            });
        });

        let idx_buffer = vulkan.create_buffer(idx_size, BufferUsage::preset_index()).unwrap();

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

                let material = if let Some(material_id) = primitive.material {
                    let base_index = gltf.materials
                        .get(material_id as usize)
                        .and_then(|mat| mat.pbrMetallicRoughness.as_ref())
                        .and_then(|pbr| pbr.baseColorTexture.as_ref())
                        .map(|tex| tex.index)
                        .unwrap_or(0) as usize;

                    let info = &gltf.textures[base_index];
                    MaterialID {
                        source_id: info.source,
                        sampler_id: info.sampler,
                    }
                } else {
                    MaterialID {
                        source_id: 0,
                        sampler_id: 0,
                    }
                };
                primitives.push(Primitive {
                    indices: resolve_amount(&gltf, primitive.indices),
                    vertices: vertex_amount as u32,
                    material,
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
            let meshes = resolve_mesh(&gltf, node).iter()
                .map(|id| meshes.get(id).expect("Tried to get nonexistent mesh").clone())
                .collect::<Vec<Mesh>>();
            nodes.push(Node {
                meshes,
                pos: Vec3::from(node.translation.unwrap_or([0.0, 0.0, 0.0])),
                rot: Rotor3::from_quaternion_array(node.rotation.unwrap_or([0.0, 0.0, 0.0, 1.0])),
                scale: Vec3::from(node.scale.unwrap_or([1.0, 1.0, 1.0])),
            });
        });

        let mut parameters: Vec<IndirectParameters> = Vec::with_capacity(gltf.meshes.len());

        // Build data structures
        let mut model_matrices: Vec<Mat4> = Vec::with_capacity(gltf.meshes.len());

        let mut index_offset = 0;
        let mut vertex_offset = 0;
        let mut instance_offset = 0;

        let mut material_ranges = Vec::with_capacity(gltf.meshes.len());
        nodes.iter().for_each(|node| {
            let meshes = &node.meshes;
            meshes.iter().for_each(|mesh| {
                mesh.primitives.iter().for_each(|primitive| {
                    let model_matrix = Mat4::from_translation(node.pos) * mat3_to_mat4(node.rot.into_matrix()) * Mat4::from_nonuniform_scale(node.scale);
                    model_matrices.push(model_matrix);

                    parameters.push(IndirectParameters {
                        index_count: primitive.indices,
                        instance_count: nodes.len() as u32,
                        first_index: index_offset,
                        vertex_offset,
                        first_instance: instance_offset,
                    });

                    index_offset += primitive.indices;
                    vertex_offset += primitive.vertices as i32;
                    instance_offset += nodes.len() as u32;

                    //resolve material
                    let material = primitive.material;
                    material_ranges.push(material);
                });
            });
        });
        let parameters = NSize::from(parameters);
        let indirect_buffer = vulkan.create_buffer(parameters.size() as u64, BufferUsage::default().transfer_dst(true).indirect_buffer(true)).unwrap();

        // Create SSBOs
        let model_matrices_size = (model_matrices.len() * size_of::<Mat4>()) as u64;
        let material_ranges_size = (material_ranges.len() * size_of::<MaterialID>()) as u64;

        let model_ssbo = vulkan.create_buffer(model_matrices_size, BufferUsage::default().storage_buffer(true).transfer_dst(true)).unwrap();
        let material_ranges_ssbo = vulkan.create_buffer(material_ranges_size, BufferUsage::default().storage_buffer(true).transfer_dst(true)).unwrap();

        let main_buffers_info = AllocationTask::device()
            .add_allocatable(idx_buffer)
            .add_allocatable(indirect_buffer)
            .add_allocatable(model_ssbo)
            .add_allocatable(material_ranges_ssbo)
            .allocate_all(&vulkan);

        let samplers: Vec<VkSampler> = read_samplers(&vulkan, &gltf);

        let mut device_task = AllocationTask::device();
        let texture_images = gltf.images.iter().map(|image_info| {
            let format = ImageFormat::from(image_info.mimeType.clone()).into();
            let byte_blob = {
                let buffer_view = &gltf.bufferViews[image_info.bufferView as usize];
                let offset = buffer_view.byteOffset.unwrap_or(0) as usize;
                let size = buffer_view.byteLength as usize;
                &bin_chunk.data[offset..offset + size]
            };
            let img = Decoder::new(Cursor::new(byte_blob));
            let mut reader = img.read_info().unwrap();

            let mut buf = vec![0; reader.output_buffer_size().unwrap()];
            let info = reader.next_frame(&mut buf).unwrap();

            let rgba = match info.color_type {
                png::ColorType::Rgba => buf,
                png::ColorType::Rgb => {
                    buf.chunks(3)
                        .flat_map(|rgb| [rgb[0], rgb[1], rgb[2], 255])
                        .collect()
                }
                _ => {
                    eprintln!("Trying to load {:?} texture as rgba, consider rechecking textures", info.color_type);
                    vec![]
                },
            };
            let resolution = VkExtent3D {
                width: info.width,
                height: info.height,
                depth: 1,
            };
            let image = vulkan.create_image(format, VkImageType::IT_2D, false, 1, 1, resolution, VkSampleCountFlagBits::SC_1_BIT, ImageUsage::default().sampled(true).transfer_dst(true));
            device_task.add_allocatable_ref(image);

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
                descriptorCount: 2048,
                stageFlags: VkShaderStageFlags::FRAGMENT_BIT,
                pImmutableSamplers: null_mut(),
            },
            VkDescriptorSetLayoutBinding {
                binding: 1,
                descriptorType: VkDescriptorType::SAMPLER,
                descriptorCount: 16,
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
                stageFlags: VkShaderStageFlags::VERTEX_BIT | VkShaderStageFlags::FRAGMENT_BIT,
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

        let descriptors = PooledDescriptors::new(vec![vp_descriptor_layout, indirect_descriptor_layout], build_pool_size(&descriptor_bindings), &vulkan);
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

        vulkan.update_descriptor_sets(vec![
            ImageDescriptorInfo {
                target_descriptor: DescriptorSetInfo {
                    descriptor_set: descriptors.descriptor_sets[1],
                    descriptor_binding: 0,
                    array_element: 0,
                },
                target_descriptor_type: VkDescriptorType::SAMPLED_IMAGE,
                image_infos,
            },
            ImageDescriptorInfo {
                target_descriptor: DescriptorSetInfo {
                    descriptor_set: descriptors.descriptor_sets[1],
                    descriptor_binding: 1,
                    array_element: 0,
                },
                target_descriptor_type: VkDescriptorType::SAMPLER,
                image_infos: sampler_infos,
            },
        ], vec![
            BufferDescriptorInfo {
                target_descriptor: DescriptorSetInfo {
                    descriptor_set: descriptors.descriptor_sets[0],
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
            // Model matrices SSBO
            BufferDescriptorInfo {
                target_descriptor: DescriptorSetInfo {
                    descriptor_set: descriptors.descriptor_sets[1],
                    descriptor_binding: 3,
                    array_element: 0,
                },
                target_descriptor_type: VkDescriptorType::STORAGE_BUFFER,
                buffer_infos: vec![VkDescriptorBufferInfo {
                    buffer: model_ssbo,
                    offset: 0,
                    range: VK_WHOLE_SIZE,
                }],
            },
            // MaterialID ranges SSBO (for texture/sampler lookup)
            BufferDescriptorInfo {
                target_descriptor: DescriptorSetInfo {
                    descriptor_set: descriptors.descriptor_sets[1],
                    descriptor_binding: 4,
                    array_element: 0,
                },
                target_descriptor_type: VkDescriptorType::STORAGE_BUFFER,
                buffer_infos: vec![VkDescriptorBufferInfo {
                    buffer: material_ranges_ssbo,
                    offset: 0,
                    range: VK_WHOLE_SIZE,
                }],
            },
        ], vec![], vec![]);

        let ubo = UniformBuffer::new(
            Mat4::identity(),
            Mat4::identity(),
            ubo_host_buffer,
            ubo_host_info.pull_buffer_info(&ubo_host_buffer),
            Some(ubo_device_buffer),
            &vulkan,
        );

        let _samplers = samplers.into_iter().map(|sampler| {
            VkDestroy::new(sampler, &vulkan)
        }).collect::<Vec<_>>();

        let combined_info = AllocationInfo::merge_all(vec![texture_image_info, ubo_host_info, ubo_device_info, main_buffers_info]);
        let _memory = combined_info.get_all_memory_objects().into_iter().map(|memory| {
            VkDestroy::new(memory, &vulkan)
        }).collect::<Vec<_>>();

        //Wrappers
        let idx = NSize::new(VkDestroy::new(idx_buffer, &vulkan), idx_size as usize);
        let indirect_buffer = NSize::new(VkDestroy::new(indirect_buffer, &vulkan), parameters.size());
        let model_ssbo = NSize::new(VkDestroy::new(model_ssbo, &vulkan), model_matrices_size as usize);
        let material_ssbo = NSize::new(VkDestroy::new(material_ranges_ssbo, &vulkan), material_ranges_size as usize);

        let mut scene = Scene {
            ubo,
            vbo,
            idx,
            indirect_buffer,
            model_ssbo,
            material_ssbo,
            parameters,
            descriptors,
            indices,
            model_matrices,
            texture_images,
            material_ranges,
            _samplers,
            _memory,
        };
        scene.prepare(&vulkan, staging);

        scene
    }
}

fn mat3_to_mat4(m: Mat3) -> Mat4 {
    Mat4::new(
        Vec4::from(m.cols[0]),
        Vec4::from(m.cols[1]),
        Vec4::from(m.cols[2]),
        Vec4::new(0.0, 0.0, 0.0, 1.0), // Translation/homogeneous row
    )
}