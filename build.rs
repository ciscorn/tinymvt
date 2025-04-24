use std::{env, io::Result, path::Path};

fn main() -> Result<()> {
    // build vector_tile.rs from vector_tile.proto
    if !Path::new("src/vector_tile.rs").exists() {
        let prev_out_dir = env::var("OUT_DIR");

        unsafe {
            env::set_var("OUT_DIR", "src/");
        }
        prost_build::compile_protos(&["src/vector_tile.proto"], &["src"])?;

        if let Ok(prev) = prev_out_dir {
            unsafe {
                env::set_var("OUT_DIR", prev);
            }
        }
    }
    Ok(())
}
