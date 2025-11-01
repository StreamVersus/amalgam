use crate::vulkan::func::Vulkan;
use crate::vulkan::gltf::scene::Vertex;
use crate::vulkan::r#impl::memory::{AllocationTask, MemoryInfo, VkDestroy};
use crate::vulkan::r#impl::pipelines::VertexBufferParameters;
use crate::vulkan::utils::BufferUsage;
use std::ffi::c_void;
use vulkan_raw::{VkBuffer, VkBufferCopy, VkCommandBuffer, VkDeviceMemory};

#[derive(Default)]
pub struct VBO {
    device_buffer: VkDestroy<VkBuffer>,
    host_buffer: VkDestroy<VkBuffer>,
    host_ptr: *mut c_void,

    _device_memory: VkDestroy<VkDeviceMemory>,
    _host_memory: VkDestroy<VkDeviceMemory>,
    host_info: MemoryInfo,

    dirty: bool,
    offset: u64,
    pub(crate) size: u64,
}

impl VBO {
    pub fn new(vulkan: &Vulkan, size: u64) -> Self {
        let device_buffer = vulkan.create_buffer(size, BufferUsage::preset_vertex()).unwrap();
        let host_buffer = vulkan.create_buffer(size, BufferUsage::preset_staging()).unwrap();

        let device_info = AllocationTask::device().add_allocatable(device_buffer).allocate_all(vulkan);
        let host_info = AllocationTask::host_cached().add_allocatable(host_buffer).allocate_all(vulkan);

        let device_memory = device_info.pull_buffer_info(&device_buffer).memory_object;
        let host_memory = host_info.pull_buffer_info(&host_buffer);

        Self {
            device_buffer: VkDestroy::new(device_buffer, vulkan),
            host_buffer: VkDestroy::new(host_buffer, vulkan),
            host_ptr: vulkan.map_memory(&host_memory),
            _device_memory: VkDestroy::new(device_memory, vulkan),
            _host_memory: VkDestroy::new(host_memory.memory_object, vulkan),
            host_info: host_memory,
            dirty: false,
            offset: 0,
            size,
        }
    }

    pub fn sync_buffer(&mut self, vulkan: &Vulkan, command_buffer: VkCommandBuffer) {
        if self.dirty {
            vulkan.flush_memory(&[self.host_info]);
            vulkan.buffer_to_buffer(vec![VkBufferCopy {
                srcOffset: 0,
                dstOffset: 0,
                size: self.size,
            }], command_buffer, *self.host_buffer.get(), *self.device_buffer.get());
            self.dirty = false;
        }
    }

    pub fn build_vertex_inplace(&mut self, pos: [f32; 3], normal: [f32; 3], uv: [f32; 2]) {
        if self.offset + size_of::<Vertex>() as u64 > self.size {
            panic!("Tried to write to VBO, but overflowed");
        }
        unsafe {
            let vertex = self.host_ptr.add(self.offset as usize) as *mut Vertex;
            let vertex = &mut (*vertex);
            vertex.position = pos;
            vertex.normal = normal;
            vertex.texcoords = uv;

            self.offset += size_of::<Vertex>() as u64;
        }
        self.dirty = true;
    }

    pub fn bind(&self, vulkan: &Vulkan, command_buffer: VkCommandBuffer) {
        vulkan.bind_vertex_buffers(command_buffer, 0, vec![VertexBufferParameters {
            buffer: *self.device_buffer.get(),
            offset: 0,
        }]);
    }
}

