mod lexer;

fn main() {
    let source = String::from("let const func if else then for use include export obj in ( ) { } [ ] + - * % / < > . ; : , || | -> => = ! && & == != <= >= ' \"");
    let tokens = lexer::tokenize(source);
    println!("{:#?}", tokens);
}