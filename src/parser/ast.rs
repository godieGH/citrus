// src/parser/ast.rs

// ─────────────────────────────────────────────
// SPAN
// ─────────────────────────────────────────────
// Every node in the AST carries a Span — where
// it came from in the source file. This is how
// error messages say "error at line 4 col 12".
// We never throw away position information.

#[derive(Debug, Clone)]
pub struct Span {
    pub line: usize,
    pub col:  usize,
}

// Wraps any AST node with its source position.
// Instead of putting line/col inside every enum
// variant, we wrap the whole node once.
#[derive(Debug, Clone)]
pub struct Spanned<T> {
    pub node: T,
    pub span: Span,
}

// Convenience aliases — shorter names for the
// most commonly spanned types.
pub type SpannedExpr = Spanned<Expr>;
pub type SpannedStmt = Spanned<Stmt>;


// ─────────────────────────────────────────────
// LITERALS
// ─────────────────────────────────────────────
// A literal is a value written directly in source
// code — 60, "hello", true, 'F'.
// We keep them in their own enum so both Expr
// and Pattern can use them without duplication.

#[derive(Debug, Clone)]
pub enum Lit {
    Int(i128),          // covers all integer types — semantic stage narrows it
    Float(f64),         // covers Float_32 and Float_64
    Str(String),        // regular string "hello"
    RawStr(String),     // raw string R"hello"
    Char(char),         // character 'F'
    Bool(bool),         // true or false
}


// ─────────────────────────────────────────────
// TYPE EXPRESSIONS
// ─────────────────────────────────────────────
// A TypeExpr represents a type annotation written
// in source code — Int_32, Option<Text>, &mutable Bool.
// This is NOT the compiler's internal type — it is
// just the syntax the user wrote. The semantic stage
// resolves these into real types later.

#[derive(Debug, Clone)]
pub enum TypeExpr {
    // ── built-in primitives ──────────────────
    Text,
    Char,
    Bool,
    Void,
    Any,

    Int8,   Int32,   Int64,   Int128,
    UInt8,  UInt32,  UInt64,  UInt128,
    Float32, Float64,

    // ── named type, possibly generic ─────────
    // covers: Animal, Option<Int_32>, Result<T, E>
    // Vector<Text>, Box<T> etc.
    Named {
        name:     String,
        generics: Vec<TypeExpr>,
    },

    // ── reference types ──────────────────────
    // &Int_32 or &mutable Int_32
    Ref {
        mutable: bool,
        inner:   Box<TypeExpr>,     // Box because TypeExpr contains TypeExpr
    },

    // ── fixed-size array ─────────────────────
    // [UInt_8:5]
    Array {
        element: Box<TypeExpr>,
        size:    u64,
    },
}


// ─────────────────────────────────────────────
// OPERATORS
// ─────────────────────────────────────────────
// Kept as separate enums so the Expr variants
// stay clean. The semantic stage checks that the
// operator is valid for the operand types.

#[derive(Debug, Clone)]
pub enum BinaryOp {
    // math
    Add, Sub, Mul, Div, Mod,
    // comparison — produce Bool
    Eq, NotEq, Lt, Gt, LtEq, GtEq,
    // logical — operands must be Bool
    And, Or,
    // bitwise — operands must be integers
    BitAnd, BitOr, BitXor, Shl, Shr,
}

#[derive(Debug, Clone)]
pub enum UnaryOp {
    Neg,    // -x    numeric negation
    Not,    // !x    logical not — Bool only
    BitNot, // ~x    bitwise complement — integers only
}

#[derive(Debug, Clone)]
pub enum AssignOp {
    Assign,        // =
    AddAssign,     // +=
    SubAssign,     // -=
    MulAssign,     // *=
    DivAssign,     // /=
    ModAssign,     // %=
}


// ─────────────────────────────────────────────
// PATTERNS
// ─────────────────────────────────────────────
// Patterns appear in match arms.
// match value {
//     0        => ...   ← Literal pattern
//     Some(x)  => ...   ← EnumVariant pattern with binding
//     _        => ...   ← Wildcard pattern
// }

#[derive(Debug, Clone)]
pub enum Pattern {
    // _ — matches anything, binds nothing
    Wildcard,

