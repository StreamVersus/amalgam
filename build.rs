use spirv_builder::{Capability, SpirvBuilder};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/engine/cache_storage.proto");

    match prost_build::compile_protos(&["src/engine/cache_storage.proto"], &["src/engine"]) {
        Ok(_) => {}
        Err(e) => panic!("Failed to compile protos: {e:?}"),
    }

    let mut shader_blob = SpirvBuilder::new("shaders", "spirv-unknown-vulkan1.3");
    shader_blob.capabilities.push(Capability::RuntimeDescriptorArray);
    shader_blob.build_script.defaults = true;
    shader_blob.build_script.forward_rustc_warnings = Some(true);
    shader_blob.build_script.env_shader_spv_path = Some(true);
    shader_blob.build()?;


    Ok(())
}
