use spirv_builder::{Capability, SpirvBuilder};
use std::thread;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/engine/cache_storage.proto");

    match prost_build::compile_protos(&["src/engine/cache_storage.proto"], &["src/engine"]) {
        Ok(_) => {}
        Err(e) => panic!("Failed to compile protos: {e:?}"),
    }

    let fragment = thread::spawn(|| -> Result<_, Box<dyn std::error::Error + Send + Sync>> {
        let mut b = SpirvBuilder::new("shaders/fragment", "spirv-unknown-vulkan1.3");
        b.capabilities.push(Capability::RuntimeDescriptorArray);
        b.build_script.defaults = true;
        b.build_script.forward_rustc_warnings = Some(true);
        b.build_script.env_shader_spv_path = Some(true);
        b.build()?;
        Ok(())
    });

    let vertex = thread::spawn(|| -> Result<_, Box<dyn std::error::Error + Send + Sync>> {
        let mut b = SpirvBuilder::new("shaders/vertex", "spirv-unknown-vulkan1.3");
        b.build_script.defaults = true;
        b.build_script.forward_rustc_warnings = Some(true);
        b.build_script.env_shader_spv_path = Some(true);
        b.build()?;
        Ok(())
    });

    fragment.join().unwrap().map_err(|e| e.to_string())?;
    vertex.join().unwrap().map_err(|e| e.to_string())?;

    Ok(())
}
