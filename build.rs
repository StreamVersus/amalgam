use spirv_builder::SpirvBuilder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=build.rs");

    // PROTO COMPILATION
    println!("cargo:rerun-if-changed=src/engine/cache_storage.proto");
    match prost_build::compile_protos(&["src/engine/cache_storage.proto"], &["src/engine"]) {
        Ok(_) => {}
        Err(e) => panic!("Failed to compile protos: {e:?}"),
    }

    // SHADER COMPILATION
    let target = "spirv-unknown-vulkan1.3";
    let mut fragment_builder = SpirvBuilder::new("shaders/fragment", target);
    fragment_builder.build_script.defaults = true;
    fragment_builder.build_script.forward_rustc_warnings = Some(true);
    fragment_builder.build_script.env_shader_spv_path = Some(true);
    fragment_builder.build()?;

    let mut vertex_builder = SpirvBuilder::new("shaders/vertex", target);
    vertex_builder.build_script.defaults = true;
    vertex_builder.build_script.forward_rustc_warnings = Some(true);
    vertex_builder.build_script.env_shader_spv_path = Some(true);
    vertex_builder.build()?;
    Ok(())
}