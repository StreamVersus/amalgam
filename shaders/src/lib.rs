#![no_std]
#![allow(unexpected_cfgs)]
#![allow(clippy::too_many_arguments)]

use spirv_std::glam::{Mat4, Vec2, Vec3, Vec4};
use spirv_std::image::Image2d;
use spirv_std::{spirv, RuntimeArray, Sampler};

pub struct UBO {
    view: Mat4,
    proj: Mat4,
}

#[spirv(vertex)]
pub fn main_vert(
    #[spirv(position)] out_position: &mut Vec4,
    in_position: Vec3,
    _in_normals: Vec3,
    in_tex_coords: Vec2,
    out_tex_coords: &mut Vec2,
    #[spirv(flat)] out_instance_index: &mut usize,
    #[spirv(uniform, descriptor_set = 0, binding = 0)] ubo: &UBO,
    #[spirv(storage_buffer, descriptor_set = 1, binding = 3)] models: &[Mat4],
    #[spirv(instance_index)] gl_instance_index: usize) {
    let model: Mat4 = models[gl_instance_index];
    *out_position = ubo.proj * ubo.view * (model * in_position.extend(1.0));

    *out_tex_coords = in_tex_coords;
    *out_instance_index = gl_instance_index;
}

#[repr(C)]
pub struct Material {
    pub source_id: u32,
    pub sampler_id: u32,
}

#[spirv(fragment)]
pub fn main_frag(
    output: &mut Vec4,
    in_tex_coords: Vec2,
    #[spirv(descriptor_set = 1, binding = 0)] textures: &RuntimeArray<Image2d>,
    #[spirv(descriptor_set = 1, binding = 1)] samplers: &RuntimeArray<Sampler>,
    #[spirv(flat)] in_instance_index: usize,
    #[spirv(storage_buffer, descriptor_set = 1, binding = 4)] materials: &[Material],
) {
    let material = &materials[in_instance_index];
    unsafe {
        let color: Vec4 = textures.index(material.source_id as usize).sample(*samplers.index(material.sampler_id as usize), in_tex_coords);
        *output = color;
    }
}
