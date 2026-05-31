// src/parser/stmts.rs
//
// Parses statements — everything that can appear inside a function body.
//
// All statements live in { } blocks. A block is a sequence of statements
// terminated by }. The entry point is parse_block().
//
// Statements:
//   let x as Int_32 = 60;                  — typed and initialized
//   let score = 100;                       — type inferred (built-ins only)
//   let mutable x as Int_32 = 0;           — mutable
//   let x as Int_32;                       — uninitialized (type required)
//   return x + y;                          — return with value
//   return;                                — bare return (Void functions)
//   while x < 10 { x += 1; }              — while loop
//   for item in items { }                  — for-in
//   for i, item in items.enumerate() { }   — indexed for-in
//   loop { break; }                        — infinite loop
//   break;                                 — break out of loop
//   continue;                              — next iteration
//   if x > 0 { } else if x == 0 { } else { } — if statement
//   match value { pattern => { } }         — match statement
//   fn_call();                             — expression statement

use super::Parser;
use super::ast::*;
use crate::diagnostics::Diagnostic;
use crate::error::CitrusError;
use crate::lexer::Token;

impl Parser {
    // ─────────────────────────────────────────────────────────────────
    // BLOCK
    // ─────────────────────────────────────────────────────────────────
    //
    // A block is { stmt* }
    // The { and } are both consumed here.
    // Called for function bodies, if/else bodies, loop bodies, etc.

    pub fn parse_block(&mut self) -> Result<Block, CitrusError> {
        let span = self.span();
        self.expect(&Token::LBrace)?;

        let mut stmts = Vec::new();
        let mut tail = None;

        while !self.check(&Token::RBrace) && !self.at_end() {
            if self.is_value_less_stmt() {
                // these keywords can never produce a value — always statements
                let stmt_span = self.span();
                match self.parse_stmt() {
                    Ok(stmt) => stmts.push(Spanned { node: stmt, span: stmt_span }),
                    Err(e) => {
                        // recover: record the error, push a sentinel, skip to a safe point
                        let (line, col) = match &e {
                            CitrusError::ParseError { line, col, .. } => (*line, *col),
                            _ => (stmt_span.line, stmt_span.col),
                        };
                        let filename = self.filename.clone();
                        self.bag.push(Diagnostic::error(e.to_string(), &filename, line, col));
                        stmts.push(Spanned { node: Stmt::Error, span: stmt_span });
                        self.synchronize();
                    }
                }
            } else {
                // everything else — parse as expression, then check the terminator
                // this includes `if` and `match` which CAN be tail values
                let expr_span = self.span();
                let expr = match self.parse_expr() {
                    Ok(e) => e,
                    Err(e) => {
                        let (line, col) = match &e {
                            CitrusError::ParseError { line, col, .. } => (*line, *col),
                            _ => (expr_span.line, expr_span.col),
                        };
                        let filename = self.filename.clone();
                        self.bag.push(Diagnostic::error(e.to_string(), &filename, line, col));
                        stmts.push(Spanned { node: Stmt::Error, span: expr_span });
                        self.synchronize();
                        continue;
                    }
                };

                if self.check(&Token::RBrace) {
                    // no semicolon before } — this is the tail value
                    tail = Some(Box::new(expr));
                    break;
                }

                // semicolon present — value is discarded, treat as statement
                if let Err(e) = self.expect(&Token::Semicolon) {
                    // missing semicolon — emit, treat expr as a stmt anyway, keep going
                    let (line, col) = match &e {
                        CitrusError::ParseError { line, col, .. } => (*line, *col),
                        _ => (expr_span.line, expr_span.col),
                    };
                    let filename = self.filename.clone();
                    self.bag.push(
                        Diagnostic::error(e.to_string(), &filename, line, col)
                            .with_hint("add `;` here"),
                    );
                }
                stmts.push(Spanned { node: Stmt::Expr(expr), span: expr_span });
            }
        }

        self.expect(&Token::RBrace)?;
        Ok(Block { stmts, tail, span })
    }

    // Keywords that syntactically cannot produce a value.
    // if/match are NOT here — they can be used as tail expressions.
    // Add new statement-only keywords here as the language grows.
    fn is_value_less_stmt(&self) -> bool {
        matches!(
            self.current(),
            Some(Token::Let)
                | Some(Token::Return)
                | Some(Token::While)
                | Some(Token::For)
                | Some(Token::Loop)
                | Some(Token::Break)
                | Some(Token::Continue)
        )
    }

