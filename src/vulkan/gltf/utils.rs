use crate::engine::vbo::VBO;
use crate::vulkan::func::{Destructible, Vulkan};
use crate::vulkan::gltf::gltf_struct::{Attributes, Gltf};
use crate::vulkan::r#impl::memory::AllocationTask;
use crate::vulkan::r#impl::memory::MemoryInfo;
use crate::vulkan::r#impl::sampler::SamplerInfo;
use crate::vulkan::utils::BufferUsage;
use vulkan_raw::{VkBorderColor, VkBuffer, VkCompareOp, VkFilter, VkFormat, VkSampler, VkSamplerAddressMode, VkSamplerMipmapMode};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ImageFormat {
    Jpeg,
    Png,
    Bmp,
    Gif,
    Tiff,
    WebP,
    Unknown,
}

impl Into<VkFormat> for ImageFormat {
    fn into(self) -> VkFormat {
        match self {
            ImageFormat::Jpeg => VkFormat::R8G8B8_UNORM,
            _ => VkFormat::R8G8B8A8_UNORM,
        }
    }
}
impl From<String> for ImageFormat {
    fn from(mime_type: String) -> Self {
        match mime_type.as_str() {
            "image/jpeg" => ImageFormat::Jpeg,
            "image/png" => ImageFormat::Png,
            "image/bmp" => ImageFormat::Bmp,
            "image/gif" => ImageFormat::Gif,
            "image/tiff" => ImageFormat::Tiff,
            "image/webp" => ImageFormat::WebP,
            _ => ImageFormat::Unknown,
        }
    }
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ChunkType {
    JSON = 0x4E4F534A, // JSON in ASCII
    BIN = 0x004E4942, // BIN in ASCII
}

impl TryFrom<u32> for ChunkType {
    type Error = ();

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        if value == ChunkType::JSON as u32 {
            Ok(ChunkType::JSON)
        } else if value == ChunkType::BIN as u32 {
            Ok(ChunkType::BIN)
        } else {
            panic!("Unknown chunk type {}", value)
        }
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct IndirectParameters {
    pub index_count: u32,
    pub instance_count: u32,
    pub first_index: u32,
    pub vertex_offset: u32,
    pub first_instance: u32,
}

pub fn resolve_size(gltf: &Gltf, accessor_id: u32) -> u32 {
    gltf.bufferViews[gltf.accessors[accessor_id as usize].bufferView as usize].byteLength
}

pub fn resolve_offset(gltf: &Gltf, accessor_id: u32) -> u32 {
    gltf.bufferViews[gltf.accessors[accessor_id as usize].bufferView as usize].byteOffset
}

pub fn resolve_amount(gltf: &Gltf, accessor_id: u32) -> u32 {
    gltf.accessors[accessor_id as usize].count
}

pub fn resolve_vertices(gltf: &Gltf, attr: Attributes) -> u32 {
    let position_amount = resolve_amount(gltf, attr.POSITION);
    let normal_amount = resolve_amount(gltf, attr.NORMAL);

    if position_amount == normal_amount {
        position_amount
    } else {
        panic!("Corrupted json");
    }
}

const GL_NEAREST: u32 = 0x2600;
const GL_LINEAR: u32 = 0x2601;
const GL_NEAREST_MIPMAP_NEAREST: u32 = 0x2700;
const GL_LINEAR_MIPMAP_NEAREST: u32 = 0x2701;
const GL_NEAREST_MIPMAP_LINEAR: u32 = 0x2702;
const GL_LINEAR_MIPMAP_LINEAR: u32 = 0x2703;

pub fn resolve_opengl_magfilter(constant: u32, info: &mut SamplerInfo) {
    let mag_filter = match constant {
        GL_NEAREST => VkFilter::NEAREST,
        GL_LINEAR => VkFilter::LINEAR,
        _ => panic!("Unknown constants"),
    };
    info.mag_filter = mag_filter;
}

pub fn resolve_opengl_minfilter(constant: u32, info: &mut SamplerInfo) {
    let (min_filter, mipmap_mode) = match constant {
        GL_NEAREST | GL_NEAREST_MIPMAP_NEAREST => (VkFilter::NEAREST, VkSamplerMipmapMode::NEAREST),
        GL_LINEAR | GL_LINEAR_MIPMAP_NEAREST => (VkFilter::LINEAR, VkSamplerMipmapMode::NEAREST),
        GL_NEAREST_MIPMAP_LINEAR => (VkFilter::NEAREST, VkSamplerMipmapMode::LINEAR),
        GL_LINEAR_MIPMAP_LINEAR => (VkFilter::LINEAR, VkSamplerMipmapMode::LINEAR),
        _ => panic!("Unknown constants"),
    };
    info.min_filter = min_filter;
    info.mipmap_mode = mipmap_mode;
}
const GL_CLAMP_TO_EDGE: u32 = 0x812F;
const GL_MIRRORED_REPEAT: u32 = 0x8370;
const GL_REPEAT: u32 = 0x2901;
pub fn resolve_opengl_wrap(constant: u32) -> VkSamplerAddressMode {
    match constant {
        GL_CLAMP_TO_EDGE => VkSamplerAddressMode::CLAMP_TO_EDGE,
        GL_MIRRORED_REPEAT => VkSamplerAddressMode::MIRRORED_REPEAT,
        GL_REPEAT => VkSamplerAddressMode::REPEAT,
        _ => panic!("Unknown constants"),
    }
}


pub fn resolve_opengl_wraps(wrap_s: Option<u32>, wrap_t: Option<u32>, info: &mut SamplerInfo) {
    if let Some(wrap_s) = wrap_s {
        info.address_mode_u = resolve_opengl_wrap(wrap_s);
    }
    if let Some(wrap_t) = wrap_t {
        info.address_mode_v = resolve_opengl_wrap(wrap_t);
    }
    info.address_mode_w = VkSamplerAddressMode::REPEAT; // default for glTF
}

pub fn resolve_vertex(gltf: &Gltf, attr: Attributes, id: usize, data: &[u8], vbo: &mut VBO) {
    let pos_offset = resolve_offset(gltf, attr.POSITION) as usize;
    let norm_offset = resolve_offset(gltf, attr.NORMAL) as usize;
    let position = unsafe {
        let ptr = data.as_ptr().add(pos_offset + id * size_of::<[f32; 3]>()) as *const [f32; 3];
        *ptr
    };
    let normal = unsafe {
        let ptr = data.as_ptr().add(norm_offset + id * size_of::<[f32; 3]>()) as *const [f32; 3];
        *ptr
    };

    let texcoords = if let Some(tex_id) = attr.TEXCOORD_0 {
        unsafe {
            let texcoord_offset = resolve_offset(gltf, tex_id) as usize;
            let texcoord_ptr = data.as_ptr().add(texcoord_offset).add(id * size_of::<[f32; 2]>()) as *const [f32; 2];
            *texcoord_ptr
        }
    } else {
        [0.0f32, 0.0f32]
    };

    vbo.build_vertex_inplace(position, normal, texcoords);
}

pub fn read_samplers(vulkan: &Vulkan, gltf: &Gltf) -> Vec<VkSampler> {
    gltf.samplers.iter().map(|sampler| {
        let mut sampler_info = SamplerInfo {
            mip_lod_bias: 0.0,
            anisotropy_enable: false,
            max_anisotropy: 0.0,
            comparison_enable: false,
            compare_op: VkCompareOp::NEVER,
            min_lod: 0.0,
            max_lod: 0.0,
            border_color: VkBorderColor::INT_OPAQUE_BLACK,
            unnormalized_coordinates: false,
            ..Default::default()
        };
        resolve_opengl_magfilter(sampler.magFilter, &mut sampler_info);
        resolve_opengl_minfilter(sampler.minFilter, &mut sampler_info);
        resolve_opengl_wraps(sampler.wrapT, sampler.wrapS, &mut sampler_info);

        vulkan.create_sampler(sampler_info)
    }).collect()
}
pub struct StagingBuffer {
    size: u64,
    buffer: VkBuffer,
    info: MemoryInfo,
}

impl Destructible for StagingBuffer {
    fn destroy(&self, vulkan: &Vulkan) {
        self.buffer.destroy(vulkan);
        self.info.memory_object.destroy(vulkan);
    }
}

impl StagingBuffer {
    pub fn new() -> Self {
        Self {
            size: 0,
            buffer: VkBuffer::none(),
            info: Default::default(),
        }
    }
    
    pub fn info(&self) -> &MemoryInfo {
        &self.info
    }
    
    pub fn pull(&mut self, size: u64, vulkan: &Vulkan) -> VkBuffer {
        if self.size < size {
            let buffer = vulkan.create_buffer(size, BufferUsage::preset_staging()).unwrap();
            self.buffer.destroy(vulkan);
            self.buffer = buffer;
            self.info = AllocationTask::host_coherent().add_allocatable(buffer).allocate_all(vulkan).get_all_info()[0];
        }

        self.buffer
    }
}