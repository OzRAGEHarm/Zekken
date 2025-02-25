mod ast;
mod lexer;
//mod parser;

use crate::ast::{*};

fn main() {
    let program = program();
    println!("{:#?}", program);

    /*
    let source = String::from("let x = 64.0;");
    let tokens = lexer::tokenize(source);
    println!("Tokens: {:#?}", tokens);
    */
}

fn program() -> Program {
    Program {
        body: vec![
            // Variable declarations
            Content::Statement(Box::new(Stmt::VarDecl(VarDecl { 
                constant: true, 
                ident: "x".to_string(), 
                value: Some(Content::Expression(Box::new(Expr::IntLit(IntLit { value: 64 }))))
            }))),

            Content::Statement(Box::new(Stmt::VarDecl(VarDecl { 
                constant: false, 
                ident: "y".to_string(), 
                value: Some(Content::Expression(Box::new(Expr::IntLit(IntLit { value: 2 }))))
            }))),

            // Function declaration
            Content::Statement(Box::new(Stmt::FuncDecl(FuncDecl {
                params: vec!["a".to_string(), "b".to_string()],
                ident: "add".to_string(),
                body: vec![
                    Content::Expression(
                        Box::new(Expr::Binary(
                            BinaryExpr { 
                                left: Box::new(Expr::Identifier(Identifier { name: "a".to_string() })), 
                                right: Box::new(Expr::Identifier(Identifier { name: "b".to_string() })),
                                operator: "+".to_string(),
                            } 
                        ))
                    ),
                ]
            }))),

            // Function call
            Content::Expression(Box::new(Expr::Call(CallExpr { 
                callee: Box::new(Expr::Identifier(Identifier { name: "add".to_string() })), 
                args: vec![
                    Box::new(Expr::Identifier(Identifier { name: "x".to_string() })),
                    Box::new(Expr::Identifier(Identifier { name: "y".to_string() }))
                ]
            }))),

            // If statement
            Content::Statement(Box::new(Stmt::IfStmt(IfStmt {
                test: Box::new(Expr::Binary(BinaryExpr {
                    left: Box::new(Expr::Identifier(Identifier { name: "x".to_string() })),
                    right: Box::new(Expr::IntLit(IntLit { value: 50 })),
                    operator: ">".to_string(),
                })),
                body: vec![
                    Box::new(Content::Statement(Box::new(Stmt::VarDecl(VarDecl {
                        constant: false,
                        ident: "result".to_string(),
                        value: Some(Content::Expression(Box::new(Expr::StringLit(StringLit { value: "x is greater than 50".to_string() }))))
                    }))))
                ],
                alt: None,
            }))),

            // For statement
            Content::Statement(Box::new(Stmt::ForStmt(ForStmt {
                init: Some(Box::new(Stmt::VarDecl(VarDecl {
                    constant: false,
                    ident: "i".to_string(),
                    value: Some(Content::Expression(Box::new(Expr::IntLit(IntLit { value: 0 }))))
                }))),
                test: Some(Box::new(Expr::Binary(BinaryExpr {
                    left: Box::new(Expr::Identifier(Identifier { name: "i".to_string() })),
                    right: Box::new(Expr::IntLit(IntLit { value: 10 })),
                    operator: "<".to_string(),
                }))),
                update: Some(Box::new(Expr::Binary(BinaryExpr {
                    left: Box::new(Expr::Identifier(Identifier { name: "i".to_string() })),
                    right: Box::new(Expr::IntLit(IntLit { value: 1 })),
                    operator: "+".to_string(),
                }))),
                body: vec![
                    Box::new(Content::Statement(Box::new(Stmt::VarDecl(VarDecl {
                        constant: false,
                        ident: "loopResult".to_string(),
                        value: Some(Content::Expression(Box::new(Expr::Binary(BinaryExpr {
                            left: Box::new(Expr::Identifier(Identifier { name: "i".to_string() })),
                            right: Box::new(Expr::IntLit(IntLit { value: 2 })),
                            operator: "*".to_string(),
                        }))))
                    }))))
                ],
            }))),

            // While statement
            Content::Statement(Box::new(Stmt::WhileStmt(WhileStmt {
                test: Box::new(Expr::Binary(BinaryExpr {
                    left: Box::new(Expr::Identifier(Identifier { name: "x".to_string() })),
                    right: Box::new(Expr::IntLit(IntLit { value: 0 })),
                    operator: ">".to_string(),
                })),
                body: vec![
                    Box::new(Content::Statement(Box::new(Stmt::VarDecl(VarDecl {
                        constant: false,
                        ident: "decrementedX".to_string(),
                        value: Some(Content::Expression(Box::new(Expr::Binary(BinaryExpr {
                            left: Box::new(Expr::Identifier(Identifier { name: "x".to_string() })),
                            right: Box::new(Expr::IntLit(IntLit { value: 1 })),
                            operator: "-".to_string(),
                        }))))
                    }))))
                ],
            }))),

            // Try-Catch statement
            Content::Statement(Box::new(Stmt::TryCatchStmt(TryCatchStmt {
                try_block: vec![
                    Box::new(Content::Statement(Box::new(Stmt::VarDecl(VarDecl {
                        constant: false,
                        ident: "tryResult".to_string(),
                        value: Some(Content::Expression(Box::new(Expr::IntLit(IntLit { value: 42 }))))
                    }))))
                ],
                catch_block: Some(vec![
                    Box::new(Content::Statement(Box::new(Stmt::VarDecl(VarDecl {
                        constant: false,
                        ident: "catchResult".to_string(),
                        value: Some(Content::Expression(Box::new(Expr::StringLit(StringLit { value: "An error occurred".to_string() }))))
                    }))))
                ]),
            }))),

            // Object declaration
            Content::Statement(Box::new(Stmt::ObjectDecl(ObjectDecl {
                ident: "myObject".to_string(),
                properties: vec![
                    Property {
                        key: "name".to_string(),
                        value: Box::new(Expr::StringLit(StringLit { value: "Example".to_string() })),
                    },
                    Property {
                        key: "value".to_string(),
                        value: Box::new(Expr::IntLit(IntLit { value: 100 })),
                    },
                ],
            }))),

            // Assignment expression
            Content::Statement(Box::new(Stmt::VarDecl(VarDecl {
                constant: false,
                ident: "result".to_string(),
                value: Some(Content::Expression(Box::new(Expr::Assign(AssignExpr {
                    left: Box::new(Expr::Identifier(Identifier { name: "x".to_string() })),
                    right: Box::new(Expr::Binary(BinaryExpr {
                        left: Box::new(Expr::Identifier(Identifier { name: "y".to_string() })),
                        right: Box::new(Expr::IntLit(IntLit { value: 10 })),
                        operator: "*".to_string(),
                    })),
                }))))
            }))),

            // Member expression
            Content::Statement(Box::new(Stmt::ObjectDecl(ObjectDecl {
                ident: "myObject".to_string(),
                properties: vec![
                    Property {
                        key: "name".to_string(),
                        value: Box::new(Expr::StringLit(StringLit { value: "Example".to_string() })),
                    },
                    Property {
                        key: "value".to_string(),
                        value: Box::new(Expr::IntLit(IntLit { value: 100 })),
                    },
                ],
            }))),

            // Accessing a property of an object
            Content::Expression(Box::new(Expr::Member(MemberExpr {
                object: Box::new(Expr::Identifier(Identifier { name: "myObject".to_string() })),
                property: Box::new(Expr::StringLit(StringLit { value: "name".to_string() })),
                computed: false,
            }))),

            // Binary expression
            Content::Expression(Box::new(Expr::Binary(BinaryExpr {
                left: Box::new(Expr::IntLit(IntLit { value: 5 })),
                right: Box::new(Expr::IntLit(IntLit { value: 3 })),
                operator: "+".to_string(),
            }))),

            // Call expression
            Content::Expression(Box::new(Expr::Call(CallExpr { 
                callee: Box::new(Expr::Identifier(Identifier { name: "add".to_string() })), 
                args: vec![
                    Box::new(Expr::IntLit(IntLit { value: 10 })),
                    Box::new(Expr::IntLit(IntLit { value: 20 })),
                ]
            }))),

            // Array literal
            Content::Expression(Box::new(Expr::ArrayLit(ArrayLit {
                elements: vec![
                    Box::new(Expr::IntLit(IntLit { value: 1 })),
                    Box::new(Expr::FloatLit(FloatLit { value: 2.45 })),
                    Box::new(Expr::IntLit(IntLit { value: 3 })),
                ],
            }))),

            // Object literal
            Content::Expression(Box::new(Expr::ObjectLit(ObjectLit {
                properties: vec![
                    Property {
                        key: "key1".to_string(),
                        value: Box::new(Expr::StringLit(StringLit { value: "value1".to_string() })),
                    },
                    Property {
                        key: "key2".to_string(),
                        value: Box::new(Expr::IntLit(IntLit { value: 42 })),
                    },
                ],
            }))),

            // Boolean literal
            Content::Expression(Box::new(Expr::BoolLit(BoolLit { value: true }))),
        ],
    }
}