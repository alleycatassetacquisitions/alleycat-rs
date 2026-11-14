fn main() -> Result<(), Box<dyn std::error::Error>> {
    let protoc = protoc_bin_vendored::protoc_bin_path()?;

    let mut prost_config = prost_build::Config::new();
    prost_config.protoc_executable(protoc);
    prost_config.type_attribute(
        ".alleycat.device.QuickdrawMatch",
        "#[derive(serde::Serialize, utoipa::ToSchema)]",
    );
    prost_config.type_attribute(
        ".alleycat.device.PdnComponentChecks",
        "#[derive(serde::Serialize, utoipa::ToSchema)]",
    );
    prost_config.include_file("_includes.rs");
    prost_config.compile_protos(&["proto/device_api.proto"], &["proto"])?;

    println!("cargo:rerun-if-changed=proto/device_api.proto");

    Ok(())
}
