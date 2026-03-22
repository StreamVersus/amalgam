use std::collections::HashMap;
use std::ffi::c_void;
use std::ops::{Deref, DerefMut};
use std::ptr::null_mut;
use crate::prelude::{AllocationInfo, AllocationStorage, Allocator};
use crate::vulkan::func::{Destructible, Vulkan};
use crate::prelude::*;
use crate::vulkan::utils::{BufferUsage, ImageUsage};

#[derive(Default, Debug, Clone)]
pub struct PoolAllocator {
    allocator: VmaAllocator,
}

impl<S: AllocationStorage> Allocator<S, PoolAllocationInfo, PoolMemoryInfo, VmaAllocation> for PoolAllocator {
    fn allocate(&self, flags: VkMemoryPropertyFlags, storage: S, _vulkan: &Vulkan) -> PoolAllocationInfo {
        let mut info = PoolAllocationInfo::default();
        unsafe {
            storage.get_buffers().into_iter().for_each(|&buffer| {
                let alloc_info = VmaAllocationCreateInfo {
                    usage: VmaMemoryUsage::AUTO,
                    requiredFlags: flags,
                    ..Default::default()
                };
                let mut allocation = VmaAllocation::none();
                let mut allocation_info = VmaAllocationInfo::default();

                let result = vmaAllocateMemoryForBuffer(self.allocator, buffer, &alloc_info, &mut allocation, &mut allocation_info);
                assert!(result.is_ok());
                assert_ne!(allocation, VmaAllocation::none());

                vmaBindBufferMemory(self.allocator, allocation, buffer);
                info.store_buffer_info(buffer, PoolMemoryInfo {
                    alloc: allocation,
                    alloc_info: allocation_info,
                    allocator: self.allocator,
                });
            });

            storage.get_images().into_iter().for_each(|&image| {
                let alloc_info = VmaAllocationCreateInfo {
                    usage: VmaMemoryUsage::AUTO,
                    requiredFlags: flags,
                    ..Default::default()
                };
                let mut allocation = VmaAllocation::none();
                let mut allocation_info = VmaAllocationInfo::default();

                let result = vmaAllocateMemoryForImage(self.allocator, image, &alloc_info, &mut allocation, &mut allocation_info);
                assert!(result.is_ok());
                assert_ne!(allocation, VmaAllocation::none());

                vmaBindImageMemory(self.allocator, allocation, image);
                info.store_image_info(image, PoolMemoryInfo {
                    alloc: allocation,
                    alloc_info: allocation_info,
                    allocator: self.allocator,
                });
            });
        }

        info
    }
}

impl PoolAllocator {
    pub fn new(vulkan: &Vulkan) -> PoolAllocator {
        PoolAllocator {
            allocator: setup_allocator(vulkan),
        }
    }

    pub fn create_buffer(&self, size: u64, usage: BufferUsage) -> Buffer {
        let alloc_info = VmaAllocationCreateInfo {
            usage: VmaMemoryUsage::AUTO,
            ..Default::default()
        };
        self.allocate_buffer(size, usage, alloc_info)
    }

    pub fn allocate_buffer(&self, size: u64, usage: BufferUsage, alloc_info: VmaAllocationCreateInfo) -> Buffer {
        let buffer_info = VkBufferCreateInfo {
            size,
            usage: usage.into(),
            ..Default::default()
        };

        let mut buffer = VkBuffer::none();
        let mut allocation = VmaAllocation::none();
        let mut allocation_info = VmaAllocationInfo::default();
        let result = unsafe { vmaCreateBuffer(self.allocator, &buffer_info, &alloc_info, &mut buffer, &mut allocation, &mut allocation_info) };
        assert!(result.is_ok());
        assert_ne!(buffer, VkBuffer::none());
        assert_ne!(allocation, VmaAllocation::none());

        Buffer {
            buffer,
            info: PoolMemoryInfo {
                alloc: allocation,
                alloc_info: allocation_info,
                allocator: self.allocator,
            },
        }
    }

