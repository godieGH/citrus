// src/parser/exprs.rs

use super::Parser;
use super::ast::*;
use crate::error::CitrusError;
use crate::lexer::Token;

impl Parser {
    // ─────────────────────────────────────────────────────────────────
    // ENTRY POINTS
    // ─────────────────────────────────────────────────────────────────
    //
    // parse_expr          — normal expression, struct init allowed
    // parse_expr_no_struct — disables `Name { }` struct init syntax
    //                        used for if/while/for conditions so
    //                        `if cond {` doesn't eat the { as struct

    pub fn parse_expr(&mut self) -> Result<SpannedExpr, CitrusError> {
        self.parse_expr_bp(0)
    }

    pub fn parse_expr_no_struct(&mut self) -> Result<SpannedExpr, CitrusError> {
        self.no_struct_expr = true;
        let result = self.parse_expr_bp(0);
        self.no_struct_expr = false;
        result
        // note: resets even when result is Err — no early return above
    }

    // ─────────────────────────────────────────────────────────────────
    // CORE PRATT FUNCTION
    // ─────────────────────────────────────────────────────────────────
    //
    // min_bp = minimum binding power the next infix operator must have
    // to be grabbed. 0 means grab everything.
    //
    // Structure:
    //   1. parse the left side (a literal, name, unary op etc.)
    //   2. loop: peek at the next token as a potential infix operator
    //      if its binding power is high enough, grab it and loop again
    //      otherwise stop and return what we have

    fn parse_expr_bp(&mut self, min_bp: u8) -> Result<SpannedExpr, CitrusError> {
        // parse the left side — prefix position
        let mut left = self.parse_prefix()?;

        loop {
            // ── POSTFIX: ?  ───────────────────────────────────────────
            // result?   — propagates Err up to caller
            // handled before the binary loop because it takes no right side
            if self.check(&Token::Question) {
                let bp = 30u8;
                if bp < min_bp {
                    break;
                }
                let span = self.span();
                self.advance(); // consume ?
                left = Spanned {
                    node: Expr::Try(Box::new(left)),
                    span,
                };
                continue;
            }

            // ── POSTFIX: . field access and method calls ──────────────
            // animal.name      — FieldAccess
            // items.push(4)    — MethodCall
            if self.check(&Token::Dot) {
                let bp = 28u8;
                if bp < min_bp {
                    break;
                }
                let span = self.span();
                self.advance(); // consume .
                let name = self.expect_identifier()?;

                if self.check(&Token::LParen) {
                    // method call: object.method(args)
                    self.advance(); // consume (
                    let args = self.parse_call_args()?;
                    self.expect(&Token::RParen)?;
                    left = Spanned {
                        node: Expr::MethodCall {
                            object: Box::new(left),
                            method: name,
                            args,
                        },
                        span,
                    };
                } else {
                    // field access: object.field
                    left = Spanned {
                        node: Expr::FieldAccess {
                            object: Box::new(left),
                            field: name,
                        },
                        span,
                    };
                }
                continue;
            }

            // ── POSTFIX: [ index access  ──────────────────────────────
            // scores[0]
            // only counts as index access when left side is already an expr
            // (a fresh [ at the start goes through parse_prefix instead)
            if self.check(&Token::LBracket) && !self.is_closure_start() {
                let bp = 28u8;
                if bp < min_bp {
                    break;
                }
                let span = self.span();
                self.advance(); // consume [
                let index = self.parse_expr()?;
                self.expect(&Token::RBracket)?;
                left = Spanned {
                    node: Expr::IndexAccess {
                        object: Box::new(left),
                        index: Box::new(index),
                    },
                    span,
                };
                continue;
            }

            // ── BINARY OPERATORS ──────────────────────────────────────
            // look up the binding power of the current token as an infix op
            let (left_bp, right_bp) = match self.infix_binding_power() {
                Some(bp) => bp,
                None => break, // not a binary operator — we are done
            };

            // if this operator doesn't bind tightly enough for our caller, stop
            if left_bp < min_bp {
                break;
            }

            // consume the operator token
            let op_lex = self.advance().unwrap();
            let op_span = Span {
                line: op_lex.line,
                col: op_lex.col,
            };

            // ── RANGE: special case ───────────────────────────────────
            // 0..10   0..=10
            // range is binary — left is start, right is end
            if op_lex.token == Token::Range || op_lex.token == Token::RangeInclusive {
                let inclusive = op_lex.token == Token::RangeInclusive;
                let right = self.parse_expr_bp(right_bp)?;
                left = Spanned {
                    node: Expr::Range {
                        start: Box::new(left),
                        end: Box::new(right),
                        inclusive,
                    },
                    span: op_span,
                };
                continue;
            }

            // all other binary ops: parse right side
            let right = self.parse_expr_bp(right_bp)?;

            let node = match op_lex.token {
                // assignment — right associative
                Token::Equals => Expr::Assign {
                    target: Box::new(left.clone()),
                    op: AssignOp::Assign,
                    value: Box::new(right),
                },
                Token::PlusAssign => Expr::Assign {
                    target: Box::new(left.clone()),
                    op: AssignOp::AddAssign,
                    value: Box::new(right),
                },
                Token::MinusAssign => Expr::Assign {
                    target: Box::new(left.clone()),
                    op: AssignOp::SubAssign,
                    value: Box::new(right),
                },
                Token::StarAssign => Expr::Assign {
                    target: Box::new(left.clone()),
                    op: AssignOp::MulAssign,
                    value: Box::new(right),
                },
                Token::SlashAssign => Expr::Assign {
                    target: Box::new(left.clone()),
                    op: AssignOp::DivAssign,
                    value: Box::new(right),
                },
                Token::PercentAssign => Expr::Assign {
                    target: Box::new(left.clone()),
                    op: AssignOp::ModAssign,
                    value: Box::new(right),
                },

                // math
                Token::Plus => Expr::BinaryOp {
                    left: Box::new(left.clone()),
                    op: BinaryOp::Add,
                    right: Box::new(right),
                },
                Token::Minus => Expr::BinaryOp {
                    left: Box::new(left.clone()),
                    op: BinaryOp::Sub,
                    right: Box::new(right),
                },
                Token::Star => Expr::BinaryOp {
                    left: Box::new(left.clone()),
                    op: BinaryOp::Mul,
                    right: Box::new(right),
                },
                Token::Slash => Expr::BinaryOp {
                    left: Box::new(left.clone()),
                    op: BinaryOp::Div,
                    right: Box::new(right),
                },
                Token::Percent => Expr::BinaryOp {
                    left: Box::new(left.clone()),
                    op: BinaryOp::Mod,
                    right: Box::new(right),
                },

                // comparison
                Token::EqEq => Expr::BinaryOp {
                    left: Box::new(left.clone()),
                    op: BinaryOp::Eq,
                    right: Box::new(right),
                },
                Token::NotEq => Expr::BinaryOp {
                    left: Box::new(left.clone()),
                    op: BinaryOp::NotEq,
                    right: Box::new(right),
                },
                Token::Lt => Expr::BinaryOp {
                    left: Box::new(left.clone()),
                    op: BinaryOp::Lt,
                    right: Box::new(right),
                },
                Token::Gt => Expr::BinaryOp {
                    left: Box::new(left.clone()),
                    op: BinaryOp::Gt,
                    right: Box::new(right),
                },
                Token::LtEq => Expr::BinaryOp {
                    left: Box::new(left.clone()),
                    op: BinaryOp::LtEq,
                    right: Box::new(right),
                },
                Token::GtEq => Expr::BinaryOp {
                    left: Box::new(left.clone()),
                    op: BinaryOp::GtEq,
                    right: Box::new(right),
                },

                // logical
                Token::And => Expr::BinaryOp {
                    left: Box::new(left.clone()),
                    op: BinaryOp::And,
                    right: Box::new(right),
                },
                Token::Or => Expr::BinaryOp {
                    left: Box::new(left.clone()),
                    op: BinaryOp::Or,
                    right: Box::new(right),
                },

                // bitwise
                Token::Ampersand => Expr::BinaryOp {
                    left: Box::new(left.clone()),
                    op: BinaryOp::BitAnd,
                    right: Box::new(right),
                },
                Token::Pipe => Expr::BinaryOp {
                    left: Box::new(left.clone()),
                    op: BinaryOp::BitOr,
                    right: Box::new(right),
                },
                Token::Caret => Expr::BinaryOp {
                    left: Box::new(left.clone()),
                    op: BinaryOp::BitXor,
                    right: Box::new(right),
                },
                Token::Shl => Expr::BinaryOp {
                    left: Box::new(left.clone()),
                    op: BinaryOp::Shl,
                    right: Box::new(right),
                },
                Token::Shr => Expr::BinaryOp {
                    left: Box::new(left.clone()),
                    op: BinaryOp::Shr,
                    right: Box::new(right),
                },

                _ => return Err(self.error_expected("binary operator".to_string())),
            };

            left = Spanned {
                node,
                span: op_span,
            };
        }

        Ok(left)
    }

