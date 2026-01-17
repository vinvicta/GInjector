use gs2_compiler::compile;

fn test_case(name: &str, source: &str) -> bool {
    match compile(source) {
        Ok(bytecode) => {
            println!("✅ {} - {} bytes", name, bytecode.len());
            true
        }
        Err(e) => {
            println!("❌ {} - Error: {}", name, e);
            false
        }
    }
}

fn main() {
    println!("Testing various GS2 code patterns:\n");

    let mut passed = 0;
    let total = 10;

    // Test 1: Simple function
    if test_case("Simple function", "function test() { return 42; }") {
        passed += 1;
    }

    // Test 2: Function with string assignment
    if test_case("String assignment", "function test() { player.chat = \"hello\"; }") {
        passed += 1;
    }

    // Test 3: Function with temp variables
    if test_case("Temp variables", "function test() { temp.x = 10; return temp.x; }") {
        passed += 1;
    }

    // Test 4: Function with if statement
    if test_case("If statement", "function test(x) { if (x > 0) { return 1; } else { return 0; } }") {
        passed += 1;
    }

    // Test 5: Function with while loop
    if test_case("While loop", "function test() { temp.i = 0; while (temp.i < 10) { temp.i++; } }") {
        passed += 1;
    }

    // Test 6: Function with for loop
    if test_case("For loop", "function test() { for (temp.i = 0; temp.i < 10; temp.i++) { } }") {
        passed += 1;
    }

    // Test 7: Function with array
    if test_case("Array literal", "function test() { temp.arr = [1, 2, 3]; return temp.arr; }") {
        passed += 1;
    }

    // Test 8: Function with object
    if test_case("Object literal", "function test() { temp.obj = {x: 10, y: 20}; return temp.obj; }") {
        passed += 1;
    }

    // Test 9: Function with function call
    if test_case("Function call", "function test() { return foo(1, 2, 3); }") {
        passed += 1;
    }

    // Test 10: Function with ternary
    if test_case("Ternary operator", "function test(x) { return x > 0 ? 1 : 0; }") {
        passed += 1;
    }

    println!("\n{}/{} tests passed", passed, total);
}
