use crate::engine::shapes::renderable::Renderable;
use crate::engine::vbo::VBO;
use crate::vulkan::func::Vulkan;
use crate::vulkan::gltf::scene::Vertex;
use crate::vulkan::gltf::utils::StagingBuffer;
use crate::vulkan::r#impl::memory::AllocationTask;
use crate::vulkan::r#impl::memory::VkDestroy;
use crate::vulkan::utils::BufferUsage;
use rand::Rng;
use ultraviolet::{f32x8, Vec3, Vec3x8};
use vulkan_raw::{vkCmdDrawIndexed, VkBuffer, VkBufferCopy, VkCommandBuffer, VkIndexType, VkPipelineBindPoint, VkPipelineLayout};

pub trait AABB {
    fn box_min(&self) -> &Vec3;
    fn box_max(&self) -> &Vec3;
}

#[derive(Default)]
pub struct SimpleAABox {
    pub box_min: Vec3,
    pub box_max: Vec3,
}

impl AABB for SimpleAABox {
    #[inline(always)]

    fn box_min(&self) -> &Vec3 {
        &self.box_min
    }
    #[inline(always)]
    fn box_max(&self) -> &Vec3 {
        &self.box_max
    }
}

impl SimpleAABox {
    pub fn new(box_min: Vec3, box_max: Vec3) -> Self {
        Self { box_min, box_max }
    }

    pub fn new_rand<R: Rng>(rng: &mut R) -> Self {
        Self::new(
            Vec3::new(rng.random_range(0f32..100f32), rng.random_range(0f32..100f32), rng.random_range(0f32..100f32)),
            Vec3::new(rng.random_range(0f32..100f32), rng.random_range(0f32..100f32), rng.random_range(0f32..100f32))
        )
    }
}

#[derive(Default)]
pub struct RenderableAABox {
    pub box_min: Vec3,
    pub box_max: Vec3,

    vertex: VBO,
    index: VkDestroy<VkBuffer>,
}

impl AABB for RenderableAABox {
    #[inline(always)]
    fn box_min(&self) -> &Vec3 {
        &self.box_min
    }
    #[inline(always)]
    fn box_max(&self) -> &Vec3 {
        &self.box_max
    }
}

impl Renderable for RenderableAABox {
    #[allow(unused_variables)]
    fn allocate(&mut self, vulkan: &Vulkan, host: &mut AllocationTask, device: &mut AllocationTask) {
        let index = vulkan.create_buffer(size_of::<u16>() as u64 * 36, BufferUsage::preset_index()).unwrap();

        host.add_allocatable_ref(index);

        self.vertex = VBO::new(vulkan, size_of::<Vertex>() as u64 * 8);
        self.index = VkDestroy::new(index, vulkan);
    }

    fn prepare(&mut self, vulkan: &Vulkan, command_buffer: VkCommandBuffer, staging: &mut StagingBuffer) {
        let staging_buffer = staging.pull(size_of::<u16>() as u64 * 36, vulkan);
        let info = staging.info();
        let staging_ptr = vulkan.map_memory(info);

        let face_indices: [u16; 36] = [
            // Back face
            0, 1, 2,  2, 3, 0,
            // Front face
            4, 6, 5,  4, 7, 6,
            // Left face
            4, 0, 3,  4, 3, 7,
            // Right face
            1, 5, 6,  1, 6, 2,
            // Bottom face
            4, 5, 1,  4, 1, 0,
            // Top face
            3, 2, 6,  3, 6, 7,
        ];

        Vulkan::copy_info(staging_ptr, &face_indices, size_of::<u16>() * 36);
        vulkan.flush_memory(&[*info]);
        vulkan.buffer_to_buffer(vec![
            VkBufferCopy {
                srcOffset: 0,
                dstOffset: 0,
                size: info.data_size(),
            }
        ], command_buffer, staging_buffer, *self.index);

        let [x0, y0, z0] = *self.box_min.as_array();
        let [x1, y1, z1] = *self.box_max.as_array();

        self.vertex.build_vertex_inplace([x0, y0, z0], [0.0, 0.0, 0.0], [0.0, 0.0]); // 0
        self.vertex.build_vertex_inplace([x1, y0, z0], [0.0, 0.0, 0.0], [1.0, 0.0]); // 1
        self.vertex.build_vertex_inplace([x1, y1, z0], [0.0, 0.0, 0.0], [1.0, 1.0]); // 2
        self.vertex.build_vertex_inplace([x0, y1, z0], [0.0, 0.0, 0.0], [0.0, 1.0]); // 3
        self.vertex.build_vertex_inplace([x0, y0, z1], [0.0, 0.0, 0.0], [0.0, 0.0]); // 4
        self.vertex.build_vertex_inplace([x1, y0, z1], [0.0, 0.0, 0.0], [1.0, 0.0]); // 5
        self.vertex.build_vertex_inplace([x1, y1, z1], [0.0, 0.0, 0.0], [1.0, 1.0]); // 6
        self.vertex.build_vertex_inplace([x0, y1, z1], [0.0, 0.0, 0.0], [0.0, 1.0]); // 7
        self.vertex.sync_buffer(vulkan, command_buffer);
    }

    fn render(&mut self, vulkan: &Vulkan, command_buffer: VkCommandBuffer, pipeline_layout: VkPipelineLayout) {
        self.vertex.bind(vulkan, command_buffer);

        vulkan.bind_index_buffer(command_buffer, *self.index.get(), 0, VkIndexType::UINT16);
        vulkan.bind_descriptor_sets(command_buffer, VkPipelineBindPoint::GRAPHICS, pipeline_layout, 0, &[], &[]);
        unsafe { vkCmdDrawIndexed(command_buffer, 36, 1, 0, 0, 0) }
    }
}

impl RenderableAABox {
    pub fn new(box_min: Vec3, box_max: Vec3) -> Self {
        Self {
            box_min,
            box_max,
            ..Default::default()
        }
    }
}

pub struct AABB4(pub Vec3x8);

impl AABB4 {
    pub fn new<A: AABB>(a: &A, b: &A, c: &A, d: &A) -> Self {
        Self {
            0: Vec3x8 {
                x: f32x8::new([a.box_min().x, a.box_min().x, b.box_min().x, b.box_min().x, c.box_min().x, c.box_min().x, d.box_min().x, d.box_min().x]),
                y: f32x8::new([a.box_min().y, a.box_min().y, b.box_min().y, b.box_min().y, c.box_min().y, c.box_min().y, d.box_min().y, d.box_min().y]),
                z: f32x8::new([a.box_min().z, a.box_min().z, b.box_min().z, b.box_min().z, c.box_min().z, c.box_min().z, d.box_min().z, d.box_min().z]),
            }
        }
    }

    pub fn from_arr<A: AABB>(arr: [&A; 4]) -> Self {
        Self::new(arr[0], arr[1], arr[2], arr[3])
    }
}