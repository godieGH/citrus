// src/parser/mod.rs

pub mod ast;
mod exprs;
mod items;
mod stmts;
mod types;

use crate::diagnostics::{Diagnostic, DiagnosticBag};
use crate::error::CitrusError;
use crate::lexer::{Lexeme, Token};
use ast::*;

// ─────────────────────────────────────────────
// THE PARSER STRUCT
// ─────────────────────────────────────────────
// The parser holds:
//   tokens   — the full list of lexemes from the lexer
//   cursor   — index of the token we are currently looking at
//   filename — for error messages (so they can say "in main.citrus")
//
// The cursor starts at 0 and moves forward as we consume tokens.
// We never go backwards — recursive descent is always forward-only.

pub struct Parser {
    tokens: Vec<Lexeme>,
    cursor: usize,
    filename: String,
    no_struct_expr: bool,
    pub bag: DiagnosticBag,  // collects errors without stopping
}

impl Parser {
    pub fn new(tokens: Vec<Lexeme>, filename: String) -> Self {
        Parser {
            tokens,
            cursor: 0,
            filename,
            no_struct_expr: false,
            bag: DiagnosticBag::new(),
        }
    }

    // ── LOOKING AT TOKENS ────────────────────────────────────────────
    //
    // These methods let us see what token we are on without consuming it.
    // "Consuming" means moving the cursor forward.
    // "Peeking" means reading without moving.

    // Look at the current token — returns None at end of file
    fn peek(&self) -> Option<&Lexeme> {
        self.tokens.get(self.cursor)
    }

    // Get just the Token kind of the current lexeme
    // We use this constantly — most decisions are based on token kind
    fn current(&self) -> Option<&Token> {
        self.peek().map(|lex| &lex.token)
    }

    // Look ahead without moving — peek(0) is current, peek(1) is next
    // Useful when two different constructs start with the same token
    // and we need to look one ahead to decide which one we are parsing
    fn peek_ahead(&self, offset: usize) -> Option<&Token> {
        self.tokens.get(self.cursor + offset).map(|lex| &lex.token)
    }

    // Are we at the end of the token stream?
    fn at_end(&self) -> bool {
        self.cursor >= self.tokens.len()
    }

    // ── CONSUMING TOKENS ─────────────────────────────────────────────
    //
    // These methods move the cursor forward.

    // Consume and return the current token
    // Returns a cloned Lexeme so the caller owns it
    // (we clone because borrowing from self while also mutating self
    //  causes lifetime problems in Rust)
    fn advance(&mut self) -> Option<Lexeme> {
        let lex = self.tokens.get(self.cursor).cloned();
        if lex.is_some() {
            self.cursor += 1;
        }
        lex
    }

    // Check if current token matches — WITHOUT consuming it
    // Returns true or false
    fn check(&self, token: &Token) -> bool {
        self.current() == Some(token)
    }

    // If current token matches, consume it and return true
    // If it does not match, do nothing and return false
    // This is the "optional consume" — use when a token may or may not appear
    //
    // Example: `public` before a function is optional
    //   let is_public = self.eat(&Token::Public);
    fn eat(&mut self, token: &Token) -> bool {
        if self.check(token) {
            self.advance();
            true
        } else {
            false
        }
    }

    // Consume a specific token or return an error
    // Use this when the token MUST be there — if it isn't, the source is invalid
    //
    // Example: after `struct Name` we MUST see `{`
    //   self.expect(&Token::LBrace)?;
    fn expect(&mut self, token: &Token) -> Result<Lexeme, CitrusError> {
        if self.check(token) {
            // safe to unwrap — we just confirmed it is there
            Ok(self.advance().unwrap())
        } else {
            Err(self.error_expected(format!("{:?}", token)))
        }
    }

    // Consume a token and extract its source text
    // Used for identifiers — we need the actual name string, not just
    // the fact that it is an identifier
    //
    // Example: the struct name after `struct`
    //   let name = self.expect_identifier()?;  →  "Animal"
    fn expect_identifier(&mut self) -> Result<String, CitrusError> {
        match self.current() {
            Some(Token::Identifier) => {
                let lex = self.advance().unwrap();
                Ok(lex.src)
            }
            _ => Err(self.error_expected("identifier".to_string())),
        }
    }

    // Same but also accepts Token::SelfKw — because `self` is a keyword
    // but also used as a parameter name in methods
    fn expect_identifier_or_self(&mut self) -> Result<String, CitrusError> {
        match self.current() {
            Some(Token::Identifier) | Some(Token::SelfKw) => {
                let lex = self.advance().unwrap();
                Ok(lex.src)
            }
            _ => Err(self.error_expected("identifier or 'self'".to_string())),
        }
    }

    // Expect an integer literal and parse it as u64
    // Used for array sizes — [UInt_8:5] — the 5 must be a positive integer
    fn expect_int_literal(&mut self) -> Result<u64, CitrusError> {
        match self.current() {
            Some(Token::IntLiteral) => {
                let lex = self.advance().unwrap();
                lex.src
                    .parse::<u64>()
                    .map_err(|_| self.error_expected("positive integer".to_string()))
            }
            _ => Err(self.error_expected("integer literal".to_string())),
        }
    }

