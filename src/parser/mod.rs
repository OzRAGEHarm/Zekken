#![allow(dead_code)]
use crate::ast::{*};
use crate::lexer::{*};

pub struct Parser {
  tokens: Vec<Token>
}

impl Parser {
  pub fn new() -> Self {
    let tokens: Vec<Token> = Vec::new();
    Parser {
      tokens
    }
  }

  pub fn at(&self) -> Token {
    self.tokens[0].expect("Token list appears empty!")
  }

  pub fn consume(&mut self) {
    self.tokens.remove(0);
  }

  fn not_eof(&self) -> bool {
    self.tokens.len() > 0
  }

  fn expect(&mut self, type_: TokenType, err: &str) -> Token {
    let prev = self.tokens.pop().expect("Token is empty");
    if prev.kind != type_ {
      panic!("Parser error:\n{}\nExpecting: {:?}", err, type_);
    }

    prev
  }

  fn match_token(&mut self, type_: TokenType, err: &str) {
    match self.consume().kind {
      type_ => (),
      _ => panic!("Error at line: {:?}, column: {:?} | {:?}\nExpected: '{:?}', got: {:?}", self.at().line, self.at().column, err, type_, self.at()),
    }
  }

  pub fn produce_ast(&mut self, source_code: String) -> Program {
    self.tokens = tokenize(source_code);

    let mut program = Program {
        body: vec![],
    };

    while self.not_eof() {
        program.body.push(self.parse_stmt());
    }

    program
  }

  fn parse_stmt(&mut self) -> Box<Stmt> {
    match self.at().kind {
        TokenType::Let | TokenType::Const => self.parse_var_decl(),
        TokenType::Func => self.parse_func_decl(),
        TokenType::If => self.parse_if_stmt(),
        TokenType::For => self.parse_for_stmt(),
        TokenType::While => self.parse_while_stmt(),
        TokenType::Use => self.parse_use_stmt(),
        TokenType::Include => self.parse_include_stmt(),
        TokenType::Export => self.parse_export_stmt(),
        TokenType::Object => self.parse_object_decl(),
        _ => self.parse_expr(),
    }
  }

  fn parse_expr(&mut self) -> Box<ast::Expr> {
    self.parse_assignment_expr()
  }

  fn parse_block_stmt(&mut self) -> Vec<Box<Stmt>> {
    self.match_token(TokenType::OpenBrace, "Opening brace (\"{{\") expected while parsing code block.");

    let mut body = vec![];

    while self.not_eof() && self.at().kind != TokenType::CloseBrace {
      let stmt = self.parse_stmt();
      body.push(stmt);
    }

    self.match_token(TokenType::CloseBrace, "Closing brace (\"}}\") expected while parsing code block.");
  }

  fn parse_for_stmt(&mut self) -> Box<Stmt> {
    self.consume(); // Consume the "for" token

    let ident = self.parse_primary_expr();
    self.match_token(*ident, "Expected identifier in for loop.");

    self.match_token(TokenType::In, "Keyword \"in\" expected following identifier in \"for\" statement.");

    let expr = self.parse_expr();
    let body = self.parse_block_stmt();

    Box::new(Stmt::ForStmt(ForStmt {
      init: Some(identifier),
      test: Some(expr),
      update: None,
      body,
    }))
  }

  fn parse_then_block(&mut self) -> Vec<Box<Stmt>> {
    let mut body = vec![];
    let mut current_token = self.at();

    // Check if the next token is "else" or EOF
    while self.not_eof() && current_token.kind != TokenType::Else {
        let stmt = self.parse_stmt();
        body.push(stmt);
        current_token = self.at();
    }

    body
  }

  fn parse_if_stmt(&mut self) -> Box<Stmt> {
    self.consume(); // Consume the "if" token

    let condition = self.parse_and_stmt();

    let body = self.parse_then_block();

    let mut alternate = Vec::new();

    if self.at().kind == TokenType::Else {
      self.eat();
      if self.at().kind == TokenType::If {
          alternate.push(self.parse_if_statement());
      } else {
          alternate.push(Box::new(Stmt::BlockStmt(BlockStmt {
              body: self.parse_then_block(),
          })));
      }
    }

    Box::new(Stmt::IfStmt(IfStmt {
      test: condition,
      body: body.into_iter().map(|stmt| Box::new(Stmt::BlockStmt(BlockStmt { body: vec![stmt] }))).collect(),
      alt: Some(alternate.into_iter().collect()),
    }))
  }

  fn parse_include_stmt(&mut self) -> Box<Stmt> {
    self.consume(); // Consume the "include" token

    self.match_token(TokenType::DoubleQuote, "Double Quote (\") expected after \"include\" keyword.");

    let mut includes = vec![];
    while self.at().kind == TokenType::Identifier {
        includes.push(Identifier {
            symbol: self.consume().value.clone(),
        });

        if self.at().kind == TokenType::Comma {
            self.consume(); // Consume the comma ","
        }
    }

    self.match_token(TokenType::DoubleQuote, "Double Quote (\") expected after \"include\" declaration(s).");
  }
}