    // ─────────────────────────────────────────────────────────────────
    // BINDING POWER TABLE
    // ─────────────────────────────────────────────────────────────────
    //
    // Returns (left_bp, right_bp) for an infix operator.
    // Left-associative:  (n, n+1) — right side must bind strictly tighter
    // Right-associative: (n, n)   — right side can grab same-strength ops
    //
    // Operators not listed here are not infix — the loop in parse_expr_bp
    // gets None and stops.

    fn infix_binding_power(&self) -> Option<(u8, u8)> {
        match self.current() {
            // assignment — right associative
            Some(Token::Equals)
            | Some(Token::PlusAssign)
            | Some(Token::MinusAssign)
            | Some(Token::StarAssign)
            | Some(Token::SlashAssign)
            | Some(Token::PercentAssign) => Some((1, 1)),

            // range — left associative
            Some(Token::Range) | Some(Token::RangeInclusive) => Some((2, 3)),

            // logical or
            Some(Token::Or) => Some((4, 5)),
            // logical and
            Some(Token::And) => Some((6, 7)),

            // equality
            Some(Token::EqEq) | Some(Token::NotEq) => Some((8, 9)),

            // comparison
            Some(Token::Lt) | Some(Token::Gt) | Some(Token::LtEq) | Some(Token::GtEq) => {
                Some((10, 11))
            }

            // bitwise or
            Some(Token::Pipe) => Some((12, 13)),
            // bitwise xor
            Some(Token::Caret) => Some((14, 15)),
            // bitwise and — Ampersand is also used as & reference prefix
            // but in INFIX position it means bitwise AND
            Some(Token::Ampersand) => Some((16, 17)),

            // shift
            Some(Token::Shl) | Some(Token::Shr) => Some((18, 19)),

            // addition / subtraction
            Some(Token::Plus) | Some(Token::Minus) => Some((20, 21)),

            // multiply / divide / modulo
            Some(Token::Star) | Some(Token::Slash) | Some(Token::Percent) => Some((22, 23)),

            // . [ ? are handled as special cases above the binary loop
            // so they never reach here
            _ => None,
        }
    }

