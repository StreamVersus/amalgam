use crate::vulkan::func::Vulkan;
use crate::vulkan::r#impl::memory::{MemoryInfo, VkDestroy};
use std::ffi::c_void;
use ultraviolet::Mat4;
use vulkan_raw::{VkBuffer, VkBufferCopy, VkCommandBuffer};

pub const MATRICES_SIZE: usize = size_of::<Mat4>() * 2;
#[repr(C)]
#[derive(Default, Debug)]
pub struct UniformBuffer {
    view: Mat4,
    proj: Mat4,
    pointer: *mut c_void,
    host_buffer: VkDestroy<VkBuffer>,
    host_buffer_info: MemoryInfo,
    device_buffer: Option<VkDestroy<VkBuffer>>,
    dirty: bool,
}

impl UniformBuffer {
    pub fn new(view: Mat4, proj: Mat4, host_buffer: VkBuffer, host_buffer_info: MemoryInfo, device_buffer: Option<VkBuffer>, vulkan: &Vulkan) -> Self {
        let pointer = vulkan.map_memory(&host_buffer_info);
        let device_buffer = match device_buffer {
            Some(device_buffer) => Some(VkDestroy::new(device_buffer, vulkan)),
            None => None,
        };
        UniformBuffer {
            view,
            proj,
            pointer,
            host_buffer: VkDestroy::new(host_buffer, vulkan),
            host_buffer_info,
            device_buffer,
            dirty: true,
        }
    }

    pub fn view(&self) -> Mat4 {
        self.view
    }

    pub fn proj(&self) -> Mat4 {
        self.proj
    }

    pub fn set_proj(&mut self, proj: Mat4) {
        self.proj = proj;
        self.dirty = true;
    }

    pub fn set_view(&mut self, view: Mat4) {
        self.view = view;
        self.dirty = true;
    }

    pub fn sync_with_buffer(&mut self, command_buffer: VkCommandBuffer, vulkan: &Vulkan) {
        if self.dirty {
            Vulkan::copy_info(self.pointer, self as *const _ as *const u8, MATRICES_SIZE);

            if let Some(buffer) = &self.device_buffer {
                vulkan.buffer_to_buffer(vec![VkBufferCopy {
                    srcOffset: 0,
                    dstOffset: 0,
                    size: self.host_buffer_info.data_size,
                }], command_buffer, *self.host_buffer.get(), *buffer.get());
            }
            self.dirty = false;
        }
    }
}