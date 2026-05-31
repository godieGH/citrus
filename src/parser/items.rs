// src/parser/items.rs
//
// Parses all top-level constructs — functions, structs, enums,
// traits, implement blocks, imports, statics, and modules.
//
// Every function here adds methods to Parser via a separate impl block.
// Rust allows multiple impl blocks for the same type across files.

use super::Parser;
use super::ast::*;
use crate::error::CitrusError;
use crate::lexer::Token;

impl Parser {
    // ── ITEM DISPATCHER ───────────────────────────────────────────────
    //
    // Called from parse() in mod.rs for each top-level item.
    // Checks for an optional `public` modifier first, then
    // looks at the next token to decide what kind of item this is.

    pub fn parse_item(&mut self) -> Result<Item, CitrusError> {
        // `public` is optional and applies to most items
        let is_public = self.eat(&Token::Public);

        match self.current() {
            Some(Token::Struct) => self.parse_struct(is_public),
            Some(Token::Enum) => self.parse_enum_def(is_public),
            Some(Token::Trait) => self.parse_trait(is_public),
            Some(Token::Implement) => self.parse_implement(is_public),
            Some(Token::Import) => self.parse_import(),
            Some(Token::Static) => self.parse_static(is_public),
            Some(Token::Module) => self.parse_module(is_public),

            // an identifier at the top level means a function
            // no keyword needed — name( is enough to identify it
            Some(Token::Identifier) => self.parse_function(is_public),

            _ => Err(self.error_expected(
                "function, struct, enum, trait, implement, import, static, or module".to_string(),
            )),
        }
    }

    // ═════════════════════════════════════════════════════════════════
    // FUNCTIONS
    // ═════════════════════════════════════════════════════════════════
    //
    // Citrus function syntax:
    //   name(params) -> RetType { body }
    //   name<T>(params) -> RetType where T implements Trait { body }
    //   public name(params) -> RetType { body }
    //
    // No `fn` keyword — the name followed by ( is enough.

    fn parse_function(&mut self, public: bool) -> Result<Item, CitrusError> {
        let def = self.parse_function_def(public)?;
        Ok(Item::Function(def))
    }

    // Separated from parse_function so impl blocks can reuse it.
    // Impl block methods are also function defs — same syntax.
    pub fn parse_function_def(&mut self, public: bool) -> Result<FunctionDef, CitrusError> {
        let span = self.span();

        // the function name — any identifier including `main`
        let name = self.expect_identifier()?;

        // optional generic params — <T, U>
        let generics = self.parse_generic_params()?;

        // parameter list — always present, may be empty
        self.expect(&Token::LParen)?;
        let params = self.parse_params()?;
        self.expect(&Token::RParen)?;

        // return type — always required in Citrus
        // use Void if nothing is returned
        self.expect(&Token::Arrow)?;
        let ret = self.parse_type()?;

        // optional where clause — where T implements Speak + Walk
        let where_clause = self.parse_where_clause()?;

        // the function body
        let body = self.parse_block()?;

        Ok(FunctionDef {
            public,
            name,
            generics,
            params,
            ret,
            where_clause,
            body,
            span,
        })
    }

    // ── PARAMETERS ────────────────────────────────────────────────────
    //
    // Parses a comma-separated list of parameters.
    // The caller is responsible for the surrounding ( ).
    //
    //   x as Int_32, y as UInt_32
    //   self
    //   mutable self, height as Int_32

    fn parse_params(&mut self) -> Result<Vec<Param>, CitrusError> {
        let mut params = Vec::new();

        // empty parameter list — ()
        if self.check(&Token::RParen) {
            return Ok(params);
        }

        params.push(self.parse_param()?);

        while self.eat(&Token::Comma) {
            // allow trailing comma before )
            if self.check(&Token::RParen) {
                break;
            }
            params.push(self.parse_param()?);
        }

        Ok(params)
    }