    // ─────────────────────────────────────────────────────────────────
    // PREFIX PARSING
    // ─────────────────────────────────────────────────────────────────
    //
    // Everything that can START an expression.
    // This is the first half of parse_expr_bp.

    fn parse_prefix(&mut self) -> Result<SpannedExpr, CitrusError> {
        let span = self.span();

        match self.current() {
            // ── numeric literals ──────────────────────────────────────
            Some(Token::IntLiteral)
            | Some(Token::HexLiteral)
            | Some(Token::BinaryLiteral)
            | Some(Token::OctalLiteral) => self.parse_int_literal(),
            Some(Token::FloatLiteral) => self.parse_float_literal(),

            // ── string literals ───────────────────────────────────────
            Some(Token::StringLiteral) => self.parse_string_literal(),
            Some(Token::RawString) => self.parse_raw_string(),
            Some(Token::RawHashString) => self.parse_raw_hash_string(),

            // ── char literal ──────────────────────────────────────────
            Some(Token::CharLiteral) => self.parse_char_literal(),

            // ── bool literals ─────────────────────────────────────────
            Some(Token::True) => {
                self.advance();
                Ok(Spanned {
                    node: Expr::Literal(Lit::Bool(true)),
                    span,
                })
            }
            Some(Token::False) => {
                self.advance();
                Ok(Spanned {
                    node: Expr::Literal(Lit::Bool(false)),
                    span,
                })
            }

            // ── unary operators ───────────────────────────────────────
            // binding power 24 — above all binary operators
            Some(Token::Minus) => {
                self.advance();
                let expr = self.parse_expr_bp(24)?;
                Ok(Spanned {
                    node: Expr::UnaryOp {
                        op: UnaryOp::Neg,
                        expr: Box::new(expr),
                    },
                    span,
                })
            }
            Some(Token::Bang) => {
                // careful: Bang is also used for macro calls (name!)
                // but here we are in PREFIX position — there is no name before it
                self.advance();
                let expr = self.parse_expr_bp(24)?;
                Ok(Spanned {
                    node: Expr::UnaryOp {
                        op: UnaryOp::Not,
                        expr: Box::new(expr),
                    },
                    span,
                })
            }
            Some(Token::Tilde) => {
                self.advance();
                let expr = self.parse_expr_bp(24)?;
                Ok(Spanned {
                    node: Expr::UnaryOp {
                        op: UnaryOp::BitNot,
                        expr: Box::new(expr),
                    },
                    span,
                })
            }

            // ── reference ─────────────────────────────────────────────
            // &x    &mutable x
            // Ampersand in PREFIX position = reference
            // Ampersand in INFIX position = bitwise AND (handled above)
            Some(Token::Ampersand) => self.parse_ref_expr(),

            // ── grouped expression ────────────────────────────────────
            // (expr)  — just grouping, same precedence control as in math
            Some(Token::LParen) => self.parse_grouped(),

            // ── closure or array literal ──────────────────────────────
            // [&](x) => x * 2       closure
            // [1, 2, 3]             array literal
            // distinguished by is_closure_start()
            Some(Token::LBracket) => {
                if self.is_closure_start() {
                    self.parse_closure()
                } else {
                    self.parse_array_literal()
                }
            }

            // ── if as expression ──────────────────────────────────────
            // let x = if cond { "yes" } else { "no" };
            Some(Token::If) => self.parse_if_expr(),

            // ── match as expression ───────────────────────────────────
            // let v = match result { Ok(v) => v, Err(_) => 0 };
            Some(Token::Match) => self.parse_match_expr(),

            Some(Token::LBrace) => {
                let block = self.parse_block()?;
                Ok(Spanned {
                    node: Expr::Block(block),
                    span,
                })
            }

            // ── identifier — the most complex case ───────────────────
            // could be: plain name, function call, macro call,
            //           struct init, or enum variant
            Some(Token::Identifier) => self.parse_identifier_expr(),

            _ => Err(self.error_expected(
                "expression — literal, identifier, operator, or keyword".to_string(),
            )),
        }
    }

