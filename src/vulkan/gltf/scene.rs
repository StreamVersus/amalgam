use crate::engine::utils::obj_n_size::NSize;
use crate::engine::vbo::VBO;
use crate::vulkan::gltf::ubo::UniformBuffer;
use crate::vulkan::gltf::utils::{ChunkType, IndirectParameters};
use crate::vulkan::r#impl::memory::VkDestroy;
use crate::vulkan::r#impl::descriptors::PooledDescriptors;
use ultraviolet::{Mat4, Rotor3, Vec3};
use vulkan_raw::{VkBuffer, VkDeviceMemory, VkExtent3D, VkImage, VkImageView, VkSampler};

type SizedBuffer = NSize<VkDestroy<VkBuffer>>;
#[derive(Default)]
pub struct Scene {
    pub ubo: UniformBuffer,

    pub vbo: VBO,
    pub idx: SizedBuffer,
    pub indirect_buffer: SizedBuffer,
    pub material_ssbo: SizedBuffer,
    pub model_ssbo: SizedBuffer,

    pub(crate) parameters: NSize<Vec<IndirectParameters>>,
    pub descriptors: PooledDescriptors,

    pub indices: Vec<u16>,

    pub(crate) texture_images: Vec<Image>,
    pub model_matrices: Vec<Mat4>,
    pub material_ranges: Vec<MaterialID>,

    pub(crate) _samplers: Vec<VkDestroy<VkSampler>>,
    pub(crate) _memory: Vec<VkDestroy<VkDeviceMemory>>,
}

pub struct Node {
    pub meshes: Vec<Mesh>,
    pub pos: Vec3,
    pub rot: Rotor3,
    pub scale: Vec3,
}
#[derive(Clone)]
pub struct Mesh {
    pub id: u32,
    pub primitives: Vec<Primitive>,
}

#[derive(Clone, Copy)]
pub struct Primitive {
    pub indices: u32,
    pub vertices: u32,
    pub material: MaterialID,
}

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct MaterialID {
    pub source_id: u32,
    pub sampler_id: u32,
}
pub const SIZE_TEXCOORDS: usize = size_of::<[f32; 2]>();
#[derive(Debug)]
#[repr(C)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub texcoords: [f32; 2],
}

pub struct Chunk {
    pub data: Vec<u8>,
}

pub struct Image {
    pub image: VkDestroy<VkImage>,
    pub image_view: VkDestroy<VkImageView>,

    pub data: Vec<u8>,
    pub size: usize,
    pub extent: VkExtent3D,
}

const GLB_MAGIC: &[u8] = b"glTF";
pub fn check_magic(bytes: &[u8]) {
    if !bytes.starts_with(GLB_MAGIC) {
        panic!("Invalid GLTF magic, corrupt scene file");
    };
}

pub fn check_length(bytes: &[u8]) {
    if !u32::from_le_bytes(bytes[8..12].try_into().unwrap()) == bytes.len() as u32 {
        panic!("Invalid length, corrupt scene file");
    }
}

pub fn raw_to_chunks(mut bytes: &[u8]) -> (Chunk, Chunk) {
    let mut json_chunk: Option<Chunk> = None;
    let mut buffer_chunk: Option<Chunk> = None;
    loop {
        if bytes.is_empty() {
            break;
        }
        let chunk_length = u32::from_le_bytes(bytes[..4].try_into().unwrap());
        let last_byte = (chunk_length + 8) as usize;
        let chunk_type = ChunkType::try_from(u32::from_le_bytes(bytes[4..8].try_into().unwrap())).unwrap();
        let mut data: Vec<u8> = Vec::with_capacity(chunk_length as usize);
        data.extend_from_slice(&bytes[8..last_byte]);
        let chunk = Chunk {
            data,
        };

        match chunk_type {
            ChunkType::JSON => {
                json_chunk = Some(chunk);
            }
            ChunkType::BIN => {
                buffer_chunk = Some(chunk);
            }
        }

        bytes = &bytes[last_byte..];
    };
    (json_chunk.expect("Corrupted .gtb, no json section"), buffer_chunk.expect("Corrupted .gtb, no bin section"))
}