pub const FRAGMENT_SHADER: &[u8] = include_bytes!(env!("fragment.spv"));
pub const VERTEX_SHADER: &[u8] = include_bytes!(env!("vertex.spv"));

pub const TEXTURE_LIMIT: usize = fragment::TEXTURE_LIMIT;
pub const SAMPLER_LIMIT: usize = fragment::SAMPLER_LIMIT;