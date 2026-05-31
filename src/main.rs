// src/main.rs
mod compiler;
mod error;
mod lexer;
mod load_source;
mod parser;

use compiler::CompileOptions;
use std::env;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();

    // parse flags and file path from args
    let mut opts = CompileOptions::default();
    let mut file_path: Option<&str> = None;

    for arg in &args[1..] {
        match arg.as_str() {
            "--release" => opts.release = true,
            "--emit-ir" => opts.emit_ir = true,
            _ => file_path = Some(arg),
        }
    }

    let path = match file_path {
        Some(p) => p,
        None => {
            eprintln!("usage: citrus [--release] [--emit-ir] <file.citrus>");
            std::process::exit(1);
        }
    };

    // load file
    let file = load_source::load(path)?;

    // runs the compiler full pipeline
    compiler::compile(file, opts)?;

    Ok(())
}
