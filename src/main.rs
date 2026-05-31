// src/main.rs
mod compiler;
mod diagnostics;
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
            _ => {
                if file_path.is_none() {
                    file_path = Some(arg);
                } else {
                    eprintln!("unexpected argument: {}", arg);
                    std::process::exit(1);
                }
            }
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
    // compile() uses () as its error type (diagnostics are already printed inside),
    // so we convert failure to a process exit rather than propagating through anyhow
    if compiler::compile(file, opts).is_err() {
        std::process::exit(1);
    }

    Ok(())
}