    fn parse_param(&mut self) -> Result<Param, CitrusError> {
        // `self` — immutable self reference
        if self.check(&Token::SelfKw) {
            self.advance();
            return Ok(Param {
                name: "self".to_string(),
                ty: TypeExpr::Void, // semantic stage handles self type
                mutable: false,
                is_self: true,
            });
        }

        // `mutable self` — mutable self reference
        // we peek at the next token to check if `mutable` is followed by `self`
        if self.check(&Token::Mutable) && matches!(self.peek_ahead(1), Some(Token::SelfKw)) {
            self.advance(); // consume `mutable`
            self.advance(); // consume `self`
            return Ok(Param {
                name: "self".to_string(),
                ty: TypeExpr::Void,
                mutable: true,
                is_self: true,
            });
        }

        // regular parameter — name as Type
        // e.g.  x as Int_32   or   msg as Text
        let name = self.expect_identifier()?;
        self.expect(&Token::As)?;
        let ty = self.parse_type()?;

        Ok(Param {
            name,
            ty,
            mutable: false,
            is_self: false,
        })
    }

    // ═════════════════════════════════════════════════════════════════
    // STRUCTS
    // ═════════════════════════════════════════════════════════════════
    //
    // struct Animal { name as Text, height as Int_32 }
    // public struct Box<T> { value as T }

    fn parse_struct(&mut self, public: bool) -> Result<Item, CitrusError> {
        let span = self.span();
        self.expect(&Token::Struct)?;

        let name = self.expect_identifier()?;
        let generics = self.parse_generic_params()?;

        self.expect(&Token::LBrace)?;

        let mut fields = Vec::new();

        // parse fields until we see }
        // each field:  name as Type
        while !self.check(&Token::RBrace) && !self.at_end() {
            let field_span = self.span();
            let field_name = self.expect_identifier()?;
            self.expect(&Token::As)?;
            let ty = self.parse_type()?;
            fields.push(StructField {
                name: field_name,
                ty,
            });

            // fields are separated by commas
            // trailing comma is allowed
            if !self.eat(&Token::Comma) {
                break;
            }
        }

        self.expect(&Token::RBrace)?;

        Ok(Item::Struct(StructDef {
            public,
            name,
            generics,
            fields,
            span,
        }))
    }

    // ═════════════════════════════════════════════════════════════════
    // ENUMS
    // ═════════════════════════════════════════════════════════════════
    //
    // enum Direction { North, South, East, West }
    // enum Shape { Circle(Float_32), Rectangle(Float_32, Float_32), Point }
    // enum Status { Active = 1, Inactive = 2 }
    // enum Message { Quit, Move { x as Int_32, y as Int_32 }, Write(Text) }

    fn parse_enum_def(&mut self, public: bool) -> Result<Item, CitrusError> {
        let span = self.span();
        self.expect(&Token::Enum)?;

        let name = self.expect_identifier()?;
        let generics = self.parse_generic_params()?;

        self.expect(&Token::LBrace)?;

        let mut variants = Vec::new();

        while !self.check(&Token::RBrace) && !self.at_end() {
            variants.push(self.parse_enum_variant()?);
            if !self.eat(&Token::Comma) {
                break;
            }
        }

        self.expect(&Token::RBrace)?;

        Ok(Item::Enum(EnumDef {
            public,
            name,
            generics,
            variants,
            span,
        }))
    }

    fn parse_enum_variant(&mut self) -> Result<EnumVariant, CitrusError> {
        let span = self.span();
        let name = self.expect_identifier()?;

        // look at what follows the variant name to determine its kind
        let kind = match self.current() {
            // Active = 1  — integer discriminant
            Some(Token::Equals) => {
                self.advance();

                // optional negative sign
                let negative = self.eat(&Token::Minus);

                match self.current() {
                    Some(Token::IntLiteral) => {
                        let lex = self.advance().unwrap();
                        let val = lex
                            .src
                            .parse::<i64>()
                            .map_err(|_| self.error_expected("integer discriminant".to_string()))?;
                        EnumVariantKind::Discriminant(if negative { -val } else { val })
                    }
                    _ => return Err(self.error_expected("integer after '='".to_string())),
                }
            }

            // Circle(Float_32)  — tuple variant with types
            Some(Token::LParen) => {
                self.advance();
                let mut types = Vec::new();

                if !self.check(&Token::RParen) {
                    types.push(self.parse_type()?);
                    while self.eat(&Token::Comma) {
                        if self.check(&Token::RParen) {
                            break;
                        }
                        types.push(self.parse_type()?);
                    }
                }

                self.expect(&Token::RParen)?;
                EnumVariantKind::Tuple(types)
            }

            // Move { x as Int_32, y as Int_32 }  — struct variant
            Some(Token::LBrace) => {
                self.advance();
                let mut fields = Vec::new();

                while !self.check(&Token::RBrace) && !self.at_end() {
                    let fname = self.expect_identifier()?;
                    self.expect(&Token::As)?;
                    let ty = self.parse_type()?;
                    fields.push(StructField { name: fname, ty });
                    if !self.eat(&Token::Comma) {
                        break;
                    }
                }

                self.expect(&Token::RBrace)?;
                EnumVariantKind::Struct(fields)
            }

            // North  — unit variant, nothing follows
            _ => EnumVariantKind::Unit,
        };

        Ok(EnumVariant { name, kind, span })
    }

