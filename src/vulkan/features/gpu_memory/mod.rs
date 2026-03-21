use crate::prelude::MemoryInfo;
use crate::vulkan::func::{Destructible, Vulkan};
use std::collections::HashMap;
use std::ops::Deref;
use vulkan_raw::{VkBindBufferMemoryInfo, VkBindImageMemoryInfo, VkBuffer, VkDeviceMemory, VkImage, VkMemoryDedicatedAllocateInfo, VkMemoryPropertyFlags, VkMemoryRequirements};
use crate::vulkan::utils::align_up;

// pub mod pool_alloc;
pub mod arena_alloc;

pub trait Allocator<S: AllocationStorage> {
    fn allocate(&self, flags: VkMemoryPropertyFlags, storage: S, vulkan: &Vulkan) -> AllocationInfo;
    fn device(&self, storage: S, vulkan: &Vulkan) -> AllocationInfo {
        self.allocate(VkMemoryPropertyFlags::DEVICE_LOCAL_BIT, storage, vulkan)
    }
    fn host_cached(&self, storage: S, vulkan: &Vulkan) -> AllocationInfo {
        self.allocate(VkMemoryPropertyFlags::HOST_VISIBLE_BIT | VkMemoryPropertyFlags::HOST_CACHED_BIT, storage, vulkan)
    }
    fn host_coherent(&self, storage: S, vulkan: &Vulkan) -> AllocationInfo {
        self.allocate(VkMemoryPropertyFlags::HOST_VISIBLE_BIT | VkMemoryPropertyFlags::HOST_COHERENT_BIT, storage, vulkan)
    }
}

pub struct AllocationRequirements {
    pub image_requirements: Vec<(VkMemoryRequirements, bool)>,
    pub buffer_requirements: Vec<(VkMemoryRequirements, bool)>,
    pub total_size: u64,
}

impl AllocationRequirements {
    pub fn merge(&self) -> Vec<&VkMemoryRequirements> {
        self.image_requirements.iter()
            .chain(&self.buffer_requirements)
            .map(|(req, _)| req)
            .collect()
    }
}

pub trait AllocationStorage {
    fn get_images(&self) -> &[VkImage];
    fn get_buffers(&self) -> &[VkBuffer];
    fn push_into(&mut self, buffs: &mut Vec<VkBuffer>, imgs: &mut Vec<VkImage>) {
        let buffers = self.get_buffers();
        buffs.reserve(buffers.len());
        buffs.extend_from_slice(buffers);

        let images = self.get_images();
        imgs.reserve(images.len());
        imgs.extend_from_slice(images);
    }

    fn is_empty(&self) -> bool {
        self.get_images().is_empty() && self.get_buffers().is_empty()
    }

    fn build_requirements(&self, vulkan: &Vulkan, atom_size: u64) -> AllocationRequirements {
        let image_requirements = self.get_images()
            .iter()
            .map(|img| img.get_memory_requirements(vulkan))
            .collect::<Vec<_>>();

        let buffer_requirements = self.get_buffers()
            .iter()
            .map(|buf| buf.get_memory_requirements(vulkan))
            .collect::<Vec<_>>();

        let total_size = image_requirements.iter()
            .chain(buffer_requirements.iter())
            .fold(0u64, |acc, (req, dedicated)| {
                if *dedicated {
                    return 0;
                }
                let aligned = align_up(acc, req.alignment);
                aligned + align_up(req.size, atom_size)
            });

        AllocationRequirements {
            image_requirements,
            buffer_requirements,
            total_size,
        }
    }
}
pub struct BatchedStorage {
    images: Vec<VkImage>,
    buffers: Vec<VkBuffer>,
}

impl AllocationStorage for BatchedStorage {
    fn get_images(&self) -> &[VkImage] {
        &self.images
    }

    fn get_buffers(&self) -> &[VkBuffer] {
        &self.buffers
    }
}

impl BatchedStorage {
    pub fn new() -> BatchedStorage {
        BatchedStorage {
            images: vec![],
            buffers: vec![],
        }
    }

    pub fn add_images(&mut self, images: Vec<VkImage>) {
        self.images.extend(images);
    }

    pub fn add_buffers(&mut self, buffers: Vec<VkBuffer>) {
        self.buffers.extend(buffers);
    }

    pub fn add_buffer(&mut self, buffer: VkBuffer) {
        self.buffers.push(buffer);
    }

    pub fn add_image(&mut self, image: VkImage) {
        self.images.push(image);
    }
}

#[derive(Default)]
pub struct AllocationInfo {
    buffer_info: HashMap<VkBuffer, MemoryInfo>,
    image_info: HashMap<VkImage, MemoryInfo>,
}

impl AllocationInfo {
    pub fn store_buffer_info(&mut self, size: u64, info: VkBindBufferMemoryInfo) {
        self.buffer_info.insert(info.buffer, MemoryInfo {
            memory_object: info.memory,
            offset: info.memoryOffset,
            data_size: size,
        });
    }

    pub fn store_image_info(&mut self, size: u64, info: VkBindImageMemoryInfo) {
        self.image_info.insert(info.image, MemoryInfo {
            memory_object: info.memory,
            offset: info.memoryOffset,
            data_size: size,
        });
    }

    pub fn merge_mut(&mut self, allocation_info: AllocationInfo) {
        self.buffer_info.extend(allocation_info.buffer_info);
        self.image_info.extend(allocation_info.image_info);
    }

