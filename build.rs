fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
        embed_resource::compile("resources.rc", &[] as &[&str]);
        println!("cargo:rerun-if-changed=resources.rc");
        println!("cargo:rerun-if-changed=icon.ico");
    }
}