    // ─────────────────────────────────────────────────────────────────
    // LITERAL PARSERS
    // ─────────────────────────────────────────────────────────────────

fn parse_int_literal(&mut self) -> Result<SpannedExpr, CitrusError> {
    let span = self.span();
    let lex = self.advance().unwrap();

    let val: i128 = match &lex.token {
        Token::HexLiteral    => {
            let s = lex.src.trim_start_matches("0x").replace('_', "");
            i128::from_str_radix(&s, 16)
        }
        Token::BinaryLiteral => {
            let s = lex.src.trim_start_matches("0b").replace('_', "");
            i128::from_str_radix(&s, 2)
        }
        Token::OctalLiteral  => {
            let s = lex.src.trim_start_matches("0o").replace('_', "");
            i128::from_str_radix(&s, 8)
        }
        _ => {
            let s = lex.src.replace('_', "");
            s.parse::<i128>()
        }
    }
    .map_err(|_| CitrusError::ParseError {
        expected: "valid integer literal".to_string(),
        found:    lex.src.clone(),
        file:     self.filename.clone(),
        line:     lex.line,
        col:      lex.col,
        hint:     None,
    })?;

    let suffix = match self.current() {
        Some(Token::TypeInt8)    => { self.advance(); Some(IntSuffix::I8)    }
        Some(Token::TypeInt16)   => { self.advance(); Some(IntSuffix::I16)   }
        Some(Token::TypeInt32)   => { self.advance(); Some(IntSuffix::I32)   }
        Some(Token::TypeInt64)   => { self.advance(); Some(IntSuffix::I64)   }
        Some(Token::TypeInt128)  => { self.advance(); Some(IntSuffix::I128)  }
        Some(Token::TypeISize)   => { self.advance(); Some(IntSuffix::ISize) }
        Some(Token::TypeUInt8)   => { self.advance(); Some(IntSuffix::U8)    }
        Some(Token::TypeUInt16)  => { self.advance(); Some(IntSuffix::U16)   }
        Some(Token::TypeUInt32)  => { self.advance(); Some(IntSuffix::U32)   }
        Some(Token::TypeUInt64)  => { self.advance(); Some(IntSuffix::U64)   }
        Some(Token::TypeUInt128) => { self.advance(); Some(IntSuffix::U128)  }
        Some(Token::TypeUSize)   => { self.advance(); Some(IntSuffix::USize) }
        _ => None,
    };

    Ok(Spanned { node: Expr::Literal(Lit::Int(val, suffix)), span })
}

fn parse_float_literal(&mut self) -> Result<SpannedExpr, CitrusError> {
    let span = self.span();
    let lex = self.advance().unwrap();
    let s = lex.src.replace('_', "");
    let val = s.parse::<f64>().map_err(|_| CitrusError::ParseError {
        expected: "valid float literal".to_string(),
        found:    lex.src.clone(),
        file:     self.filename.clone(),
        line:     lex.line,
        col:      lex.col,
        hint:     None,
    })?;

    let suffix = match self.current() {
        Some(Token::TypeFloat32) => { self.advance(); Some(FloatSuffix::F32) }
        Some(Token::TypeFloat64) => { self.advance(); Some(FloatSuffix::F64) }
        _ => None,
    };

    Ok(Spanned { node: Expr::Literal(Lit::Float(val, suffix)), span })
}
 
    fn parse_string_literal(&mut self) -> Result<SpannedExpr, CitrusError> {
        let span = self.span();
        let lex = self.advance().unwrap();
        // strip surrounding double quotes
        let inner = lex.src[1..lex.src.len() - 1].to_string();
        Ok(Spanned {
            node: Expr::Literal(Lit::Str(inner)),
            span,
        })
    }

    fn parse_raw_string(&mut self) -> Result<SpannedExpr, CitrusError> {
        let span = self.span();
        let lex = self.advance().unwrap();
        // strip R" prefix and " suffix
        let inner = lex.src[2..lex.src.len() - 1].to_string();
        Ok(Spanned {
            node: Expr::Literal(Lit::RawStr(inner)),
            span,
        })
    }

    fn parse_raw_hash_string(&mut self) -> Result<SpannedExpr, CitrusError> {
        let span = self.span();
        let lex = self.advance().unwrap();
        // strip R#" prefix and "# suffix
        let inner = lex.src[3..lex.src.len() - 2].to_string();
        Ok(Spanned {
            node: Expr::Literal(Lit::RawStr(inner)),
            span,
        })
    }

    fn parse_char_literal(&mut self) -> Result<SpannedExpr, CitrusError> {
        let span = self.span();
        let lex = self.advance().unwrap();
        // strip single quotes — content is either a char or escape sequence
        let inner = &lex.src[1..lex.src.len() - 1];
        let ch = if inner.starts_with('\\') {
            match inner.chars().nth(1) {
                Some('n') => '\n',
                Some('t') => '\t',
                Some('r') => '\r',
                Some('\\') => '\\',
                Some('\'') => '\'',
                Some('0') => '\0',
                _ => {
                    return Err(CitrusError::ParseError {
                        expected: "valid escape sequence".to_string(),
                        found: lex.src.clone(),
                        file: self.filename.clone(),
                        line: lex.line,
                        col: lex.col,
                        hint: None,
                    });
                }
            }
        } else {
            inner.chars().next().unwrap_or('\0')
        };
        Ok(Spanned {
            node: Expr::Literal(Lit::Char(ch)),
            span,
        })
    }

    // ─────────────────────────────────────────────────────────────────
    // REFERENCE EXPRESSION
    // ─────────────────────────────────────────────────────────────────

    fn parse_ref_expr(&mut self) -> Result<SpannedExpr, CitrusError> {
        let span = self.span();
        self.advance(); // consume &
        let mutable = self.eat(&Token::Mutable);
        // reference binds very tightly — binding power 24 means it
        // only grabs the immediate next expression, not a whole chain
        let expr = self.parse_expr_bp(24)?;
        Ok(Spanned {
            node: Expr::Ref {
                mutable,
                expr: Box::new(expr),
            },
            span,
        })
    }

    // ─────────────────────────────────────────────────────────────────
    // GROUPED EXPRESSION
    // ─────────────────────────────────────────────────────────────────

    fn parse_grouped(&mut self) -> Result<SpannedExpr, CitrusError> {
        let span = self.span();
        self.advance(); // consume (

        // () — unit value
        if self.check(&Token::RParen) {
            self.advance();
            return Ok(Spanned {
                node: Expr::Tuple(vec![]),
                span,
            });
        }

        let first = self.parse_expr()?;

        // (expr) — grouping, no tuple node
        if self.check(&Token::RParen) {
            self.advance();
            return Ok(first);
        }

        // (expr,) or (a, b, ...) — tuple construction
        self.expect(&Token::Comma)?;
        let mut elements = vec![first];

        while !self.check(&Token::RParen) && !self.at_end() {
            elements.push(self.parse_expr()?);
            if !self.eat(&Token::Comma) {
                break;
            }
        }

        self.expect(&Token::RParen)?;
        Ok(Spanned {
            node: Expr::Tuple(elements),
            span,
        })
    }

