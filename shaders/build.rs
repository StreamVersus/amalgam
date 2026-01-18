use spirv_builder::{MetadataPrintout, SpirvBuilder};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let target = "spirv-unknown-vulkan1.3";
    SpirvBuilder::new("fragment", target)
        .print_metadata(MetadataPrintout::Full)
        .build()?;

    SpirvBuilder::new("vertex", target)
        .print_metadata(MetadataPrintout::Full)
        .build()?;
    Ok(())
}