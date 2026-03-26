use crate::ast::*;
use crate::environment::{FunctionValue, Value};
use std::collections::HashSet;
use std::sync::Arc;

use super::inst::{BinaryOpCode, Inst, Reg};
use super::libraries::fs::FsOpCode;
use super::libraries::path::PathOpCode;
use super::libraries::encoding::EncodingOpCode;
use super::libraries::http::HttpOpCode;
use super::libraries::math::MathOpCode;
use super::libraries::os::OsOpCode;

fn expr_location(expr: &Expr) -> Location {
    match expr {
        Expr::Assign(e) => e.location.clone(),
        Expr::Member(e) => e.location.clone(),
        Expr::Call(e) => e.location.clone(),
        Expr::Binary(e) => e.location.clone(),
        Expr::Identifier(e) => e.location.clone(),
        Expr::Property(e) => e.location.clone(),
        Expr::IntLit(e) => e.location.clone(),
        Expr::FloatLit(e) => e.location.clone(),
        Expr::StringLit(e) => e.location.clone(),
        Expr::BoolLit(e) => e.location.clone(),
        Expr::ArrayLit(e) => e.location.clone(),
        Expr::ObjectLit(e) => e.location.clone(),
    }
}

pub(super) struct Compiler {
    pub(super) insts: Vec<Inst>,
    pub(super) next_reg: Reg,
}

#[derive(Default)]
pub(super) struct ParentUsage {
    pub(super) requires_parent_clone: bool,
    pub(super) captures: HashSet<String>,
}

pub(super) fn analyze_function_parent_usage(params: &[Param], body: &[Box<Content>]) -> ParentUsage {
    let mut locals = HashSet::new();
    for p in params {
        locals.insert(p.ident.clone());
    }

    let mut usage = ParentUsage::default();
    analyze_contents_parent_usage(body, &mut locals, &mut usage);
    usage
}

fn analyze_contents_parent_usage(
    contents: &[Box<Content>],
    locals: &mut HashSet<String>,
    usage: &mut ParentUsage,
) {
    for content in contents {
        match content.as_ref() {
            Content::Statement(stmt) => analyze_stmt_parent_usage(stmt, locals, usage),
            Content::Expression(expr) => analyze_expr_parent_usage(expr, locals, usage),
        }
        if usage.requires_parent_clone {
            return;
        }
    }
}

fn analyze_stmt_parent_usage(stmt: &Stmt, locals: &mut HashSet<String>, usage: &mut ParentUsage) {
    match stmt {
        Stmt::VarDecl(v) => {
            if let Some(value) = &v.value {
                match value {
                    Content::Statement(s) => analyze_stmt_parent_usage(s.as_ref(), locals, usage),
                    Content::Expression(e) => analyze_expr_parent_usage(e.as_ref(), locals, usage),
                }
            }
            locals.insert(v.ident.clone());
        }
        Stmt::FuncDecl(_) | Stmt::Lambda(_) | Stmt::TryCatchStmt(_) | Stmt::Use(_) | Stmt::Include(_) => {
            usage.requires_parent_clone = true;
        }
        Stmt::ObjectDecl(obj) => {
            for p in &obj.properties {
                analyze_expr_parent_usage(&p.value, locals, usage);
                if usage.requires_parent_clone {
                    return;
                }
            }
        }
        Stmt::IfStmt(i) => {
            analyze_expr_parent_usage(&i.test, locals, usage);
            if usage.requires_parent_clone {
                return;
            }
            let mut then_locals = locals.clone();
            analyze_contents_parent_usage(&i.body, &mut then_locals, usage);
            if usage.requires_parent_clone {
                return;
            }
            if let Some(alt) = &i.alt {
                let mut alt_locals = locals.clone();
                analyze_contents_parent_usage(alt, &mut alt_locals, usage);
            }
        }
        Stmt::ForStmt(f) => {
            if let Some(init) = &f.init {
                analyze_stmt_parent_usage(init, locals, usage);
            }
            let mut body_locals = locals.clone();
            analyze_contents_parent_usage(&f.body, &mut body_locals, usage);
        }
        Stmt::WhileStmt(w) => {
            analyze_expr_parent_usage(&w.test, locals, usage);
            if usage.requires_parent_clone {
                return;
            }
            let mut body_locals = locals.clone();
            analyze_contents_parent_usage(&w.body, &mut body_locals, usage);
        }
        Stmt::BlockStmt(b) => {
            let mut body_locals = locals.clone();
            analyze_contents_parent_usage(&b.body, &mut body_locals, usage);
        }
        Stmt::Return(r) => {
            if let Some(v) = &r.value {
                match v.as_ref() {
                    Content::Statement(s) => analyze_stmt_parent_usage(s.as_ref(), locals, usage),
                    Content::Expression(e) => analyze_expr_parent_usage(e.as_ref(), locals, usage),
                }
            }
        }
        Stmt::Export(_) | Stmt::Program(_) => {}
    }
}