    // ═════════════════════════════════════════════════════════════════
    // TRAITS
    // ═════════════════════════════════════════════════════════════════
    //
    // trait Speak { speak(self) -> Void; }
    // trait Describe {
    //     describe(self) -> Text;
    //     print_description(self) -> Void { println!("{}", self.describe()); }
    // }

    fn parse_trait(&mut self, public: bool) -> Result<Item, CitrusError> {
        let span = self.span();
        self.expect(&Token::Trait)?;

        let name = self.expect_identifier()?;
        let generics = self.parse_generic_params()?;

        self.expect(&Token::LBrace)?;

        let mut methods = Vec::new();

        while !self.check(&Token::RBrace) && !self.at_end() {
            methods.push(self.parse_trait_method()?);
        }

        self.expect(&Token::RBrace)?;

        Ok(Item::Trait(TraitDef {
            public,
            name,
            generics,
            methods,
            span,
        }))
    }

    fn parse_trait_method(&mut self) -> Result<TraitMethod, CitrusError> {
        let span = self.span();
        let name = self.expect_identifier()?;

        self.expect(&Token::LParen)?;
        let params = self.parse_params()?;
        self.expect(&Token::RParen)?;

        self.expect(&Token::Arrow)?;
        let ret = self.parse_type()?;

        // a method either has a default body { ... }
        // or just a signature ending with ;
        let default = if self.check(&Token::LBrace) {
            // default implementation provided
            Some(self.parse_block()?)
        } else {
            // signature only — no default
            self.expect(&Token::Semicolon)?;
            None
        };

        Ok(TraitMethod {
            name,
            params,
            ret,
            default,
            span,
        })
    }

    // ═════════════════════════════════════════════════════════════════
    // IMPLEMENT BLOCKS
    // ═════════════════════════════════════════════════════════════════
    //
    // Two forms:
    //   implement Animal { ... }              ← plain impl
    //   implement Speak for Animal { ... }    ← trait impl

    fn parse_implement(&mut self, _public: bool) -> Result<Item, CitrusError> {
        let span = self.span();
        self.expect(&Token::Implement)?;

        // read the first name — could be the trait or the target
        let first_name = self.expect_identifier()?;
        let generics = self.parse_generic_params()?;

        if self.eat(&Token::For) {
            // implement TraitName for TargetName { ... }
            let target = self.expect_identifier()?;
            let target_generics = self.parse_generic_params()?;
            let where_clause = self.parse_where_clause()?;

            self.expect(&Token::LBrace)?;
            let methods = self.parse_impl_methods()?;
            self.expect(&Token::RBrace)?;

            Ok(Item::ImplTrait(ImplTraitBlock {
                trait_name: first_name,
                target,
                generics,
                where_clause,
                methods,
                span,
            }))
        } else {
            // implement TargetName { ... }
            let where_clause = self.parse_where_clause()?;

            self.expect(&Token::LBrace)?;
            let methods = self.parse_impl_methods()?;
            self.expect(&Token::RBrace)?;

            Ok(Item::Implement(ImplBlock {
                target: first_name,
                generics,
                methods,
                span,
            }))
        }
    }

    // Parse the methods inside an implement block.
    // Each method is a full function definition — same syntax as top-level functions.
    // The only difference is that methods can have `self` params.
    fn parse_impl_methods(&mut self) -> Result<Vec<FunctionDef>, CitrusError> {
        let mut methods = Vec::new();

        while !self.check(&Token::RBrace) && !self.at_end() {
            // methods inside impl blocks can be public individually
            let is_public = self.eat(&Token::Public);
            let method = self.parse_function_def(is_public)?;
            methods.push(method);
        }

        Ok(methods)
    }