    pub fn allocator(&self) -> VmaAllocator {
        self.allocator
    }

    pub fn allocate_image(&self, format: VkFormat, image_type: VkImageType, is_cubemap: bool, mipmaps: u32, layers: u32, size: VkExtent3D, samples: VkSampleCountFlags, usage: ImageUsage) -> Image {
        let image_create_info = VkImageCreateInfo {
            flags: {
                if is_cubemap {
                    VkImageCreateFlagBits::CUBE_COMPATIBLE_BIT
                } else {
                    VkImageCreateFlagBits::empty()
                }
            },
            imageType: image_type,
            format,
            extent: size,
            mipLevels: mipmaps,
            arrayLayers: layers,
            samples,
            tiling: VkImageTiling::OPTIMAL,
            usage: usage.into(),
            sharingMode: VkSharingMode::EXCLUSIVE,
            ..Default::default()
        };

        let mut image = VkImage::none();
        let mut allocation = VmaAllocation::none();
        let mut allocation_info = VmaAllocationInfo::default();
        let result = unsafe { vmaCreateImage(self.allocator, &image_create_info, null_mut(), &mut image, &mut allocation, &mut allocation_info) };
        assert_eq!(VkResult::SUCCESS, result);

        Image {
            image,
            info: PoolMemoryInfo {
                alloc: allocation,
                alloc_info: allocation_info,
                allocator: self.allocator,
            },
        }
    }

    pub fn create_staging_buffer(&mut self, size: u64) -> Buffer {
        self.create_buffer(size, BufferUsage::preset_staging())
    }

    pub fn finish(&self) {
        unsafe { vmaDestroyAllocator(self.allocator) };
    }
}

fn setup_allocator(vulkan: &Vulkan) -> VmaAllocator {
    let mut flags = VmaAllocatorCreateFlagBits::EXT_MEMORY_BUDGET_BIT |
        VmaAllocatorCreateFlagBits::KHR_DEDICATED_ALLOCATION_BIT;

    let device_info = &vulkan.get_loaded_device().device_info;
    if device_info.coherent_memory.deviceCoherentMemory.into() {
        flags |= VmaAllocatorCreateFlagBits::AMD_DEVICE_COHERENT_MEMORY_BIT;
    }
    if device_info.features12.bufferDeviceAddress.into() {
        flags |= VmaAllocatorCreateFlagBits::BUFFER_DEVICE_ADDRESS_BIT;
    }
    if device_info.memory_priority.memoryPriority.into() {
        flags |= VmaAllocatorCreateFlagBits::EXT_MEMORY_PRIORITY_BIT;
    }

    let vulkan_functions = VmaVulkanFunctions::default();
    let allocator_info = VmaAllocatorCreateInfo {
        flags,
        physicalDevice: vulkan.get_loaded_device().device,
        device: vulkan.get_loaded_device().logical_device,
        instance: vulkan.get_instance(),
        vulkanApiVersion: vulkan.get_api_version().into(),
        pVulkanFunctions: &vulkan_functions,
        ..Default::default()
    };

    let mut alloc = VmaAllocator::none();
    let result = unsafe {
        vmaCreateAllocator(&allocator_info, &mut alloc)
    };
    assert!(result.is_ok());
    assert_ne!(alloc, VmaAllocator::none());

    alloc
}

#[derive(Default)]
pub struct PoolAllocationInfo {
    buffer_info: HashMap<VkBuffer, PoolMemoryInfo>,
    image_info: HashMap<VkImage, PoolMemoryInfo>,
}

impl AllocationInfo<PoolMemoryInfo, VmaAllocation> for PoolAllocationInfo {
    fn store_buffer_info(&mut self, buffer: VkBuffer, info: PoolMemoryInfo) {
        self.buffer_info.insert(buffer, info);
    }

    fn store_image_info(&mut self, image: VkImage, info: PoolMemoryInfo) {
        self.image_info.insert(image, info);
    }

    fn merge_mut(&mut self, allocation_info: Self) {
        self.buffer_info.extend(allocation_info.buffer_info);
        self.image_info.extend(allocation_info.image_info);
    }