fn analyze_expr_parent_usage(expr: &Expr, locals: &HashSet<String>, usage: &mut ParentUsage) {
    match expr {
        Expr::Identifier(i) => {
            if !locals.contains(&i.name) {
                usage.captures.insert(i.name.clone());
            }
        }
        Expr::Binary(b) => {
            analyze_expr_parent_usage(&b.left, locals, usage);
            if usage.requires_parent_clone {
                return;
            }
            analyze_expr_parent_usage(&b.right, locals, usage);
        }
        Expr::Call(c) => {
            analyze_expr_parent_usage(&c.callee, locals, usage);
            if usage.requires_parent_clone {
                return;
            }
            for a in &c.args {
                analyze_expr_parent_usage(a, locals, usage);
                if usage.requires_parent_clone {
                    return;
                }
            }
        }
        Expr::Member(m) => {
            analyze_expr_parent_usage(&m.object, locals, usage);
            if usage.requires_parent_clone {
                return;
            }
            match m.property.as_ref() {
                Expr::Identifier(_) | Expr::Property(_) => {}
                other => analyze_expr_parent_usage(other, locals, usage),
            }
        }
        Expr::Assign(a) => {
            if let Expr::Identifier(id) = a.left.as_ref() {
                if !locals.contains(&id.name) {
                    usage.requires_parent_clone = true;
                    return;
                }
            }
            analyze_expr_parent_usage(&a.left, locals, usage);
            if usage.requires_parent_clone {
                return;
            }
            analyze_expr_parent_usage(&a.right, locals, usage);
        }
        Expr::ArrayLit(a) => {
            for e in &a.elements {
                analyze_expr_parent_usage(e, locals, usage);
                if usage.requires_parent_clone {
                    return;
                }
            }
        }
        Expr::ObjectLit(o) => {
            for p in &o.properties {
                analyze_expr_parent_usage(&p.value, locals, usage);
                if usage.requires_parent_clone {
                    return;
                }
            }
        }
        Expr::IntLit(_) | Expr::FloatLit(_) | Expr::StringLit(_) | Expr::BoolLit(_) | Expr::Property(_) => {}
    }
}

impl Compiler {
    pub(super) fn new() -> Self {
        Self { insts: Vec::new(), next_reg: 0 }
    }

    fn new_reg(&mut self) -> Reg {
        let r = self.next_reg;
        self.next_reg += 1;
        r
    }

    fn emit(&mut self, inst: Inst) -> usize {
        self.insts.push(inst);
        self.insts.len() - 1
    }

    fn patch_jump_target(&mut self, at: usize, target: usize) {
        match self.insts.get_mut(at) {
            Some(Inst::Jump { target: t }) => *t = target,
            Some(Inst::JumpIfFalse { target: t, .. }) => *t = target,
            Some(Inst::JumpIfCmpFalse { target: t, .. }) => *t = target,
            _ => {}
        }
    }