    // ── SPAN HELPERS ─────────────────────────────────────────────────
    //
    // Produce a Span from the current cursor position.
    // We call this BEFORE consuming tokens — to record where something started.

    fn span(&self) -> Span {
        match self.peek() {
            Some(lex) => Span {
                line: lex.line,
                col: lex.col,
            },
            None => Span { line: 0, col: 0 }, // end of file
        }
    }

    // Wrap a node with the span of where it started
    // Call span() BEFORE parsing, then spanned() AFTER
    //
    //   let start = self.span();
    //   let node  = self.parse_something()?;
    //   Ok(Spanned { node, span: start })
    fn spanned<T>(&self, node: T, span: Span) -> Spanned<T> {
        Spanned { node, span }
    }
    
    
      // ── RECOVERY ─────────────────────────────────────────────────────
    //
    // Called after pushing a diagnostic when we want to keep parsing.
    // Skips tokens until we're at a point where a new statement or
    // item can start — avoids a cascade of follow-on errors.

    pub fn synchronize(&mut self) {
        // a semicolon ends a statement — safe to continue after it
        if self.eat(&Token::Semicolon) {
            return;
        }
        while !self.at_end() {
            // stop BEFORE tokens that start something new
            match self.current() {
                Some(
                    Token::Let    | Token::Return | Token::If     |
                    Token::While  | Token::For    | Token::Loop   |
                    Token::Break  | Token::Continue               |
                    Token::Struct | Token::Enum   | Token::Trait  |
                    Token::Implement | Token::Import | Token::Static |
                    Token::Module | Token::Macro
                ) => return,
                // a closing brace ends a block — stop before it so
                // the parent block parser can consume it properly
                Some(Token::RBrace) => return,
                _ => { self.advance(); }
            }
        }
    }
    
       // ── EMIT HELPERS ─────────────────────────────────────────────────
    //
    // Push a diagnostic and return an error sentinel — lets call sites do:
    //   return Ok(self.emit_error("...", line, col));
    // keeping the function signature as Result while still recovering.

    pub fn emit_error(&mut self, msg: impl Into<String>, line: usize, col: usize) -> SpannedExpr {
        self.bag.push(Diagnostic::error(msg, &self.filename, line, col));
        Spanned { node: Expr::Error, span: Span { line, col } }
    }

    // ── ERROR HELPERS ─────────────────────────────────────────────────
    //
    // Build error values from current parser state.
    // These are not returned yet — the caller decides when to return them.

    fn error_expected(&self, expected: String) -> CitrusError {
        // kept for the ? sites that remain truly fatal
        let (line, col, found) = self.current_description();
        CitrusError::ParseError {
            expected,
            found,
            line,
            col,
            file: self.filename.clone(),
            hint: None,
        }
    }

    fn error_eof(&self, message: String) -> CitrusError {
        CitrusError::UnexpectedEof {
            file: self.filename.clone(),
            message,
            hint: None,
        }
    }

    // Describe the current token for use in error messages
    // Returns (line, col, description_string)
    fn current_description(&self) -> (usize, usize, String) {
        match self.peek() {
            Some(lex) => {
                // for identifiers and literals, show the actual text
                // for keywords and symbols, show the token name
                let desc = match &lex.token {
                    Token::Identifier => format!("'{}'", lex.src),
                    Token::IntLiteral => format!("integer '{}'", lex.src),
                    Token::FloatLiteral => format!("float '{}'", lex.src),
                    Token::StringLiteral => format!("string {}", lex.src),
                    other => format!("'{:?}'", other),
                };
                (lex.line, lex.col, desc)
            }
            None => (0, 0, "end of file".to_string()),
        }
    }

    // ── ENTRY POINT ──────────────────────────────────────────────────
    //
    // parse() is called by compiler.rs — it drives the whole parse.
    // A Citrus file is just a sequence of top-level items until we
    // reach the end of the file.
    
       // ── ENTRY POINT ──────────────────────────────────────────────────

    pub fn parse(mut self) -> (Program, DiagnosticBag) {
        let filename = self.filename.clone();
        let mut items = Vec::new();

        while !self.at_end() {
            let start = self.span();
            match self.parse_item() {
                Ok(item) => items.push(Spanned { node: item, span: start }),
                Err(e) => {
                    // fatal parse error — convert to diagnostic, try to recover
                    let (line, col) = match &e {
                        CitrusError::ParseError { line, col, .. } => (*line, *col),
                        _ => (0, 0),
                    };
                    self.bag.push(Diagnostic::error(e.to_string(), &filename, line, col));
                    items.push(Spanned { node: Item::Error, span: start });
                    self.synchronize();
                }
            }
        }

        let program = Program { items, filename };
        (program, self.bag)
    }

}

// ─────────────────────────────────────────────
// PUBLIC ENTRY POINT
// ─────────────────────────────────────────────
// Called from compiler.rs:
//   let (ast, parse_diags) = parser::parse(lexemes, filename);
pub fn parse(tokens: Vec<Lexeme>, filename: String) -> (Program, DiagnosticBag) {
    Parser::new(tokens, filename).parse()
}
