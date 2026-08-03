fn main() -> Result<(), Box<dyn std::error::Error>> {
    let protoc = protoc_bin_vendored::protoc_bin_path()?;

    let mut config = prost_build::Config::new();
    config.protoc_executable(protoc);
    config.include_file("_includes.rs");
    config.compile_protos(&["proto/device_api.proto"], &["proto"])?;

    println!("cargo:rerun-if-changed=proto/device_api.proto");

    Ok(())
}