    // a literal value — 0, "hello", true, 'F'
    Literal(Lit),

    // a plain name — either a variable binding or
    // a unit enum variant. The semantic stage decides which.
    // Example: None, x, my_var
    Identifier(String),

    // an enum variant, possibly with inner patterns
    // Direction::North        — path=["Direction","North"], fields=[]
    // Shape::Circle(r)        — path=["Shape","Circle"],    fields=[Identifier("r")]
    // Message::Move { x, y } — uses StructVariant below
    EnumVariant {
        path:   Vec<String>,
        fields: Vec<Pattern>,
    },

    // struct-style enum variant
    // Message::Move { x, y }
    StructVariant {
        path:   Vec<String>,
        fields: Vec<(String, Pattern)>,  // field name → pattern
    },

    // multiple patterns on one arm
    // 1 | 2 | 3 => { }
    Or(Vec<Pattern>),

    // range pattern in match
    // 0..=59 => { }
    Range {
        start:     Box<Pattern>,
        end:       Box<Pattern>,
        inclusive: bool,
    },
}


// ─────────────────────────────────────────────
// CAPTURE CLAUSE — anonymous functions
// ─────────────────────────────────────────────
// The [capture] part of [&](x) => x * 2

#[derive(Debug, Clone)]
pub enum CaptureClause {
    CopyAll,    // []          — copy everything (default, dangerous for non-Copy)
    RefAll,     // [&]         — borrow everything (recommended default)
    MutRefAll,  // [&mutable]  — mutably borrow everything
    MoveAll,    // [=]         — move everything

    // [&a, =b, c]  — per-variable explicit captures
    Explicit(Vec<ExplicitCapture>),
}

#[derive(Debug, Clone)]
pub struct ExplicitCapture {
    pub name: String,
    pub kind: ExplicitCaptureKind,
}

#[derive(Debug, Clone)]
pub enum ExplicitCaptureKind {
    Copy,       // bare name  — t
    Ref,        // &name      — &a
    MutRef,     // &mutable   — &mutable a
    Move,       // =name      — =c
}


// ─────────────────────────────────────────────
// EXPRESSIONS
// ─────────────────────────────────────────────
// An expression is anything that produces a value.
// Expressions can be nested — the left and right
// sides of BinaryOp are themselves Exprs, hence Box.
//
// Box<SpannedExpr> means:
//   Box    — heap allocated (needed for recursive types)
//   Spanned — carries line/col
//   Expr   — the actual expression node

#[derive(Debug, Clone)]
pub enum Expr {

    // ── literals ─────────────────────────────
    Literal(Lit),

    // ── a name — variable, function, constant ─
    Identifier(String),

    // ── binary operation ─────────────────────
    // x + y   x == y   x && y   etc.
    BinaryOp {
        left:  Box<SpannedExpr>,
        op:    BinaryOp,
        right: Box<SpannedExpr>,
    },

    // ── unary operation ──────────────────────
    // -x   !x   ~x
    UnaryOp {
        op:   UnaryOp,
        expr: Box<SpannedExpr>,
    },

    // ── assignment ───────────────────────────
    // x = 5   x += 1   etc.
    // target must resolve to something assignable
    Assign {
        target: Box<SpannedExpr>,
        op:     AssignOp,
        value:  Box<SpannedExpr>,
    },

    // ── member access ────────────────────────
    // animal.name
    FieldAccess {
        object: Box<SpannedExpr>,
        field:  String,
    },

    // ── index access ─────────────────────────
    // scores[0]
    IndexAccess {
        object: Box<SpannedExpr>,
        index:  Box<SpannedExpr>,
    },

    // ── method call ──────────────────────────
    // items.push(4)   animal.speak()
    // Separate from FieldAccess because it has args
    MethodCall {
        object: Box<SpannedExpr>,
        method: String,
        args:   Vec<CallArg>,
    },

    // ── function call ────────────────────────
    // add(1, 2)   transform<Animal>(my_animal)
    FunctionCall {
        name:     String,
        generics: Vec<TypeExpr>,    // the <T> in fn<T>(...)
        args:     Vec<CallArg>,
    },

    // ── reference ────────────────────────────
    // &x   &mutable x
    Ref {
        mutable: bool,
        expr:    Box<SpannedExpr>,
    },