    // ─────────────────────────────────────────────────────────────────
    // STATEMENT DISPATCHER
    // ─────────────────────────────────────────────────────────────────
    //
    // Look at the current token to decide which statement we are parsing.
    // Most begin with a dedicated keyword.
    // Anything else is treated as an expression statement.

    fn parse_stmt(&mut self) -> Result<Stmt, CitrusError> {
        match self.current() {
            Some(Token::Let) => self.parse_let(),
            Some(Token::Return) => self.parse_return(),
            Some(Token::While) => self.parse_while(),
            Some(Token::For) => self.parse_for_in(),
            Some(Token::Loop) => self.parse_loop(),
            Some(Token::Break) => self.parse_break(),
            Some(Token::Continue) => self.parse_continue(),
            Some(Token::If) => self.parse_if_stmt(),
            Some(Token::Match) => self.parse_match_stmt(),

            // anything else that can start an expression
            // covers: fn_call(), x += 1, a.method(), println!(...), etc.
            _ => self.parse_expr_stmt(),
        }
    }

    // ─────────────────────────────────────────────────────────────────
    // LET — variable declaration
    // ─────────────────────────────────────────────────────────────────
    //
    // let score as Int_32 = 100;     — typed and initialized
    // let score = 100;               — inferred (built-in types only)
    // let mutable score as Int_32 = 0; — mutable
    // let score as Int_32;           — uninitialized — type required

    fn parse_let(&mut self) -> Result<Stmt, CitrusError> {
        self.expect(&Token::Let)?;

        // optional `mutable` — let mutable x ...
        let mutable = self.eat(&Token::Mutable);

        // variable name
        let name = self.expect_identifier()?;

        // optional type annotation — `as Type`
        // omitting it is only valid when an initializer is present
        // (the semantic stage enforces this — parser just allows both)
        let ty = if self.eat(&Token::As) {
            Some(self.parse_type()?)
        } else {
            None
        };

        // optional initializer — `= expr`
        let value = if self.eat(&Token::Equals) {
            Some(self.parse_expr()?)
        } else {
            None
        };

        self.expect(&Token::Semicolon)?;

        Ok(Stmt::Let {
            mutable,
            name,
            ty,
            value,
        })
    }

    // ─────────────────────────────────────────────────────────────────
    // RETURN
    // ─────────────────────────────────────────────────────────────────
    //
    // return x + y;   — with a value
    // return;         — bare return (Void functions)

    fn parse_return(&mut self) -> Result<Stmt, CitrusError> {
        self.expect(&Token::Return)?;

        // if the next token is `;`, this is a bare `return;`
        // otherwise parse an expression
        let value = if self.check(&Token::Semicolon) {
            None
        } else {
            Some(self.parse_expr()?)
        };

        self.expect(&Token::Semicolon)?;

        Ok(Stmt::Return(value))
    }

    // ─────────────────────────────────────────────────────────────────
    // WHILE
    // ─────────────────────────────────────────────────────────────────
    //
    // while x < 10 { x += 1; }
    //
    // The condition uses parse_expr_no_struct so `while cond {`
    // doesn't treat the opening `{` as a struct literal.

    fn parse_while(&mut self) -> Result<Stmt, CitrusError> {
        self.expect(&Token::While)?;

        let condition = self.parse_expr_no_struct()?;
        let body = self.parse_block()?;

        Ok(Stmt::While {
            condition: Box::new(condition),
            body,
        })
    }

    // ─────────────────────────────────────────────────────────────────
    // FOR-IN
    // ─────────────────────────────────────────────────────────────────
    //
    // for item in items { }
    // for i in 0..10 { }
    // for i, item in items.enumerate() { }   — indexed: index + value

    fn parse_for_in(&mut self) -> Result<Stmt, CitrusError> {
        self.expect(&Token::For)?;

        // parse the binding variable(s)
        // `for name` or `for index, name`
        let first = self.expect_identifier()?;

        let var = if self.eat(&Token::Comma) {
            // indexed form — `for i, item in ...`
            let value = self.expect_identifier()?;
            ForVar::Indexed {
                index: first,
                value,
            }
        } else {
            // simple form — `for item in ...`
            ForVar::Single(first)
        };

        self.expect(&Token::In)?;

        // the iterable — no_struct so `for i in items {` is unambiguous
        let iterable = self.parse_expr_no_struct()?;

        let body = self.parse_block()?;

        Ok(Stmt::ForIn {
            var,
            iterable: Box::new(iterable),
            body,
        })
    }

