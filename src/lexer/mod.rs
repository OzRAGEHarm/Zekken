#![allow(dead_code)]

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
pub enum TokenType {
    // Literals
    Int,            // Integer (1, 2, etc.)
    Float,          // Float (1.1, 2.2, etc.)
    Identifier,     // Identifier (x, add, etc.)
    String,         // String ("Hello World!", "This is a string", etc.)

    // Keywords
    Let,
    Const,
    Func,
    If,
    Else,
    Then,
    For,
    While,
    Use,
    Include,
    Export,
    Obj,
    In,
    From,
    Return,

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

    // Comments
    SingleLineComment,
    MultiLineComment,

    // End Of File
    EOF,
    
    // Additional Operators
    ThinArrow,      // ->
    FatArrow,       // =>
    Pipe,           // |
    Ampersand,      // &
}

// Static HashMaps for keywords and token characters
pub static KEYWORDS: &[(&str, TokenType)] = &[
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
    ("from", TokenType::From),
    ("return", TokenType::Return),
];

pub static TOKEN_CHAR: &[(&str, TokenType)] = &[
    ("(", TokenType::OpenParen),
    (")", TokenType::CloseParen),
    ("{", TokenType::OpenBrace),
    ("}", TokenType::CloseBrace),
    ("[", TokenType::OpenBracket),
    ("]", TokenType::CloseBracket),
    ("+", TokenType::ArithOp(ArithOp::Add)),
    ("-", TokenType::ArithOp(ArithOp::Sub)),
    ("*", TokenType::ArithOp(ArithOp::Mul)),
    ("%", TokenType::ArithOp(ArithOp::Mod)),
    ("/", TokenType::ArithOp(ArithOp::Div)),
    (".", TokenType::Dot),
    (";", TokenType::Semicolon),
    (":", TokenType::Colon),
    (",", TokenType::Comma),
    ("||", TokenType::BinOp(BinOp::Or)),
    ("|", TokenType::Pipe), // Single pipe can also be a bitwise OR
    ("->", TokenType::ThinArrow),
    ("=>", TokenType::FatArrow),
    ("=", TokenType::AssignOp(AssignOp::Assign)),
    ("!", TokenType::BinOp(BinOp::Not)),
    ("&&", TokenType::BinOp(BinOp::And)),
    ("&", TokenType::Ampersand), // Single ampersand can also be a bitwise AND
    ("==", TokenType::BinOp(BinOp::Eq)),
    ("!=", TokenType::BinOp(BinOp::Neq)),
    (">=", TokenType::BinOp(BinOp::GreaterEq)),
    ("<=", TokenType::BinOp(BinOp::LessEq)),
    ("'", TokenType::SingleQuote),
    ("\"", TokenType::DoubleQuote),
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
}

fn is_skippable(input: &str) -> bool {
    input.trim().is_empty()
}

pub fn tokenize(source: String) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut src: Vec<_> = source.chars().collect();
    let mut line = 1;
    let mut column = 1;

    while !src.is_empty() {
        let char = src.remove(0);
    
        if char == '\n' {
            line += 1;
            column = 1;
        } else if char.is_whitespace() {
            column += 1;
        }
    
        if is_skippable(char.to_string().as_str()) {
            continue;
        }
    
        // Tokenize using a helper function
        if let Some(token) = tokenize_char(&mut src, char, line, column) {
            column += token.length;
            tokens.push(token);
        }
    }

    // Add EOF token
    tokens.push(Token::new("".to_string(), TokenType::EOF, line, column));
    
    tokens
}

fn tokenize_char(src: &mut Vec<char>, char: char, line: usize, column: usize) -> Option<Token> {
    
    // Check for token character types
    if char == '/' {
        if let Some(&next_char) = src.get(0) {
            if next_char == '/' {
                return Some(parse_single_line_comment(src, line, column + 1));
            } else if next_char == '*' {
                return Some(parse_multi_line_comment(src, line, column + 1));
            }
        }
    }

    // Check for 2-character tokens
    if let Some(&next_char) = src.get(0) {
        match (char, next_char) {
            ('=', '>') => {
                src.remove(0);
                return Some(Token::new("=>".to_string(), TokenType::FatArrow, line, column));
            }
            ('-', '>') => {
                src.remove(0);
                return Some(Token::new("->".to_string(), TokenType::ThinArrow, line, column));
            }
            ('|', '|') => {
                src.remove(0);
                return Some(Token::new("||".to_string(), TokenType::BinOp(BinOp::Or), line, column));
            }
            ('&', '&') => {
                src.remove(0);
                return Some(Token::new("&&".to_string(), TokenType::BinOp(BinOp::And), line, column));
            }
            ('!', '=') => {
                src.remove(0);
                return Some(Token::new("!=".to_string(), TokenType::BinOp(BinOp::Neq), line, column));
            }
            ('=', '=') => {
                src.remove(0);
                return Some(Token::new("==".to_string(), TokenType::BinOp(BinOp::Eq), line, column));
            }
            ('<', '=') => {
                src.remove(0);
                return Some(Token::new("<=".to_string(), TokenType::BinOp(BinOp::LessEq), line, column));
            }
            ('>', '=') => {
                src.remove(0);
                return Some(Token::new(">=".to_string(), TokenType::BinOp(BinOp::GreaterEq), line, column));
            }
            _ => {}
        }
    }

    // Check for token character types
    if let Some(token_type) = TOKEN_CHAR.iter().find_map(|&(c, ref token_type)| {
        if c == char.to_string() { Some(*token_type) } else { None }
    }) {
        return Some(Token::new(char.to_string(), token_type, line, column));
    }

    // Handle numbers (integer)
    if char.is_digit(10) || (char == '-' && !src.is_empty() && src[0].is_digit(10)) {
        return Some(parse_number(src, char, line, column, true)); // Check for integer
    }

    // Handle string literals
    if char == '"' || char == '\'' {
        return Some(parse_string(src, line, column));
    }

    // Handle operators and keywords
    if let Some(token) = parse_operators(src, line, column, char) {
        return Some(token);
    }

    if char.is_alphabetic() || char == '_' {
        return Some(parse_identifier(src, char, line, column));
    }

    None
}