    // ── range ────────────────────────────────
    // 0..10   0..=10
    Range {
        start:     Box<SpannedExpr>,
        end:       Box<SpannedExpr>,
        inclusive: bool,
    },

    // ── struct instantiation ─────────────────
    // Animal { name: "Lion", height: 120 }
    StructInit {
        name:     String,
        generics: Vec<TypeExpr>,
        fields:   Vec<FieldInit>,
    },

    // ── enum variant ─────────────────────────
    // Direction::North
    // Shape::Circle(5.0)
    // Message::Move { x: 10, y: 20 }
    EnumVariant {
        path:   Vec<String>,         // ["Direction", "North"]
        kind:   EnumVariantInit,
    },

    // ── anonymous function ───────────────────
    // [&](x as Int_32) => x * 2
    // [&](x as Int_32) -> Int_32 { return x * 2; }
    Closure {
        capture: CaptureClause,
        params:  Vec<ClosureParam>,
        ret:     Option<TypeExpr>,
        body:    ClosureBody,
    },

    // ── if as expression ─────────────────────
    // let x = if condition { "yes" } else { "no" };
    // else branch is required when used as expression
    IfExpr {
        condition:  Box<SpannedExpr>,
        then_block: Block,
        else_block: Box<Block>,     // required — no else means no value
    },

    // ── match as expression ──────────────────
    // let v = match result { Ok(v) => v, Err(_) => 0 };
    MatchExpr {
        value: Box<SpannedExpr>,
        arms:  Vec<MatchArm>,
    },

    // ── ? operator ───────────────────────────
    // result?   — propagates Err early
    Try(Box<SpannedExpr>),
}


// ─────────────────────────────────────────────
// EXPRESSION SUPPORT TYPES
// ─────────────────────────────────────────────

// A function/method call argument — positional or named
// add(10, y=20)
#[derive(Debug, Clone)]
pub enum CallArg {
    Positional(SpannedExpr),
    Named { name: String, value: SpannedExpr },
}

// A field in a struct initializer
// Animal { name: "Lion", height: 120 }
//          ^^^^^^^^^^^^  ^^^^^^^^^^^^^
#[derive(Debug, Clone)]
pub struct FieldInit {
    pub name:  String,
    pub value: SpannedExpr,
}

// How an enum variant is initialized
#[derive(Debug, Clone)]
pub enum EnumVariantInit {
    Unit,                              // Direction::North
    Tuple(Vec<SpannedExpr>),           // Shape::Circle(5.0)
    Struct(Vec<FieldInit>),            // Message::Move { x: 10, y: 20 }
}

// A closure parameter — type optional (can be inferred in shorthand)
// [&](x as Int_32)   or   [&](x)
#[derive(Debug, Clone)]
pub struct ClosureParam {
    pub name: String,
    pub ty:   Option<TypeExpr>,
}

// The body of a closure — either shorthand or full block
// [&](x) => x * 2          — Expr body
// [&](x) -> Int_32 { ... } — Block body
#[derive(Debug, Clone)]
pub enum ClosureBody {
    Expr(Box<SpannedExpr>),
    Block(Block),
}

// A match arm — pattern => body
#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub body:    MatchBody,
    pub span:    Span,
}

// Match arm body — either a block or a single expression
// 0 => { println!("zero"); }   — block
// 0 => "zero"                  — expression
#[derive(Debug, Clone)]
pub enum MatchBody {
    Block(Block),
    Expr(Box<SpannedExpr>),
}

// The else branch of an if statement
// else { }          — plain block
// else if ... { }   — chained if (recursive)
#[derive(Debug, Clone)]
pub enum ElseBranch {
    Block(Block),
    If(Box<SpannedStmt>),    // else if chains as nested Stmt::If
}


// ─────────────────────────────────────────────
// BLOCK
// ─────────────────────────────────────────────
// A block is a sequence of statements inside { }.
// Blocks appear in function bodies, if/else, loops.
// In Citrus a block does NOT implicitly return a value
// — you must use explicit return.

#[derive(Debug, Clone)]
pub struct Block {
    pub stmts: Vec<SpannedStmt>,
    pub span:  Span,
}


