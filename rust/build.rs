use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=app.rc");
    println!("cargo:rerun-if-changed=app.manifest");
    println!("cargo:rerun-if-changed=../assets/antigravity-icon.ico");

    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR missing");
    let res_obj = format!("{out_dir}/app_res.o");

    let status = Command::new("windres")
        .args(["app.rc", "-O", "coff", "-o", &res_obj])
        .status()
        .expect("Failed to execute windres");
    assert!(status.success(), "windres failed");

    println!("cargo:rustc-link-arg={res_obj}");
}
