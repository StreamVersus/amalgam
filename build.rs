use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    // PROTO COMPILATION
    println!("cargo:rerun-if-changed=src/engine/cache_storage.proto");
    match prost_build::compile_protos(&["src/engine/cache_storage.proto"], &["src/engine"]) {
        Ok(_) => {}
        Err(e) => panic!("Failed to compile protos: {:?}", e),
    }

    // SHADER COMPILATION STUFF
    let out_dir = env::var("OUT_DIR").unwrap();
    let shader_dir = "src/shaders";
    let compiled_dir = Path::new(&out_dir).join("shaders");

    // Create output directory
    fs::create_dir_all(&compiled_dir).unwrap();

    // Tell cargo to rerun if shader directory changes
    println!("cargo:rerun-if-changed={}", shader_dir);

    let mut shader_module_code = String::new();
    shader_module_code.push_str("// Auto-generated shader module\n\n");

    // Read shader directory
    let entries = fs::read_dir(shader_dir).unwrap();

    for entry in entries {
        let entry = entry.unwrap();
        let path = entry.path();

        if let Some(extension) = path.extension() {
            let ext_str = extension.to_str().unwrap();

            // Check if it's a shader file
            if matches!(ext_str, "vert" | "frag" | "comp" | "geom" | "tesc" | "tese") {
                let file_stem = path.file_stem().unwrap().to_str().unwrap();
                let input_path = path.to_str().unwrap();
                let output_path = compiled_dir.join(format!("{}_{}.spirv", file_stem, ext_str));

                println!("Compiling shader: {} -> {:?}", input_path, output_path);

                let output = Command::new("glslc")
                    .arg(input_path)
                    .arg("-o")
                    .arg(&output_path)
                    .output()
                    .expect("Failed to execute glslc. Make sure Vulkan SDK is installed and glslc is in PATH");

                if !output.status.success() {
                    eprintln!(
                        "Failed to compile shader {}: {}",
                        input_path,
                        String::from_utf8_lossy(&output.stderr)
                    );
                    continue;
                }
                let const_name = format!("{}_{}", file_stem.to_uppercase(), ext_str.to_uppercase());
                shader_module_code.push_str(&format!(
                    "pub const {}: &[u8] = include_bytes!(\"{}\");\n",
                    const_name,
                    output_path.to_str().unwrap()
                ));
            }
        }
    }

    // Write the generated module
    let module_path = Path::new(&out_dir).join("shaders.rs");
    fs::write(module_path, shader_module_code).unwrap();
}