#![no_std]
#![allow(unexpected_cfgs)]

use spirv_std::glam::{Vec2, Vec4};
use spirv_std::{spirv, RuntimeArray, Sampler};
use spirv_std::image::Image2d;

#[repr(C)]
pub struct Material {
    pub source_id: u32,
    pub sampler_id: u32,
}

#[spirv(fragment)]
pub fn main(
    output: &mut Vec4,
    in_tex_coords: Vec2,
    #[spirv(descriptor_set = 1, binding = 0)] textures: &RuntimeArray<Image2d>,
    #[spirv(descriptor_set = 1, binding = 1)] samplers: &RuntimeArray<Sampler>,
    #[spirv(flat)] in_instance_index: usize,
    #[spirv(storage_buffer, descriptor_set = 1, binding = 4)] materials: &[Material]
) {
    let material = &materials[in_instance_index];
    unsafe {
        let color: Vec4 = textures.index(material.source_id as usize).sample(*samplers.index(material.sampler_id as usize), in_tex_coords);
        *output = color;
    }
}