    fn pull_buffer_info(&self, buffer: &VkBuffer) -> &PoolMemoryInfo {
        self.buffer_info.get(buffer).unwrap()
    }

    fn pull_image_info(&self, image: &VkImage) -> &PoolMemoryInfo {
        self.image_info.get(image).unwrap()
    }

    fn get_all_memory_objects(&self) -> Vec<VmaAllocation> {
        let mut memory_objects = Vec::with_capacity(self.buffer_info.len() + self.image_info.len());

        self.buffer_info.iter().for_each(|(_, info)| {
            memory_objects.push(info.memory_object());
        });
        self.image_info.iter().for_each(|(_, info)| {
            memory_objects.push(info.memory_object());
        });

        memory_objects
    }

    fn get_all_info(&self) -> Vec<PoolMemoryInfo> {
        let mut images: Vec<PoolMemoryInfo> = self.image_info.values().cloned().collect();
        let buffers: Vec<PoolMemoryInfo> = self.buffer_info.values().cloned().collect();
        images.extend(buffers);

        images
    }
}

impl Destructible for VmaAllocation {
    fn destroy(&self, vulkan: &Vulkan) {
        unsafe { vmaFreeMemory(vulkan.pool().allocator, *self) }
    }
}

#[derive(Clone)]
#[derive(Default)]
pub struct PoolMemoryInfo {
    pub alloc: VmaAllocation,
    pub alloc_info: VmaAllocationInfo,
    allocator: VmaAllocator,
}

impl MemoryInfo<VmaAllocation> for PoolMemoryInfo {
    fn memory_object(&self) -> VmaAllocation {
        self.alloc
    }
    fn data_size(&self) -> u64 {
        self.alloc_info.size
    }
    fn map_memory(&self, _vulkan: &Vulkan) -> *mut c_void {
        let mut ptr = null_mut();
        let result = unsafe { vmaMapMemory(self.allocator, self.alloc, &mut ptr) };
        assert!(result.is_ok());
        assert_ne!(ptr, null_mut());

        ptr
    }
    fn unmap_memory(&self, _vulkan: &Vulkan) {
        unsafe { vmaUnmapMemory(self.allocator, self.alloc) }
    }

    fn flush_memory(&self, _vulkan: &Vulkan) -> bool {
        unsafe { vmaFlushAllocation(self.allocator, self.alloc, self.alloc_info.offset, self.alloc_info.size) == VkResult::SUCCESS }
    }
}

impl PoolMemoryInfo {
    pub fn allocator(&self) -> VmaAllocator {
        self.allocator
    }
}

#[derive(Default)]
pub struct Buffer {
    pub buffer: VkBuffer,
    pub info: PoolMemoryInfo,
}

impl Drop for Buffer {
    fn drop(&mut self) {
        if self.info.allocator != VmaAllocator::none() {
            unsafe { vmaDestroyBuffer(self.info.allocator, self.buffer, self.info.alloc) }
        }
    }
}

impl Deref for Buffer {
    type Target = VkBuffer;

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

impl DerefMut for Buffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.buffer
    }
}

impl Buffer {
    pub fn map_memory(&self, vulkan: &Vulkan) -> *mut c_void {
        self.info.map_memory(vulkan)
    }

    pub fn unmap_memory(&self, vulkan: &Vulkan) {
        self.info.unmap_memory(vulkan)
    }

    pub fn flush_memory(&self, vulkan: &Vulkan) -> bool {
        self.info.flush_memory(vulkan)
    }
}

pub struct Image {
    pub image: VkImage,
    pub info: PoolMemoryInfo,
}

impl Drop for Image {
    fn drop(&mut self) {
        if self.info.allocator != VmaAllocator::none() {
            unsafe { vmaDestroyImage(self.info.allocator, self.image, self.info.alloc) }
        }
    }
}

impl Deref for Image {
    type Target = VkImage;

    fn deref(&self) -> &Self::Target {
        &self.image
    }
}

impl DerefMut for Image {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.image
    }
}