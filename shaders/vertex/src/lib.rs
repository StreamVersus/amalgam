#![no_std]
#![allow(unexpected_cfgs)]

use spirv_std::glam::{Mat4, Vec2, Vec3, Vec4};
use spirv_std::spirv;

pub struct UBO {
    view: Mat4,
    proj: Mat4,
}

#[spirv(vertex)]
pub fn main(
    #[spirv(position)] out_position: &mut Vec4,
        in_position: Vec3,
        _in_normals: Vec3,
        in_tex_coords: Vec2,
        out_tex_coords: &mut Vec2,
        #[spirv(uniform, descriptor_set = 0, binding = 0)] ubo: &UBO) {
    *out_position = ubo.proj * ubo.view * in_position.extend(1.0);
    *out_tex_coords = in_tex_coords;
}