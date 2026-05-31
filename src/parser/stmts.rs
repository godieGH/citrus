// src/parser/stmts.rs
// STUB — replaced in the next step when we build statement parsing.

use super::Parser;
use super::ast::*;
use crate::error::CitrusError;
use crate::lexer::Token;

impl Parser {
    // parse_block — parses a { ... } block.
    // STUB: skips all tokens inside the braces and returns an empty block.
    // We replace this fully when we build statement parsing.
    pub fn parse_block(&mut self) -> Result<Block, CitrusError> {
        let span = self.span();
        self.expect(&Token::LBrace)?;

        // skip everything inside the block, counting nested braces
        // so we stop at the right }
        let mut depth = 1usize;
        while !self.at_end() && depth > 0 {
            match self.current() {
                Some(Token::LBrace) => {
                    depth += 1;
                    self.advance();
                }
                Some(Token::RBrace) => {
                    depth -= 1;
                    if depth > 0 {
                        self.advance();
                    }
                }
                _ => {
                    self.advance();
                }
            }
        }

        self.expect(&Token::RBrace)?;

        // empty block for now — filled in when stmts are built
        Ok(Block {
            stmts: Vec::new(),
            span,
        })
    }

    // parse_stmt — STUB
    pub fn parse_stmt(&mut self) -> Result<Stmt, CitrusError> {
        Err(CitrusError::ParseError {
            expected: "statement".to_string(),
            found: "stub not yet implemented".to_string(),
            file: self.filename.clone(),
            line: 0,
            col: 0,
        })
    }
}
