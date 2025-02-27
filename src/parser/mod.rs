#![allow(dead_code)]
use crate::ast::{*};
use crate::lexer::{*};

pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

impl Parser {
    pub fn new() -> Self {
        Parser {
            tokens: Vec::new(),
            current: 0,
        }
    }

    pub fn produce_ast(&mut self, source_code: String) -> Program {
        self.tokens = tokenize(source_code);

        for token in &self.tokens {
            println!("{:?}", token);
        }

        let mut program = Program { body: vec![] };

        while self.not_eof() {
            if self.skip_comments() {
                continue;
            }
            program.body.push(self.parse_stmt());
        }

        program
    }

    fn not_eof(&self) -> bool {
        self.current < self.tokens.len() && self.tokens[self.current].kind != TokenType::EOF
    }

    fn at(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn consume(&mut self) {
        if self.not_eof() {
            self.current += 1;
        }
    }

    fn expect(&mut self, type_: TokenType, err: &str) -> Token {
        let token = self.at().clone();
        if token.kind != type_ {
            panic!("Parser error: {}\nExpecting: {:?}, Got: {:?} at Line: {:?}, Column: {:?}", err, type_, token.value, token.line, token.column);
        }
        self.consume();
        token
    }

    fn skip_comments(&mut self) -> bool {
        match self.at().kind {
            TokenType::SingleLineComment | TokenType::MultiLineComment => {
                self.consume();
                true
            }
            _ => false
        }
    }

    fn parse_stmt(&mut self) -> Content {
        match self.at().kind {
            TokenType::Let | TokenType::Const => self.parse_var_decl(),
            TokenType::Func => self.parse_func_decl(),
            TokenType::If => self.parse_if_stmt(),
            TokenType::For => self.parse_for_stmt(),
            TokenType::While => self.parse_while_stmt(),
            TokenType::Use => self.parse_use_stmt(),
            TokenType::Include => self.parse_include_stmt(),
            TokenType::Export => self.parse_export_stmt(),
            TokenType::Obj => self.parse_object_decl(),
            TokenType::Return => self.parse_return_stmt(),
            _ => self.parse_expr(),
        }
    }

    fn parse_var_decl(&mut self) -> Content {
        let constant = matches!(self.at().kind, TokenType::Const);
        self.consume();
        let ident = self.expect(TokenType::Identifier, "Expected variable identifier").value;
        self.expect(TokenType::Colon, "Expected ':' after variable identifier");
        let type_ = match self.at().kind {
            TokenType::DataType(DataType::Int) => {
                self.consume();
                DataType::Int
            },
            TokenType::DataType(DataType::Float) => {
                self.consume();
                DataType::Float
            },
            TokenType::DataType(DataType::String) => {
                self.consume();
                DataType::String
            },
            TokenType::DataType(DataType::Bool) => {
                self.consume();
                DataType::Bool
            },
            _ => panic!("Expected type after ':', got: {:?}", self.at().kind),
        };
        self.expect(TokenType::AssignOp(AssignOp::Assign), "Expected '=' after type declaration");
        let value = Some(self.parse_expr());
        self.expect(TokenType::Semicolon, "Expected ';' after variable declaration");
        Content::Statement(Box::new(Stmt::VarDecl(VarDecl { constant, ident, type_, value })))
    }

    fn parse_func_decl(&mut self) -> Content {
        self.expect(TokenType::Func, "Expected 'func' keyword");
        let ident = self.expect(TokenType::Identifier, "Expected function identifier").value;
        self.expect(TokenType::Pipe, "Expected '|' after function identifier"); // Expect the opening pipe
        let params = self.parse_params(); // Parse parameters
        self.expect(TokenType::Pipe, "Expected '|' after parameters"); // Expect the closing pipe
        self.expect(TokenType::OpenBrace, "Expected '{' after parameters"); // Expect the opening brace
        let body = self.parse_block_stmt().into_iter().map(|b| *b).collect();
        Content::Statement(Box::new(Stmt::FuncDecl(FuncDecl { params, ident, body })))
    }

    fn parse_params(&mut self) -> Vec<Param> {
        let mut params = Vec::new();
        while self.at().kind != TokenType::Pipe {
            let ident = self.expect(TokenType::Identifier, "Expected parameter identifier").value;
            self.expect(TokenType::Colon, "Expected ':' after parameter identifier");
    
            // Expect a type token
            let type_ = match self.at().kind {
                TokenType::DataType(DataType::Int) => {
                    self.consume();
                    DataType::Int
                },
                TokenType::DataType(DataType::Float) => {
                    self.consume();
                    DataType::Float
                },
                TokenType::DataType(DataType::String) => {
                    self.consume();
                    DataType::String
                },
                TokenType::DataType(DataType::Bool) => {
                    self.consume();
                    DataType::Bool
                },
                _ => panic!("Expected type after ':', got: {:?}", self.at().kind),
            };
    
            params.push(Param { ident, type_ }); // Create a new Param instance
    
            if self.at().kind == TokenType::Comma {
                self.consume(); // Consume the comma
            } else {
                break;
            }
        }
        params
    }

    fn parse_block_stmt(&mut self) -> Vec<Box<Content>> {
        let mut body = Vec::new();
        while self.at().kind != TokenType::CloseBrace {
            body.push(Box::new(self.parse_stmt()));
        }
        self.expect(TokenType::CloseBrace, "Expected '}' to close block");
        body
    }

    fn parse_if_stmt(&mut self) -> Content {
        self.expect(TokenType::If, "Expected 'if' keyword");
        let test = match self.parse_expr() {
            Content::Expression(expr) => expr,
            _ => panic!("Expected expression"),
        };
        self.expect(TokenType::Then, "Expected 'then' after condition");
        let body = self.parse_block_stmt();
        let alt = if self.at().kind == TokenType::Else {
            self.consume();
            Some(self.parse_block_stmt().into_iter().collect())
        } else {
            None
        };
        Content::Statement(Box::new(Stmt::IfStmt(IfStmt { test, body, alt })))
    }

    fn parse_for_stmt(&mut self) -> Content {
        self.expect(TokenType::For, "Expected 'for' keyword");
        let init = if let Content::Statement(stmt) = self.parse_var_decl() {
            Some(stmt)
        } else {
            panic!("Expected statement in for loop initialization")
        };
        self.expect(TokenType::In, "Expected 'in' after for initialization");
        let test = match self.parse_expr() {
            Content::Expression(expr) => Some(expr),
            _ => panic!("Expected expression"),
        };
        let update = if self.at().kind != TokenType::OpenBrace {
            match self.parse_expr() {
                Content::Expression(expr) => Some(expr),
                _ => panic!("Expected expression"),
            }
        } else {
            None
        };
        self.expect(TokenType::OpenBrace, "Expected '{' after for condition");
        let body = self.parse_block_stmt();
        Content::Statement(Box::new(Stmt::ForStmt(ForStmt { init, test, update, body })))
    }

    fn parse_while_stmt(&mut self) -> Content {
        self.expect(TokenType::While, "Expected 'while' keyword");
        let test = match self.parse_expr() {
            Content::Expression(expr) => expr,
            _ => panic!("Expected expression"),
        };
        self.expect(TokenType::OpenBrace, "Expected '{' after while condition");
        let body = self.parse_block_stmt();
        Content::Statement(Box::new(Stmt::WhileStmt(WhileStmt { test, body })))
    }

    fn parse_use_stmt(&mut self) -> Content {
        self.expect(TokenType::Use, "Expected 'use' keyword");
        let ident = self.expect(TokenType::Identifier, "Expected identifier after 'use'").value;
        Content::Statement(Box::new(Stmt::Use(ident)))
    }

    fn parse_include_stmt(&mut self) -> Content {
        self.expect(TokenType::Include, "Expected 'include' keyword");
        let ident = self.expect(TokenType::Identifier, "Expected identifier after 'include'").value;
        Content::Statement(Box::new(Stmt::Include(ident)))
    }

    fn parse_export_stmt(&mut self) -> Content {
        self.expect(TokenType::Export, "Expected 'export' keyword");
        let mut exports = Vec::new();
        while self.at().kind != TokenType::EOF {
            let ident = self.expect(TokenType::Identifier, "Expected identifier after 'export'").value;
            exports.push(ident);
            if self.at().kind == TokenType::Comma {
                self.consume(); // Consume the comma
            } else {
                break;
            }
        }
        Content::Statement(Box::new(Stmt::Export(exports)))
    }

    fn parse_object_decl(&mut self) -> Content {
        self.expect(TokenType::Obj, "Expected 'obj' keyword");
        let ident = self.expect(TokenType::Identifier, "Expected object identifier").value;
        self.expect(TokenType::OpenBrace, "Expected '{' after object identifier");
        let properties = self.parse_object_properties();
        self.expect(TokenType::CloseBrace, "Expected '}' to close object declaration");
        Content::Statement(Box::new(Stmt::ObjectDecl(ObjectDecl { ident, properties })))
    }

    fn parse_return_stmt(&mut self) -> Content {
        self.expect(TokenType::Return, "Expected 'return' keyword");

        let value = if self.at().kind != TokenType::Semicolon {
            match self.parse_expr() {
                Content::Expression(expr) => Some(Box::new(Content::Expression(expr))),
                _ => panic!("Expected expression after 'return'"),
            }
        } else {
            None
        };
        
        Content::Statement(Box::new(Stmt::Return(ReturnStmt { value })))
    }

    fn parse_object_properties(&mut self) -> Vec<Property> {
        let mut properties = Vec::new();
        while self.at().kind != TokenType::CloseBrace {
            let key = self.expect(TokenType::Identifier, "Expected property key").value;
            self.expect(TokenType::Colon, "Expected ':' after property key");
            let value = match self.parse_expr() {
                Content::Expression(expr) => expr,
                _ => panic!("Expected expression for property value"),
            };
            properties.push(Property { key, value });
            if self.at().kind == TokenType::Comma {
                self.consume(); // Consume the comma
            } else {
                break;
            }
        }
        properties
    }

    fn parse_string_literal(&mut self) -> Content {
        let quote_token = self.at().clone(); // Capture the opening quote token
        self.consume(); // Consume the opening quote

        let mut string_content = String::new();

        while self.not_eof() {
            let current_token = self.at().clone();

            // Check for the closing quote
            if current_token.kind == quote_token.kind {
                self.consume(); // Consume the closing quote
                break;
            }

            string_content.push_str(&current_token.value);
            self.consume(); // Consume the current token
        }

        Content::Expression(Box::new(Expr::StringLit(StringLit { value: string_content })))
    }

    fn parse_expr(&mut self) -> Content {
        self.parse_assignment_expr()
    }

    fn parse_assignment_expr(&mut self) -> Content {
        let mut expr = self.parse_binary_expr();
    
        while self.at().kind != TokenType::Semicolon && self.at().kind != TokenType::CloseBrace && self.at().kind != TokenType::CloseParen {
            if matches!(self.at().kind, TokenType::AssignOp(_)) {
                let operator = self.at().kind.clone();
                self.consume(); // Consume the assignment operator
                let right = self.parse_binary_expr(); // Parse the right-hand side expression
                if let (Content::Expression(left_expr), Content::Expression(right_expr)) = (expr, right) {
                    let assign_expr = AssignExpr { left: left_expr, right: right_expr };
                    expr = Content::Expression(Box::new(Expr::Assign(assign_expr)));
                } else {
                    panic!("Expected expressions in assignment");
                }
            } else {
                break;
            }
        }
    
        expr // Return the parsed expression
    }
    
    fn parse_binary_expr(&mut self) -> Content {
        let mut expr = self.parse_primary_expr();
    
        while self.at().kind != TokenType::Semicolon && self.at().kind != TokenType::CloseBrace && self.at().kind != TokenType::CloseParen && !matches!(self.at().kind, TokenType::AssignOp(_)) {
            if matches!(self.at().kind, TokenType::BinOp(_)) {
                let operator = self.at().kind.clone();
                self.consume(); // Consume the binary operator
                let right = self.parse_primary_expr(); // Parse the right-hand side expression
                if let (Content::Expression(left_expr), Content::Expression(right_expr)) = (expr, right) {
                    let binary_expr = BinaryExpr {
                        left: left_expr,
                        right: right_expr,
                        operator: format!("{:?}", operator),
                    };
                    expr = Content::Expression(Box::new(Expr::Binary(binary_expr)));
                } else {
                    panic!("Expected expressions in binary operation");
                }
            } else {
                break;
            }
        }
    
        expr // Return the parsed expression
    }

    fn parse_primary_expr(&mut self) -> Content {
        match self.at().kind {
            TokenType::Identifier => {
                let ident = self.expect(TokenType::Identifier, "Expected identifier").value;
                Content::Expression(Box::new(Expr::Identifier(Identifier { name: ident })))
            },
            TokenType::Int => {
                let int_lit = self.expect(TokenType::Int, "Expected integer literal");
                Content::Expression(Box::new(Expr::IntLit(IntLit { value: int_lit.value.parse().unwrap() })))
            },
            TokenType::Float => {
                let float_lit = self.expect(TokenType::Float, "Expected float literal");
                Content::Expression(Box::new(Expr::FloatLit(FloatLit { value: float_lit.value.parse().unwrap() })))
            },
            TokenType::String => {
                let string_token = self.expect(TokenType::String, "Expected string literal");
                Content::Expression(Box::new(Expr::StringLit(StringLit { value: string_token.value })))
            },
            TokenType::OpenParen => {
                self.consume(); // Consume '('
                let expr = self.parse_expr();
                self.expect(TokenType::CloseParen, "Expected ')' after expression");
                expr
            },
            _ => panic!("Unexpected token: {:?}", self.at().kind),
        }
    }
}