// ─────────────────────────────────────────────
// STATEMENTS
// ─────────────────────────────────────────────
// Statements are inside function bodies.
// They do things but do not produce a value
// (except if/match which can be used as either).

#[derive(Debug, Clone)]
pub enum Stmt {

    // ── variable declaration ─────────────────
    // let x as Int_32 = 60;
    // let mutable x as Int_32;
    Let {
        mutable: bool,
        name:    String,
        ty:      Option<TypeExpr>,   // None when inferred
        value:   Option<SpannedExpr>, // None when uninitialized
    },

    // ── return ───────────────────────────────
    // return x + y;
    // return;
    Return(Option<SpannedExpr>),

    // ── while loop ───────────────────────────
    // while x < 10 { x += 1; }
    While {
        condition: Box<SpannedExpr>,
        body:      Block,
    },

    // ── for-in loop ──────────────────────────
    // for i in 0..10 { }
    // for item in items { }
    // for i, item in items.enumerate() { }
    ForIn {
        var:        ForVar,         // single var or index+var
        iterable:   Box<SpannedExpr>,
        body:       Block,
    },

    // ── infinite loop ────────────────────────
    // loop { break; }
    Loop(Block),

    // ── break / continue ─────────────────────
    Break,
    Continue,

    // ── if as statement ──────────────────────
    // if x > 0 { } else if x == 0 { } else { }
    If {
        condition:   Box<SpannedExpr>,
        then_block:  Block,
        else_branch: Option<ElseBranch>,
    },

    // ── match as statement ───────────────────
    // match value { 0 => { } _ => { } }
    Match {
        value: Box<SpannedExpr>,
        arms:  Vec<MatchArm>,
    },

    // ── expression statement ─────────────────
    // An expression used as a statement — typically
    // a function call or assignment.
    // fn_call();   x += 1;
    Expr(SpannedExpr),
}

// The loop variable(s) in a for-in
// for i in ...           — Single
// for i, item in ...     — Indexed (index, value)
#[derive(Debug, Clone)]
pub enum ForVar {
    Single(String),
    Indexed { index: String, value: String },
}


// ─────────────────────────────────────────────
// TOP-LEVEL ITEMS
// ─────────────────────────────────────────────
// Items are declarations at the top level of a file
// or module. They are not inside any function.

#[derive(Debug, Clone)]
pub enum Item {
    Function(FunctionDef),
    Struct(StructDef),
    Enum(EnumDef),
    Trait(TraitDef),
    Implement(ImplBlock),
    ImplTrait(ImplTraitBlock),
    Import(ImportDecl),
    Static(StaticDecl),
    Module(ModuleDecl),
}


// ─────────────────────────────────────────────
// FUNCTION DEFINITION
// ─────────────────────────────────────────────
// add(x as Int_32, y as Int_32) -> Int_32 { }
// public transform<T>(item as T) -> T where T implements Speak { }

#[derive(Debug, Clone)]
pub struct FunctionDef {
    pub public:       bool,
    pub name:         String,
    pub generics:     Vec<String>,          // the T in fn<T>
    pub params:       Vec<Param>,
    pub ret:          TypeExpr,             // return type — Void if nothing
    pub where_clause: Vec<WhereBound>,      // where T implements Speak + Walk
    pub body:         Block,
    pub span:         Span,
}

// A function parameter — x as Int_32
// or self / mutable self
#[derive(Debug, Clone)]
pub struct Param {
    pub name:    String,
    pub ty:      TypeExpr,
    pub mutable: bool,         // mutable self
    pub is_self: bool,         // true when param is `self`
}

// A generic where bound — T implements Speak + Walk
#[derive(Debug, Clone)]
pub struct WhereBound {
    pub param:  String,          // T
    pub bounds: Vec<String>,     // ["Speak", "Walk"]
}


// ─────────────────────────────────────────────
// STRUCT DEFINITION
// ─────────────────────────────────────────────
// struct Animal { name as Text, height as Int_32 }
// public struct Box<T> { value as T }

#[derive(Debug, Clone)]
pub struct StructDef {
    pub public:   bool,
    pub name:     String,
    pub generics: Vec<String>,
    pub fields:   Vec<StructField>,
    pub span:     Span,
}

