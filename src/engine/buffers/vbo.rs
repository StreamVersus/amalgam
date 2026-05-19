use crate::prelude::pool_alloc::Buffer;
use crate::prelude::*;
use crate::vulkan::func::Vulkan;
use crate::vulkan::gltf::scene::Vertex;
use crate::vulkan::utils::BufferUsage;
use std::ffi::c_void;
use std::ptr::null_mut;

#[derive(Default)]
pub struct VBO {
    buffer: Buffer,
    staging: bool,
    offset: u64,
    ptr: *mut c_void,
}

impl VBO {
    pub fn new(vulkan: &Vulkan, size: u64, staging: bool) -> Self {
        let pool = vulkan.pool();

        let alloc_info = VmaAllocationCreateInfo {
            usage: if staging {VmaMemoryUsage::AUTO_PREFER_HOST} else {VmaMemoryUsage::AUTO_PREFER_DEVICE},
            flags: VmaAllocationCreateFlagBits::HOST_ACCESS_SEQUENTIAL_WRITE_BIT,
            ..Default::default()
        };
        let mut buffer = pool.allocate_buffer(size, if staging { BufferUsage::preset_staging() } else { BufferUsage::preset_vertex() }, alloc_info);
        let ptr = if staging {buffer.map_memory(vulkan)} else {null_mut()};
        Self {
            buffer,
            staging,
            offset: 0,
            ptr,
        }
    }

    pub fn sync_buffer(&mut self, vulkan: &Vulkan, command_buffer: VkCommandBuffer, staging: &VBO) {
        if self.staging {
            eprintln!("SYNCING STAGING VBO, UNSTABLE BEHAVIOUR")
        }
        vulkan.buffer_to_buffer(&[VkBufferCopy {
            srcOffset: 0,
            dstOffset: 0,
            size: self.buffer.info.alloc_info.size,
        }], command_buffer, *staging.buffer, *self.buffer);

        vulkan.transition_buffers(vec![
            BufferTransition {
                buffer: *self.buffer,
                offset: 0,
                size: VK_WHOLE_SIZE,
                src_stage: VkPipelineStageFlags2::TRANSFER_BIT,
                dst_stage: VkPipelineStageFlags2::VERTEX_INPUT_BIT,
                src_access: VkAccessFlags2::TRANSFER_WRITE_BIT,
                dst_access: VkAccessFlags2::VERTEX_ATTRIBUTE_READ_BIT,
                src_queue_family: VK_QUEUE_FAMILY_IGNORED,
                dst_queue_family: VK_QUEUE_FAMILY_IGNORED,
            }
        ], command_buffer);
    }

    pub fn build_vertex_inplace(&mut self, pos: [f32; 3], normal: [f32; 3], uv: [f32; 2]) {
        if !self.staging {
            panic!("BUILDING IN DEVICE VBO")
        }

        if self.offset + size_of::<Vertex>() as u64 > self.buffer.info.alloc_info.size {
            panic!("Tried to write to VBO, but overflowed");
        }
        unsafe {
            let vertex = self.ptr.add(self.offset as usize) as *mut Vertex;
            let vertex = &mut (*vertex);
            vertex.position = pos;
            vertex.normal = normal;
            vertex.texcoords = uv;

            self.offset += size_of::<Vertex>() as u64;
        }
    }

    pub fn bind(&self, vulkan: &Vulkan, command_buffer: VkCommandBuffer) {
        if self.staging {
            eprintln!("MOUNTING STAGING VBO, UNSTABLE BEHAVIOUR")
        }
        vulkan.bind_vertex_buffers(command_buffer, 0, vec![VertexBufferParameters {
            buffer: *self.buffer,
            offset: 0,
        }]);
    }
}
