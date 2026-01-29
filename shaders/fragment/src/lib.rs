#![no_std]
#![allow(unexpected_cfgs)]
use spirv_std::glam::{vec4, Vec2, Vec4};
use spirv_std::image::Image2d;
use spirv_std::{spirv, Sampler};

pub const TEXTURE_LIMIT: usize = 2048;
pub const SAMPLER_LIMIT: usize = 16;

#[repr(C)]
pub struct Material {
    pub source_id: u32,
    pub sampler_id: u32,
}

#[spirv(fragment)]
pub fn main(
    output: &mut Vec4,
    in_tex_coords: Vec2,
    #[spirv(descriptor_set = 1, binding = 0)] textures: &[Image2d; TEXTURE_LIMIT],
    #[spirv(descriptor_set = 1, binding = 1)] samplers: &[Sampler; SAMPLER_LIMIT],
    #[spirv(flat)] in_instance_index: usize,
    #[spirv(storage_buffer, descriptor_set = 1, binding = 4)] materials: &[Material]
) {
    let material = &materials[in_instance_index];
    let color: Vec4 = textures[material.source_id as usize].sample(samplers[material.sampler_id as usize], in_tex_coords);
    *output = color;
}
