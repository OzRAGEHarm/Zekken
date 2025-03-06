#![allow(dead_code)]

use std::fmt::Debug;
use crate::lexer::DataType;

#[derive(Debug, Clone)]
pub struct Location {
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone)]
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
  Use(UseStmt),
  Include(IncludeStmt),
  Export(ExportStmt),
  Return(ReturnStmt),
  Lambda(LambdaDecl),
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub enum Content {
  Statement(Box<Stmt>),
  Expression(Box<Expr>),
}

#[derive(Debug, Clone)]
pub struct Program {
    pub imports: Vec<Content>,
    pub comments: Vec<String>,
    pub content: Vec<Box<Content>>,
    pub location: Location,
}

#[derive(Debug, Clone)]
pub struct VarDecl {
    pub constant: bool,
    pub ident: String,
    pub type_: DataType,
    pub value: Option<Content>,
    pub location: Location,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub ident: String,
    pub type_: DataType,
    pub location: Location,
}

#[derive(Debug, Clone)]
pub struct FuncDecl {
    pub params: Vec<Param>,
    pub ident: String,
    pub body: Vec<Box<Content>>,
    pub location: Location,
}

#[derive(Debug, Clone)]
pub struct ObjectDecl {
    pub ident: String,
    pub properties: Vec<Property>,
    pub location: Location,
}

#[derive(Debug, Clone)]
pub struct IfStmt {
    pub test: Box<Expr>,
    pub body: Vec<Box<Content>>,
    pub alt: Option<Vec<Box<Content>>>,
    pub location: Location,
}

#[derive(Debug, Clone)]
pub struct ForStmt {
    pub init: Option<Box<Stmt>>,
    pub test: Option<Box<Expr>>,
    pub update: Option<Box<Expr>>,
    pub body: Vec<Box<Content>>,
    pub location: Location,
}

#[derive(Debug, Clone)]
pub struct WhileStmt {
    pub test: Box<Expr>,
    pub body: Vec<Box<Content>>,
    pub location: Location,
}

#[derive(Debug, Clone)]
pub struct TryCatchStmt {
    pub try_block: Vec<Box<Content>>,
    pub catch_block: Option<Vec<Box<Content>>>,
    pub location: Location,
}

#[derive(Debug, Clone)]
pub struct BlockStmt {
    pub body: Vec<Box<Content>>,
    pub location: Location,
}

#[derive(Debug, Clone)]
pub struct UseStmt {
    pub methods: Option<Vec<String>>,
    pub module: String,
    pub location: Location,
}

#[derive(Debug, Clone)]
pub struct IncludeStmt {
    pub methods: Option<Vec<String>>,
    pub file_path: String,
    pub location: Location,
}

#[derive(Debug, Clone)]
pub struct ExportStmt {
    pub exports: Vec<String>,
    pub location: Location,
}

#[derive(Debug, Clone)]
pub struct ReturnStmt {
    pub value: Option<Box<Content>>,
    pub location: Location,
}

#[derive(Debug, Clone)]
pub struct LambdaDecl {
    pub constant: bool,
    pub ident: String,
    pub params: Vec<Param>,
    pub body: Vec<Box<Content>>,
    pub location: Location,
}

#[derive(Debug, Clone)]
pub struct AssignExpr {
    pub left: Box<Expr>,
    pub right: Box<Expr>,
    pub location: Location,
}

#[derive(Debug, Clone)]
pub struct MemberExpr {
    pub object: Box<Expr>,
    pub property: Box<Expr>,
    pub computed: bool,
    pub location: Location,
}

#[derive(Debug, Clone)]
pub struct CallExpr {
    pub callee: Box<Expr>,
    pub args: Vec<Box<Expr>>,
    pub location: Location,
}

#[derive(Debug, Clone)]
pub struct BinaryExpr {
    pub left: Box<Expr>,
    pub right: Box<Expr>,
    pub operator: String,
    pub location: Location,
}

#[derive(Debug, Clone)]
pub struct Identifier {
    pub name: String,
    pub location: Location,
}

#[derive(Debug, Clone)]
pub struct Property {
    pub key: String,
    pub value: Box<Expr>,
    pub location: Location,
}

#[derive(Debug, Clone)]
pub struct IntLit {
    pub value: i64,
    pub location: Location,
}

#[derive(Debug, Clone)]
pub struct FloatLit {
    pub value: f64,
    pub location: Location,
}

#[derive(Debug, Clone)]
pub struct StringLit {
    pub value: String,
    pub location: Location,
}

#[derive(Debug, Clone)]
pub struct BoolLit {
    pub value: bool,
    pub location: Location,
}

#[derive(Debug, Clone)]
pub struct ArrayLit {
    pub elements: Vec<Box<Expr>>,
    pub location: Location,
}

#[derive(Debug, Clone)]
pub struct ObjectLit {
    pub properties: Vec<Property>,
    pub location: Location,
}

#[derive(Debug, Clone)]
pub struct ComplexLit {
    pub real: f64,
    pub imag: f64,
}

#[derive(Debug, Clone)]
pub struct VectorLit {
    pub elements: Vec<f64>,
}

#[derive(Debug, Clone)]
pub struct MatrixLit {
    pub rows: Vec<Vec<f64>>,
}