    // ─────────────────────────────────────────────────────────────────
    // IDENTIFIER EXPRESSIONS
    // ─────────────────────────────────────────────────────────────────
    //
    // After consuming an identifier, we look at the NEXT token
    // to decide what kind of expression this is.

    fn parse_identifier_expr(&mut self) -> Result<SpannedExpr, CitrusError> {
        let span = self.span();
        let name = self.expect_identifier()?;

        // macro call: name!(...)  name![...]  name!{...}
        if self.check(&Token::Bang) {
            return self.parse_macro_call(span, name);
        }

        // function call with generics: name<T>(args)
        // is_generic_call() peeks ahead to avoid confusing < with less-than
        if self.check(&Token::Lt) && self.is_generic_call() {
            let generics = self.parse_generic_args()?;
            self.expect(&Token::LParen)?;
            let args = self.parse_call_args()?;
            self.expect(&Token::RParen)?;
            return Ok(Spanned {
                node: Expr::FunctionCall {
                    name,
                    generics,
                    args,
                },
                span,
            });
        }

        // function call: name(args)
        if self.check(&Token::LParen) {
            self.advance(); // consume (
            let args = self.parse_call_args()?;
            self.expect(&Token::RParen)?;
            return Ok(Spanned {
                node: Expr::FunctionCall {
                    name,
                    generics: Vec::new(),
                    args,
                },
                span,
            });
        }

        // path or enum variant: Name::Variant  Name::Variant(data)
        if self.check(&Token::PathSep) {
            return self.parse_path_expr(span, name);
        }

        // struct init: Name { field: value, ... }
        // disabled in condition context (no_struct_expr = true)
        if self.check(&Token::LBrace) && !self.no_struct_expr {
            return self.parse_struct_init_expr(span, name);
        }

        // plain identifier — variable, parameter, etc.
        Ok(Spanned {
            node: Expr::Identifier(name),
            span,
        })
    }

    fn parse_macro_call(&mut self, span: Span, name: String) -> Result<SpannedExpr, CitrusError> {
        self.advance(); // consume !

        // which delimiter was used — determines the closing token
        let (delim, close_tok) = match self.current() {
            Some(Token::LParen) => {
                self.advance();
                (MacroDelim::Paren, Token::RParen)
            }
            Some(Token::LBracket) => {
                self.advance();
                (MacroDelim::Bracket, Token::RBracket)
            }
            Some(Token::LBrace) => {
                self.advance();
                (MacroDelim::Brace, Token::RBrace)
            }
            _ => return Err(self.error_expected("'(', '[', or '{' after '!'".to_string())),
        };

        // collect raw tokens inside the macro call — track nesting depth
        // so nested parens/brackets/braces don't prematurely end collection
        let mut args = Vec::new();
        let mut depth = 1usize;

        while !self.at_end() {
            if self.check(&close_tok) {
                depth -= 1;
                if depth == 0 {
                    break;
                }
                args.push(self.advance().unwrap());
            } else if matches!(
                self.current(),
                Some(Token::LParen) | Some(Token::LBracket) | Some(Token::LBrace)
            ) {
                depth += 1;
                args.push(self.advance().unwrap());
            } else {
                args.push(self.advance().unwrap());
            }
        }

        self.advance(); // consume the closing delimiter

        Ok(Spanned {
            node: Expr::MacroCall { name, delim, args },
            span,
        })
    }

    // parse a :: separated path, then decide variant kind
    // Direction::North          — unit
    // Shape::Circle(5.0)        — tuple data
    // Message::Move { x: 1 }   — struct data
    fn parse_path_expr(&mut self, span: Span, first: String) -> Result<SpannedExpr, CitrusError> {
        let mut path = vec![first];

        while self.eat(&Token::PathSep) {
            path.push(self.expect_identifier()?);
        }

        let kind = match self.current() {
            Some(Token::LParen) => {
                self.advance();
                let mut exprs = Vec::new();
                if !self.check(&Token::RParen) {
                    exprs.push(self.parse_expr()?);
                    while self.eat(&Token::Comma) {
                        if self.check(&Token::RParen) {
                            break;
                        }
                        exprs.push(self.parse_expr()?);
                    }
                }
                self.expect(&Token::RParen)?;
                EnumVariantInit::Tuple(exprs)
            }
            Some(Token::LBrace) if !self.no_struct_expr => {
                self.advance();
                let fields = self.parse_field_inits()?;
                self.expect(&Token::RBrace)?;
                EnumVariantInit::Struct(fields)
            }
            _ => EnumVariantInit::Unit,
        };

        Ok(Spanned {
            node: Expr::EnumVariant { path, kind },
            span,
        })
    }

    fn parse_struct_init_expr(
        &mut self,
        span: Span,
        name: String,
    ) -> Result<SpannedExpr, CitrusError> {
        self.advance(); // consume {
        let fields = self.parse_field_inits()?;
        self.expect(&Token::RBrace)?;
        Ok(Spanned {
            node: Expr::StructInit {
                name,
                generics: Vec::new(),
                fields,
            },
            span,
        })
    }

