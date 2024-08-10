use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    // Get the output directory from the environment variables
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");

    // Define the files to copy
    let files_to_copy = ["config.toml"];

    for file in &files_to_copy {
        // Construct the source and destination paths
        let src_path = PathBuf::from(file);
        let dest_path = PathBuf::from(&out_dir).join(file);

        // Copy the file
        fs::copy(&src_path, &dest_path).expect(&format!("Failed to copy {}", file));
    }
}

