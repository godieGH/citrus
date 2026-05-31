// src/compiler.rs
use crate::lexer;
use crate::load_source::SourceFile;
use crate::parser;
// use crate::semantics;
// use crate::llvm;

#[derive(Debug)]
/// Options passed to the compiler —
/// grows as we add flags like --release
pub struct CompileOptions {
    pub release: bool, // enables LLVM optimizations
    pub emit_ir: bool, // dumps LLVM IR to stdout for debugging
}

impl CompileOptions {
    pub fn default() -> Self {
        CompileOptions {
            release: false,
            emit_ir: false,
        }
    }
}

/// The full compiler pipeline —
/// each stage feeds into the next
pub fn compile(file: SourceFile, opts: CompileOptions) -> Result<(), ()> {
    // stage 1 — lex
    let (lexemes, lex_diags) = lexer::tokenize(&file.content, &file.path);
    lex_diags.emit_all(&file.content);
    if lex_diags.has_errors() { return Err(()); }

    // stage 2 — parse
    let (ast, parse_diags) = parser::parse(lexemes, file.path.clone());
    parse_diags.emit_all(&file.content);
    if parse_diags.has_errors() { return Err(()); }

    // stage 3 — semantic analysis (same pattern when you build it)
    // let (typed_ast, sem_diags) = semantics::analyse(ast);
    // sem_diags.emit_all(&file.content);
    // if sem_diags.has_errors() { return Err(()); }

    // stage 4 — codegen
    // llvm::codegen(typed_ast, &opts);

    println!("{:#?}", ast);
    Ok(())
}