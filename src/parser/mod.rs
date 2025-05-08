#![allow(dead_code)]

use crate::ast::*;
use crate::lexer::*;
use crate::errors::*;
use crate::ast::Location;

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

    pub fn produce_ast(&mut self, source_code: String) -> Program {
        let source_lines: Vec<String> = source_code.lines().map(String::from).collect();
        std::env::set_var("ZEKKEN_SOURCE_LINES", source_lines.join("\n"));
        
        let tokens = tokenize(source_code);
        let tokens_str = tokens
            .iter()
            .map(|t| format!("{:?}", t))
            .collect::<Vec<String>>()
            .join("\n");
        std::env::set_var("ZEKKEN_TOKENS", tokens_str);

        self.source_lines = source_lines;
        self.tokens = tokens;
    
        let start_location = self.at().location();
        let mut program = Program {
            imports: Vec::new(),
            content: Vec::new(),
            location: start_location,
        };
    
        // First pass: collect imports and comments
        while self.not_eof() {
            match self.at().kind {
                TokenType::SingleLineComment | TokenType::MultiLineComment => {
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

    fn skip_comments(&mut self) {
        if matches!(self.at().kind, TokenType::SingleLineComment | TokenType::MultiLineComment) {
            self.consume();
        }
    }

    fn not_eof(&self) -> bool {
        self.current < self.tokens.len() && self.tokens[self.current].kind != TokenType::EOF
    }

    fn consume(&mut self) {
        if self.not_eof() {
            self.current += 1;
        }
    }
    
    fn at(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn expect(&mut self, type_: TokenType, err: &str) -> Token {
        let token = self.at().clone();
        if token.kind != type_ {
            let expected = format!("{:?}", type_);
            let found = if token.kind == TokenType::EOF {
                String::from("End Of File")
            } else {
                format!("{:?} ({})", token.kind, token.value)
            };
            let error = ZekkenError::syntax(
                err,
                token.line,
                token.column,
                Some(&expected),
                Some(&found)
            );
            eprintln!("{}", error);
            std::process::exit(1);
        }
        self.consume(); // Consume the token after validating it
        token
    }

    fn parse_stmt(&mut self) -> Content {
        match self.at().kind {
            TokenType::SingleLineComment | TokenType::MultiLineComment => {
                self.skip_comments();
                return self.parse_stmt();
            }
            TokenType::Let | TokenType::Const => self.parse_var_decl(),
            TokenType::Func => self.parse_func_decl(),
            TokenType::If => self.parse_if_stmt(),
            TokenType::For => self.parse_for_stmt(),
            TokenType::While => self.parse_while_stmt(),
            TokenType::Use => self.parse_use_stmt(),
            TokenType::Include => self.parse_include_stmt(),
            TokenType::Export => self.parse_export_stmt(),
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
        self.parse_normal_var_decl(constant, ident, start_location)
    }

    fn parse_lambda_decl(&mut self, constant: bool, ident: String) -> Content {
        let start_location = self.at().location();
        
        self.consume(); // Consume the ->
        self.expect(TokenType::Pipe, "Expected '|' after '->'");
        let params = self.parse_params();
        
        self.expect(TokenType::Pipe, "Expected '|' after parameters");
        self.expect(TokenType::OpenBrace, "Expected '{' after parameters");
        let body = self.parse_block_stmt();
        self.expect(TokenType::CloseBrace, "Expected '}' after lambda body");
        self.expect(TokenType::Semicolon, "Expected ';' after lambda declaration");
        
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
            TokenType::DataType(t) => {
                self.consume();
                if t == DataType::Fn {
                    return self.parse_lambda_decl(constant, ident);
                }
                t
            },
            _ => {
                let token = self.expect(
                    TokenType::DataType(DataType::Any),
                    "Expected type (int, float, string, bool, obj, arr, fn) after ':'"
                );
                match token.kind {
                    TokenType::DataType(t) => t,
                    _ => unreachable!()
                }
            }
        };
    
        self.expect(TokenType::AssignOp(AssignOp::Assign), "Expected '=' after type declaration");
    
        // Add special handling for boolean literals
        let value = if matches!(self.at().kind, TokenType::Boolean(_)) {
            let bool_token = self.at().clone();
            self.consume();
            Some(Content::Expression(Box::new(Expr::BoolLit(BoolLit {
                value: matches!(bool_token.kind, TokenType::Boolean(true)),
                location: bool_token.location(),
            }))))
        } else {
            Some(self.parse_expr())
        };
    
        self.expect(TokenType::Semicolon, "Expected ';' after variable declaration");
        
        Content::Statement(Box::new(Stmt::VarDecl(VarDecl {
            constant,
            ident,
            type_,
            value,
            location: start_location
        })))
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
                TokenType::DataType(DataType::Object) => {
                    self.consume();
                    DataType::Object
                },
                TokenType::DataType(DataType::Array) => {
                    self.consume();
                    DataType::Array
                },
                _ => {
                    let token = self.expect(
                        TokenType::DataType(DataType::Any), 
                        "Expected type (int, float, string, bool, obj, arr) after ':'"
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
        
        self.expect(TokenType::Pipe, "Expected '|' after 'for'");
        let mut idents = Vec::new();
        while self.at().kind != TokenType::Pipe {
            let ident = self.expect(TokenType::Identifier, "Expected identifier").value;
            idents.push(ident);
            if self.at().kind == TokenType::Comma {
                self.consume(); // Consume the comma
            } else {
                break;
            }
        }
        self.expect(TokenType::Pipe, "Expected '|' after identifiers");
        self.expect(TokenType::In, "Expected 'in' after identifiers");
        
        let collection = self.parse_expr();
        
        self.expect(TokenType::OpenBrace, "Expected '{' after for condition");
        
        let body = self.parse_block_stmt();
        
        self.expect(TokenType::CloseBrace, "Expected '}' after for body");
        
        let init = Some(Box::new(Stmt::VarDecl(VarDecl {
            constant: false,
            ident: idents.join(", "), // Join identifiers as a single string
            type_: DataType::Any,
            value: Some(collection),
            location: start_location.clone(),
        })));
        
        Content::Statement(Box::new(Stmt::ForStmt(ForStmt {
            init,
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

    fn parse_object_lit(&mut self) -> Content {
        let start_location = self.at().location();
        self.expect(TokenType::OpenBrace, "Expected '{' to start object literal");
        let properties = self.parse_object_properties();
        self.expect(TokenType::CloseBrace, "Expected '}' to end object literal");
        Content::Expression(Box::new(Expr::ObjectLit(ObjectLit { properties, location: start_location })))
    }
    
    fn parse_array_lit(&mut self) -> Content {
        let start_location = self.at().location();
        self.expect(TokenType::OpenBracket, "Expected '[' to start array literal");
        let elements = self.parse_array_elements();
        self.expect(TokenType::CloseBracket, "Expected ']' to end array literal");
        Content::Expression(Box::new(Expr::ArrayLit(ArrayLit { elements, location: start_location })))
    }
    
    
    fn parse_object_properties(&mut self) -> Vec<Property> {
        let mut properties = Vec::new();
        let mut key_order = Vec::new();
        while self.at().kind != TokenType::CloseBrace {
            let start_location = self.at().location();
            let key = self.expect(TokenType::Identifier, "Expected property key").value;
            key_order.push(key.clone());
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
        // Add the key order as a hidden property
        properties.push(Property {
            key: "__keys__".to_string(),
            value: Box::new(Expr::ArrayLit(ArrayLit {
                elements: key_order.iter().map(|k| Box::new(Expr::StringLit(StringLit {
                    value: k.clone(),
                    location: Location { line: 0, column: 0 }
                }))).collect(),
                location: Location { line: 0, column: 0 }
            })),
            location: Location { line: 0, column: 0 }
        });
        properties
    }
    
    fn parse_array_elements(&mut self) -> Vec<Box<Expr>> {
        let mut elements = Vec::new();
        while self.at().kind != TokenType::CloseBracket {
            let element = self.parse_expr();
            if let Content::Expression(expr) = element {
                elements.push(expr);
            } else {
                panic!("Expected expression for array element");
            }
            if self.at().kind == TokenType::Comma {
                self.consume(); // Consume the comma
            } else {
                break;
            }
        }
        elements
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
        self.parse_expression(0)
    }

    // New Pratt parser routine using precedence climbing
    fn parse_expression(&mut self, min_prec: u8) -> Content {
        let mut left = self.parse_prefix();
        loop {
            if self.at().kind == TokenType::Dot {
                self.consume(); // consume the dot
                let ident_token = self.expect(TokenType::Identifier, "Expected property identifier after '.'");
                let is_method = self.at().kind == TokenType::FatArrow; // Check if followed by =>
                let member_expr = Expr::Member(MemberExpr {
                    object: match left {
                        Content::Expression(expr) => expr,
                        _ => panic!("Expected expression")
                    },
                    property: Box::new(Expr::Identifier(Identifier {
                        name: ident_token.value.clone(),
                        location: ident_token.location(),
                    })),
                    is_method,
                    location: ident_token.location(),
                });
                left = Content::Expression(Box::new(member_expr));
                continue;
            }
            // Handle all assignment operators
            if matches!(self.at().kind, 
                TokenType::AssignOp(AssignOp::Assign) | 
                TokenType::AssignOp(AssignOp::AddAssign) |
                TokenType::AssignOp(AssignOp::SubAssign) |
                TokenType::AssignOp(AssignOp::MulAssign) |
                TokenType::AssignOp(AssignOp::DivAssign) |
                TokenType::AssignOp(AssignOp::ModAssign)) {
                
                let operator = match self.at().kind {
                    TokenType::AssignOp(AssignOp::Assign) => "=",
                    TokenType::AssignOp(AssignOp::AddAssign) => "+=",
                    TokenType::AssignOp(AssignOp::SubAssign) => "-=",
                    TokenType::AssignOp(AssignOp::MulAssign) => "*=",
                    TokenType::AssignOp(AssignOp::DivAssign) => "/=",
                    TokenType::AssignOp(AssignOp::ModAssign) => "%=",
                    _ => unreachable!(),
                };
                
                self.consume(); // consume operator
                let right = self.parse_expression(0);
                
                return Content::Expression(Box::new(Expr::Assign(AssignExpr {
                    left: match left {
                        Content::Expression(expr) => expr,
                        _ => panic!("Expected expression")
                    },
                    right: match right {
                        Content::Expression(expr) => expr,
                        _ => panic!("Expected expression")
                    },
                    operator: operator.to_string(),
                    location: self.at().location(),
                })));
            }

            // Process binary/infix operators
            if let Some(op_prec) = self.get_infix_precedence() {
                if op_prec < min_prec {
                    break;
                }
                let op_token = self.at().clone();
                self.consume(); // consume operator
                let next_min_prec = op_prec + 1;
                let right = self.parse_expression(next_min_prec);
                left = Content::Expression(Box::new(Expr::Binary(BinaryExpr {
                    left: match left {
                        Content::Expression(expr) => expr,
                        _ => panic!("Expected expression")
                    },
                    operator: self.operator_string_from_token(&op_token),
                    right: match right {
                        Content::Expression(expr) => expr,
                        _ => panic!("Expected expression")
                    },
                    location: op_token.location(),
                })));
                continue;
            }
            break;
        }
        left
    }
    
    fn parse_prefix(&mut self) -> Content {
        // Handle unary minus
        if self.at().kind == TokenType::ArithOp(ArithOp::Sub) {
            self.consume(); // consume the minus
            let expr = self.parse_prefix();
            match expr {
                Content::Expression(e) => {
                    return Content::Expression(Box::new(Expr::Binary(BinaryExpr {
                        left: Box::new(Expr::FloatLit(FloatLit { 
                            value: 0.0, 
                            location: self.at().location() 
                        })),
                        operator: "-".to_string(),
                        right: e,
                        location: self.at().location(),
                    })));
                },
                _ => panic!("Expected expression after '-'"),
            }
        }

        // Handle native function calls prefixed with '@'
        if self.at().kind == TokenType::At {
            self.consume(); // consume '@'
            let ident_token = self.expect(TokenType::Identifier, "Expected identifier after '@'");
            let ident = Identifier { name: ident_token.value.clone(), location: ident_token.location() };
            self.expect(TokenType::FatArrow, "Expected '=>' after native function identifier");
            self.expect(TokenType::Pipe, "Expected '|' before native function arguments");

            let mut args = Vec::new();
            if self.at().kind != TokenType::Pipe {
                loop {
                    let expr = self.parse_expression_until(&[TokenType::Comma, TokenType::Pipe]);
                    match expr {
                        Content::Expression(e) => args.push(e),
                        _ => panic!("Expected expression in native function arguments"),
                    }
                    if self.at().kind == TokenType::Comma {
                        self.consume();
                    } else {
                        break;
                    }
                }
            }

            self.expect(TokenType::Pipe, "Expected '|' after native function arguments");

            return Content::Expression(Box::new(Expr::Call(CallExpr {
                callee: Box::new(Expr::Identifier(ident)),
                args,
                location: ident_token.location(),
            })));
        }
    
        // Parse primary expressions (identifiers, literals, grouping, etc.)
        let mut expr = match self.at().kind {
            TokenType::Identifier => {
                let ident_token = self.expect(TokenType::Identifier, "Expected identifier");
                Content::Expression(Box::new(Expr::Identifier(Identifier {
                    name: ident_token.value.clone(),
                    location: ident_token.location(),
                })))
            },
            TokenType::Int => {
                let int_lit = self.expect(TokenType::Int, "Expected integer literal");
                Content::Expression(Box::new(Expr::IntLit(IntLit {
                    value: int_lit.value.parse().unwrap(),
                    location: int_lit.location(),
                })))
            },
            TokenType::Float => {
                let float_lit = self.expect(TokenType::Float, "Expected float literal");
                Content::Expression(Box::new(Expr::FloatLit(FloatLit {
                    value: float_lit.value.parse().unwrap(),
                    location: float_lit.location(),
                })))
            },
            TokenType::String => {
                let string_token = self.expect(TokenType::String, "Expected string literal");
                Content::Expression(Box::new(Expr::StringLit(StringLit {
                    value: string_token.value.clone(),
                    location: string_token.location(),
                })))
            },
            TokenType::Boolean(value) => {
                let token = self.at().clone();
                self.consume();
                Content::Expression(Box::new(Expr::BoolLit(BoolLit {
                    value,
                    location: token.location(),
                })))
            },
            TokenType::OpenParen => {
                self.consume(); // consume '('
                let expr = self.parse_expression(0);
                self.expect(TokenType::CloseParen, "Expected ')' after expression");
                expr
            },
            TokenType::OpenBrace => self.parse_object_lit(),
            TokenType::OpenBracket => self.parse_array_lit(),
            _ => {
                let token = self.at().clone();
                let error = ZekkenError::syntax(
                    "Unexpected token in expression",
                    token.line,
                    token.column,
                    Some("expression"),
                    Some(&format!("{:?} ({})", token.kind, token.value)),
                );
                panic!("{}", error);
            }
        };
    
        // Handle member access (dot operator) and array indexing (brackets)
        loop {
            if self.at().kind == TokenType::Dot {
                self.consume(); // consume the dot
                let ident_token = self.expect(TokenType::Identifier, "Expected property identifier after '.'");
                let is_method = self.at().kind == TokenType::FatArrow; // Check if followed by =>
                expr = Content::Expression(Box::new(Expr::Member(MemberExpr {
                    object: match expr {
                        Content::Expression(e) => e,
                        _ => panic!("Expected expression before '.'"),
                    },
                    property: Box::new(Expr::Identifier(Identifier {
                        name: ident_token.value.clone(),
                        location: ident_token.location(),
                    })),
                    is_method,
                    location: ident_token.location(),
                })));
                continue;
            }
            if self.at().kind == TokenType::OpenBracket {
                self.consume(); // consume '['
                let index = self.parse_expression(0);
                self.expect(TokenType::CloseBracket, "Expected ']' after index");
                expr = Content::Expression(Box::new(Expr::Member(MemberExpr {
                    object: match expr {
                        Content::Expression(e) => e,
                        _ => panic!("Expected expression before '['"),
                    },
                    property: match index {
                        Content::Expression(e) => e,
                        _ => panic!("Expected expression for index"),
                    },
                    is_method: true,
                    location: self.at().location(),
                })));
                continue;
            }
            break;
        }
    
        // Support fat arrow call on identifiers and member expressions
        if self.at().kind == TokenType::FatArrow {
            self.consume(); // consume '=>'
            self.expect(TokenType::Pipe, "Expected '|' before function arguments");
            let mut args = Vec::new();
            while self.at().kind != TokenType::Pipe {
                let arg = self.parse_expression(0);
                match arg {
                    Content::Expression(e) => args.push(e),
                    _ => panic!("Expected expression in call arguments"),
                }
                if self.at().kind == TokenType::Comma {
                    self.consume();
                } else {
                    break;
                }
            }
            self.expect(TokenType::Pipe, "Expected '|' after function arguments");
            let callee = match expr {
                Content::Expression(e) => e,
                _ => panic!("Expected expression as callee"),
            };
            return Content::Expression(Box::new(Expr::Call(CallExpr {
                callee,
                args,
                location: self.at().location(),
            })));
        }
    
        expr
    }
    
    // Helper to retrieve the binding power of an infix operator
    fn get_infix_precedence(&self) -> Option<u8> {
        match self.at().kind {
            TokenType::ArithOp(ref op) => {
                let prec = match op {
                    ArithOp::Add | ArithOp::Sub => 10,
                    ArithOp::Mul | ArithOp::Div | ArithOp::Mod => 20,
                };
                Some(prec)
            },
            TokenType::BinOp(ref op) => {
                let prec = match op {
                    BinOp::And | BinOp::Or => 5,
                    BinOp::Eq | BinOp::Neq | BinOp::Less | BinOp::Greater | BinOp::LessEq | BinOp::GreaterEq => 7,
                    _ => 0,
                };
                Some(prec)
            },
            TokenType::AssignOp(_) => Some(2),
            _ => None,
        }
    }
    
    // Helper to convert an operator token to its string representation
    fn operator_string_from_token(&self, token: &Token) -> String {
        match &token.kind {
            TokenType::ArithOp(op) => match op {
                ArithOp::Add => "+".to_string(),
                ArithOp::Sub => "-".to_string(),
                ArithOp::Mul => "*".to_string(),
                ArithOp::Div => "/".to_string(),
                ArithOp::Mod => "%".to_string(),
            },
            TokenType::BinOp(op) => match op {
                BinOp::And => "&&".to_string(),
                BinOp::Or  => "||".to_string(),
                BinOp::Eq  => "==".to_string(),
                BinOp::Neq => "!=".to_string(),
                BinOp::Less => "<".to_string(),
                BinOp::Greater => ">".to_string(),
                BinOp::LessEq => "<=".to_string(),
                BinOp::GreaterEq => ">=".to_string(),
                _ => "".to_string(),
            },
            TokenType::AssignOp(_) => "=".to_string(),
            _ => "".to_string(),
        }
    }

    // Add this helper function:
    fn parse_expression_until(&mut self, stop_tokens: &[TokenType]) -> Content {
        let mut expr = self.parse_prefix();
        loop {
            // Stop if the next token is in stop_tokens
            if stop_tokens.iter().any(|t| self.at().kind == *t) {
                break;
            }
            if self.at().kind == TokenType::AssignOp(AssignOp::Assign) {
                self.consume();
                let right = self.parse_expression(0);
                return Content::Expression(Box::new(Expr::Assign(AssignExpr {
                    left: match expr {
                        Content::Expression(e) => e,
                        _ => panic!("Expected expression"),
                    },
                    right: match right {
                        Content::Expression(e) => e,
                        _ => panic!("Expected expression"),
                    },
                    operator: "=".to_string(),
                    location: self.at().location(),
                })));
            }
            if self.at().kind == TokenType::Dot {
                self.consume();
                let ident_token = self.expect(TokenType::Identifier, "Expected property identifier after '.'");
                expr = Content::Expression(Box::new(Expr::Member(MemberExpr {
                    object: match expr {
                        Content::Expression(e) => e,
                        _ => panic!("Expected expression before '.'"),
                    },
                    property: Box::new(Expr::Identifier(Identifier {
                        name: ident_token.value.clone(),
                        location: ident_token.location(),
                    })),
                    is_method: true,
                    location: ident_token.location(),
                })));
                continue;
            }
            if let Some(op_prec) = self.get_infix_precedence() {
                let op_token = self.at().clone();
                self.consume();
                let next_min_prec = op_prec + 1;
                let right = self.parse_expression(next_min_prec);
                expr = Content::Expression(Box::new(Expr::Binary(BinaryExpr {
                    left: match expr {
                        Content::Expression(e) => e,
                        _ => panic!("Expected expression"),
                    },
                    operator: self.operator_string_from_token(&op_token),
                    right: match right {
                        Content::Expression(e) => e,
                        _ => panic!("Expected expression"),
                    },
                    location: op_token.location(),
                })));
                continue;
            }
            break;
        }
        expr
    }
}