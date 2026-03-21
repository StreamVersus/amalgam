use crate::prelude::*;
use crate::vulkan::features::gpu_memory::Allocatable;
use crate::vulkan::func::Vulkan;
use crate::vulkan::utils::align_up;

#[derive(Default)]
#[derive(Debug)]
#[derive(Clone)]
pub struct ArenaAllocator {}

impl<S: AllocationStorage> Allocator<S> for ArenaAllocator {
    fn allocate(&self, flags: VkMemoryPropertyFlags, storage: S, vulkan: &Vulkan) -> AllocationInfo {
        let mut info = AllocationInfo::default();
        if storage.is_empty() {
            return info;
        }
        let atom_size = vulkan.get_loaded_device().device_info.properties.limits.nonCoherentAtomSize;

        let alloc_reqs = storage.build_requirements(vulkan, atom_size);
        let mut buffer_bind_tasks: Vec<VkBindBufferMemoryInfo> = Vec::with_capacity(alloc_reqs.buffer_requirements.len());
        let mut image_bind_tasks: Vec<VkBindImageMemoryInfo> = Vec::with_capacity(alloc_reqs.image_requirements.len());

        let total_size = alloc_reqs.total_size;
        let all_reqs = alloc_reqs.merge();
        let memory_type_index = vulkan.find_memory_type(
            &all_reqs,
            flags,
        ).expect("Failed to find suitable memory type");

        let memory = vulkan.allocate_memory_object(total_size, memory_type_index);
        
        let mut current_offset = 0u64;

        let buffer_iter = storage.get_buffers().into_iter();
        let buffer_requirements_iter = alloc_reqs.buffer_requirements.iter();
        for (buffer, (req, dedicated)) in buffer_iter.zip(buffer_requirements_iter) {
            let (offset, bind_memory) = if *dedicated {
                (0, vulkan.allocate_dedicated_memory(*buffer, req.clone(), flags))
            } else {
                (align_up(current_offset, req.alignment.max(atom_size)), memory)
            };
            buffer.add_bind_task(&mut buffer_bind_tasks, &mut image_bind_tasks, &mut info, bind_memory, offset, align_up(req.size, atom_size));
            current_offset += align_up(req.size, atom_size);
        }

        let image_iter = storage.get_images().into_iter();
        let image_requirements_iter = alloc_reqs.image_requirements.iter();
        for (image, (req, dedicated)) in image_iter.zip(image_requirements_iter) {
            let (offset, bind_memory) = if *dedicated {
                (0, vulkan.allocate_dedicated_memory(*image, req.clone(), flags))
            } else {
                (align_up(current_offset, req.alignment.max(atom_size)), memory)
            };
            image.add_bind_task(&mut buffer_bind_tasks, &mut image_bind_tasks, &mut info, bind_memory, offset, align_up(req.size, atom_size));
            current_offset += align_up(req.size, atom_size);
        }

        vulkan.bind_memory_to_buffer(buffer_bind_tasks);
        vulkan.bind_memory_to_image(image_bind_tasks);

        info
    }
}
