use spirv_builder::{MetadataPrintout, SpirvBuilder};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=build.rs");

    // PROTO COMPILATION
    println!("cargo:rerun-if-changed=src/engine/cache_storage.proto");
    match prost_build::compile_protos(&["src/engine/cache_storage.proto"], &["src/engine"]) {
        Ok(_) => {}
        Err(e) => panic!("Failed to compile protos: {e:?}"),
    }

    let target = "spirv-unknown-vulkan1.3";
    SpirvBuilder::new("shaders/fragment", target)
        .print_metadata(MetadataPrintout::Full)
        .build()?;

    SpirvBuilder::new("shaders/vertex", target)
            .print_metadata(MetadataPrintout::Full)
            .build()?;

    Ok(())
}