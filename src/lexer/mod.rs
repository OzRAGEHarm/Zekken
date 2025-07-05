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
    Fn,
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
    ("fn", TokenType::DataType(DataType::Fn)),
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

    pub fn with_length(mut self, length: usize) -> Self {
        self.length = length;
        self
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

        // Handle newlines
        if c == '\n' {
            line += 1;
            column = 1;
            index += 1;
            continue;
        }

        // Handle whitespace (except newlines)
        if c.is_whitespace() && c != '\n' {
            index += 1;
            column += 1;
            continue;
        }

        // Get token
        if let Some((token, consumed)) = tokenize_char(&src, index, line, column) {
            // Update line/column for multi-line tokens
            //let mut lines = token.value.lines();
            //let first_line = lines.next();
            let num_newlines = token.value.matches('\n').count();
            if num_newlines > 0 {
                line += num_newlines;
                // Set column to the length of the last line + 1
                let last_line_len = token.value.rsplit('\n').next().unwrap_or("").len();
                column = last_line_len + 1;
            } else {
                column += consumed;
            }
            tokens.push(token);
            index += consumed;
        } else {
            index += 1;
            column += 1;
        }
    }

    tokens.push(Token::new("".to_string(), TokenType::EOF, line, column));
    tokens
}

fn tokenize_char(src: &Vec<char>, start: usize, line: usize, column: usize) -> Option<(Token, usize)> {
    let len = src.len();
    if start >= len {
        return None;
    }

    let cur = src[start];

    // Handle comments first - must check before any operator parsing
    if cur == '/' && start + 1 < len {
        let next_char = src[start + 1];
        if next_char == '/' {
            let mut idx = start + 2;
            let mut content = String::new();
            while idx < len && src[idx] != '\n' {
                content.push(src[idx]);
                idx += 1;
            }
            return Some((
                Token::new(content, TokenType::SingleLineComment, line, column)
                    .with_length(idx - start),
                idx - start
            ));
        } else if next_char == '*' {
            let mut idx = start + 2;
            let mut content = String::new();
            while idx < len - 1 {
                if src[idx] == '*' && src[idx + 1] == '/' {
                    idx += 2;
                    break;
                }
                content.push(src[idx]);
                idx += 1;
            }
            return Some((
                Token::new(content, TokenType::MultiLineComment, line, column)
                    .with_length(idx - start),
                idx - start
            ));
        }
    }

    // Check for multi-character tokens like '=>' and '->'
    if start + 1 < len {
        let two_chars = format!("{}{}", cur, src[start + 1]);
        if two_chars == "//" || two_chars == "/*" {
            // Already handled above
        } else {
            for &(ch, ref token_type) in TOKEN_CHAR.iter() {
                if ch == two_chars {
                    return Some((Token::new(ch.to_string(), *token_type, line, column), 2));
                }
            }
        }
    }

    // Check for identifiers
    if cur.is_alphabetic() || cur == '_' {
        let token = parse_identifier(src, start, line, column);
        let consumed = token.value.len();
        return Some((token, consumed));
    }

    // Check for operators
    if let Some(token) = parse_operators(src, start, line, column, cur) {
        // Only return '/' as ArithOp(Div) if not followed by '/' or '*'
        if cur == '/' && start + 1 < len {
            let next_char = src[start + 1];
            if next_char == '/' || next_char == '*' {
                // Already handled above
                return None;
            }
        }
        return Some((token, 1));
    }

    // Check for numbers
    if cur.is_digit(10) || (cur == '-' && start + 1 < len && src[start + 1].is_digit(10)) {
        let token = parse_number(src, start, line, column);
        let consumed = token.value.len();
        return Some((token, consumed));
    }

    // Check for strings
    if cur == '"' || cur == '\'' {
        let token = parse_string(src, start, line, column);
        return Some((token.clone(), token.length));
    }

    // Check for single character tokens like colon, comma, semicolon, etc.
    for &(ch, ref token_type) in TOKEN_CHAR.iter() {
        if ch.len() == 1 && ch.chars().next().unwrap() == cur {
            return Some((Token::new(ch.to_string(), *token_type, line, column), 1));
        }
    }

    None
}

fn parse_number(src: &Vec<char>, start: usize, line: usize, column: usize) -> Token {
    let mut num = String::new();
    let mut idx = start;
    let len = src.len();
    let mut is_integer = true;
    if src[idx] == '-' {
        num.push('-');
        idx += 1;
    }
    while idx < len {
        let c = src[idx];
        if c.is_digit(10) {
            num.push(c);
            idx += 1;
        } else if c == '.' && is_integer {
            num.push(c);
            idx += 1;
            is_integer = false;
        } else {
            break;
        }
    }
    let token_type = if is_integer { TokenType::Int } else { TokenType::Float };
    Token::new(num, token_type, line, column)
}

fn parse_string(src: &Vec<char>, start: usize, line: usize, column: usize) -> Token {
    let quote = src[start];
    let mut content = String::new();
    let mut escaped = false;
    let mut idx = start + 1;
    let len = src.len();

    while idx < len {
        let c = src[idx];
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
            idx += 1; // Consume the closing quote
            break;
        } else {
            content.push(c);
        }
        idx += 1;
    }
    let length = idx - start;
    Token::new(content, TokenType::String, line, column).with_length(length)
}

fn parse_identifier(src: &Vec<char>, start: usize, line: usize, column: usize) -> Token {
    let mut ident = String::new();
    let mut idx = start;
    let len = src.len();
    while idx < len {
        let c = src[idx];
        if c.is_alphanumeric() || c == '_' {
            ident.push(c);
            idx += 1;
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

fn parse_operators(_src: &Vec<char>, _start: usize, line: usize, column: usize, cur: char) -> Option<Token> {
    match cur {
        '=' => Some(Token::new("=".to_string(), TokenType::AssignOp(AssignOp::Assign), line, column)),
        '!' => Some(Token::new("!".to_string(), TokenType::BinOp(BinOp::Not), line, column)),
        '&' => Some(Token::new("&".to_string(), TokenType::Ampersand, line, column)),
        '|' => Some(Token::new("|".to_string(), TokenType::Pipe, line, column)),
        '+' => Some(Token::new("+".to_string(), TokenType::ArithOp(ArithOp::Add), line, column)),
        '-' => Some(Token::new("-".to_string(), TokenType::ArithOp(ArithOp::Sub), line, column)),
        '*' => Some(Token::new("*".to_string(), TokenType::ArithOp(ArithOp::Mul), line, column)),
        '/' => Some(Token::new("/".to_string(), TokenType::ArithOp(ArithOp::Div), line, column)),
        '%' => Some(Token::new("%".to_string(), TokenType::ArithOp(ArithOp::Mod), line, column)),
        '<' => Some(Token::new("<".to_string(), TokenType::BinOp(BinOp::Less), line, column)),
        '>' => Some(Token::new(">".to_string(), TokenType::BinOp(BinOp::Greater), line, column)),
        _ => None,
    }
}