fn parse_number(src: &mut Vec<char>, initial_char: char, line: usize, column: usize, mut is_integer: bool) -> Token {
    let mut num = initial_char.to_string();
    
    while let Some(&next_char) = src.get(0) {
        if next_char.is_digit(10) {
            num.push(src.remove(0));
        } else if next_char == '.' && is_integer {
            num.push(src.remove(0));
            is_integer = false;
        } else {
            break;
        }
    }

    // Determine if we are adding an `Int` or `Float` token
    let token_type = if is_integer { TokenType::Int } else { TokenType::Float };
    
    Token::new(num.clone(), token_type, line, column)
}

fn parse_string(src: &mut Vec<char>, line: usize, column: usize) -> Token {
    let mut string_content = String::new();
    let quote_char = if src[0] == '"' { '"' } else { '\'' };

    src.remove(0); // Remove the first quote character
    while let Some(&next_char) = src.get(0) {
        if next_char == quote_char {
            src.remove(0); // Remove the closing quote
            break;
        } else if next_char == '\\' {
            src.remove(0); // Remove the backslash
            if let Some(&escaped_char) = src.get(0) {
                string_content.push('\\');
                string_content.push(escaped_char);
                src.remove(0); // Remove the escaped character
            } else {
                panic!("Unterminated escape sequence in string literal");
            }
        } else {
            string_content.push(next_char);
            src.remove(0); // Remove the character
        }
    }

    Token::new(string_content, TokenType::String, line, column)
}

fn parse_single_line_comment(src: &mut Vec<char>, line: usize, column: usize) -> Token {
    let mut comment_content = String::new();
    let mut column = column;

    src.remove(0); // Remove the second '/' character

    while let Some(&next_char) = src.get(0) {
        if next_char == '\n' {
            break; // End of comment at new line
        }
        comment_content.push(src.remove(0));
        column += 1;
    }

    Token::new(comment_content, TokenType::SingleLineComment, line, column)
}

fn parse_multi_line_comment(src: &mut Vec<char>, line: usize, column: usize) -> Token {
    let mut comment_content = String::new();
    let mut line = line;
    let mut column = column;

    src.remove(0); // Remove the '*' character

    loop {
        if let Some(&next_char) = src.get(0) {
            if next_char == '*' {
                src.remove(0); 
                if let Some(&next_after_star) = src.get(0) {
                    if next_after_star == '/' {
                        src.remove(0);
                        break; // End of comment
                    }
                }
            } else if next_char == '\n' {
                line += 1;
                column = 0;
            } else {
                comment_content.push(next_char);
                column += 1;
            }
            src.remove(0);
        } else {
            panic!("Unterminated multi-line comment");
        }
    }

    Token::new(comment_content, TokenType::MultiLineComment, line, column)
}

fn parse_operators(src: &mut Vec<char>, line: usize, column: usize, char: char) -> Option<Token> {
    match char {
        '=' => parse_operator(src, line, column, "=", TokenType::AssignOp(AssignOp::Assign)),
        '!' => parse_operator(src, line, column, "!", TokenType::BinOp(BinOp::Not)),
        '&' => parse_operator(src, line, column, "&", TokenType::Ampersand), // Single ampersand
        '|' => parse_operator(src, line, column, "|", TokenType::Pipe), // Single pipe
        '+' => parse_operator(src, line, column, "+", TokenType::ArithOp(ArithOp::Add)),
        '-' => parse_operator(src, line, column, "-", TokenType::ArithOp(ArithOp::Sub)),
        '*' => parse_operator(src, line, column, "*", TokenType::ArithOp(ArithOp::Mul)),
        '/' => parse_operator(src, line, column, "/", TokenType::ArithOp(ArithOp::Div)),
        '%' => parse_operator(src, line, column, "%", TokenType::ArithOp(ArithOp::Mod)),
        '<' => parse_operator(src, line, column, "<", TokenType::BinOp(BinOp::Less)),
        '>' => parse_operator(src, line, column, ">", TokenType::BinOp(BinOp::Greater)),
        _ => None,
    }
}

fn parse_operator(src: &mut Vec<char>, line: usize, column: usize, single: &str, single_type: TokenType) -> Option<Token> {
    let mut token_value = single.to_string();
    let next_char = src.get(0).cloned();

    if next_char.filter(|&c| c == single.chars().next().unwrap()).is_some() {
        src.remove(0);
        token_value.push(single.chars().next().unwrap());
        return Some(Token::new(token_value, single_type, line, column));
    }

    Some(Token::new(token_value, single_type, line, column))
}

fn parse_identifier(src: &mut Vec<char>, initial_char: char, line: usize, column: usize) -> Token {
    let mut keyword = initial_char.to_string();
    
    while let Some(&next_char) = src.get(0) {
        if next_char.is_alphanumeric() || next_char == '_' {
            keyword.push(src.remove(0));
        } else {
            break;
        }
    }

    let token_type = KEYWORDS
        .iter()
        .find_map(|&(kw, ref token_type)| { if kw == keyword { Some(*token_type) } else { None } })
        .unwrap_or(TokenType::Identifier);
    
    Token::new(keyword.clone(), token_type, line, column)
}