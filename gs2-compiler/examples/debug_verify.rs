use gs2_compiler::compile;

fn main() {
    let source = r#"function test() { temp.x = 10; return temp.x; }"#;
    match compile(source) {
        Ok(bytecode) => {
            println!("Bytecode length: {}", bytecode.len());
            
            // Find bytecode section (section type 4 at offset 39 for this case)
            // Section 4 header is 8 bytes, so actual bytecode starts at 39 + 8 = 47
            let idx = 47;
            
            // Print first 20 bytes of bytecode section
            println!("First 20 bytes:");
            for i in 0..20 {
                if idx + i < bytecode.len() {
                    print!("{:02x} ", bytecode[idx + i]);
                }
            }
            println!("\n");
            
            // Decode the sequence
            println!("Decoding:");
            let mut pos = idx;
            println!("  0x{:02x} = OP_SET_INDEX (1)", bytecode[pos]);
            println!("  0x{:02x} = F4 encoder", bytecode[pos+1]);
            let jump_target = i16::from_be_bytes([bytecode[pos+2], bytecode[pos+3]]);
            println!("  0x{:02x} 0x{:02x} = jump target {}", bytecode[pos+2], bytecode[pos+3], jump_target);
            println!("  0x{:02x} = OP_TYPE_ARRAY (23)", bytecode[pos+4]);
            println!("  0x{:02x} = ?? ({})", bytecode[pos+5], bytecode[pos+5]);
            println!("  0x{:02x} = OP_JMP (10)", bytecode[pos+6]);
        }
        Err(e) => eprintln!("Error: {}", e),
    }
}