    // parse field: value  pairs inside { } for struct init
    fn parse_field_inits(&mut self) -> Result<Vec<FieldInit>, CitrusError> {
        let mut fields = Vec::new();
        while !self.check(&Token::RBrace) && !self.at_end() {
            let name = self.expect_identifier()?;
            self.expect(&Token::Colon)?;
            let value = self.parse_expr()?;
            fields.push(FieldInit { name, value });
            if !self.eat(&Token::Comma) {
                break;
            }
        }
        Ok(fields)
    }

    // ─────────────────────────────────────────────────────────────────
    // CALL ARGUMENTS
    // ─────────────────────────────────────────────────────────────────
    //
    // Parses the content BETWEEN ( and ) for function and method calls.
    // The caller is responsible for consuming ( before and ) after.
    //
    // Handles both positional and named arguments:
    //   add(1, 2)                  — positional
    //   add(x=1, y=2)              — named
    //   process(&value, count=5)   — mixed

    pub fn parse_call_args(&mut self) -> Result<Vec<CallArg>, CitrusError> {
        let mut args = Vec::new();
        if self.check(&Token::RParen) {
            return Ok(args);
        }

        args.push(self.parse_call_arg()?);
        while self.eat(&Token::Comma) {
            if self.check(&Token::RParen) {
                break;
            }
            args.push(self.parse_call_arg()?);
        }
        Ok(args)
    }

    fn parse_call_arg(&mut self) -> Result<CallArg, CitrusError> {
        // named arg: name = value
        // detect: Identifier followed by = (but NOT ==)
        // peek_ahead(1) gives the token AFTER the current one
        let is_named = matches!(self.current(), Some(Token::Identifier))
            && matches!(self.peek_ahead(1), Some(Token::Equals));

        if is_named {
            let name = self.expect_identifier()?;
            self.advance(); // consume =
            let value = self.parse_expr()?;
            Ok(CallArg::Named { name, value })
        } else {
            Ok(CallArg::Positional(self.parse_expr()?))
        }
    }

    // ─────────────────────────────────────────────────────────────────
    // ARRAY LITERAL
    // ─────────────────────────────────────────────────────────────────
    //
    // [1, 2, 3, 4, 5]
    //
    // Note: we need Expr::ArrayLiteral in ast.rs for this.
    // For now using MacroCall as a placeholder — add to ast.rs:
    //   ArrayLiteral(Vec<SpannedExpr>),
    // then replace this implementation.

    fn parse_array_literal(&mut self) -> Result<SpannedExpr, CitrusError> {
        let span = self.span();
        self.advance(); // consume [

        let mut elements = Vec::new();
        if !self.check(&Token::RBracket) {
            elements.push(self.parse_expr()?);
            while self.eat(&Token::Comma) {
                if self.check(&Token::RBracket) {
                    break;
                }
                elements.push(self.parse_expr()?);
            }
        }
        self.expect(&Token::RBracket)?;

        Ok(Spanned {
            node: Expr::ArrayLiteral(elements),
            span,
        })
    }
    // ─────────────────────────────────────────────────────────────────
    // CLOSURES
    // ─────────────────────────────────────────────────────────────────
    //
    // [&](x as Int_32) => x * 2
    // [](x as Int_32, y as Int_32) -> Int_32 { return x + y; }
    // [&mutable a, =b](x) => x + a

    fn parse_closure(&mut self) -> Result<SpannedExpr, CitrusError> {
        let span = self.span();
        let capture = self.parse_capture_clause()?;

        self.expect(&Token::LParen)?;
        let params = self.parse_closure_params()?;
        self.expect(&Token::RParen)?;

        // optional explicit return type
        let ret = if self.eat(&Token::Arrow) {
            Some(self.parse_type()?)
        } else {
            None
        };

        // body — shorthand `=> expr` or full block `{ stmts }`
        let body = if self.eat(&Token::FatArrow) {
            ClosureBody::Expr(Box::new(self.parse_expr()?))
        } else {
            ClosureBody::Block(self.parse_block()?)
        };

        Ok(Spanned {
            node: Expr::Closure {
                capture,
                params,
                ret,
                body,
            },
            span,
        })
    }

    fn parse_capture_clause(&mut self) -> Result<CaptureClause, CitrusError> {
        self.expect(&Token::LBracket)?;

        // [] — copy all (default)
        if self.eat(&Token::RBracket) {
            return Ok(CaptureClause::CopyAll);
        }

        // [&] — borrow all
        // [&mutable] — mutably borrow all
        if self.check(&Token::Ampersand) {
            // peek ahead: if next is ] it's [&], if next is `mutable` then ] it's [&mutable]
            match self.peek_ahead(1) {
                Some(Token::RBracket) => {
                    self.advance(); // &
                    self.advance(); // ]
                    return Ok(CaptureClause::RefAll);
                }
                Some(Token::Mutable) => {
                    if matches!(self.peek_ahead(2), Some(Token::RBracket)) {
                        self.advance(); // &
                        self.advance(); // mutable
                        self.advance(); // ]
                        return Ok(CaptureClause::MutRefAll);
                    }
                }
                _ => {}
            }
        }

        // [=] — move all
        if self.check(&Token::Equals) {
            if matches!(self.peek_ahead(1), Some(Token::RBracket)) {
                self.advance(); // =
                self.advance(); // ]
                return Ok(CaptureClause::MoveAll);
            }
        }

        // explicit per-variable: [&a, =b, c, &mutable d]
        let mut captures = Vec::new();
        loop {
            let kind = if self.eat(&Token::Equals) {
                ExplicitCaptureKind::Move
            } else if self.check(&Token::Ampersand) {
                self.advance(); // consume &
                if self.eat(&Token::Mutable) {
                    ExplicitCaptureKind::MutRef
                } else {
                    ExplicitCaptureKind::Ref
                }
            } else {
                ExplicitCaptureKind::Copy // bare name — copy
            };

            let name = self.expect_identifier()?;
            captures.push(ExplicitCapture { name, kind });

            if !self.eat(&Token::Comma) {
                break;
            }
            if self.check(&Token::RBracket) {
                break;
            }
        }

        self.expect(&Token::RBracket)?;
        Ok(CaptureClause::Explicit(captures))
    }

