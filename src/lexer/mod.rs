use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenType {
    // Literals
    Int,
    Float,
    Identifier,
    String,

    // Keywords
    Let,
    Const,
    Func,
    If,
    Else,
    Then,
    For,
    Use,
    Include,
    Export,
    Obj,
    In,

    // Grouping & Operators
    BinaryOperator,
    Assignment,
    Equal,
    Comma,
    Colon,
    Semicolon,
    Dot,
    OpenParen,
    CloseParen,
    OpenBrace,
    CloseBrace,
    OpenBracket,
    CloseBracket,
    SingleQuote,
    DoubleQuote,
    Greater,
    Less,
    EqualCompare,
    NotEqualCompare,
    Exclamation,
    And,
    Ampersand,
    Bar,
    Pipe,
    ThinArrow,
    FatArrow,

    // Comments
    SingleLineComment,
    MultiLineComment,

    // End Of File
    EOF,
}

// Static HashMaps for keywords and token characters
static KEYWORDS: &[(&str, TokenType)] = &[
    ("let", TokenType::Let),
    ("const", TokenType::Const),
    ("func", TokenType::Func),
    ("if", TokenType::If),
    ("else", TokenType::Else),
    ("then", TokenType::Then),
    ("for", TokenType::For),
    ("use", TokenType::Use),
    ("include", TokenType::Include),
    ("export", TokenType::Export),
    ("obj", TokenType::Obj),
    ("in", TokenType::In),
];

static TOKEN_CHAR: &[(&str, TokenType)] = &[
    ("(", TokenType::OpenParen),
    (")", TokenType::CloseParen),
    ("{", TokenType::OpenBrace),
    ("}", TokenType::CloseBrace),
    ("[", TokenType::OpenBracket),
    ("]", TokenType::CloseBracket),
    ("+", TokenType::BinaryOperator),
    ("-", TokenType::BinaryOperator),
    ("*", TokenType::BinaryOperator),
    ("%", TokenType::BinaryOperator),
    ("/", TokenType::BinaryOperator),
    ("<", TokenType::Less),
    (">", TokenType::Greater),
    (".", TokenType::Dot),
    (";", TokenType::Semicolon),
    (":", TokenType::Colon),
    (",", TokenType::Comma),
    ("||", TokenType::Bar),
    ("|", TokenType::Pipe),
    ("->", TokenType::ThinArrow),
    ("=>", TokenType::FatArrow),
    ("=", TokenType::Equal),
    ("!", TokenType::Exclamation),
    ("&&", TokenType::And),
    ("&", TokenType::Ampersand),
    ("==", TokenType::EqualCompare),
    ("!=", TokenType::NotEqualCompare),
    ("'", TokenType::SingleQuote),
    ("\"", TokenType::DoubleQuote),
];