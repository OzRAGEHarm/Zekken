use crate::ast::*;
use crate::environment::Value;
use crate::lexer::DataType;

use super::libraries::fs::FsOpCode;
use super::libraries::path::PathOpCode;
use super::libraries::encoding::EncodingOpCode;
use super::libraries::http::HttpOpCode;
use super::libraries::math::MathOpCode;
use super::libraries::os::OsOpCode;

pub(super) type Reg = usize;

#[derive(Clone, Copy)]
pub(super) enum BinaryOpCode {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
}

impl BinaryOpCode {
    #[inline]
    pub(super) fn from_str(op: &str) -> Option<Self> {
        match op {
            "+" => Some(Self::Add),
            "-" => Some(Self::Sub),
            "*" => Some(Self::Mul),
            "/" => Some(Self::Div),
            "%" => Some(Self::Mod),
            "==" => Some(Self::Eq),
            "!=" => Some(Self::Ne),
            "<" => Some(Self::Lt),
            ">" => Some(Self::Gt),
            "<=" => Some(Self::Le),
            ">=" => Some(Self::Ge),
            _ => None,
        }
    }
}

#[derive(Clone)]
pub(super) enum Inst {
    LoadConst { dst: Reg, value: Value },
    LoadIdent { dst: Reg, name: String, location: Location },
    LoadIndex { dst: Reg, object: Reg, index: Reg, location: Location },
    Binary { dst: Reg, left: Reg, right: Reg, op: BinaryOpCode, location: Location },
    CallMath { dst: Reg, method: MathOpCode, argc: u8, args: [Reg; 3], location: Location },
    CallFs { dst: Reg, method: FsOpCode, argc: u8, args: [Reg; 3], location: Location },
    CallOs { dst: Reg, method: OsOpCode, argc: u8, args: [Reg; 3], location: Location },
    CallPath { dst: Reg, method: PathOpCode, argc: u8, args: [Reg; 3], location: Location },
    CallEncoding { dst: Reg, method: EncodingOpCode, argc: u8, args: [Reg; 3], location: Location },
    CallHttp { dst: Reg, method: HttpOpCode, argc: u8, args: [Reg; 3], location: Location },
    EvalExprNative { dst: Reg, expr: Expr },
    ExecStmtNative { stmt: Stmt },
    DeclareVar { name: String, ty: DataType, constant: bool, src: Reg, location: Location },
    DeclareFunc { func: FuncDecl },
    DeclareLambda { lambda: LambdaDecl },
    DeclareObject { object: ObjectDecl },
    AssignIdent { dst: Reg, name: String, src: Reg, location: Location },
    StoreIndexIdent { dst: Reg, name: String, index: Reg, src: Reg, location: Location },
    Jump { target: usize },
    JumpIfFalse { cond: Reg, target: usize, location: Location },
    JumpIfCmpFalse { left: Reg, right: Reg, op: BinaryOpCode, target: usize, location: Location },
    SetLast { src: Reg },
    Return { src: Reg },
    AddIntAssignIdent { dst: Reg, name: String, delta: i64, location: Location },
}
