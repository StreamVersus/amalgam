use crate::engine::buffers::virtual_buffer::VirtualBuffer;
use crate::vulkan::func::Vulkan;
use std::ffi::c_void;
use ultraviolet::Mat4;
use vulkan_raw::{VkBuffer, VkBufferCopy, VkCommandBuffer, VkDeviceSize};
use crate::vulkan::r#impl::memory::VkDestroy;

pub const MATRICES_SIZE: usize = size_of::<Matrices>();
#[repr(C)]
#[derive(Default, Debug)]
pub struct Matrices {
    view: Mat4,
    proj: Mat4,
}

#[derive(Default)]
pub struct UniformBuffer {
    matrices: Matrices,
    host_pointer: *mut c_void,
    host_buffer: VirtualBuffer,
    device_buffer: VkDestroy<VkBuffer>,
    dirty: bool,
}

impl UniformBuffer {
    pub fn new(view: Mat4, proj: Mat4, mut host_buffer: VirtualBuffer, device_buffer: VkBuffer, vulkan: &Vulkan) -> Self {
        Self {
            matrices: Matrices {
                 view,
                 proj,
            },
            host_pointer: host_buffer.map_memory(vulkan),
            host_buffer,
            device_buffer: VkDestroy::new(device_buffer, vulkan),
            dirty: true,
        }
    }

    pub fn view(&self) -> Mat4 {
        self.matrices.view
    }

    pub fn proj(&self) -> Mat4 {
        self.matrices.proj
    }

    pub fn set_proj(&mut self, proj: Mat4) {
        self.matrices.proj = proj;
        self.dirty = true;
    }

    pub fn set_view(&mut self, view: Mat4) {
        self.matrices.view = view;
        self.dirty = true;
    }

    //TODO: dont update whole buffer on one matrix change
    pub fn sync_with_buffer(&mut self, command_buffer: VkCommandBuffer, vulkan: &Vulkan) {
        if self.dirty {
            Vulkan::copy_info(self.host_pointer, &self.matrices as *const _ as *const u8, MATRICES_SIZE);

            vulkan.buffer_to_buffer(vec![VkBufferCopy {
                srcOffset: self.host_buffer.offset as VkDeviceSize,
                dstOffset: 0,
                size: MATRICES_SIZE as VkDeviceSize,
            }], command_buffer, *self.host_buffer, *self.device_buffer);
            self.dirty = false;
        }
    }
}