#[derive(Debug, Clone)]
pub struct StructField {
    pub name: String,
    pub ty:   TypeExpr,
}


// ─────────────────────────────────────────────
// ENUM DEFINITION
// ─────────────────────────────────────────────
// enum Direction { North, South, East, West }
// enum Shape { Circle(Float_32), Rectangle(Float_32, Float_32) }
// enum Status { Active = 1, Inactive = 2 }

#[derive(Debug, Clone)]
pub struct EnumDef {
    pub public:   bool,
    pub name:     String,
    pub generics: Vec<String>,
    pub variants: Vec<EnumVariant>,
    pub span:     Span,
}

#[derive(Debug, Clone)]
pub struct EnumVariant {
    pub name: String,
    pub kind: EnumVariantKind,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum EnumVariantKind {
    Unit,                           // North
    Tuple(Vec<TypeExpr>),           // Circle(Float_32)
    Struct(Vec<StructField>),       // Move { x as Int_32, y as Int_32 }
    Discriminant(i64),              // Active = 1
}


// ─────────────────────────────────────────────
// TRAIT DEFINITION
// ─────────────────────────────────────────────
// trait Speak { speak(self) -> Void; }
// trait Describe { describe(self) -> Text; print_description(self) -> Void { } }

#[derive(Debug, Clone)]
pub struct TraitDef {
    pub public:  bool,
    pub name:    String,
    pub generics: Vec<String>,
    pub methods: Vec<TraitMethod>,
    pub span:    Span,
}

// A method inside a trait — may have a default body or just a signature
#[derive(Debug, Clone)]
pub struct TraitMethod {
    pub name:    String,
    pub params:  Vec<Param>,
    pub ret:     TypeExpr,
    pub default: Option<Block>,    // None = signature only, Some = default impl
    pub span:    Span,
}


// ─────────────────────────────────────────────
// IMPLEMENT BLOCKS
// ─────────────────────────────────────────────

// implement Animal { ... }
#[derive(Debug, Clone)]
pub struct ImplBlock {
    pub target:   String,
    pub generics: Vec<String>,
    pub methods:  Vec<FunctionDef>,
    pub span:     Span,
}

// implement Speak for Animal { ... }
#[derive(Debug, Clone)]
pub struct ImplTraitBlock {
    pub trait_name:     String,
    pub target:         String,
    pub generics:       Vec<String>,
    pub where_clause:   Vec<WhereBound>,
    pub methods:        Vec<FunctionDef>,
    pub span:           Span,
}


// ─────────────────────────────────────────────
// IMPORT
// ─────────────────────────────────────────────
// import animals::Animal;
// import animals::{Animal, Speak};
// import animals::*;

#[derive(Debug, Clone)]
pub struct ImportDecl {
    pub path:  Vec<String>,      // ["animals"] from  animals::Animal
    pub items: ImportItems,
    pub span:  Span,
}

#[derive(Debug, Clone)]
pub enum ImportItems {
    Single(String),              // import math::calculate  → Single("calculate")
    Multiple(Vec<String>),       // import animals::{Animal, Speak}
    All,                         // import animals::*
}


// ─────────────────────────────────────────────
// STATIC CONSTANT
// ─────────────────────────────────────────────
// static MAX_SCORE as UInt_32 = 9999;
// public static PI as Float_64 = 3.14159;

#[derive(Debug, Clone)]
pub struct StaticDecl {
    pub public: bool,
    pub name:   String,
    pub ty:     TypeExpr,
    pub value:  SpannedExpr,
    pub span:   Span,
}


// ─────────────────────────────────────────────
// MODULE
// ─────────────────────────────────────────────
// module geometry { ... }

#[derive(Debug, Clone)]
pub struct ModuleDecl {
    pub public: bool,
    pub name:   String,
    pub items:  Vec<Spanned<Item>>,
    pub span:   Span,
}


// ─────────────────────────────────────────────
// PROGRAM — the root of the whole AST
// ─────────────────────────────────────────────
// A Citrus source file is a list of top-level items.
// The parser produces one Program per file.

#[derive(Debug, Clone)]
pub struct Program {
    pub items:    Vec<Spanned<Item>>,
    pub filename: String,
}
