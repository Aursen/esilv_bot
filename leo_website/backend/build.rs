use std::{path::Path, process::Command};

const FRONTEND_DIR: &str = "../frontend";

fn main() {
    println!("cargo:rerun-if-changed={}/src", FRONTEND_DIR);

    build_frontend(FRONTEND_DIR);
}

fn build_frontend<P: AsRef<Path>>(source: P) {
    if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(["/C", "npm", "run", "build"])
            .current_dir(source.as_ref())
            .status()
            .expect("Failed to build frontend");
    } else {
        Command::new("sh")
            .args(["-c", "npm", "run", "build"])
            .current_dir(source.as_ref())
            .status()
            .expect("Failed to build frontend");
    };
}
