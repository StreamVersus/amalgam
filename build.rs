fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    // PROTO COMPILATION
    println!("cargo:rerun-if-changed=src/engine/cache_storage.proto");
    match prost_build::compile_protos(&["src/engine/cache_storage.proto"], &["src/engine"]) {
        Ok(_) => {}
        Err(e) => panic!("Failed to compile protos: {:?}", e),
    }
}