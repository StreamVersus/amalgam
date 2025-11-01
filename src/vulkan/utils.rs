use std::ffi::{c_char, CStr};
use ultraviolet::{Mat3, Mat4, Vec3, Vec4};
use vulkan_raw::{VkBufferUsageFlagBits, VkBufferUsageFlags, VkDescriptorPoolSize, VkDescriptorSetLayoutBinding, VkImageUsageFlagBits, VkImageUsageFlags};

#[macro_export]
macro_rules! safe_ptr {
    ($val:expr) => {
        if $val.len() > 0 {
            $val.as_ptr()
        } else {
            null_mut()
        }
    };
}

#[macro_export]
macro_rules! safe_mut_ptr {
    ($val:expr) => {
        if $val.len() > 0 {
            $val.as_mut_ptr()
        } else {
            null_mut()
        }
    };
}

#[macro_export]
macro_rules! null_if_none {
    ($val:expr) => {
        match $val {
            Some(val) => &val,
            None => null_mut(),
        }
    };
}
#[derive(Default, Debug, Clone, PartialEq)]
pub struct BufferUsage {
    transfer_src: bool,
    transfer_dst: bool,
    uniform_texel_buffer: bool,
    storage_texel_buffer: bool,
    uniform_buffer: bool,
    storage_buffer: bool,
    index_buffer: bool,
    vertex_buffer: bool,
    indirect_buffer: bool,
    shader_device_address: bool,
}

impl BufferUsage {
    pub fn transfer_src(mut self, enabled: bool) -> Self {
        self.transfer_src = enabled;
        self
    }

    pub fn transfer_dst(mut self, enabled: bool) -> Self {
        self.transfer_dst = enabled;
        self
    }

    pub fn uniform_texel_buffer(mut self, enabled: bool) -> Self {
        self.uniform_texel_buffer = enabled;
        self
    }

    pub fn storage_texel_buffer(mut self, enabled: bool) -> Self {
        self.storage_texel_buffer = enabled;
        self
    }

    pub fn uniform_buffer(mut self, enabled: bool) -> Self {
        self.uniform_buffer = enabled;
        self
    }

    pub fn storage_buffer(mut self, enabled: bool) -> Self {
        self.storage_buffer = enabled;
        self
    }

    pub fn index_buffer(mut self, enabled: bool) -> Self {
        self.index_buffer = enabled;
        self
    }

    pub fn vertex_buffer(mut self, enabled: bool) -> Self {
        self.vertex_buffer = enabled;
        self
    }

    pub fn indirect_buffer(mut self, enabled: bool) -> Self {
        self.indirect_buffer = enabled;
        self
    }

    pub fn shader_device_address(mut self, enabled: bool) -> Self {
        self.shader_device_address = enabled;
        self
    }

    pub fn preset_fat() -> Self {
        Self::default().index_buffer(true).vertex_buffer(true).indirect_buffer(true).transfer_dst(true)
    }

    pub fn preset_vertex() -> Self {
        Self::default().vertex_buffer(true).transfer_dst(true)
    }
    pub fn preset_index() -> Self {
        Self::default().index_buffer(true).transfer_dst(true)
    }

    pub fn preset_uniform_storage() -> Self {
        Self::default().uniform_buffer(true).storage_buffer(true)
    }

    pub fn preset_staging() -> Self {
        Self::default().transfer_src(true)
    }

    pub fn is_transfer_src(&self) -> bool { self.transfer_src }
    pub fn is_transfer_dst(&self) -> bool { self.transfer_dst }
    pub fn is_uniform_texel_buffer(&self) -> bool { self.uniform_texel_buffer }
    pub fn is_storage_texel_buffer(&self) -> bool { self.storage_texel_buffer }
    pub fn is_uniform_buffer(&self) -> bool { self.uniform_buffer }
    pub fn is_storage_buffer(&self) -> bool { self.storage_buffer }
    pub fn is_index_buffer(&self) -> bool { self.index_buffer }
    pub fn is_vertex_buffer(&self) -> bool { self.vertex_buffer }
    pub fn is_indirect_buffer(&self) -> bool { self.indirect_buffer }
    pub fn is_shader_device_address(&self) -> bool { self.shader_device_address }
}

impl From<BufferUsage> for VkBufferUsageFlags {
    fn from(usage: BufferUsage) -> Self {
        let mut flags = VkBufferUsageFlags::empty();

        if usage.transfer_src { flags |= VkBufferUsageFlagBits::TRANSFER_SRC_BIT; }
        if usage.transfer_dst { flags |= VkBufferUsageFlagBits::TRANSFER_DST_BIT; }
        if usage.uniform_texel_buffer { flags |= VkBufferUsageFlagBits::UNIFORM_TEXEL_BUFFER_BIT; }
        if usage.storage_texel_buffer { flags |= VkBufferUsageFlagBits::STORAGE_TEXEL_BUFFER_BIT; }
        if usage.uniform_buffer { flags |= VkBufferUsageFlagBits::UNIFORM_BUFFER_BIT; }
        if usage.storage_buffer { flags |= VkBufferUsageFlagBits::STORAGE_BUFFER_BIT; }
        if usage.index_buffer { flags |= VkBufferUsageFlagBits::INDEX_BUFFER_BIT; }
        if usage.vertex_buffer { flags |= VkBufferUsageFlagBits::VERTEX_BUFFER_BIT; }
        if usage.indirect_buffer { flags |= VkBufferUsageFlagBits::INDIRECT_BUFFER_BIT; }
        if usage.shader_device_address { flags |= VkBufferUsageFlagBits::SHADER_DEVICE_ADDRESS_BIT; }

        flags
    }
}

