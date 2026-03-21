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
        if storage.is_empty() { return info; }

        let atom_size = vulkan.get_loaded_device().device_info.properties.limits.nonCoherentAtomSize;
        let alloc_reqs = storage.build_requirements(vulkan, atom_size);

        let mut buffers: Vec<(VkBuffer, VkMemoryRequirements, bool)> =
            storage.get_buffers().into_iter()
                .zip(alloc_reqs.buffer_requirements)
                .map(|(&b, (req, ded))| (b, req, ded))
                .collect();

        let mut images: Vec<(VkImage, VkMemoryRequirements, bool)> =
            storage.get_images().into_iter()
                .zip(alloc_reqs.image_requirements)
                .map(|(&i, (req, ded))| (i, req, ded))
                .collect();

        buffers.sort_unstable_by_key(|(_, req, _)| std::cmp::Reverse(req.alignment));
        images.sort_unstable_by_key(|(_, req, _)| std::cmp::Reverse(req.alignment));

        let total_size = {
            let buf_size = buffers.iter()
                .filter(|(_, _, ded)| !ded)
                .fold(0u64, |acc, (_, req, _)| {
                    align_up(acc, req.alignment.max(atom_size)) + align_up(req.size, atom_size)
                });
            let img_size = images.iter()
                .filter(|(_, _, ded)| !ded)
                .fold(buf_size, |acc, (_, req, _)| {
                    align_up(acc, req.alignment.max(atom_size)) + align_up(req.size, atom_size)
                });
            img_size
        };

        let all_reqs: Vec<VkMemoryRequirements> = buffers.iter()
            .map(|(_, req, _)| req.clone())
            .chain(images.iter().map(|(_, req, _)| req.clone()))
            .collect();

        let memory_type_index = vulkan.find_memory_type(&all_reqs, flags)
            .expect("Failed to find suitable memory type");
        let memory = vulkan.allocate_memory_object(total_size, memory_type_index);

        let mut buffer_bind_tasks: Vec<VkBindBufferMemoryInfo> = Vec::with_capacity(buffers.len());
        let mut image_bind_tasks:  Vec<VkBindImageMemoryInfo>  = Vec::with_capacity(images.len());
        let mut current_offset = 0u64;

        for (buffer, req, dedicated) in &buffers {
            let (offset, bind_memory) = if *dedicated {
                (0, vulkan.allocate_dedicated_memory(*buffer, req.clone(), flags))
            } else {
                current_offset = align_up(current_offset, req.alignment.max(atom_size));
                let off = current_offset;
                current_offset += align_up(req.size, atom_size);
                (off, memory)
            };
            buffer.add_bind_task(&mut buffer_bind_tasks, &mut image_bind_tasks, &mut info, bind_memory, offset, align_up(req.size, atom_size));
        }

        for (image, req, dedicated) in &images {
            let (offset, bind_memory) = if *dedicated {
                (0, vulkan.allocate_dedicated_memory(*image, req.clone(), flags))
            } else {
                current_offset = align_up(current_offset, req.alignment.max(atom_size));
                let off = current_offset;
                current_offset += align_up(req.size, atom_size);
                (off, memory)
            };
            image.add_bind_task(&mut buffer_bind_tasks, &mut image_bind_tasks, &mut info, bind_memory, offset, align_up(req.size, atom_size));
        }

        vulkan.bind_memory_to_buffer(buffer_bind_tasks);
        vulkan.bind_memory_to_image(image_bind_tasks);
        info
    }
}
