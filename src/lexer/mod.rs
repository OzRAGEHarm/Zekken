#![allow(dead_code)]

use crate::ast::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArithOp {
    Add,            // +
    Sub,            // -
    Mul,            // *
    Div,            // /
    Mod,            // %
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    And,            // &&
    Or,             // ||
    Not,            // !
    Eq,             // ==
    Neq,            // !=
    Less,           // <
    Greater,        // >
    LessEq,         // <=
    GreaterEq,      // >=
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignOp {
    Assign,         // =
    AddAssign,      // +=
    SubAssign,      // -=
    MulAssign,      // *=
    DivAssign,      // /=
    ModAssign,      // %=
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataType {
    Int,
    Float,
    String,
    Bool,
    Object,
    Array,
    Any,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenType {
    // Literals
    Int,
    Float,
    Identifier,
    String,
    Boolean(bool),

    // Data Types
    DataType(DataType),

    // Keywords
    Let,
    Const,
    Func,
    If,
    Else,
    For,
    While,
    Use,
    Include,
    Export,
    In,
    From,
    Return,
    Try,
    Catch,
    
    // Grouping
    At,
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

    // Operators
    ArithOp(ArithOp),
    BinOp(BinOp),
    AssignOp(AssignOp),
    ThinArrow,
    FatArrow,
    Pipe,
    Ampersand,

    // Comments
    SingleLineComment,
    MultiLineComment,

    Undefined,

    // End Of File
    EOF,
}

// Static HashMaps for keywords and token characters
pub static KEYWORDS: &[(&str, TokenType)] = &[
    ("let", TokenType::Let),
    ("const", TokenType::Const),
    ("func", TokenType::Func),
    ("if", TokenType::If),
    ("else", TokenType::Else),
    ("for", TokenType::For),
    ("while", TokenType::While),
    ("use", TokenType::Use),
    ("include", TokenType::Include),
    ("export", TokenType::Export),
    ("in", TokenType::In),
    ("from", TokenType::From),
    ("return", TokenType::Return),
    ("try", TokenType::Try),
    ("catch", TokenType::Catch),
    ("int", TokenType::DataType(DataType::Int)),
    ("float", TokenType::DataType(DataType::Float)),
    ("string", TokenType::DataType(DataType::String)),
    ("bool", TokenType::DataType(DataType::Bool)),
    ("obj", TokenType::DataType(DataType::Object)),
    ("arr", TokenType::DataType(DataType::Array)),
    ("true", TokenType::Boolean(true)),
    ("false", TokenType::Boolean(false)),
];

pub static TOKEN_CHAR: &[(&str, TokenType)] = &[
    ("@", TokenType::At),
    ("(", TokenType::OpenParen),
    (")", TokenType::CloseParen),
    ("{", TokenType::OpenBrace),
    ("}", TokenType::CloseBrace),
    ("[", TokenType::OpenBracket),
    ("]", TokenType::CloseBracket),
    (".", TokenType::Dot),
    (";", TokenType::Semicolon),
    (":", TokenType::Colon),
    (",", TokenType::Comma),
    ("|", TokenType::Pipe),
    ("->", TokenType::ThinArrow),
    ("=>", TokenType::FatArrow),
    ("&", TokenType::Ampersand),
    ("'", TokenType::SingleQuote),
    ("\"", TokenType::DoubleQuote),
    ("+", TokenType::ArithOp(ArithOp::Add)),
    ("-", TokenType::ArithOp(ArithOp::Sub)),
    ("*", TokenType::ArithOp(ArithOp::Mul)),
    ("%", TokenType::ArithOp(ArithOp::Mod)),
    ("/", TokenType::ArithOp(ArithOp::Div)),
    ("=", TokenType::AssignOp(AssignOp::Assign)),
    ("+=", TokenType::AssignOp(AssignOp::AddAssign)),
    ("-=", TokenType::AssignOp(AssignOp::SubAssign)),
    ("*=", TokenType::AssignOp(AssignOp::MulAssign)),
    ("/=", TokenType::AssignOp(AssignOp::DivAssign)),
    ("%=", TokenType::AssignOp(AssignOp::ModAssign)),
    ("||", TokenType::BinOp(BinOp::Or)),
    ("!", TokenType::BinOp(BinOp::Not)),
    ("&&", TokenType::BinOp(BinOp::And)),
    ("==", TokenType::BinOp(BinOp::Eq)),
    ("!=", TokenType::BinOp(BinOp::Neq)),
    (">=", TokenType::BinOp(BinOp::GreaterEq)),
    ("<=", TokenType::BinOp(BinOp::LessEq)),
    (">", TokenType::BinOp(BinOp::Greater)),
    ("<", TokenType::BinOp(BinOp::Less)),
];

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Token {
    pub value: String,
    pub kind: TokenType,
    pub line: usize,
    pub column: usize,
    pub length: usize,
}

impl Token {
    pub fn new(value: String, kind: TokenType, line: usize, column: usize) -> Token {
        let length = value.len();
        Token {
            value,
            kind,
            line,
            column,
            length,
        }
    }

    pub fn location(&self) -> Location {
        Location {
            line: self.line,
            column: self.column,
        }
    }
}

fn is_skippable(input: &str) -> bool {
    input.trim().is_empty()
}

pub fn tokenize(source: String) -> Vec<Token> {
    let src: Vec<char> = source.chars().collect();
    let mut tokens = Vec::new();
    let mut index: usize = 0;
    let len = src.len();
    let mut line = 1;
    let mut column = 1;

    while index < len {
        let c = src[index];
        index += 1;
        if c == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
        if c.is_whitespace() {
            continue;
        }
        if let Some(token) = tokenize_char(&src, &mut index, c, line, column) {
            tokens.push(token);
        }
    }
    tokens.push(Token::new("".to_string(), TokenType::EOF, line, column));
    tokens
}

fn tokenize_char(src: &Vec<char>, index: &mut usize, cur: char, line: usize, column: usize) -> Option<Token> {
    let len = src.len();
    // Handle comment cases (for '/' we check next char)
    if cur == '/' && *index < len {
        if src[*index] == '/' {
            return Some(parse_single_line_comment(src, index, line, column));
        } else if src[*index] == '*' {
            return Some(parse_multi_line_comment(src, index, line, column));
        }
    }
    // Check for 2-character tokens
    if *index < len {
        let next_char = src[*index];
        match (cur, next_char) {
            ('=', '>') => { *index += 1; return Some(Token::new("=>".to_string(), TokenType::FatArrow, line, column)); }
            ('-', '>') => { *index += 1; return Some(Token::new("->".to_string(), TokenType::ThinArrow, line, column)); }
            ('|', '|') => { *index += 1; return Some(Token::new("||".to_string(), TokenType::BinOp(BinOp::Or), line, column)); }
            ('&', '&') => { *index += 1; return Some(Token::new("&&".to_string(), TokenType::BinOp(BinOp::And), line, column)); }
            ('!', '=') => { *index += 1; return Some(Token::new("!=".to_string(), TokenType::BinOp(BinOp::Neq), line, column)); }
            ('=', '=') => { *index += 1; return Some(Token::new("==".to_string(), TokenType::BinOp(BinOp::Eq), line, column)); }
            ('<', '=') => { *index += 1; return Some(Token::new("<=".to_string(), TokenType::BinOp(BinOp::LessEq), line, column)); }
            ('>', '=') => { *index += 1; return Some(Token::new(">=".to_string(), TokenType::BinOp(BinOp::GreaterEq), line, column)); }
            _ => {}
        }
    }
    // Handle numbers
    if cur.is_digit(10) || (cur == '-' && *index < len && src[*index].is_digit(10)) {
        return Some(parse_number(src, index, cur, line, column, true));
    }
    // Handle string literals
    if cur == '"' || cur == '\'' {
        return Some(parse_string(src, index, cur, line, column));
    }
    // Check TOKEN_CHAR mapping (compare cur.to_string())
    if let Some(token_type) = TOKEN_CHAR.iter().find_map(|&(ref s, ref tt)| {
        if s == &cur.to_string() { Some(*tt) } else { None }
    }) {
        return Some(Token::new(cur.to_string(), token_type, line, column));
    }
    // Handle operators
    if let Some(token) = parse_operators(src, index, line, column, cur) {
        return Some(token);
    }
    // Handle identifier & keywords
    if cur.is_alphabetic() || cur == '_' {
        return Some(parse_identifier(src, index, cur, line, column));
    }
    None
}

fn parse_number(src: &Vec<char>, index: &mut usize, initial: char, line: usize, column: usize, mut is_integer: bool) -> Token {
    let mut num = initial.to_string();
    while *index < src.len() {
        let c = src[*index];
        if c.is_digit(10) {
            num.push(c);
            *index += 1;
        } else if c == '.' && is_integer {
            num.push(c);
            *index += 1;
            is_integer = false;
        } else {
            break;
        }
    }
    let token_type = if is_integer { TokenType::Int } else { TokenType::Float };
    Token::new(num, token_type, line, column)
}

fn parse_string(src: &Vec<char>, index: &mut usize, quote: char, line: usize, column: usize) -> Token {
    let mut content = String::new();
    let mut escaped = false;

    while *index < src.len() {
        let c = src[*index];
        *index += 1;

        if escaped {
            match c {
                'n' => content.push('\n'),
                't' => content.push('\t'),
                'r' => content.push('\r'),
                '\\' => content.push('\\'),
                '"' => content.push('"'),
                '\'' => content.push('\''),
                _ => content.push(c),
            }
            escaped = false;
        } else if c == '\\' {
            escaped = true;
        } else if c == quote {
            break;
        } else {
            content.push(c);
        }
    }
    
    Token::new(content, TokenType::String, line, column)
}

fn parse_single_line_comment(src: &Vec<char>, index: &mut usize, line: usize, column: usize) -> Token {
    let mut content = String::new();
    // Skip the already-consumed '/' and the next '/'
    *index += 1;
    while *index < src.len() && src[*index] != '\n' {
        content.push(src[*index]);
        *index += 1;
    }
    Token::new(content, TokenType::SingleLineComment, line, column)
}

fn parse_multi_line_comment(src: &Vec<char>, index: &mut usize, mut line: usize, mut column: usize) -> Token {
    let mut content = String::new();
    // Skip the already-consumed '*' after '/'
    *index += 1;
    while *index < src.len() {
        let c = src[*index];
        *index += 1;
        if c == '*' && *index < src.len() && src[*index] == '/' {
            *index += 1;
            break;
        }
        if c == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
        content.push(c);
    }
    Token::new(content, TokenType::MultiLineComment, line, column)
}

fn parse_operators(src: &Vec<char>, index: &mut usize, line: usize, column: usize, cur: char) -> Option<Token> {
    match cur {
        '=' => parse_operator(src, index, line, column, "=", TokenType::AssignOp(AssignOp::Assign)),
        '!' => parse_operator(src, index, line, column, "!", TokenType::BinOp(BinOp::Not)),
        '&' => parse_operator(src, index, line, column, "&", TokenType::Ampersand),
        '|' => parse_operator(src, index, line, column, "|", TokenType::Pipe),
        '+' => parse_operator(src, index, line, column, "+", TokenType::ArithOp(ArithOp::Add)),
        '-' => parse_operator(src, index, line, column, "-", TokenType::ArithOp(ArithOp::Sub)),
        '*' => parse_operator(src, index, line, column, "*", TokenType::ArithOp(ArithOp::Mul)),
        '/' => parse_operator(src, index, line, column, "/", TokenType::ArithOp(ArithOp::Div)),
        '%' => parse_operator(src, index, line, column, "%", TokenType::ArithOp(ArithOp::Mod)),
        '<' => parse_operator(src, index, line, column, "<", TokenType::BinOp(BinOp::Less)),
        '>' => parse_operator(src, index, line, column, ">", TokenType::BinOp(BinOp::Greater)),
        _ => None,
    }
}

fn parse_operator(src: &Vec<char>, index: &mut usize, line: usize, column: usize, single: &str, single_type: TokenType) -> Option<Token> {
    let mut token_val = single.to_string();
    if *index < src.len() && src[*index] == single.chars().next().unwrap() {
        token_val.push(src[*index]);
        *index += 1;
    }
    Some(Token::new(token_val, single_type, line, column))
}

fn parse_identifier(src: &Vec<char>, index: &mut usize, initial: char, line: usize, column: usize) -> Token {
    let mut ident = initial.to_string();
    while *index < src.len() {
        let c = src[*index];
        if c.is_alphanumeric() || c == '_' {
            ident.push(c);
            *index += 1;
        } else {
            break;
        }
    }
    let token_type = KEYWORDS
        .iter()
        .find_map(|&(kw, ref tt)| if kw == ident { Some(*tt) } else { None })
        .unwrap_or(TokenType::Identifier);
    Token::new(ident, token_type, line, column)
}