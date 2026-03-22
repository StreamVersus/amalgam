use std::collections::HashMap;
use std::ffi::c_void;
use crate::prelude::*;
use crate::vulkan::features::gpu_memory::Allocatable;
use crate::vulkan::func::Vulkan;
use crate::vulkan::utils::align_up;

#[derive(Default, Debug, Clone)]
pub struct ArenaAllocator {}

impl<S: AllocationStorage> Allocator<S, ArenaAllocationInfo, ArenaMemoryInfo, VkDeviceMemory> for ArenaAllocator {
    fn allocate(&self, flags: VkMemoryPropertyFlags, storage: S, vulkan: &Vulkan) -> ArenaAllocationInfo {
        let mut info = ArenaAllocationInfo::default();
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
            let (offset, bind_memory, size) = if *dedicated {
                (0, vulkan.allocate_dedicated_memory(*buffer, req.clone(), flags), req.size)
            } else {
                let size = align_up(req.size, atom_size);
                current_offset = align_up(current_offset, req.alignment.max(atom_size));
                let off = current_offset;
                current_offset += size;
                (off, memory, size)
            };
            let bind_task = buffer.build_bind_task(bind_memory, offset);
            buffer_bind_tasks.push(bind_task);
            info.store_buffer_info(size, bind_task);
        }

        for (image, req, dedicated) in &images {
            let (offset, bind_memory, size) = if *dedicated {
                (0, vulkan.allocate_dedicated_memory(*image, req.clone(), flags), req.size)
            } else {
                let size = align_up(req.size, atom_size);
                current_offset = align_up(current_offset, req.alignment.max(atom_size));
                let off = current_offset;
                current_offset += size;
                (off, memory, size)
            };
            let bind_task = image.build_bind_task(bind_memory, offset);
            image_bind_tasks.push(bind_task);
            info.store_image_info(size, bind_task);
        }

        vulkan.bind_memory_to_buffer(buffer_bind_tasks);
        vulkan.bind_memory_to_image(image_bind_tasks);
        info
    }
}

#[derive(Default)]
pub struct ArenaAllocationInfo {
    buffer_info: HashMap<VkBuffer, ArenaMemoryInfo>,
    image_info: HashMap<VkImage, ArenaMemoryInfo>,
}

impl AllocationInfo<ArenaMemoryInfo, VkDeviceMemory> for ArenaAllocationInfo {
    fn store_buffer_info(&mut self, buffer: VkBuffer, info: ArenaMemoryInfo) {
        self.buffer_info.insert(buffer, info);
    }

    fn store_image_info(&mut self, image: VkImage, info: ArenaMemoryInfo) {
        self.image_info.insert(image, info);
    }

    fn merge_mut(&mut self, allocation_info: Self) {
        self.buffer_info.extend(allocation_info.buffer_info);
        self.image_info.extend(allocation_info.image_info);
    }

    fn pull_buffer_info(&self, buffer: &VkBuffer) -> &ArenaMemoryInfo {
        self.buffer_info.get(buffer).unwrap()
    }

    fn pull_image_info(&self, image: &VkImage) -> &ArenaMemoryInfo {
        self.image_info.get(image).unwrap()
    }

    fn get_all_memory_objects(&self) -> Vec<VkDeviceMemory> {
        let mut memory_objects = Vec::with_capacity(self.buffer_info.len() + self.image_info.len());

        self.buffer_info.iter().for_each(|(_, info)| {
            memory_objects.push(info.memory_object);
        });
        self.image_info.iter().for_each(|(_, info)| {
            memory_objects.push(info.memory_object);
        });

        memory_objects
    }

    fn get_all_info(&self) -> Vec<ArenaMemoryInfo> {
        let mut images: Vec<ArenaMemoryInfo> = self.image_info.values().cloned().collect();
        let buffers: Vec<ArenaMemoryInfo> = self.buffer_info.values().cloned().collect();
        images.extend(buffers);

        images
    }
}

impl ArenaAllocationInfo {
    fn store_buffer_info(&mut self, size: u64, info: VkBindBufferMemoryInfo) {
        self.buffer_info.insert(info.buffer, ArenaMemoryInfo {
            memory_object: info.memory,
            offset: info.memoryOffset,
            data_size: size,
        });
    }

    fn store_image_info(&mut self, size: u64, info: VkBindImageMemoryInfo) {
        self.image_info.insert(info.image, ArenaMemoryInfo {
            memory_object: info.memory,
            offset: info.memoryOffset,
            data_size: size,
        });
    }
}

#[derive(Debug, Default, Copy, Clone)]
pub struct ArenaMemoryInfo {
    pub memory_object: VkDeviceMemory,
    pub offset: u64,
    pub data_size: u64,
}

impl MemoryInfo<VkDeviceMemory> for ArenaMemoryInfo {
    fn memory_object(&self) -> VkDeviceMemory {
        self.memory_object
    }
    fn data_size(&self) -> u64 {
        self.data_size
    }
    fn map_memory(&self, vulkan: &Vulkan) -> *mut c_void {
        vulkan.map_memory(self.memory_object, self.offset, self.data_size)
    }
    fn unmap_memory(&self, vulkan: &Vulkan) {
        vulkan.unmap_memory(self.memory_object)
    }

    fn flush_memory(&self, vulkan: &Vulkan) -> bool{
        vulkan.flush_memory(&[(self.memory_object, self.offset, self.data_size)])
    }
}