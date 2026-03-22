use crate::prelude::pool_alloc::Buffer;
use crate::prelude::*;
use crate::vulkan::func::Vulkan;
use std::ffi::c_void;
use ultraviolet::Mat4;
use vulkan_raw::{VkBufferCopy, VkCommandBuffer, VkDeviceSize};
use crate::vulkan::utils::BufferUsage;

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
    host_buffer: Buffer,
    device_buffer: Option<Buffer>,
    dirty: bool,
}

impl UniformBuffer {
    pub fn new(view: Mat4, proj: Mat4, vulkan: &Vulkan) -> Self {
        let alloc_info = VmaAllocationCreateInfo {
            usage: VmaMemoryUsage::AUTO,
            flags: VmaAllocationCreateFlagBits::HOST_ACCESS_SEQUENTIAL_WRITE_BIT,
            requiredFlags: VkMemoryPropertyFlagBits::HOST_VISIBLE_BIT
                | VkMemoryPropertyFlagBits::HOST_COHERENT_BIT,
            preferredFlags: VkMemoryPropertyFlagBits::DEVICE_LOCAL_BIT,
            ..Default::default()
        };

        let host_buffer = vulkan.pool().allocate_buffer(MATRICES_SIZE as u64, BufferUsage::preset_staging().uniform_buffer(true), alloc_info);
        let flags = vulkan.get_loaded_device().memory_properties.memoryTypes[host_buffer.info.alloc_info.memoryType as usize].propertyFlags;
        let device_buffer = if flags.contains(VkMemoryPropertyFlagBits::DEVICE_LOCAL_BIT) {
            None
        } else {
            let alloc_info = VmaAllocationCreateInfo {
                usage: VmaMemoryUsage::AUTO_PREFER_DEVICE,
                ..Default::default()
            };
            Some(vulkan.pool().allocate_buffer(MATRICES_SIZE as u64, BufferUsage::preset_uniform_storage(), alloc_info))
        };
        Self {
            matrices: Matrices {
                 view,
                 proj,
            },
            host_pointer: host_buffer.map_memory(vulkan),
            host_buffer,
            device_buffer,
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

    pub fn sync_with_buffer(&mut self, command_buffer: VkCommandBuffer, vulkan: &Vulkan) {
        if self.dirty {
            Vulkan::copy_info(self.host_pointer, &self.matrices as *const _ as *const u8, MATRICES_SIZE);

            if let Some(device_buffer) = self.device_buffer.as_ref() {
                let regions = [VkBufferCopy {
                    srcOffset: 0,
                    dstOffset: 0,
                    size: MATRICES_SIZE as VkDeviceSize,
                }];
                vulkan.buffer_to_buffer(&regions, command_buffer, *self.host_buffer, **device_buffer);

                // vulkan.transition_buffers(
                //     vec![BufferTransition {
                //         buffer: **device_buffer,
                //         offset: 0,
                //         size: VK_WHOLE_SIZE,
                //         src_stage: VkPipelineStageFlags2::TRANSFER_BIT,
                //         dst_stage: VkPipelineStageFlags2::VERTEX_SHADER_BIT,
                //         src_access: VkAccessFlags2::TRANSFER_WRITE_BIT,
                //         dst_access: VkAccessFlags2::UNIFORM_READ_BIT,
                //         src_queue_family: VK_QUEUE_FAMILY_IGNORED,
                //         dst_queue_family: VK_QUEUE_FAMILY_IGNORED,
                //     }],
                //     command_buffer,
                // );
            }

            self.dirty = false;
        }
    }

    pub fn provide_buffer(&self) -> VkBuffer {
        if let Some(device_buffer) = self.device_buffer.as_ref() {
            device_buffer.buffer
        } else {
            self.host_buffer.buffer
        }
    }
}