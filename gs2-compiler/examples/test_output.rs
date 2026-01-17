use gs2_compiler::compile;

fn main() {
    // Test case: function onCreated() { player.chat = "hello"; }
    let source = r#"function onCreated() { player.chat = "hello"; }"#;

    match compile(source) {
        Ok(bytecode) => {
            println!("Bytecode output ({} bytes):", bytecode.len());
            for (i, b) in bytecode.iter().enumerate() {
                print!("{:02x} ", b);
                if (i + 1) % 16 == 0 {
                    println!();
                }
            }
            println!();

            // Verify bytecode section matches official compiler
            // Official bytecode section: 01 f4 00 0c 17 33 0a b6 16 f0 00 23 15 f0 01 32 14 f3 00 07 07 0a
            // Bytecode section starts after all headers (offset 61)
            let section_start = 61;
            let bytecode_section: Vec<u8> = bytecode[section_start..].to_vec();
            let expected_section = vec![
                0x01, 0xf4, 0x00, 0x0c, 0x17, 0x33, 0x0a, 0xb6,
                0x16, 0xf0, 0x00, 0x23, 0x15, 0xf0, 0x01, 0x32,
                0x14, 0xf3, 0x00, 0x07, 0x07, 0x0a,
            ];

            if bytecode_section == expected_section {
                println!("✅ Bytecode matches official compiler!");
            } else {
                println!("❌ Bytecode differs from official compiler");
                println!("Expected: {:?}", expected_section);
                println!("Got:      {:?}", bytecode_section);
            }
        }
        Err(e) => {
            eprintln!("Compilation error: {}", e);
        }
    }
}