    fn emit_test_jump_false(&mut self, test: &Expr, location: &Location) -> usize {
        if let Expr::Binary(binary) = test {
            if binary.operator != "&&" && binary.operator != "||" {
                if let Some(op) = BinaryOpCode::from_str(binary.operator.as_str()) {
                    match op {
                        BinaryOpCode::Eq
                        | BinaryOpCode::Ne
                        | BinaryOpCode::Lt
                        | BinaryOpCode::Gt
                        | BinaryOpCode::Le
                        | BinaryOpCode::Ge => {
                            let left = self.compile_expr(&binary.left);
                            let right = self.compile_expr(&binary.right);
                            return self.emit(Inst::JumpIfCmpFalse {
                                left,
                                right,
                                op,
                                target: usize::MAX,
                                location: location.clone(),
                            });
                        }
                        _ => {}
                    }
                }
            }
        }

        let cond = self.compile_expr(test);
        self.emit(Inst::JumpIfFalse {
            cond,
            target: usize::MAX,
            location: location.clone(),
        })
    }

    pub(super) fn compile_contents(&mut self, contents: &[Box<Content>]) {
        for content in contents {
            self.compile_content(content.as_ref());
        }
    }

    pub(super) fn compile_content(&mut self, content: &Content) {
        match content {
            Content::Statement(stmt) => self.compile_stmt(stmt.as_ref()),
            Content::Expression(expr) => {
                let r = self.compile_expr(expr.as_ref());
                self.emit(Inst::SetLast { src: r });
            }
        }
    }