#[derive(Default, Debug, Clone, PartialEq)]
pub struct ImageUsage {
    transfer_src: bool,
    transfer_dst: bool,
    sampled: bool,
    storage: bool,
    color_attachment: bool,
    depth_stencil_attachment: bool,
    transient_attachment: bool,
    input_attachment: bool,
}

impl ImageUsage {
    pub fn transfer_src(mut self, transfer_src: bool) -> Self{
        self.transfer_src = transfer_src;
        self
    }

    pub fn transfer_dst(mut self, transfer_dst: bool) -> Self {
        self.transfer_dst = transfer_dst;
        self
    }

    pub fn sampled(mut self, sampled: bool) -> Self {
        self.sampled = sampled;
        self
    }

    pub fn storage(mut self, storage: bool) -> Self {
        self.storage = storage;
        self
    }

    pub fn color_attachment(mut self, color_attachment: bool) -> Self {
        self.color_attachment = color_attachment;
        self
    }

    pub fn depth_stencil_attachment(mut self, depth_stencil_attachment: bool) -> Self {
        self.depth_stencil_attachment = depth_stencil_attachment;
        self
    }

    pub fn transient_attachment(mut self, transient_attachment: bool) -> Self {
        self.transient_attachment = transient_attachment;
        self
    }

    pub fn input_attachment(mut self, input_attachment: bool) -> Self {
        self.input_attachment = input_attachment;
        self
    }

    pub fn is_transfer_src(&self) -> bool {
        self.transfer_src
    }

    pub fn is_transfer_dst(&self) -> bool {
        self.transfer_dst
    }

    pub fn is_sampled(&self) -> bool {
        self.sampled
    }

    pub fn is_storage(&self) -> bool {
        self.storage
    }

    pub fn is_color_attachment(&self) -> bool {
        self.color_attachment
    }

    pub fn is_depth_stencil_attachment(&self) -> bool {
        self.depth_stencil_attachment
    }

    pub fn is_transient_attachment(&self) -> bool {
        self.transient_attachment
    }

    pub fn is_input_attachment(&self) -> bool {
        self.input_attachment
    }
}

impl From<ImageUsage> for VkImageUsageFlags {
    fn from(usage: ImageUsage) -> Self {
        let mut flags = VkImageUsageFlags::empty();

        if usage.transfer_src { flags |= VkImageUsageFlagBits::TRANSFER_SRC_BIT; }
        if usage.transfer_dst { flags |= VkImageUsageFlagBits::TRANSFER_DST_BIT; }
        if usage.sampled { flags |= VkImageUsageFlagBits::SAMPLED_BIT; }
        if usage.storage { flags |= VkImageUsageFlagBits::STORAGE_BIT; }
        if usage.color_attachment { flags |= VkImageUsageFlagBits::COLOR_ATTACHMENT_BIT; }
        if usage.depth_stencil_attachment  { flags |= VkImageUsageFlagBits::DEPTH_STENCIL_ATTACHMENT_BIT; }
        if usage.transient_attachment { flags |= VkImageUsageFlagBits::TRANSIENT_ATTACHMENT_BIT; }
        if usage.input_attachment  { flags |= VkImageUsageFlagBits::INPUT_ATTACHMENT_BIT; }

        flags
    }
}
#[inline(always)]
pub fn align_up(value: u64, alignment: u64) -> u64 {
    (value + alignment - 1) & !(alignment - 1)
}

#[inline(always)]
pub fn clamp(value: u32, min: u32, max: u32) -> u32 {
    value.min(max).max(min)
}

#[inline(always)]
pub fn array_to_string(arr: &[c_char; 256]) -> String {
    unsafe { CStr::from_ptr(arr.as_ptr()) }.to_str().unwrap().to_string()
}

#[inline(always)]
pub fn null_terminated_string(str: &[c_char; 256]) -> String {
    format!("{}\0", array_to_string(str))
}

#[inline(always)]
pub fn null_terminated_str(str: &str) -> String {
    format!("{}\0", str.to_string())
}

pub fn create_model_matrix(rot_mat: Mat3, pos: Vec3) -> Mat4 {
    Mat4::new(
        Vec4::new(rot_mat.cols[0].x, rot_mat.cols[0].y, rot_mat.cols[0].z, 0.0),
        Vec4::new(rot_mat.cols[1].x, rot_mat.cols[1].y, rot_mat.cols[1].z, 0.0),
        Vec4::new(rot_mat.cols[2].x, rot_mat.cols[2].y, rot_mat.cols[2].z, 0.0),
        Vec4::new(pos.x, pos.y, pos.z, 1.0),
    )
}

pub fn build_pool_size(bindings: &[VkDescriptorSetLayoutBinding]) -> Vec<VkDescriptorPoolSize> {
    bindings.into_iter().map(|binding| {
        VkDescriptorPoolSize {
            descriptorType: binding.descriptorType,
            descriptorCount: binding.descriptorCount,
        }
    }).collect()
}

pub fn max_of_slice(data: &[u64]) -> u64 {
    *data.into_iter().max().unwrap()
}