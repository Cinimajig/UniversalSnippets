fn main() -> std::io::Result<()> {
    println!("cargo:rerun-if-changed=Assets/manifest.xml");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/Cargo.toml");

    winres::WindowsResource::new()
    .set_manifest_file("Assets/manifest.xml")
    .compile()?;

    Ok(())
}