    fn compile_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::VarDecl(decl) => {
                let src = match decl.value.as_ref() {
                    Some(Content::Expression(expr)) => self.compile_expr(expr.as_ref()),
                    Some(Content::Statement(stmt)) => {
                        self.emit(Inst::ExecStmtNative { stmt: stmt.as_ref().clone() });
                        let r = self.new_reg();
                        self.emit(Inst::LoadConst { dst: r, value: Value::Void });
                        r
                    }
                    None => {
                        let r = self.new_reg();
                        self.emit(Inst::LoadConst { dst: r, value: Value::Void });
                        r
                    }
                };

                self.emit(Inst::DeclareVar {
                    name: decl.ident.clone(),
                    ty: decl.type_,
                    constant: decl.constant,
                    src,
                    location: decl.location.clone(),
                });
            }
            Stmt::FuncDecl(func) => {
                self.emit(Inst::DeclareFunc { func: func.clone() });
            }
            Stmt::Lambda(lambda) => {
                self.emit(Inst::DeclareLambda { lambda: lambda.clone() });
            }
            Stmt::ObjectDecl(object) => {
                self.emit(Inst::DeclareObject { object: object.clone() });
            }
            Stmt::IfStmt(if_stmt) => {
                let jump_false = self.emit_test_jump_false(&if_stmt.test, &if_stmt.location);

                self.compile_contents(&if_stmt.body);

                if let Some(alt) = if_stmt.alt.as_ref() {
                    let jump_end = self.emit(Inst::Jump { target: usize::MAX });
                    let alt_start = self.insts.len();
                    self.patch_jump_target(jump_false, alt_start);
                    self.compile_contents(alt);
                    let end = self.insts.len();
                    self.patch_jump_target(jump_end, end);
                } else {
                    let end = self.insts.len();
                    self.patch_jump_target(jump_false, end);
                }
            }
            Stmt::WhileStmt(while_stmt) => {
                let loop_start = self.insts.len();
                let jump_false = self.emit_test_jump_false(&while_stmt.test, &while_stmt.location);
                self.compile_contents(&while_stmt.body);
                self.emit(Inst::Jump { target: loop_start });
                let end = self.insts.len();
                self.patch_jump_target(jump_false, end);
            }
            Stmt::BlockStmt(block) => self.compile_contents(&block.body),
            Stmt::Return(ret) => {
                let src = match ret.value.as_ref() {
                    Some(content) => match content.as_ref() {
                        Content::Expression(expr) => self.compile_expr(expr.as_ref()),
                        Content::Statement(stmt) => {
                            self.emit(Inst::ExecStmtNative { stmt: stmt.as_ref().clone() });
                            let r = self.new_reg();
                            self.emit(Inst::LoadConst { dst: r, value: Value::Void });
                            r
                        }
                    },
                    None => {
                        let r = self.new_reg();
                        self.emit(Inst::LoadConst { dst: r, value: Value::Void });
                        r
                    }
                };
                self.emit(Inst::Return { src });
            }
            _ => {
                self.emit(Inst::ExecStmtNative { stmt: stmt.clone() });
            }
        }
    }

    fn compile_expr(&mut self, expr: &Expr) -> Reg {
        match expr {
            Expr::Assign(assign) => {
                if let Expr::Identifier(id) = assign.left.as_ref() {
                    if let Expr::IntLit(i) = assign.right.as_ref() {
                        let delta = match assign.operator.as_str() {
                            "+=" => Some(i.value),
                            "-=" => Some(-i.value),
                            _ => None,
                        };
                        if let Some(delta) = delta {
                            let dst = self.new_reg();
                            self.emit(Inst::AddIntAssignIdent {
                                dst,
                                name: id.name.clone(),
                                delta,
                                location: expr_location(assign.right.as_ref()),
                            });
                            return dst;
                        }
                    }
                    if assign.operator == "=" {
                        let src = self.compile_expr(&assign.right);
                        let dst = self.new_reg();
                        self.emit(Inst::AssignIdent {
                            dst,
                            name: id.name.clone(),
                            src,
                            location: expr_location(assign.right.as_ref()),
                        });
                        return dst;
                    }
                }
                if assign.operator == "=" {
                    if let Expr::Member(member) = assign.left.as_ref() {
                        if member.is_method {
                            if let Expr::Identifier(id) = member.object.as_ref() {
                                let index = self.compile_expr(&member.property);
                                let src = self.compile_expr(&assign.right);
                                let dst = self.new_reg();
                                self.emit(Inst::StoreIndexIdent {
                                    dst,
                                    name: id.name.clone(),
                                    index,
                                    src,
                                    location: expr_location(assign.right.as_ref()),
                                });
                                return dst;
                            }
                        }
                    }
                }
                let dst = self.new_reg();
                self.emit(Inst::EvalExprNative { dst, expr: expr.clone() });
                dst
            }
            Expr::IntLit(v) => {
                let dst = self.new_reg();
                self.emit(Inst::LoadConst { dst, value: Value::Int(v.value) });
                dst
            }
            Expr::FloatLit(v) => {
                let dst = self.new_reg();
                self.emit(Inst::LoadConst { dst, value: Value::Float(v.value) });
                dst
            }
            Expr::BoolLit(v) => {
                let dst = self.new_reg();
                self.emit(Inst::LoadConst { dst, value: Value::Boolean(v.value) });
                dst
            }
            Expr::StringLit(v) if !v.value.as_bytes().contains(&b'{') => {
                let dst = self.new_reg();
                self.emit(Inst::LoadConst { dst, value: Value::String(v.value.clone()) });
                dst
            }
            Expr::Identifier(ident) => {
                let dst = self.new_reg();
                self.emit(Inst::LoadIdent { dst, name: ident.name.clone(), location: ident.location.clone() });
                dst
            }
            Expr::Member(member) if member.is_method => {
                let object = self.compile_expr(&member.object);
                let index = self.compile_expr(&member.property);
                let dst = self.new_reg();
                self.emit(Inst::LoadIndex {
                    dst,
                    object,
                    index,
                    location: member.location.clone(),
                });
                dst
            }
            Expr::Binary(binary) if binary.operator != "&&" && binary.operator != "||" => {
                let op = match BinaryOpCode::from_str(binary.operator.as_str()) {
                    Some(op) => op,
                    None => {
                        let dst = self.new_reg();
                        self.emit(Inst::EvalExprNative { dst, expr: expr.clone() });
                        return dst;
                    }
                };
                let left = self.compile_expr(&binary.left);
                let right = self.compile_expr(&binary.right);
                let dst = self.new_reg();
                self.emit(Inst::Binary {
                    dst,
                    left,
                    right,
                    op,
                    location: binary.location.clone(),
                });
                dst
            }
            Expr::Call(call) => {
                if let Expr::Member(member) = call.callee.as_ref() {
                    if let Expr::Identifier(object) = member.object.as_ref() {
                        if let Expr::Identifier(method_ident) = member.property.as_ref() {
                            let argc = call.args.len();
                            if argc <= 3 {
                                let mut regs = [0usize; 3];
                                for (idx, arg) in call.args.iter().enumerate() {
                                    regs[idx] = self.compile_expr(arg);
                                }
                                let dst = self.new_reg();
                                if object.name == "math" {
                                    if let Some(method) = MathOpCode::from_method(method_ident.name.as_str()) {
                                        self.emit(Inst::CallMath {
                                            dst,
                                            method,
                                            argc: argc as u8,
                                            args: regs,
                                            location: call.location.clone(),
                                        });
                                        return dst;
                                    }
                                } else if object.name == "fs" {
                                    if let Some(method) = FsOpCode::from_method(method_ident.name.as_str()) {
                                        self.emit(Inst::CallFs {
                                            dst,
                                            method,
                                            argc: argc as u8,
                                            args: regs,
                                            location: call.location.clone(),
                                        });
                                        return dst;
                                    }
                                } else if object.name == "os" {
                                    if let Some(method) = OsOpCode::from_method(method_ident.name.as_str()) {
                                        self.emit(Inst::CallOs {
                                            dst,
                                            method,
                                            argc: argc as u8,
                                            args: regs,
                                            location: call.location.clone(),
                                        });
                                        return dst;
                                    }
                                } else if object.name == "path" {
                                    if let Some(method) = PathOpCode::from_method(method_ident.name.as_str()) {
                                        self.emit(Inst::CallPath {
                                            dst,
                                            method,
                                            argc: argc as u8,
                                            args: regs,
                                            location: call.location.clone(),
                                        });
                                        return dst;
                                    }
                                } else if object.name == "encoding" {
                                    if let Some(method) = EncodingOpCode::from_method(method_ident.name.as_str()) {
                                        self.emit(Inst::CallEncoding {
                                            dst,
                                            method,
                                            argc: argc as u8,
                                            args: regs,
                                            location: call.location.clone(),
                                        });
                                        return dst;
                                    }
                                } else if object.name == "http" {
                                    if let Some(method) = HttpOpCode::from_method(method_ident.name.as_str()) {
                                        self.emit(Inst::CallHttp {
                                            dst,
                                            method,
                                            argc: argc as u8,
                                            args: regs,
                                            location: call.location.clone(),
                                        });
                                        return dst;
                                    }
                                }
                            }
                        }
                    }
                }
                let dst = self.new_reg();
                self.emit(Inst::EvalExprNative { dst, expr: expr.clone() });
                dst
            }
            _ => {
                let dst = self.new_reg();
                self.emit(Inst::EvalExprNative { dst, expr: expr.clone() });
                dst
            }
        }
    }
}

pub(super) fn make_function_value(params: &[Param], body: &[Box<Content>]) -> FunctionValue {
    let usage = analyze_function_parent_usage(params, body);
    let captures = if usage.requires_parent_clone {
        vec![]
    } else {
        let mut v: Vec<String> = usage.captures.into_iter().collect();
        v.sort_unstable();
        v
    };
    FunctionValue {
        params: Arc::new(params.to_vec()),
        body: Arc::new(body.to_vec()),
        return_type: None,
        needs_parent: usage.requires_parent_clone,
        captures: Arc::new(captures),
    }
}