    // ═════════════════════════════════════════════════════════════════
    // IMPORTS
    // ═════════════════════════════════════════════════════════════════
    //
    // import math::calculate;
    // import animals::{Animal, Speak, Walk};
    // import animals::*;
    // import engine::http::Request;       ← nested path

    fn parse_import(&mut self) -> Result<Item, CitrusError> {
        let span = self.span();
        self.expect(&Token::Import)?;

        // The path is a sequence of identifiers separated by ::
        // We keep reading until we hit the terminal part
        // (a single item, a {list}, or a *)
        //
        // The loop works like this:
        //   read identifier
        //   expect ::
        //   if next is { or * → we are at the items, stop
        //   if next is identifier → could be more path OR the final item
        //     read identifier
        //     if next is :: → it was a path segment, loop
        //     if next is ; → it was the final item, done

        let mut path = Vec::new();

        // read the first module segment
        path.push(self.expect_identifier()?);

        loop {
            // every path segment must be followed by ::
            self.expect(&Token::PathSep)?;

            match self.current() {
                // import animals::*
                Some(Token::Star) => {
                    self.advance();
                    self.expect(&Token::Semicolon)?;
                    return Ok(Item::Import(ImportDecl {
                        path,
                        items: ImportItems::All,
                        span,
                    }));
                }

                // import animals::{Animal, Speak, Walk}
                Some(Token::LBrace) => {
                    self.advance();
                    let mut names = Vec::new();

                    names.push(self.expect_identifier()?);
                    while self.eat(&Token::Comma) {
                        if self.check(&Token::RBrace) {
                            break;
                        }
                        names.push(self.expect_identifier()?);
                    }

                    self.expect(&Token::RBrace)?;
                    self.expect(&Token::Semicolon)?;

                    return Ok(Item::Import(ImportDecl {
                        path,
                        items: ImportItems::Multiple(names),
                        span,
                    }));
                }

                // either another path segment or the final single item
                Some(Token::Identifier) => {
                    let name = self.expect_identifier()?;

                    if self.check(&Token::PathSep) {
                        // there is another :: so this was a path segment
                        // example: engine::http::Request — `http` leads to another ::
                        path.push(name);
                        // loop again to consume the next ::
                    } else {
                        // no :: follows — this is the final item
                        self.expect(&Token::Semicolon)?;
                        return Ok(Item::Import(ImportDecl {
                            path,
                            items: ImportItems::Single(name),
                            span,
                        }));
                    }
                }

                _ => {
                    return Err(self
                        .error_expected("import item — identifier, '*', or '{...}'".to_string()));
                }
            }
        }
    }

    // ═════════════════════════════════════════════════════════════════
    // STATIC CONSTANTS
    // ═════════════════════════════════════════════════════════════════
    //
    // static MAX_SCORE as UInt_32 = 9999;
    // public static PI as Float_64 = 3.14159;

    fn parse_static(&mut self, public: bool) -> Result<Item, CitrusError> {
        let span = self.span();
        self.expect(&Token::Static)?;

        let name = self.expect_identifier()?;
        self.expect(&Token::As)?;
        let ty = self.parse_type()?;
        self.expect(&Token::Equals)?;

        // the value must be an expression
        let value = self.parse_expr()?;
        self.expect(&Token::Semicolon)?;

        Ok(Item::Static(StaticDecl {
            public,
            name,
            ty,
            value,
            span,
        }))
    }

    // ═════════════════════════════════════════════════════════════════
    // MODULES
    // ═════════════════════════════════════════════════════════════════
    //
    // module geometry { ... }
    // public module animals { ... }

    fn parse_module(&mut self, public: bool) -> Result<Item, CitrusError> {
        let span = self.span();
        self.expect(&Token::Module)?;

        let name = self.expect_identifier()?;
        self.expect(&Token::LBrace)?;

        // a module contains items — same as a file
        let mut items = Vec::new();
        while !self.check(&Token::RBrace) && !self.at_end() {
            let item_span = self.span();
            let item = self.parse_item()?;
            items.push(Spanned {
                node: item,
                span: item_span,
            });
        }

        self.expect(&Token::RBrace)?;

        Ok(Item::Module(ModuleDecl {
            public,
            name,
            items,
            span,
        }))
    }
}