    fn parse_closure_params(&mut self) -> Result<Vec<ClosureParam>, CitrusError> {
        let mut params = Vec::new();
        if self.check(&Token::RParen) {
            return Ok(params);
        }

        loop {
            let name = self.expect_identifier()?;
            // type annotation is optional in closures — can be inferred
            let ty = if self.eat(&Token::As) {
                Some(self.parse_type()?)
            } else {
                None
            };
            params.push(ClosureParam { name, ty });
            if !self.eat(&Token::Comma) {
                break;
            }
            if self.check(&Token::RParen) {
                break;
            }
        }
        Ok(params)
    }

    // ─────────────────────────────────────────────────────────────────
    // IF AS EXPRESSION
    // ─────────────────────────────────────────────────────────────────
    //
    // let label = if score > 50 { "pass" } else { "fail" };
    //
    // else is REQUIRED when if is used as an expression — both branches
    // must exist so the expression always has a value.

    fn parse_if_expr(&mut self) -> Result<SpannedExpr, CitrusError> {
        let span = self.span();
        self.advance(); // consume `if`

        // parse condition with no_struct_expr so `if cond {` works
        let condition = self.parse_expr_no_struct()?;
        let then_block = self.parse_block()?;

        // else is required for expression form
        self.expect(&Token::Else)?;
        let else_block = self.parse_block()?;

        Ok(Spanned {
            node: Expr::IfExpr {
                condition: Box::new(condition),
                then_block,
                else_block: Box::new(else_block),
            },
            span,
        })
    }

    // ─────────────────────────────────────────────────────────────────
    // MATCH AS EXPRESSION
    // ─────────────────────────────────────────────────────────────────
    //
    // let v = match result { Ok(x) => x, Err(_) => 0 };

    fn parse_match_expr(&mut self) -> Result<SpannedExpr, CitrusError> {
        let span = self.span();
        self.advance(); // consume `match`

        let value = self.parse_expr_no_struct()?;
        self.expect(&Token::LBrace)?;
        let arms = self.parse_match_arms()?;
        self.expect(&Token::RBrace)?;

        Ok(Spanned {
            node: Expr::MatchExpr {
                value: Box::new(value),
                arms,
            },
            span,
        })
    }

    // shared between match-as-expression and match-as-statement
    pub fn parse_match_arms(&mut self) -> Result<Vec<MatchArm>, CitrusError> {
        let mut arms = Vec::new();
        while !self.check(&Token::RBrace) && !self.at_end() {
            arms.push(self.parse_match_arm()?);
            self.eat(&Token::Comma); // optional trailing comma between arms
        }
        Ok(arms)
    }

    fn parse_match_arm(&mut self) -> Result<MatchArm, CitrusError> {
        let span = self.span();
        let pattern = self.parse_pattern()?;
        self.expect(&Token::FatArrow)?;

        // arm body — block { } or single expression
        let body = if self.check(&Token::LBrace) {
            MatchBody::Block(self.parse_block()?)
        } else {
            MatchBody::Expr(Box::new(self.parse_expr()?))
        };

        Ok(MatchArm {
            pattern,
            body,
            span,
        })
    }

    // ─────────────────────────────────────────────────────────────────
    // PATTERNS
    // ─────────────────────────────────────────────────────────────────
    //
    // Used inside match arms.
    //   0           — literal
    //   _           — wildcard
    //   x           — variable binding
    //   Some(x)     — enum variant with inner binding
    //   1 | 2 | 3   — multiple patterns (or)
    //   0..=59      — range pattern

    pub fn parse_pattern(&mut self) -> Result<Pattern, CitrusError> {
        let pattern = self.parse_single_pattern()?;

        // check for | — multiple patterns on one arm: 1 | 2 | 3
        if self.check(&Token::Pipe) {
            let mut patterns = vec![pattern];
            while self.eat(&Token::Pipe) {
                patterns.push(self.parse_single_pattern()?);
            }
            return Ok(Pattern::Or(patterns));
        }

        // check for range pattern: 0..=59  or  0..10
        if self.check(&Token::Range) || self.check(&Token::RangeInclusive) {
            let inclusive = self.eat(&Token::RangeInclusive);
            if !inclusive {
                self.eat(&Token::Range);
            }
            let end = self.parse_single_pattern()?;
            return Ok(Pattern::Range {
                start: Box::new(pattern),
                end: Box::new(end),
                inclusive,
            });
        }

        Ok(pattern)
    }

