fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto_file = "./proto/store.proto";

    tonic_build::configure()
        .protoc_arg("--experimental_allow_proto3_optional") // for older systems
        .build_client(true)
        .build_server(true)
        .out_dir("./src")
        .compile(&[proto_file], &["."])?;

    Ok(())
}