    pub fn merge(allocation_info: AllocationInfo) -> Self {
        let mut info = AllocationInfo::default();
        info.merge_mut(allocation_info);
        info
    }

    pub fn merge_all(allocation_infos: Vec<AllocationInfo>) -> Self {
        let mut info = AllocationInfo::default();
        allocation_infos.into_iter().for_each(|all_info| info.merge_mut(all_info));
        info
    }

    pub fn pull_buffer_info(&self, buffer: &VkBuffer) -> MemoryInfo {
        *self.buffer_info.get(buffer).unwrap()
    }

    pub fn pull_image_info(&self, image: &VkImage) -> MemoryInfo {
        *self.image_info.get(image).unwrap()
    }

    pub fn get_all_memory_objects(&self) -> Vec<VkDeviceMemory> {
        let mut memory_objects = Vec::with_capacity(self.buffer_info.len() + self.image_info.len());

        self.buffer_info.iter().for_each(|(_, info)| {
            memory_objects.push(info.memory_object);
        });
        self.image_info.iter().for_each(|(_, info)| {
            memory_objects.push(info.memory_object);
        });

        memory_objects
    }

    pub fn get_all_info(&self) -> Vec<MemoryInfo> {
        let mut images: Vec<MemoryInfo> = self.image_info.values().cloned().collect();
        let buffers: Vec<MemoryInfo> = self.buffer_info.values().cloned().collect();
        images.extend(buffers);

        images
    }
}
pub trait Allocatable: Send + Sync {
    fn get_memory_requirements(&self, vulkan: &Vulkan) -> (VkMemoryRequirements, bool);

    fn push_into(self, buffs: &mut Vec<VkBuffer>, imgs: &mut Vec<VkImage>);

    fn add_bind_task(&self, buff_tasks: &mut Vec<VkBindBufferMemoryInfo>, img_tasks: &mut Vec<VkBindImageMemoryInfo>, info: &mut AllocationInfo, memory: VkDeviceMemory, offset: u64, size: u64);

    fn dedicated(&self) -> VkMemoryDedicatedAllocateInfo;
}

impl Allocatable for VkBuffer {
    fn get_memory_requirements(&self, vulkan: &Vulkan) -> (VkMemoryRequirements, bool) {
        vulkan.get_buffer_memory_requirements(self)
    }

    fn push_into(self, buffs: &mut Vec<VkBuffer>, _: &mut Vec<VkImage>) {
        buffs.push(self);
    }

    fn add_bind_task(&self, buff_tasks: &mut Vec<VkBindBufferMemoryInfo>, _: &mut Vec<VkBindImageMemoryInfo>, info: &mut AllocationInfo, memory: VkDeviceMemory, offset: u64, size: u64) {
        let task = VkBindBufferMemoryInfo {
            buffer: *self,
            memory,
            memoryOffset: offset,
            ..Default::default()
        };
        buff_tasks.push(task);
        info.store_buffer_info(size, task);
    }

    fn dedicated(&self) -> VkMemoryDedicatedAllocateInfo {
        VkMemoryDedicatedAllocateInfo {
            buffer: *self,
            ..Default::default()
        }
    }
}

impl AllocationStorage for Vec<VkBuffer> {
    fn get_images(&self) -> &[VkImage] {
        &[]
    }

    fn get_buffers(&self) -> &[VkBuffer] {
        self
    }
}

impl Allocatable for VkImage {
    fn get_memory_requirements(&self, vulkan: &Vulkan) -> (VkMemoryRequirements, bool) {
        vulkan.get_image_memory_requirements(self)
    }

    fn push_into(self, _: &mut Vec<VkBuffer>, imgs: &mut Vec<VkImage>) {
        imgs.push(self);
    }

    fn add_bind_task(&self, _: &mut Vec<VkBindBufferMemoryInfo>, img_tasks: &mut Vec<VkBindImageMemoryInfo>, info: &mut AllocationInfo, memory: VkDeviceMemory, offset: u64, size: u64) {
        let task = VkBindImageMemoryInfo {
            image: *self,
            memory,
            memoryOffset: offset,
            ..Default::default()
        };
        img_tasks.push(task);
        info.store_image_info(size, task);
    }

    fn dedicated(&self) -> VkMemoryDedicatedAllocateInfo {
        VkMemoryDedicatedAllocateInfo {
            image: *self,
            ..Default::default()
        }
    }
}

impl AllocationStorage for Vec<VkImage> {
    fn get_images(&self) -> &[VkImage] {
        self
    }

    fn get_buffers(&self) -> &[VkBuffer] {
        &[]
    }
}

#[derive(Default, Debug)]
pub struct VkDestroy<T: Destructible + Default + PartialEq> {
    object: T,
    vulkan: Vulkan,
}

impl<T: Destructible + Default + PartialEq> VkDestroy<T> {
    pub fn new(object: T, vulkan: &Vulkan) -> Self {
        Self { object, vulkan: vulkan.clone() }
    }

    pub fn get(&self) -> &T {
        &self.object
    }
}

impl<T: Destructible + Default + PartialEq> Drop for VkDestroy<T> {
    fn drop(&mut self) {
        if self.object != T::default() {
            self.object.destroy(&self.vulkan);
        }
    }
}

impl<T: Destructible + Default + PartialEq> Deref for VkDestroy<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.object
    }
}