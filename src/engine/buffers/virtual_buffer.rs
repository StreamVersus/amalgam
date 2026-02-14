use std::ffi::c_void;
use std::ops::Deref;
use crate::vulkan::r#impl::memory::MemoryInfo;
use vulkan_raw::{VkBuffer, VkDeviceSize};
use crate::vulkan::func::Vulkan;

#[derive(Default)]
pub struct VirtualBuffer {
    pub buffer: VkBuffer,
    pub offset: VkDeviceSize,
    pub size: VkDeviceSize,
    pub info: MemoryInfo,
    pfn: Option<*mut c_void>,
}

impl VirtualBuffer {
    pub fn new(buffer: VkBuffer, offset: VkDeviceSize, size: VkDeviceSize, info: MemoryInfo) -> Self {
        Self { buffer, offset, size, info, pfn: None }
    }
    
    pub fn map_memory(&mut self, vulkan: &Vulkan) -> *mut c_void {
        self.pfn.unwrap_or_else(|| unsafe {
            let pfn = vulkan.map_memory(&self.info).add(self.offset as usize);
            self.pfn = Some(pfn);
            
            pfn
        })
    }
}

impl Deref for VirtualBuffer {
    type Target = VkBuffer;
    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}
