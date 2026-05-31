// src/lexer/mod.rs
mod tokens;
pub use tokens::{Logos, Token};

/// A single token with its source text and position
#[derive(Debug, Clone)]
pub struct Lexeme {
    pub token: Token,
    pub src: String,
    // the actual text that matched
    pub line: usize,
    pub col: usize,
}

/// Tokenize a source string into a list of Lexemes
/// Returns errors as Err with position info
pub fn tokenize(source: &str, filename: &str) -> Result<Vec<Lexeme>, crate::error::CitrusError> {
    let mut lexemes = Vec::new();
    let mut line = 1usize;
    let mut col = 1usize;
    let mut lexer = Token::lexer(source);

    // track line/col by walking the source
    let mut last_end = 0usize;

    while let Some(result) = lexer.next() {
        let span = lexer.span();

        // count newlines between last token and this one to update line/col
        for ch in source[last_end..span.start].chars() {
            if ch == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
        }

        let token_text = lexer.slice().to_string();

        match result {
            Ok(token) => {
                lexemes.push(Lexeme {
                    token,
                    src: token_text,
                    line,
                    col,
                });
            }
            Err(_) => {
                return Err(crate::error::CitrusError::LexError {
                    file: filename.to_string(),
                    line,
                    col,
                    src: token_text,
                });
            }
        }

        last_end = span.end;
    }

    Ok(lexemes)
}
