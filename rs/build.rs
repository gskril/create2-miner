use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=src/shader/Keccak256.cu");

    // Only compile CUDA if we're on a system with nvcc
    if Command::new("nvcc").arg("--version").output().is_ok() {
        let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
        let cuda_dir = PathBuf::from("src/shader");

        // Compile CUDA to PTX
        let status = Command::new("nvcc")
            .args(&[
                "--ptx",
                "-arch=sm_50", // Minimum compute capability
                "-o",
                &out_dir.join("Keccak256.ptx").to_str().unwrap(),
                &cuda_dir.join("Keccak256.cu").to_str().unwrap(),
            ])
            .status()
            .expect("Failed to execute nvcc");

        if !status.success() {
            panic!("Failed to compile CUDA shader");
        }

        // Copy PTX file to shader directory
        std::fs::copy(
            out_dir.join("Keccak256.ptx"),
            cuda_dir.join("Keccak256.ptx"),
        ).expect("Failed to copy PTX file");
    } else {
        println!("cargo:warning=nvcc not found, skipping CUDA compilation");
    }
} 