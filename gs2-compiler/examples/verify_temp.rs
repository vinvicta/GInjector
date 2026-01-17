use gs2_compiler::compile;

fn main() {
    let source = r#"function test() { temp.x = 10; return temp.x; }"#;
    match compile(source) {
        Ok(bytecode) => {
            println!("Our compiler output:");
            for (i, chunk) in bytecode.chunks(16).enumerate() {
                print!("{:08x}: ", i * 16);
                for b in chunk {
                    print!("{:02x} ", b);
                }
                println!();
            }
        }
        Err(e) => eprintln!("Error: {}", e),
    }
}
