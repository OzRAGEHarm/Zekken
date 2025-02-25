#![allow(dead_code)]
use std::fmt::Debug;

#[derive(Debug)]
pub enum Stmt {
  Program(Program),
  VarDecl(VarDecl),
  FuncDecl(FuncDecl),
  ObjectDecl(ObjectDecl),
  IfStmt(IfStmt),
  ForStmt(ForStmt),
  WhileStmt(WhileStmt),
  TryCatchStmt(TryCatchStmt),
  BlockStmt(BlockStmt),
  Use(String),
  Include(String),
  Export(Vec<String>),
  Return(ReturnStmt)
}

#[derive(Debug)]
pub enum Expr {
  Assign(AssignExpr),
  Member(MemberExpr),
  Call(CallExpr),
  Binary(BinaryExpr),
  Identifier(Identifier),
  Property(Property),
  IntLit(IntLit),
  FloatLit(FloatLit),
  StringLit(StringLit),
  BoolLit(BoolLit),
  ArrayLit(ArrayLit),
  ObjectLit(ObjectLit),
}

#[derive(Debug)]
pub enum Content {
  Statement(Box<Stmt>),
  Expression(Box<Expr>),
}

#[derive(Debug)]
pub struct Program {
  pub body: Vec<Content>,
}

#[derive(Debug)]
pub struct VarDecl {
  pub constant: bool,
  pub ident: String,
  pub value: Option<Content>,
}

#[derive(Debug)]
pub struct FuncDecl {
  pub params: Vec<String>,
  pub ident: String,
  pub body: Vec<Content>,
}

#[derive(Debug)]
pub struct ObjectDecl {
  pub ident: String,
  pub properties: Vec<Property>,
}

#[derive(Debug)]
pub struct IfStmt {
  pub test: Box<Expr>,
  pub body: Vec<Box<Content>>,
  pub alt: Option<Vec<Box<Content>>>,
}

#[derive(Debug)]
pub struct ForStmt {
  pub init: Option<Box<Stmt>>,
  pub test: Option<Box<Expr>>,
  pub update: Option<Box<Expr>>,
  pub body: Vec<Box<Content>>,
}

#[derive(Debug)]
pub struct WhileStmt {
  pub test: Box<Expr>,
  pub body: Vec<Box<Content>>,
}

#[derive(Debug)]
pub struct TryCatchStmt {
  pub try_block: Vec<Box<Content>>,
  pub catch_block: Option<Vec<Box<Content>>>,
}

#[derive(Debug)]
pub struct BlockStmt {
  pub body: Vec<Box<Content>>,
}

#[derive(Debug)]
pub struct ReturnStmt {
    pub value: Option<Box<Expr>>,
}

#[derive(Debug)]
pub struct AssignExpr {
  pub left: Box<Expr>,
  pub right: Box<Expr>,
}

#[derive(Debug)]
pub struct MemberExpr {
  pub object: Box<Expr>,
  pub property: Box<Expr>,
  pub computed: bool,
}

#[derive(Debug)]
pub struct CallExpr {
  pub callee: Box<Expr>,
  pub args: Vec<Box<Expr>>,
}

#[derive(Debug)]
pub struct BinaryExpr {
  pub left: Box<Expr>,
  pub right: Box<Expr>,
  pub operator: String,
}

#[derive(Debug)]
pub struct Identifier {
  pub name: String,
}

#[derive(Debug)]
pub struct Property {
  pub key: String,
  pub value: Box<Expr>,
}

#[derive(Debug)]
pub struct IntLit {
  pub value: i64,
}

#[derive(Debug)]
pub struct FloatLit {
  pub value: f64,
}

#[derive(Debug)]
pub struct StringLit {
  pub value: String,
}

#[derive(Debug)]
pub struct BoolLit {
  pub value: bool,
}

#[derive(Debug)]
pub struct ArrayLit {
  pub elements: Vec<Box<Expr>>,
}

#[derive(Debug)]
pub struct ObjectLit {
  pub properties: Vec<Property>,
}