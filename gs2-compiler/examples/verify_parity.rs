use gs2_compiler::compile;

fn compare_with_official(name: &str, source: &str, official_hex: &str) -> bool {
    match compile(source) {
        Ok(bytecode) => {
            let ours_hex: String = bytecode.iter().map(|b| format!("{:02x}", b)).collect();
            if ours_hex == official_hex.to_lowercase() {
                println!("✅ {}: PERFECT MATCH!", name);
                true
            } else {
                println!("❌ {}: MISMATCH", name);
                println!("   Expected: {}", official_hex);
                println!("   Got:      {}", ours_hex);
                false
            }
        }
        Err(e) => {
            println!("❌ {}: ERROR - {}", name, e);
            false
        }
    }
}

fn main() {
    println!("Testing bytecode parity with official compiler:\n");

    let mut passed = 0;
    let total = 3;

    // Test 1: function onCreated() { player.chat = "hello"; }
    // Official: 000000010000000400000000000000020000000e000000016f6e4372656174656400000000030000000b636861740068656c6c6f00000000040000001501f4000c17330ab616f0002315f0013214f30007070a
    if compare_with_official(
        "player.chat assignment",
        r#"function onCreated() { player.chat = "hello"; }"#,
        "000000010000000400000000000000020000000e000000016f6e4372656174656400000000030000000b636861740068656c6c6f00000000040000001501f4000c17330ab616f0002315f0013214f30007070a"
    ) {
        passed += 1;
    }

    // Test 2: function test() { temp.x = 10; return temp.x; }
    // Official from actual compiler run
    if compare_with_official(
        "temp.x assignment and return",
        r#"function test() { temp.x = 10; return temp.x; }"#,
        "000000010000000400000000000000020000000900000001746573740000000003000000027800000000040000001701f4000e17330abd16f0002314f30a32bd16f0002307070a"
    ) {
        passed += 1;
    }

    // Test 3: Simple function
    if compare_with_official(
        "simple function",
        r#"function test() { return 42; }"#,
        "00000001000000040000000000000002000000090000000174657374000000000300000000000000040000000c01f4000717330a14f32a07070a"
    ) {
        passed += 1;
    }

    println!("\n{}/{} tests passed", passed, total);
    if passed == total {
        println!("✅ ALL TESTS PASSED - 1:1 parity achieved!");
    }
}
