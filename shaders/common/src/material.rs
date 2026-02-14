#![allow(unused)]
use bytemuck::{bytes_of, Pod, Zeroable};

#[repr(C)]
#[derive(Copy, Clone)]
pub union MaterialData {
    pub diffuse: DiffuseMaterial,
    pub specular: SpecularMaterial,
    pub texture: TextureMaterial,
    pub bytes: [u8; MATERIAL_DATA_SIZE],
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct MaterialBinary {
    pub mat_type: u32,
    pub data: MaterialData,
}

#[repr(u32)]
pub enum Material {
    Diffuse(DiffuseMaterial) = 0,
    Specular(SpecularMaterial) = 1,
    Texture(TextureMaterial) = 2,
}

impl Material {
    pub fn discriminant(&self) -> u32 {
        match self {
            Material::Diffuse(_) => 0,
            Material::Specular(_) => 1,
            Material::Texture(_) => 2,
        }
    }

    pub fn bytes(&self) -> &[u8] {
        match self {
            Material::Diffuse(diff) => diff.bytes(),
            Material::Specular(spec) => spec.bytes(),
            Material::Texture(texture) => texture.bytes(),
        }
    }
}

const fn max_size(a: usize, b: usize) -> usize {
    if a > b { a } else { b }
}

const MATERIAL_DATA_SIZE: usize = max_size(
    size_of::<DiffuseMaterial>(),
    max_size(
        size_of::<SpecularMaterial>(),
        size_of::<TextureMaterial>()
    )
);
#[repr(C)]
#[derive(Copy, Clone)]
pub struct DiffuseMaterial {

}

impl DiffuseMaterial {
    pub fn parse(bytes: &[u8]) -> DiffuseMaterial {
        *bytemuck::from_bytes(&bytes[0..size_of::<DiffuseMaterial>()])
    }

    pub fn bytes(&self) -> &[u8] {
        bytes_of(self)
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct SpecularMaterial {

}

impl SpecularMaterial {
    pub fn parse(bytes: &[u8]) -> SpecularMaterial {
        *bytemuck::from_bytes(&bytes[0..size_of::<SpecularMaterial>()])
    }

    pub fn bytes(&self) -> &[u8] {
        bytes_of(self)
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct TextureMaterial {
    pub source_id: u32,
    pub sampler_id: u32,
}

impl TextureMaterial {
    pub fn parse(bytes: &[u8]) -> TextureMaterial {
        *bytemuck::from_bytes(&bytes[0..size_of::<TextureMaterial>()])
    }

    pub fn bytes(&self) -> &[u8] {
        bytes_of(self)
    }
}


unsafe impl Pod for SpecularMaterial {}
unsafe impl Zeroable for SpecularMaterial {}
unsafe impl Pod for DiffuseMaterial {}
unsafe impl Zeroable for DiffuseMaterial {}
unsafe impl Pod for TextureMaterial {}
unsafe impl Zeroable for TextureMaterial {}
unsafe impl Pod for MaterialData {}
unsafe impl Zeroable for MaterialData {}