    // ─────────────────────────────────────────────────────────────────
    // LOOP — infinite loop
    // ─────────────────────────────────────────────────────────────────
    //
    // loop { break; }
    // loop { if done { break; } }

    fn parse_loop(&mut self) -> Result<Stmt, CitrusError> {
        self.expect(&Token::Loop)?;
        let body = self.parse_block()?;
        Ok(Stmt::Loop(body))
    }

    // ─────────────────────────────────────────────────────────────────
    // BREAK / CONTINUE
    // ─────────────────────────────────────────────────────────────────
    //
    // break;
    // continue;

    fn parse_break(&mut self) -> Result<Stmt, CitrusError> {
        self.expect(&Token::Break)?;
        self.expect(&Token::Semicolon)?;
        Ok(Stmt::Break)
    }

    fn parse_continue(&mut self) -> Result<Stmt, CitrusError> {
        self.expect(&Token::Continue)?;
        self.expect(&Token::Semicolon)?;
        Ok(Stmt::Continue)
    }

    // ─────────────────────────────────────────────────────────────────
    // IF — as statement
    // ─────────────────────────────────────────────────────────────────
    //
    // if x > 0 { }
    // if x > 0 { } else { }
    // if x > 0 { } else if x == 0 { } else { }
    //
    // else is optional in statement form.
    // (In expression form `if ... else ...`, else is required — that is
    //  handled separately in exprs.rs as Expr::IfExpr.)
    //
    // else-if chains are represented as ElseBranch::If(Box<SpannedStmt>)
    // where the inner statement is itself a Stmt::If — matching the AST
    // design in ast.rs.

    fn parse_if_stmt(&mut self) -> Result<Stmt, CitrusError> {
        self.expect(&Token::If)?;

        let condition = self.parse_expr_no_struct()?;
        let then_block = self.parse_block()?;

        // optional else / else-if
        let else_branch = if self.eat(&Token::Else) {
            Some(self.parse_else_branch()?)
        } else {
            None
        };

        Ok(Stmt::If {
            condition: Box::new(condition),
            then_block,
            else_branch,
        })
    }

    // Parse the part that comes after `else` has already been consumed.
    //
    //   else { ... }         → ElseBranch::Block(block)
    //   else if cond { ... } → ElseBranch::If(Box<SpannedStmt::If>)
    //
    // For else-if we build a full Stmt::If and wrap it in a SpannedStmt
    // so the chain is represented as nested if-statements — matching
    // the ElseBranch::If(Box<SpannedStmt>) definition in ast.rs.

    fn parse_else_branch(&mut self) -> Result<ElseBranch, CitrusError> {
        if self.check(&Token::If) {
            // else if — parse the entire nested if-statement
            let start = self.span();
            let stmt = self.parse_if_stmt()?; // consumes `if` and everything after
            Ok(ElseBranch::If(Box::new(Spanned {
                node: stmt,
                span: start,
            })))
        } else {
            // plain else { }
            let block = self.parse_block()?;
            Ok(ElseBranch::Block(block))
        }
    }

    // ─────────────────────────────────────────────────────────────────
    // MATCH — as statement
    // ─────────────────────────────────────────────────────────────────
    //
    // match value {
    //     0       => { println!("zero"); }
    //     Some(x) => { println!("{}", x); }
    //     _       => { }
    // }
    //
    // parse_match_arms() lives in exprs.rs and is shared with
    // match-as-expression — the arm syntax is identical in both forms.

    fn parse_match_stmt(&mut self) -> Result<Stmt, CitrusError> {
        self.expect(&Token::Match)?;

        let value = self.parse_expr_no_struct()?;
        self.expect(&Token::LBrace)?;
        let arms = self.parse_match_arms()?;
        self.expect(&Token::RBrace)?;

        Ok(Stmt::Match {
            value: Box::new(value),
            arms,
        })
    }

    // ─────────────────────────────────────────────────────────────────
    // EXPRESSION STATEMENT
    // ─────────────────────────────────────────────────────────────────
    //
    // Any expression used as a statement — must be terminated by ;
    //
    //   println!("hello");
    //   items.push(value);
    //   x += 1;
    //   result = compute(a, b);
    //
    // Block-terminating statements (if, match, while, for, loop) do
    // NOT need a semicolon — they end with } and are handled above.

    fn parse_expr_stmt(&mut self) -> Result<Stmt, CitrusError> {
        let expr = self.parse_expr()?;
        self.expect(&Token::Semicolon)?;
        Ok(Stmt::Expr(expr))
    }
}
