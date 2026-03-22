use crate::prelude::*;
use crate::vulkan::func::{Destructible, Vulkan};
use crate::vulkan::utils::align_up;
use std::os::raw::c_void;
use std::ptr::null_mut;
impl Vulkan {
    pub fn calculate_total_size(memory_requirements: &Vec<VkMemoryRequirements>) -> u64 {
        let mut total_size = 0u64;

        for req in memory_requirements {
            total_size = align_up(total_size, req.alignment);
            total_size += req.size;
        }

        total_size
    }

    pub fn allocate_memory_object(&self, total_size: u64, memory_type: u32) -> VkDeviceMemory {
        let allocate_info = VkMemoryAllocateInfo {
            allocationSize: total_size,
            memoryTypeIndex: memory_type,
            ..Default::default()
        };
        let mut memory_object = VkDeviceMemory::none();
        let result = unsafe { vkAllocateMemory(self.get_loaded_device().logical_device, &allocate_info, null_mut(), &mut memory_object) };
        assert!(result.is_ok());

        memory_object
    }

    #[allow(private_bounds)]
    pub fn allocate_dedicated_memory<T: Allocatable<A> + 'static, A>(&self, res: T, req: VkMemoryRequirements, flags: VkMemoryPropertyFlags) -> VkDeviceMemory {
        let dedicated_info = res.dedicated();
        let allocate_info = VkMemoryAllocateInfo {
            pNext: &dedicated_info as *const _ as *const c_void,
            allocationSize: req.size,
            memoryTypeIndex: self.find_memory_type(&[req], flags).unwrap(),
            ..Default::default()
        };

        let mut memory_object = VkDeviceMemory::none();
        let result = unsafe { vkAllocateMemory(self.get_loaded_device().logical_device, &allocate_info, null_mut(), &mut memory_object) };
        assert!(result.is_ok());

        memory_object
    }

    pub fn map_memory(&self, memory_object: VkDeviceMemory, offset: u64, data_size: u64) -> *mut c_void {
        let mut pointer: *mut c_void = null_mut();
        let result = unsafe{ vkMapMemory(self.get_loaded_device().logical_device, memory_object, offset, data_size, VkMemoryMapFlagBits::empty(), &mut pointer) };
        assert!(result.is_ok());
        
        pointer
    }

    pub fn flush_memory(&self, info: &[(VkDeviceMemory, u64, u64)]) -> bool {
        let memory_ranges = info.iter().map(|info| VkMappedMemoryRange {
            memory: info.0,
            offset: info.1,
            size: info.2,
            ..Default::default()
        }).collect::<Vec<_>>();

        let result = unsafe { vkFlushMappedMemoryRanges(self.get_loaded_device().logical_device, memory_ranges.len() as u32, memory_ranges.as_ptr()) };
        result == VkResult::SUCCESS
    }

    /// Safety: use with caution, double check for alignment
    #[allow(deprecated)]
    pub fn copy_info<T>(dst_pointer: *mut c_void, src_pointer: *const T, count: usize) {
        unsafe { std::intrinsics::copy_nonoverlapping(src_pointer as *const u8, dst_pointer as *mut u8, size_of::<T>() * count) };
    }

    pub fn destroy_memory(&self, memory: VkDeviceMemory) {
        unsafe { vkFreeMemory(self.get_loaded_device().logical_device, memory, null_mut()) };
    }

    pub fn unmap_memory(&self, memory: VkDeviceMemory) {
        unsafe { vkUnmapMemory(self.get_loaded_device().logical_device, memory) };
    }

    pub fn find_memory_type(&self, memory_requirements: &[VkMemoryRequirements], properties: VkMemoryPropertyFlags) -> Option<u32> {
        let memory_type_bits = memory_requirements
            .iter()
            .map(|req| req.memoryTypeBits)
            .fold(u32::MAX, |acc, bits| acc & bits);

        let mem_properties = &self.get_loaded_device().memory_properties;

        for i in 0..mem_properties.memoryTypeCount {
            let type_supported = (memory_type_bits & (1 << i)) != 0;

            let has_properties = (mem_properties.memoryTypes[i as usize].propertyFlags & properties) == properties;

            if type_supported && has_properties {
                return Some(i);
            }
        }

        None
    }
}



impl Destructible for VkDeviceMemory {
    fn destroy(&self, vulkan: &Vulkan) {
        vulkan.destroy_memory(*self);
    }
}
