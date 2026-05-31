pub use logos::Logos;

#[derive(Logos, Debug, PartialEq, Clone)]
#[logos(skip r"[ \t\r\n]+")]
#[logos(skip(r"//[^\n]*", allow_greedy = true))]
#[logos(skip(r"#[^\n]*", allow_greedy = true))]
pub enum Token {
    // --- Keywords ---
    #[token("let")]
    Let,
    #[token("as")]
    As,
    #[token("mutable")]
    Mutable,
    #[token("public")]
    Public,
    #[token("struct")]
    Struct,
    #[token("enum")]
    Enum,
    #[token("implement")]
    Implement,
    #[token("implements")]
    Implements,
    #[token("for")]
    For,
    #[token("trait")]
    Trait,
    #[token("where")]
    Where,
    #[token("return")]
    Return,
    #[token("true")]
    True,
    #[token("false")]
    False,
    #[token("if")]
    If,
    #[token("else")]
    Else,
    #[token("while")]
    While,
    #[token("loop")]
    Loop,
    #[token("break")]
    Break,
    #[token("continue")]
    Continue,
    #[token("match")]
    Match,
    #[token("import")]
    Import,
    #[token("module")]
    Module,
    #[token("self")]
    SelfKw,
    #[token("macro")]
    Macro,
    #[token("static")]
    Static,

    // --- Built-in Types ---
    #[token("Text")]
    TypeText,
    #[token("Char")]
    TypeChar,
    #[token("Bool")]
    TypeBool,

    #[token("Int_8")]
    #[token("i8")]
    TypeInt8,
    #[token("Int_16")]
    #[token("i16")]
    TypeInt16,
    #[token("Int_32")]
    #[token("i32")]
    TypeInt32,
    #[token("Int_64")]
    #[token("i64")]
    TypeInt64,
    #[token("Int_128")]
    #[token("i128")]
    TypeInt128,
    #[token("ISize")]
    #[token("isize")]
    TypeISize,

    #[token("UInt_8")]
    #[token("u8")]
    TypeUInt8,
    #[token("UInt_16")]
    #[token("u16")]
    TypeUInt16,
    #[token("UInt_32")]
    #[token("u32")]
    TypeUInt32,
    #[token("UInt_64")]
    #[token("u64")]
    TypeUInt64,
    #[token("UInt_128")]
    #[token("u128")]
    TypeUInt128,
    #[token("USize")]
    #[token("usize")]
    TypeUSize,

    #[token("Float_32")]
    #[token("f32")]
    TypeFloat32,
    #[token("Float_64")]
    #[token("f64")]
    TypeFloat64,

    // --- Compound Assignment (must be before single operators) ---
    #[token("+=")]
    PlusAssign,
    #[token("-=")]
    MinusAssign,
    #[token("*=")]
    StarAssign,
    #[token("/=")]
    SlashAssign,
    #[token("%=")]
    PercentAssign,

    // --- Comparison (must be before < and >) ---
    #[token("==")]
    EqEq,
    #[token("!=")]
    NotEq,
    #[token("<=")]
    LtEq,
    #[token(">=")]
    GtEq,

    // --- Logical ---
    #[token("&&")]
    And,
    #[token("||")]
    Or,
    #[token("!")]
    Bang,

    // --- Bitwise ---
    #[token("<<")]
    Shl,
    #[token(">>")]
    Shr,
    #[token("|")]
    Pipe,
    #[token("^")]
    Caret,
    #[token("~")]
    Tilde,

    // --- Math ---
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("%")]
    Percent,

    // --- Symbols ---
    #[token("->")]
    Arrow,
    #[token("=")]
    Equals,
    #[token("&")]
    Ampersand,
    #[token("<")]
    Lt,
    #[token(">")]
    Gt,
    #[token(",")]
    Comma,
    #[token(";")]
    Semicolon,
    #[token(".")]
    Dot,
    #[token(":")]
    Colon,
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,

    // --- String Literals ---

    // R#"..."# — raw hash string
    #[regex(r##"R#"[^"]*"#"##)]
    RawHashString,

    // R"..." — raw string
    #[regex(r##"R"[^"]*""##)]
    RawString,

    // "..." — regular string, handles escape sequences like \"  \\  \n
    #[regex(r#""([^"\\]|\\.)*""#)]
    StringLiteral,

    // '.' — char literal, handles escape sequences like '\n'
    #[regex(r"'([^'\\]|\\.)'")]
    CharLiteral,

    // --- Numeric Literals ---

    // hex — must come before plain int
    #[regex(r"0x[0-9a-fA-F][0-9a-fA-F_]*")]
    HexLiteral,

    // binary
    #[regex(r"0b[01][01_]*")]
    BinaryLiteral,

    // octal
    #[regex(r"0o[0-7][0-7_]*")]
    OctalLiteral,

    // float — must come before int
    #[regex(r"[0-9][0-9_]*\.[0-9][0-9_]*")]
    FloatLiteral,

    // integer
    #[regex(r"[0-9][0-9_]*")]
    IntLiteral,

    #[token("::")]
    PathSep,
    #[token("@")]
    At,
    #[token("?")]
    Question,
    #[token("=>")]
    FatArrow,
    #[token("$")]
    Dollar,

    #[token("in")]
    In,
    #[token("..=")]
    RangeInclusive,
    #[token("..")]
    Range,

    // --- Identifier ---
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*")]
    Identifier,
}
