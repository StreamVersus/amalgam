use crate::vulkan::func::{bool_to_vkbool, Destructible, Vulkan};
use std::ptr::null_mut;
use vulkan_raw::{vkCreateSampler, vkDestroySampler, VkBorderColor, VkCompareOp, VkFilter, VkResult, VkSampler, VkSamplerAddressMode, VkSamplerCreateInfo, VkSamplerMipmapMode};

impl Vulkan {
    pub fn create_sampler(&self, info: SamplerInfo) -> VkSampler {
        let mut sampler = VkSampler::none();
        let result = unsafe { vkCreateSampler(self.get_loaded_device().logical_device, &info.into(), null_mut(), &mut sampler) };
        assert_eq!(result, VkResult::SUCCESS);

        sampler
    }

    fn destroy_sampler(&self, sampler: VkSampler) {
        unsafe { vkDestroySampler(self.get_loaded_device().logical_device, sampler, null_mut()) };
    }
}

pub struct SamplerInfo {
    pub min_filter: VkFilter,
    pub mag_filter: VkFilter,
    pub mipmap_mode: VkSamplerMipmapMode,
    pub address_mode_u: VkSamplerAddressMode,
    pub address_mode_v: VkSamplerAddressMode,
    pub address_mode_w: VkSamplerAddressMode,
    pub mip_lod_bias: f32,
    pub anisotropy_enable: bool,
    pub max_anisotropy: f32,
    pub comparison_enable: bool,
    pub compare_op: VkCompareOp,
    pub min_lod: f32,
    pub max_lod: f32,
    pub border_color: VkBorderColor,
    pub unnormalized_coordinates: bool,
}

impl Default for SamplerInfo {
    fn default() -> Self {
        SamplerInfo {
            min_filter: VkFilter::NEAREST,
            mag_filter: VkFilter::NEAREST,
            mipmap_mode: VkSamplerMipmapMode::NEAREST,
            address_mode_u: VkSamplerAddressMode::REPEAT,
            address_mode_v: VkSamplerAddressMode::REPEAT,
            address_mode_w: VkSamplerAddressMode::REPEAT,
            mip_lod_bias: 0.0,
            anisotropy_enable: false,
            max_anisotropy: 0.0,
            comparison_enable: false,
            compare_op: VkCompareOp::NEVER,
            min_lod: 0.0,
            max_lod: 0.0,
            border_color: VkBorderColor::FLOAT_TRANSPARENT_BLACK,
            unnormalized_coordinates: false,
        }
    }
}

impl Into<VkSamplerCreateInfo> for SamplerInfo {
    fn into(self) -> VkSamplerCreateInfo {
        VkSamplerCreateInfo {
            magFilter: self.mag_filter,
            minFilter: self.min_filter,
            mipmapMode: self.mipmap_mode,
            addressModeU: self.address_mode_u,
            addressModeV: self.address_mode_v,
            addressModeW: self.address_mode_w,
            mipLodBias: self.mip_lod_bias,
            anisotropyEnable: bool_to_vkbool(self.anisotropy_enable),
            maxAnisotropy: self.max_anisotropy,
            compareEnable: bool_to_vkbool(self.comparison_enable),
            compareOp: self.compare_op,
            minLod: self.min_lod,
            maxLod: self.max_lod,
            borderColor: self.border_color,
            unnormalizedCoordinates: bool_to_vkbool(self.unnormalized_coordinates),
            ..Default::default()
        }
    }
}

impl Destructible for VkSampler {
    fn destroy(&self, vulkan: &Vulkan) {
        vulkan.destroy_sampler(*self);
    }
}