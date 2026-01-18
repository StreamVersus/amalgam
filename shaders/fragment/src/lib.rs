#![no_std]
#![allow(unexpected_cfgs)]

use spirv_std::glam::{Vec2, Vec4};
use spirv_std::image::Image2d;
use spirv_std::{spirv, Sampler};

pub const TEXTURE_LIMIT: usize = 2048;
pub const SAMPLER_LIMIT: usize = 16;
#[spirv(fragment)]
pub fn main(
    output: &mut Vec4,
    in_tex_coords: Vec2,
    #[spirv(descriptor_set = 1, binding = 0)] textures: &[Image2d; TEXTURE_LIMIT],
    #[spirv(descriptor_set = 1, binding = 1)] samplers: &[Sampler; SAMPLER_LIMIT],
) {
    let color: Vec4 = textures[0].sample(samplers[0], in_tex_coords);
    *output = color;
}
