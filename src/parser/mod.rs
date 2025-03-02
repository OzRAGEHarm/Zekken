#![allow(dead_code)]

use crate::ast::*;
use crate::lexer::*;

pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
    source_lines: Vec<String>,
}

impl Parser {
    pub fn new() -> Self {
        Parser {
            tokens: Vec::new(),
            current: 0,
            source_lines: Vec::new(),
        }
    }

    fn skip_comments(&mut self) -> bool {
        if matches!(self.at().kind, TokenType::SingleLineComment | TokenType::MultiLineComment) {
            self.consume();
            true
        } else {
            false
        }
    }

    pub fn produce_ast(&mut self, source_code: String) -> Program {
        self.source_lines = source_code.lines().map(String::from).collect();
        self.tokens = tokenize(source_code);
    
        let start_location = self.at().location();
        let mut program = Program {
            imports: Vec::new(),
            comments: Vec::new(),
            content: Vec::new(),
            location: start_location,
        };
    
        // First pass: collect imports and comments
        while self.not_eof() {
            match self.at().kind {
                TokenType::SingleLineComment | TokenType::MultiLineComment => {
                    program.comments.push(self.at().value.clone());
                    self.consume();
                },
                TokenType::Use | TokenType::Include => {
                    program.imports.push(self.parse_stmt());
                },
                TokenType::EOF => break,
                _ => {
                    // Parse any other statements as program content
                    program.content.push(Box::new(self.parse_stmt()));
                }
            }
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
            let line_content = self.source_lines.get(token.line - 1)
                .unwrap_or(&String::from("<unknown>"))
                .clone();
            
            let pointer = " ".repeat(token.column - 1) + "\x1b[1;31m^\x1b[0m";
            let filename = std::env::var("ZEKKEN_CURRENT_FILE").unwrap_or_else(|_| String::from("<unknown>"));

            let token_value = if token.kind == TokenType::EOF {
                String::from("End Of File")
            } else {
                token.value
            };
            
            let error = format!(
                 "\x1b[1;31mSyntax Error\x1b[0m: {}\n\
                 \x1b[1;90m  ┌─\x1b[0m \x1b[1;37m{}\x1b[0m\n\
                 \x1b[1;90m  ├─[\x1b[0m Line \x1b[1;37m{}\x1b[0m, Column \x1b[1;37m{}\x1b[0m \x1b[1;90m]\x1b[0m\n\
                 \x1b[1;90m  │\x1b[0m\n\
                 \x1b[1;90m{:>4} │\x1b[0m {}\n\
                 \x1b[1;90m     │\x1b[0m {}\n\
                 \x1b[1;90m     │\x1b[0m\n\
                 \x1b[1;90m     │\x1b[0m Expected: \x1b[1;32m{:?}\x1b[0m\n\
                 \x1b[1;90m     │\x1b[0m Found:    \x1b[1;31m{:?}\x1b[0m (\"{}\")
                 \n
                 ",
                err,
                filename,
                token.line,
                token.column,
                token.line,
                line_content,
                pointer,
                type_,
                token.kind,
                token_value,
            );
            panic!("{}", error);
        }
        self.consume();
        token
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
            TokenType::Try => self.parse_try_catch_stmt(),
            _ => self.parse_expr(),
        }
    }

    fn parse_var_decl(&mut self) -> Content {
        let start_location = self.at().location();
        let constant = matches!(self.at().kind, TokenType::Const);
        self.consume();
        let ident = self.expect(TokenType::Identifier, "Expected variable identifier").value;

        if self.at().kind == TokenType::ThinArrow {
            self.parse_lambda_decl(constant, ident)
        } else {
            self.parse_normal_var_decl(constant, ident, start_location)
        }
    }

    fn parse_lambda_decl(&mut self, constant: bool, ident: String) -> Content {
        let start_location = self.at().location();
        self.expect(TokenType::ThinArrow, "Expected '->' after variable identifier");
        self.expect(TokenType::Pipe, "Expected '|' after '->'");
        let params = self.parse_params();
        self.expect(TokenType::Pipe, "Expected '|' after parameters");
        self.expect(TokenType::OpenBrace, "Expected '{' after parameters");
        let body = self.parse_block_stmt();
        self.expect(TokenType::CloseBrace, "Expected '}' after lambda body");
        
        Content::Statement(Box::new(Stmt::Lambda(LambdaDecl {
            constant,
            ident,
            params,
            body,
            location: start_location,
        })))
    }

    fn parse_normal_var_decl(&mut self, constant: bool, ident: String, start_location: Location) -> Content {
        self.expect(TokenType::Colon, "Expected ':' after variable identifier");
        let type_ = match self.at().kind {
            TokenType::DataType(DataType::Int) => {
                self.consume();
                DataType::Int
            }
            TokenType::DataType(DataType::Float) => {
                self.consume();
                DataType::Float
            }
            TokenType::DataType(DataType::String) => {
                self.consume();
                DataType::String
            }
            TokenType::DataType(DataType::Bool) => {
                self.consume();
                DataType::Bool
            }
            _ => {
                let token = self.expect(
                    TokenType::DataType(DataType::Any), 
                    "Expected type (int, float, string, bool) after ':'"
                );
                match token.kind {
                    TokenType::DataType(t) => t,
                    _ => unreachable!()
                }
            }
        };
        self.expect(TokenType::AssignOp(AssignOp::Assign), "Expected '=' after type declaration");
        let value = Some(self.parse_expr());
        self.expect(TokenType::Semicolon, "Expected ';' after variable declaration");
        Content::Statement(Box::new(Stmt::VarDecl(VarDecl { constant, ident, type_, value, location: start_location })))
    }

    fn parse_func_decl(&mut self) -> Content {
        let start_location = self.at().location();
        self.expect(TokenType::Func, "Expected 'func' keyword");
        let ident = self.expect(TokenType::Identifier, "Expected function identifier").value;
        self.expect(TokenType::Pipe, "Expected '|' after function identifier");
        let params = self.parse_params();
        self.expect(TokenType::Pipe, "Expected closing '|' after parameters");
        self.expect(TokenType::OpenBrace, "Expected '{' after parameters");
        let body = self.parse_block_stmt();
        self.expect(TokenType::CloseBrace, "Expected '}' after parameters");
    
        Content::Statement(Box::new(Stmt::FuncDecl(FuncDecl { params, ident, body, location: start_location })))
    }

    fn parse_params(&mut self) -> Vec<Param> {
        let mut params = Vec::new();
        while self.at().kind != TokenType::Pipe {
            let start_location = self.at().location();
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
                _ => {
                    let token = self.expect(
                        TokenType::DataType(DataType::Any), 
                        "Expected type (int, float, string, bool) after ':'"
                    );
                    match token.kind {
                        TokenType::DataType(t) => t,
                        _ => unreachable!()
                    }
                }
            };
    
            params.push(Param { ident, type_, location: start_location }); // Create a new Param instance
    
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
        
        body
    }

    fn parse_if_stmt(&mut self) -> Content {
        let start_location = self.at().location();
        self.expect(TokenType::If, "Expected 'if' keyword");
        
        let test = match self.parse_expr() {
            Content::Expression(expr) => expr,
            _ => panic!("Expected expression after 'if'"),
        };
        
        self.expect(TokenType::OpenBrace, "Expected '{' after condition"); // Expect the opening brace
        
        let body = self.parse_block_stmt(); // Parse the body of the if statement
        
        self.expect(TokenType::CloseBrace, "Expected '}' after if body"); // Expect the closing brace
        
        let alt = self.parse_else(); // Parse the else statement
        
        Content::Statement(Box::new(Stmt::IfStmt(IfStmt { test, body, alt, location: start_location })))
    }

    fn parse_else(&mut self) -> Option<Vec<Box<Content>>> {
        if self.at().kind == TokenType::Else {
            self.consume(); // Consume the else keyword
            
            if self.at().kind == TokenType::If {
                self.consume(); // Consume the if keyword
                
                let test = match self.parse_expr() {
                    Content::Expression(expr) => expr,
                    _ => panic!("Expected expression after 'else if'"),
                };
                
                self.expect(TokenType::OpenBrace, "Expected '{' after else if condition"); // Expect the opening brace
                
                let body = self.parse_block_stmt(); // Parse the body of the else if statement
                
                self.expect(TokenType::CloseBrace, "Expected '}' after else if body"); // Expect the closing brace
                
                let alt = self.parse_else(); // Recursively parse the next else statement
                
                return Some(vec![Box::new(Content::Statement(Box::new(Stmt::IfStmt(IfStmt {
                    test,
                    body,
                    alt,
                    location: self.at().location(),
                }))))]);
            } else {
                // If it's just else, we can parse the body
                self.expect(TokenType::OpenBrace, "Expected '{' after else"); // Expect the opening brace
                
                let body = self.parse_block_stmt(); // Parse the body of the else statement
                
                self.expect(TokenType::CloseBrace, "Expected '}' after else body"); // Expect the closing brace
                
                return Some(vec![Box::new(Content::Statement(Box::new(Stmt::BlockStmt(BlockStmt { body, location: self.at().location() }))))]);
            }
        }
        
        None
    }

    fn parse_for_stmt(&mut self) -> Content {
        let start_location = self.at().location();
        self.expect(TokenType::For, "Expected 'for' keyword");
    
        // Parse the variable declaration
        let ident = self.expect(TokenType::Identifier, "Expected identifier after 'for'").value;
        self.expect(TokenType::In, "Expected 'in' after identifier");
    
        // Parse the collection (which can be an identifier or an expression)
        let collection = self.parse_expr();
    
        // Expect the opening brace
        self.expect(TokenType::OpenBrace, "Expected '{' after for condition");
    
        // Parse the body of the loop
        let body = self.parse_block_stmt();

        self.expect(TokenType::CloseBrace, "Expected '}' after else body");
    
        Content::Statement(Box::new(Stmt::ForStmt(ForStmt {
            init: Some(Box::new(Stmt::VarDecl(VarDecl {
                constant: false, // Assuming it's not a constant declaration
                ident,
                type_: DataType::String,
                value: Some(collection),
                location: start_location.clone(),
            }))),
            test: None,
            update: None,
            body,
            location: start_location,
        })))
    }

    fn parse_while_stmt(&mut self) -> Content {
        let start_location = self.at().location();
        self.expect(TokenType::While, "Expected 'while' keyword");
        let test = match self.parse_expr() {
            Content::Expression(expr) => expr,
            _ => panic!("Expected expression"),
        };
        self.expect(TokenType::OpenBrace, "Expected '{' after while condition");
        let body = self.parse_block_stmt();
        self.expect(TokenType::CloseBrace, "Expected '}' after while body");
        Content::Statement(Box::new(Stmt::WhileStmt(WhileStmt { test, body, location: start_location })))
    }

    fn parse_use_stmt(&mut self) -> Content {
        let start_location = self.at().location().clone();
        self.expect(TokenType::Use, "Expected 'use' keyword");
        
        if self.at().kind == TokenType::OpenBrace {
            self.consume(); // Consume '{'
            
            let methods = self.parse_method_list();
            
            self.expect(TokenType::CloseBrace, "Expected '}' after method list");
            self.expect(TokenType::From, "Expected 'from' keyword after method list");
            
            let module = self.expect(TokenType::Identifier, "Expected module name after 'from'").value; // Expect the module name
            self.expect(TokenType::Semicolon, "Expected ';' after use statement");
    
            return Content::Statement(Box::new(Stmt::Use(UseStmt {
                methods: Some(methods),
                module,
                location: start_location,
            })));
        } else {
            let module = self.expect(TokenType::Identifier, "Expected module name").value; // Expect the module name
            self.expect(TokenType::Semicolon, "Expected ';' after use statement");
    
            return Content::Statement(Box::new(Stmt::Use(UseStmt {
                methods: None,
                module,
                location: start_location,
            })));
        }
    }

    fn parse_include_stmt(&mut self) -> Content {
        let start_location = self.at().location();
        self.expect(TokenType::Include, "Expected 'include' keyword");
    
        let methods = if self.at().kind == TokenType::OpenBrace {
            self.consume(); // Consume '{'
            let methods = self.parse_method_list();
            self.expect(TokenType::CloseBrace, "Expected '}' after method list");
            Some(methods)
        } else if self.at().kind == TokenType::Identifier {
            let method = self.expect(TokenType::Identifier, "Expected method name").value;
            self.expect(TokenType::From, "Expected 'from' keyword after method name");
            let file_path = self.expect(TokenType::String, "Expected file path after 'from'").value; // Expect the file path
            self.expect(TokenType::Semicolon, "Expected ';' after include statement");
    
            return Content::Statement(Box::new(Stmt::Include(IncludeStmt {
                methods: Some(vec![method]),
                file_path,
                location: start_location,
            })));
        } else if self.at().kind == TokenType::String {
            let file_path = self.expect(TokenType::String, "Expected file path").value;
            self.expect(TokenType::Semicolon, "Expected ';' after include statement");
    
            return Content::Statement(Box::new(Stmt::Include(IncludeStmt {
                methods: None,
                file_path,
                location: start_location,
            })));
        } else {
            panic!("Unexpected token after 'include'");
        };
    
        self.expect(TokenType::From, "Expected 'from' keyword after method list");
        let file_path = self.expect(TokenType::String, "Expected file path after 'from'").value;
        self.expect(TokenType::Semicolon, "Expected ';' after include statement");
    
        Content::Statement(Box::new(Stmt::Include(IncludeStmt {
            methods,
            file_path,
            location: start_location,
        })))
    }
    
    fn parse_method_list(&mut self) -> Vec<String> {
        let mut methods = Vec::new();
        
        while self.at().kind != TokenType::CloseBrace {
            let method = self.expect(TokenType::Identifier, "Expected method name").value;
            methods.push(method);
            
            if self.at().kind == TokenType::Comma {
                self.consume(); // Consume the comma
            } else {
                break;
            }
        }
        
        methods
    }

    fn parse_export_stmt(&mut self) -> Content {
        let start_location = self.at().location();
        self.expect(TokenType::Export, "Expected 'export' keyword");
        
        let mut exports = Vec::new();
        
        loop {
            let ident = self.expect(TokenType::Identifier, "Expected identifier after 'export'").value;
            exports.push(ident);
            
            if self.at().kind == TokenType::Comma {
                self.consume(); // Consume the comma
            } else {
                break;
            }
        }

        self.expect(TokenType::Semicolon, "Expected ';' after export statement");
        
        Content::Statement(Box::new(Stmt::Export(ExportStmt {
            exports,
            location: start_location,
        })))
    }

    fn parse_object_decl(&mut self) -> Content {
        let start_location = self.at().location().clone();
        self.expect(TokenType::Obj, "Expected 'obj' keyword");
        let ident = self.expect(TokenType::Identifier, "Expected object identifier").value;
        self.expect(TokenType::OpenBrace, "Expected '{' after object identifier");
        let properties = self.parse_object_properties();
        self.expect(TokenType::CloseBrace, "Expected '}' to close object declaration");
        Content::Statement(Box::new(Stmt::ObjectDecl(ObjectDecl { ident, properties, location: start_location })))
    }

    fn parse_return_stmt(&mut self) -> Content {
        let start_location = self.at().location();
        self.expect(TokenType::Return, "Expected 'return' keyword");

        let value = if self.at().kind != TokenType::Semicolon {
            match self.parse_expr() {
                Content::Expression(expr) => Some(Box::new(Content::Expression(expr))),
                _ => panic!("Expected expression after 'return'"),
            }
        } else {
            None
        };

        self.expect(TokenType::Semicolon, "Expected ';' after return statement");
        
        Content::Statement(Box::new(Stmt::Return(ReturnStmt { value, location: start_location })))
    }

    fn parse_try_catch_stmt(&mut self) -> Content {
        let start_location = self.at().location();
        self.expect(TokenType::Try, "Expected 'try' keyword");
    
        // Parse the try block
        self.expect(TokenType::OpenBrace, "Expected '{' after 'try'");
        let try_block = self.parse_block_stmt();
        self.expect(TokenType::CloseBrace, "Expected '}' after try block");
    
        // Parse the catch clause
        self.expect(TokenType::Catch, "Expected 'catch' keyword");
        self.expect(TokenType::Pipe, "Expected '|' after 'catch'");
        
        // Parse the catch parameter
        let _param_ident = self.expect(TokenType::Identifier, "Expected identifier in catch clause").value;
        self.expect(TokenType::Pipe, "Expected '|' after catch parameter");
        
        self.expect(TokenType::OpenBrace, "Expected '{' after catch clause");
        let catch_block = self.parse_block_stmt();
        self.expect(TokenType::CloseBrace, "Expected '}' after catch block");
    
        Content::Statement(Box::new(Stmt::TryCatchStmt(TryCatchStmt {
            try_block,
            catch_block: Some(catch_block),
            location: start_location,
        })))
    }

    fn parse_object_properties(&mut self) -> Vec<Property> {
        let mut properties = Vec::new();
        while self.at().kind != TokenType::CloseBrace {
            let start_location = self.at().location();
            let key = self.expect(TokenType::Identifier, "Expected property key").value;
            self.expect(TokenType::Colon, "Expected ':' after property key");
            let value = match self.parse_expr() {
                Content::Expression(expr) => expr,
                _ => panic!("Expected expression for property value"),
            };
            properties.push(Property { key, value, location: start_location });
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

        Content::Expression(Box::new(Expr::StringLit(StringLit { value: string_content, location: quote_token.location() })))
    }

    fn parse_expr(&mut self) -> Content {
        self.parse_assignment_expr()
    }

    fn parse_assignment_expr(&mut self) -> Content {
        let mut expr = self.parse_binary_expr();
    
        while self.at().kind != TokenType::Semicolon && self.at().kind != TokenType::CloseBrace && self.at().kind != TokenType::CloseParen {
            if matches!(self.at().kind, TokenType::AssignOp(_)) {
                let _operator = self.at().kind.clone();
                self.consume(); // Consume the assignment operator
                let right = self.parse_binary_expr(); // Parse the right-hand side expression
                if let (Content::Expression(left_expr), Content::Expression(right_expr)) = (expr, right) {
                    let assign_expr = AssignExpr { left: left_expr, right: right_expr, location: self.at().location() };
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

        while self.not_eof() && !matches!(
            self.at().kind,
            TokenType ::Semicolon | TokenType::CloseBrace | TokenType::CloseParen
        ) {
            let operator = if matches!(self.at().kind, TokenType::BinOp(_) | TokenType::ArithOp(_)) {
                let op = match &self.at().kind {
                    TokenType::ArithOp(arith_op) => match arith_op {
                        ArithOp::Add => "+",
                        ArithOp::Sub => "-",
                        ArithOp::Mul => "*",
                        ArithOp::Div => "/",
                        ArithOp::Mod => "%",
                    }.to_string(),
                    TokenType::BinOp(bin_op) => match bin_op {
                        BinOp::And => "&&",
                        BinOp::Or => "||",
                        BinOp::Not => "!",
                        BinOp::Eq => "==",
                        BinOp::Neq => "!=",
                        BinOp::Less => "<",
                        BinOp::Greater => ">",
                        BinOp::LessEq => "<=",
                        BinOp::GreaterEq => ">=",
                    }.to_string(),
                    _ => unreachable!(),
                };
                self.consume(); // Consume the operator
                op
            } else {
                break;
            };

            let right = self.parse_primary_expr();
            if let (Content::Expression(left_expr), Content::Expression(right_expr)) = (expr, right) {
                expr = Content::Expression(Box::new(Expr::Binary(BinaryExpr {
                    left: left_expr,
                    right: right_expr,
                    operator,
                    location: self.at().location(),
                })));
            } else {
                panic!("Expected expressions in binary operation");
            }
        }

        expr
    }

    fn parse_primary_expr(&mut self) -> Content {
        match self.at().kind {
            TokenType::Identifier => {
                let ident = Identifier { name: self.expect(TokenType::Identifier, "Expected identifier").value, location: self.at().location() };
                
                // Check for call expression syntax
                if self.at().kind == TokenType::FatArrow {
                    self.consume(); // Consume =>
                    self.expect(TokenType::Pipe, "Expected '|' after =>");
                    
                    let mut args = Vec::new();
                    while self.at().kind != TokenType::Pipe {
                        args.push(Box::new(match self.parse_expr() {
                            Content::Expression(e) => *e,
                            _ => panic!("Expected expression in call arguments"),
                        }));
                        
                        if self.at().kind == TokenType::Comma {
                            self.consume();
                        } else {
                            break;
                        }
                    }
                    self.expect(TokenType::Pipe, "Expected closing '|' in call expression");
                    
                    return Content::Expression(Box::new(Expr::Call(CallExpr {
                        callee: Box::new(Expr::Identifier(ident)),
                        args,
                        location: self.at().location(),
                    })));
                }
                
                Content::Expression(Box::new(Expr::Identifier(ident)))
            },
            TokenType::Int => {
                let int_lit = self.expect(TokenType::Int, "Expected integer literal");
                Content::Expression(Box::new(Expr::IntLit(IntLit { value: int_lit.value.parse().unwrap(), location: int_lit.location() })))
            },
            TokenType::Float => {
                let float_lit = self.expect(TokenType::Float, "Expected float literal");
                Content::Expression(Box::new(Expr::FloatLit(FloatLit { value: float_lit.value.parse().unwrap(), location: float_lit.location() })))
            },
            TokenType::String => {
                let string_token = self.expect(TokenType::String, "Expected string literal");
                Content::Expression(Box::new(Expr::StringLit(StringLit { value: string_token.value.clone(), location: string_token.location() })))
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