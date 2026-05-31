// src/parser/exprs.rs
// STUB — replaced in the next step when we build expression parsing.

use super::Parser;
use super::ast::*;
use crate::error::CitrusError;

impl Parser {
    // parse_expr — STUB
    pub fn parse_expr(&mut self) -> Result<SpannedExpr, CitrusError> {
        Err(CitrusError::ParseError {
            expected: "expression".to_string(),
            found: "stub not yet implemented".to_string(),
            file: self.filename.clone(),
            line: 0,
            col: 0,
        })
    }
}
