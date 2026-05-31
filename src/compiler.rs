// src/compiler.rs
use crate::load_source::SourceFile;
use crate::error::CitrusError;
use crate::lexer;
// use crate::parser;
// use crate::semantics;
// use crate::llvm;

#[derive(Debug)]
/// Options passed to the compiler —
/// grows as we add flags like --release
pub struct CompileOptions {
    pub release: bool,    // enables LLVM optimizations
    pub emit_ir: bool,    // dumps LLVM IR to stdout for debugging
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
pub fn compile(file: SourceFile, opts: CompileOptions) -> Result<(), CitrusError> {

    // stage 1 — lex
    let lexemes = lexer::tokenize(&file.content, &file.path)?;

    // stage 2 — parse
    // let ast = parser::parse(lexemes)?;

    // stage 3 — semantic analysis
    // let typed_ast = semantics::analyse(ast)?;

    // stage 4 — llvm ir + codegen
    // llvm::codegen(typed_ast, &opts)?;

    // temporary — print tokens until parser exists
    for lex in &lexemes {
        println!("[{}:{}]  {:?}  =>  {:?}", lex.line, lex.col, lex.token, lex.src);
    }
    
    println!("{:?}", opts);

    Ok(())
}