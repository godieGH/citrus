// src/parser/types.rs
//
// Type expression parsing.
// Called whenever the grammar expects a type annotation —
// after `as`, after `->`, in struct fields, in parameters, etc.
//
// Examples of what we parse here:
//   Int_32
//   Text
//   Option<Int_32>
//   Vector<Text>
//   Result<Animal, Text>
//   &Int_32
//   &mutable Int_32
//   [UInt_8:5]
//   MyStruct
//   Box<T>

use super::Parser;
use super::ast::*;
use crate::error::CitrusError;
use crate::lexer::Token;

impl Parser {
    // ── MAIN TYPE ENTRY POINT ─────────────────────────────────────────
    //
    // Look at the current token and decide what kind of type it is.
    // This is the function all other parsers call when they need a type.

    pub fn parse_type(&mut self) -> Result<TypeExpr, CitrusError> {
        match self.current() {
            // reference type — starts with &
            Some(Token::Ampersand) => self.parse_ref_type(),

            // array type — starts with [
            Some(Token::LBracket) => self.parse_array_type(),

            // all built-in primitive types
            Some(Token::TypeText) => {
                self.advance();
                Ok(TypeExpr::Text)
            }
            Some(Token::TypeChar) => {
                self.advance();
                Ok(TypeExpr::Char)
            }
            Some(Token::TypeBool) => {
                self.advance();
                Ok(TypeExpr::Bool)
            }

            Some(Token::TypeInt8) => {
                self.advance();
                Ok(TypeExpr::Int8)
            }
            Some(Token::TypeInt32) => {
                self.advance();
                Ok(TypeExpr::Int32)
            }
            Some(Token::TypeInt64) => {
                self.advance();
                Ok(TypeExpr::Int64)
            }
            Some(Token::TypeInt128) => {
                self.advance();
                Ok(TypeExpr::Int128)
            }

            Some(Token::TypeUInt8) => {
                self.advance();
                Ok(TypeExpr::UInt8)
            }
            Some(Token::TypeUInt32) => {
                self.advance();
                Ok(TypeExpr::UInt32)
            }
            Some(Token::TypeUInt64) => {
                self.advance();
                Ok(TypeExpr::UInt64)
            }
            Some(Token::TypeUInt128) => {
                self.advance();
                Ok(TypeExpr::UInt128)
            }

            Some(Token::TypeFloat32) => {
                self.advance();
                Ok(TypeExpr::Float32)
            }
            Some(Token::TypeFloat64) => {
                self.advance();
                Ok(TypeExpr::Float64)
            }

            // a name — either a custom type or a generic like Option<T>
            // Option, Result, Vector, Animal, MyStruct, T ...
            Some(Token::Identifier) => self.parse_named_type(),
            Some(Token::LParen) => self.parse_tuple_type(),

            // anything else is not a valid type
            _ => Err(self.error_expected(
                "type (e.g. Int_32, Text, Option<T>, &Bool, [UInt_8:5])".to_string(),
            )),
        }
    }

    // ── REFERENCE TYPE ────────────────────────────────────────────────
    //
    // &Int_32          — immutable reference
    // &mutable Int_32  — mutable reference
    //
    // Token stream:
    //   Ampersand  Mutable?  <inner type>

    fn parse_ref_type(&mut self) -> Result<TypeExpr, CitrusError> {
        self.expect(&Token::Ampersand)?;

        // is there a `mutable` keyword after the `&`?
        let mutable = self.eat(&Token::Mutable);

        // now parse whatever type is being referenced
        let inner = self.parse_type()?;

        Ok(TypeExpr::Ref {
            mutable,
            inner: Box::new(inner),
        })
    }

    // ── ARRAY TYPE ────────────────────────────────────────────────────
    //
    // [UInt_8:5]   — array of 5 UInt_8 values
    //
    // Token stream:
    //   LBracket  <element type>  Colon  <integer>  RBracket

    fn parse_array_type(&mut self) -> Result<TypeExpr, CitrusError> {
        self.expect(&Token::LBracket)?;

        // the element type
        let element = self.parse_type()?;

        // the colon separating type from size
        self.expect(&Token::Colon)?;

        // the size — must be a plain positive integer
        let size = self.expect_int_literal()?;

        self.expect(&Token::RBracket)?;

        Ok(TypeExpr::Array {
            element: Box::new(element),
            size,
        })
    }

    // ── NAMED TYPE ────────────────────────────────────────────────────
    //
    // A named type is an identifier, optionally followed by <generics>.
    //
    // Animal                     — no generics
    // Option<Int_32>             — one generic
    // Result<Text, MyError>      — two generics
    // HashMap<Text, Vector<Int_32>> — nested generics
    //
    // Token stream for Option<Int_32>:
    //   Identifier("Option")  Lt  TypeInt32  Gt

