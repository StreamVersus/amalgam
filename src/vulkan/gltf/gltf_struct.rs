#![allow(non_snake_case)]
use serde::Deserialize;
#[derive(Debug, Deserialize)]
pub struct Gltf {
    pub asset: Asset,
    pub scene: u32,
    pub scenes: Vec<Scene>,
    pub nodes: Vec<Node>,
    pub materials: Vec<Material>,
    pub meshes: Vec<Mesh>,
    pub textures: Vec<Texture>,
    pub images: Vec<Image>,
    pub accessors: Vec<Accessor>,
    pub bufferViews: Vec<BufferView>,
    pub samplers: Vec<Sampler>,
}

#[derive(Debug, Deserialize)]
pub struct Asset {
    pub generator: String,
    pub version: String,
}

#[derive(Debug, Default, Deserialize)]
pub struct Scene {
    pub name: String,
    pub nodes: Vec<u32>,
}

#[derive(Debug, Deserialize)]
pub struct Node {
    pub mesh: Option<u32>,
    pub children: Option<Vec<u32>>,
    pub name: String,
    pub translation: Option<[f32; 3]>,
    pub rotation: Option<[f32; 4]>,
    pub scale: Option<[f32; 3]>,
}

#[derive(Debug, Deserialize)]
pub struct Material {
    pub doubleSided: Option<bool>,
    pub name: String,
    pub pbrMetallicRoughness: Option<MetallicRoughness>,
}

#[derive(Debug, Deserialize)]
pub struct MetallicRoughness {
    pub baseColorFactor: Option<[f32; 4]>,
    pub metallicFactor: Option<f32>,
    pub roughnessFactor: Option<f32>,
    pub baseColorTexture: Option<BaseColorTexture>
}

#[derive(Debug, Deserialize)]
pub struct BaseColorTexture {
    pub index: u32,
}

#[derive(Debug, Deserialize)]
pub struct Mesh {
    pub name: String,
    pub primitives: Vec<Primitive>,
}

#[derive(Debug, Deserialize)]
pub struct Primitive {
    pub attributes: Attributes,
    pub indices: u32,
    pub material: Option<u32>,
}

#[derive(Debug, Deserialize, Eq, PartialEq, Copy, Clone, Hash)]
pub struct Attributes {
    pub POSITION: u32,
    pub NORMAL: u32,
    pub TEXCOORD_0: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct Texture {
    pub source: u32,
    pub sampler: u32,
}

#[derive(Debug, Deserialize)]
pub struct Image {
    pub bufferView: u16,
    pub mimeType: String,
    pub name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Accessor {
    pub bufferView: u32,
    pub componentType: u32,
    pub count: u32,
    pub max: Option<Vec<f32>>,
    pub min: Option<Vec<f32>>,
    pub r#type: String,
}

#[derive(Debug, Deserialize)]
pub struct BufferView {
    pub buffer: u32,
    pub byteLength: u32,
    pub byteOffset: Option<u32>,
    pub target: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct Sampler {
    pub magFilter: u32,
    pub minFilter: u32,
    pub wrapS: Option<u32>,
    pub wrapT: Option<u32>,
}