    fn parse_single_pattern(&mut self) -> Result<Pattern, CitrusError> {
        match self.current() {
            // _ wildcard
            Some(Token::Identifier) if self.current_text() == "_" => {
                self.advance();
                Ok(Pattern::Wildcard)
            }

            // literals
            Some(Token::IntLiteral)
            | Some(Token::HexLiteral)
            | Some(Token::BinaryLiteral)
            | Some(Token::OctalLiteral) => {
                let e = self.parse_int_literal()?;
                if let Expr::Literal(lit) = e.node {
                    Ok(Pattern::Literal(lit))
                } else {
                    unreachable!()
                }
            }

            Some(Token::FloatLiteral) => {
                let e = self.parse_float_literal()?;
                if let Expr::Literal(lit) = e.node {
                    Ok(Pattern::Literal(lit))
                } else {
                    unreachable!()
                }
            }

            Some(Token::StringLiteral) => {
                let lex = self.advance().unwrap();
                let inner = lex.src[1..lex.src.len() - 1].to_string();
                Ok(Pattern::Literal(Lit::Str(inner)))
            }

            Some(Token::CharLiteral) => {
                let e = self.parse_char_literal()?;
                if let Expr::Literal(lit) = e.node {
                    Ok(Pattern::Literal(lit))
                } else {
                    unreachable!()
                }
            }

            Some(Token::True) => {
                self.advance();
                Ok(Pattern::Literal(Lit::Bool(true)))
            }
            Some(Token::False) => {
                self.advance();
                Ok(Pattern::Literal(Lit::Bool(false)))
            }

            // negative number in pattern: -60
            Some(Token::Minus) => {
                self.advance();
                let e = self.parse_int_literal()?;
                if let Expr::Literal(Lit::Int(n, suffix)) = e.node {
                    Ok(Pattern::Literal(Lit::Int(-n, suffix)))
                } else {
                    Err(self.error_expected("integer after '-' in pattern".to_string()))
                }
            }

            // identifier — could be:
            //   _            wildcard (handled above)
            //   x            variable binding
            //   None         unit enum variant (no path separator)
            //   Some(x)      enum variant with data
            //   Dir::North   enum variant with path
            Some(Token::Identifier) => {
                let name = self.expect_identifier()?;

                if self.check(&Token::PathSep) {
                    // path: Direction::North or Shape::Circle(r)
                    let mut path = vec![name];
                    while self.eat(&Token::PathSep) {
                        path.push(self.expect_identifier()?);
                    }
                    let fields = self.parse_variant_pattern_fields()?;
                    Ok(Pattern::EnumVariant { path, fields })
                } else if self.check(&Token::LParen) {
                    // Some(x) — variant without explicit path
                    let fields = self.parse_variant_pattern_fields()?;
                    Ok(Pattern::EnumVariant {
                        path: vec![name],
                        fields,
                    })
                } else {
                    // plain binding — variable name
                    Ok(Pattern::Identifier(name))
                }
            }

            // tuple pattern: () or (a, b)
            Some(Token::LParen) => {
                self.advance(); // consume (

                if self.eat(&Token::RParen) {
                    return Ok(Pattern::Tuple(vec![]));
                }

                let mut patterns = Vec::new();
                patterns.push(self.parse_pattern()?);

                while self.eat(&Token::Comma) {
                    if self.check(&Token::RParen) {
                        break;
                    }
                    patterns.push(self.parse_pattern()?);
                }

                self.expect(&Token::RParen)?;
                Ok(Pattern::Tuple(patterns))
            }

            _ => Err(self.error_expected("pattern".to_string())),
        }
    }

    // parse ( pattern, pattern ) for tuple variants in patterns
    fn parse_variant_pattern_fields(&mut self) -> Result<Vec<Pattern>, CitrusError> {
        if !self.check(&Token::LParen) {
            return Ok(Vec::new());
        }
        self.advance(); // consume (
        let mut fields = Vec::new();
        if !self.check(&Token::RParen) {
            fields.push(self.parse_pattern()?);
            while self.eat(&Token::Comma) {
                if self.check(&Token::RParen) {
                    break;
                }
                fields.push(self.parse_pattern()?);
            }
        }
        self.expect(&Token::RParen)?;
        Ok(fields)
    }

    // ─────────────────────────────────────────────────────────────────
    // HELPER METHODS
    // ─────────────────────────────────────────────────────────────────

    // checks if [ starts a closure rather than an array literal
    // scans forward to find the matching ] and checks if ( follows
    fn is_closure_start(&self) -> bool {
        let mut i = self.cursor + 1;
        let mut depth = 1usize;
        while i < self.tokens.len() {
            match &self.tokens[i].token {
                Token::LBracket => depth += 1,
                Token::RBracket => {
                    depth -= 1;
                    if depth == 0 {
                        return matches!(
                            self.tokens.get(i + 1).map(|l| &l.token),
                            Some(Token::LParen)
                        );
                    }
                }
                _ => {}
            }
            i += 1;
        }
        false
    }

    // checks if < is generic args rather than a less-than comparison
    // heuristic: if the token after < looks like a type, it's generic
    fn is_generic_call(&self) -> bool {
        matches!(
            self.peek_ahead(1),
            Some(Token::Identifier)
                | Some(Token::TypeText)
                | Some(Token::TypeChar)
                | Some(Token::TypeBool)
                | Some(Token::TypeInt8)
                | Some(Token::TypeInt32)
                | Some(Token::TypeInt64)
                | Some(Token::TypeInt128)
                | Some(Token::TypeUInt8)
                | Some(Token::TypeUInt32)
                | Some(Token::TypeUInt64)
                | Some(Token::TypeUInt128)
                | Some(Token::TypeFloat32)
                | Some(Token::TypeFloat64)
        )
    }

    // get source text of current token — used for _ wildcard detection
    fn current_text(&self) -> &str {
        self.peek().map(|l| l.src.as_str()).unwrap_or("")
    }
}