    fn parse_named_type(&mut self) -> Result<TypeExpr, CitrusError> {
        // consume the name
        let name = self.expect_identifier()?;

        // is there a < following? if yes, parse generic arguments
        let generics = if self.check(&Token::Lt) {
            self.parse_generic_args()?
        } else {
            Vec::new()
        };

        Ok(TypeExpr::Named { name, generics })
    }

    // ── GENERIC ARGUMENTS ─────────────────────────────────────────────
    //
    // The <T, U, ...> part of a generic type.
    // Called after we see a < when parsing a named type.
    //
    // <Int_32>              — one arg
    // <Text, MyError>       — two args
    // <Vector<Int_32>>      — nested — parse_type handles this recursively
    //
    // Token stream for <Text, Int_32>:
    //   Lt  TypeText  Comma  TypeInt32  Gt

    pub fn parse_generic_args(&mut self) -> Result<Vec<TypeExpr>, CitrusError> {
        self.expect(&Token::Lt)?;

        let mut args = Vec::new();

        // parse the first argument — there must be at least one
        // (a bare <> with nothing inside is not valid)
        args.push(self.parse_type()?);

        // parse any additional arguments separated by commas
        while self.eat(&Token::Comma) {
            // allow a trailing comma — <Int_32,> is accepted
            // stop if we see > immediately after the comma
            if self.check(&Token::Gt) {
                break;
            }
            args.push(self.parse_type()?);
        }

        self.expect(&Token::Gt)?;

        Ok(args)
    }

    // ── GENERIC PARAMETERS ────────────────────────────────────────────
    //
    // The <T, U> in a *definition* — struct Box<T>, fn transform<T, U>
    // These are just plain names — not full types.
    // The actual constraints go in the where clause separately.
    //
    // Token stream for <T, U>:
    //   Lt  Identifier("T")  Comma  Identifier("U")  Gt
    //
    // Returns a Vec<String> of parameter names.
    // Called by parse_function, parse_struct, parse_enum, parse_trait.

    pub fn parse_generic_params(&mut self) -> Result<Vec<String>, CitrusError> {
        // if there is no < then this definition has no generics
        if !self.check(&Token::Lt) {
            return Ok(Vec::new());
        }

        self.expect(&Token::Lt)?;

        let mut params = Vec::new();
        params.push(self.expect_identifier()?);

        while self.eat(&Token::Comma) {
            if self.check(&Token::Gt) {
                break;
            }
            params.push(self.expect_identifier()?);
        }

        self.expect(&Token::Gt)?;

        Ok(params)
    }

    // ── WHERE CLAUSE ──────────────────────────────────────────────────
    //
    // The constraint list after a generic function or impl block.
    //
    // where T implements Speak
    // where T implements Speak + Walk
    // where T implements Speak + Walk, U implements Clone
    //
    // Token stream for `where T implements Speak + Walk`:
    //   Where  Identifier("T")  Implements  Identifier("Speak")
    //   Plus   Identifier("Walk")
    //
    // Called by parse_function and parse_impl_trait.

    pub fn parse_where_clause(&mut self) -> Result<Vec<WhereBound>, CitrusError> {
        if !self.check(&Token::Where) {
            return Ok(Vec::new());
        }

        self.expect(&Token::Where)?;

        let mut bounds = Vec::new();

        loop {
            // the generic parameter name — T, U, etc.
            let param = self.expect_identifier()?;

            self.expect(&Token::Implements)?;

            // first trait bound
            let mut traits = vec![self.expect_identifier()?];

            // additional bounds separated by +
            while self.eat(&Token::Plus) {
                traits.push(self.expect_identifier()?);
            }

            bounds.push(WhereBound {
                param,
                bounds: traits,
            });

            // multiple bounds are separated by comma
            // where T implements Speak, U implements Clone
            if !self.eat(&Token::Comma) {
                break;
            }

            // stop if next token is not an identifier
            // (the where clause is over)
            if !matches!(self.current(), Some(Token::Identifier)) {
                break;
            }
        }

        Ok(bounds)
    }

    fn parse_tuple_type(&mut self) -> Result<TypeExpr, CitrusError> {
        self.expect(&Token::LParen)?;

        // () — unit type
        if self.eat(&Token::RParen) {
            return Ok(TypeExpr::Tuple(vec![]));
        }

        let first = self.parse_type()?;

        // (T) — grouping only, not a tuple
        if self.check(&Token::RParen) {
            self.advance();
            return Ok(first);
        }

        // (T,) or (T, U, ...) — real tuple
        self.expect(&Token::Comma)?;
        let mut types = vec![first];

        while !self.check(&Token::RParen) && !self.at_end() {
            types.push(self.parse_type()?);
            if !self.eat(&Token::Comma) {
                break;
            }
        }

        self.expect(&Token::RParen)?;
        Ok(TypeExpr::Tuple(types))
    }
}
