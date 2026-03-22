use std::ffi::c_void;
use crate::vulkan::func::{Destructible, Vulkan};
use std::ops::{Deref};
use vulkan_raw::{VkBindBufferMemoryInfo, VkBindImageMemoryInfo, VkBuffer, VkDeviceMemory, VkImage, VkMemoryDedicatedAllocateInfo, VkMemoryPropertyFlags, VkMemoryRequirements};
use crate::vulkan::utils::align_up;

pub mod pool_alloc;
pub mod arena_alloc;

pub trait AllocationInfo<M, D>: Default {
    fn store_buffer_info(&mut self, buffer: VkBuffer, info: M);
    fn store_image_info(&mut self, image: VkImage, info: M);
    fn merge_mut(&mut self, allocation_info: Self);
    fn pull_buffer_info(&self, buffer: &VkBuffer) -> &M;
    fn pull_image_info(&self, image: &VkImage) -> &M;
    fn get_all_memory_objects(&self) -> Vec<D>;
    fn get_all_info(&self) -> Vec<M>;

    fn merge(allocation_info: Self) -> Self {
        let mut info = Self::default();
        info.merge_mut(allocation_info);
        info
    }

    fn merge_all(allocation_infos: Vec<Self>) -> Self {
        let mut info = Self::default();
        allocation_infos.into_iter().for_each(|all_info| info.merge_mut(all_info));
        info
    }
}

pub trait Allocator<S: AllocationStorage, I: AllocationInfo<M, D>, M, D> {
    fn allocate(&self, flags: VkMemoryPropertyFlags, storage: S, vulkan: &Vulkan) -> I;
    fn device(&self, storage: S, vulkan: &Vulkan) -> I {
        self.allocate(VkMemoryPropertyFlags::DEVICE_LOCAL_BIT, storage, vulkan)
    }
    fn host_cached(&self, storage: S, vulkan: &Vulkan) -> I {
        self.allocate(VkMemoryPropertyFlags::HOST_VISIBLE_BIT | VkMemoryPropertyFlags::HOST_CACHED_BIT, storage, vulkan)
    }
    fn host_coherent(&self, storage: S, vulkan: &Vulkan) -> I {
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

pub trait Allocatable<T>: Send + Sync {
    fn get_memory_requirements(&self, vulkan: &Vulkan) -> (VkMemoryRequirements, bool);

    fn push_into(self, buffs: &mut Vec<VkBuffer>, imgs: &mut Vec<VkImage>);

    fn build_bind_task(&self, memory: VkDeviceMemory, offset: u64) -> T;

    fn dedicated(&self) -> VkMemoryDedicatedAllocateInfo;
}

impl Allocatable<VkBindBufferMemoryInfo> for VkBuffer {
    fn get_memory_requirements(&self, vulkan: &Vulkan) -> (VkMemoryRequirements, bool) {
        vulkan.get_buffer_memory_requirements(self)
    }

    fn push_into(self, buffs: &mut Vec<VkBuffer>, _: &mut Vec<VkImage>) {
        buffs.push(self);
    }

    fn build_bind_task(&self, memory: VkDeviceMemory, offset: u64) -> VkBindBufferMemoryInfo {
        VkBindBufferMemoryInfo {
            buffer: *self,
            memory,
            memoryOffset: offset,
            ..Default::default()
        }
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

impl Allocatable<VkBindImageMemoryInfo> for VkImage {
    fn get_memory_requirements(&self, vulkan: &Vulkan) -> (VkMemoryRequirements, bool) {
        vulkan.get_image_memory_requirements(self)
    }

    fn push_into(self, _: &mut Vec<VkBuffer>, imgs: &mut Vec<VkImage>) {
        imgs.push(self);
    }

    fn build_bind_task(&self, memory: VkDeviceMemory, offset: u64) -> VkBindImageMemoryInfo {
        VkBindImageMemoryInfo {
            image: *self,
            memory,
            memoryOffset: offset,
            ..Default::default()
        }
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

pub trait MemoryInfo<T: ?Sized> {
    fn memory_object(&self) -> T;
    fn data_size(&self) -> u64;
    fn map_memory(&self, vulkan: &Vulkan) -> *mut c_void;
    fn unmap_memory(&self, vulkan: &Vulkan);
    fn flush_memory(&self, vulkan: &Vulkan) -> bool;
}
