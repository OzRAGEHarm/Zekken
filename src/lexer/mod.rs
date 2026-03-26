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
            // Track position using consumed source chars, not token value formatting.
            for ch in &src[index..index + consumed] {
                if *ch == '\n' {
                    line += 1;
                    column = 1;
                } else {
                    column += 1;
                }
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

fn tokenize_char(src: &[char], start: usize, line: usize, column: usize) -> Option<(Token, usize)> {
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

    // Multi-character operators.
    if start + 1 < len {
        let next = src[start + 1];
        let tk = match (cur, next) {
            ('-', '>') => Some(TokenType::ThinArrow),
            ('=', '>') => Some(TokenType::FatArrow),
            ('+', '=') => Some(TokenType::AssignOp(AssignOp::AddAssign)),
            ('-', '=') => Some(TokenType::AssignOp(AssignOp::SubAssign)),
            ('*', '=') => Some(TokenType::AssignOp(AssignOp::MulAssign)),
            ('/', '=') => Some(TokenType::AssignOp(AssignOp::DivAssign)),
            ('%', '=') => Some(TokenType::AssignOp(AssignOp::ModAssign)),
            ('|', '|') => Some(TokenType::BinOp(BinOp::Or)),
            ('&', '&') => Some(TokenType::BinOp(BinOp::And)),
            ('=', '=') => Some(TokenType::BinOp(BinOp::Eq)),
            ('!', '=') => Some(TokenType::BinOp(BinOp::Neq)),
            ('>', '=') => Some(TokenType::BinOp(BinOp::GreaterEq)),
            ('<', '=') => Some(TokenType::BinOp(BinOp::LessEq)),
            _ => None,
        };
        if let Some(kind) = tk {
            return Some((Token::new(format!("{}{}", cur, next), kind, line, column), 2));
        }
    }

    // Check for identifiers
    if cur.is_alphabetic() || cur == '_' {
        let token = parse_identifier(src, start, line, column);
        let consumed = token.value.len();
        return Some((token, consumed));
    }

    // Check for numbers
    if cur.is_ascii_digit() {
        let token = parse_number(src, start, line, column);
        let consumed = token.value.len();
        return Some((token, consumed));
    }

    // Check for strings
    if cur == '"' || cur == '\'' {
        let token = parse_string(src, start, line, column);
        return Some((token.clone(), token.length));
    }

    // Single-char tokens.
    let single = match cur {
        '@' => Some(TokenType::At),
        '(' => Some(TokenType::OpenParen),
        ')' => Some(TokenType::CloseParen),
        '{' => Some(TokenType::OpenBrace),
        '}' => Some(TokenType::CloseBrace),
        '[' => Some(TokenType::OpenBracket),
        ']' => Some(TokenType::CloseBracket),
        '.' => Some(TokenType::Dot),
        ';' => Some(TokenType::Semicolon),
        ':' => Some(TokenType::Colon),
        ',' => Some(TokenType::Comma),
        '|' => Some(TokenType::Pipe),
        '&' => Some(TokenType::Ampersand),
        '\'' => Some(TokenType::SingleQuote),
        '"' => Some(TokenType::DoubleQuote),
        '+' => Some(TokenType::ArithOp(ArithOp::Add)),
        '-' => Some(TokenType::ArithOp(ArithOp::Sub)),
        '*' => Some(TokenType::ArithOp(ArithOp::Mul)),
        '/' => Some(TokenType::ArithOp(ArithOp::Div)),
        '%' => Some(TokenType::ArithOp(ArithOp::Mod)),
        '=' => Some(TokenType::AssignOp(AssignOp::Assign)),
        '!' => Some(TokenType::BinOp(BinOp::Not)),
        '>' => Some(TokenType::BinOp(BinOp::Greater)),
        '<' => Some(TokenType::BinOp(BinOp::Less)),
        _ => None,
    };
    if let Some(kind) = single {
        return Some((Token::new(cur.to_string(), kind, line, column), 1));
    }

    None
}

fn parse_number(src: &[char], start: usize, line: usize, column: usize) -> Token {
    let mut num = String::new();
    let mut idx = start;
    let len = src.len();
    let mut is_integer = true;
    while idx < len {
        let c = src[idx];
        if c.is_ascii_digit() {
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

fn parse_string(src: &[char], start: usize, line: usize, column: usize) -> Token {
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
                '0' => content.push('\0'),
                'e' => content.push('\x1b'),
                '\\' => content.push('\\'),
                '"' => content.push('"'),
                '\'' => content.push('\''),
                'x' => {
                    // Hex escape: \xNN
                    if idx + 2 < len {
                        let h1 = src[idx + 1];
                        let h2 = src[idx + 2];
                        let hex = [h1, h2].iter().collect::<String>();
                        if let Ok(v) = u8::from_str_radix(&hex, 16) {
                            content.push(v as char);
                            idx += 2; // Consume both hex digits.
                        } else {
                            // Keep behavior resilient for malformed escapes.
                            content.push('x');
                        }
                    } else {
                        content.push('x');
                    }
                }
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

fn keyword_token(ident: &str) -> TokenType {
    match ident {
        "let" => TokenType::Let,
        "const" => TokenType::Const,
        "func" => TokenType::Func,
        "if" => TokenType::If,
        "else" => TokenType::Else,
        "for" => TokenType::For,
        "while" => TokenType::While,
        "use" => TokenType::Use,
        "include" => TokenType::Include,
        "export" => TokenType::Export,
        "in" => TokenType::In,
        "from" => TokenType::From,
        "return" => TokenType::Return,
        "try" => TokenType::Try,
        "catch" => TokenType::Catch,
        "int" => TokenType::DataType(DataType::Int),
        "float" => TokenType::DataType(DataType::Float),
        "string" => TokenType::DataType(DataType::String),
        "bool" => TokenType::DataType(DataType::Bool),
        "obj" => TokenType::DataType(DataType::Object),
        "arr" => TokenType::DataType(DataType::Array),
        "fn" => TokenType::DataType(DataType::Fn),
        "true" => TokenType::Boolean(true),
        "false" => TokenType::Boolean(false),
        _ => TokenType::Identifier,
    }
}

fn parse_identifier(src: &[char], start: usize, line: usize, column: usize) -> Token {
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
    let token_type = keyword_token(&ident);
    Token::new(ident, token_type, line, column)
}
