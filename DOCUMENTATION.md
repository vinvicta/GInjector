# GInjector Documentation

Complete documentation for the GS2 language, compiler, and injection system.

## Table of Contents

1. [GS2 Language Reference](#gs2-language-reference)
2. [Bytecode Format](#bytecode-format)
3. [Compiler Internals](#compiler-internals)
4. [Injection System](#injection-system)
5. [API Reference](#api-reference)

---

## GS2 Language Reference

### Overview

GS2 (Graal Script 2) is a scripting language used by Graal Online for client-side and server-side scripting. It compiles to bytecode that is executed by the Graal engine.

### Lexical Structure

#### Comments

```gs2
// Single-line comment

/*
   Multi-line comment
*/
```

#### Identifiers

Identifiers start with a letter or underscore, followed by letters, digits, or underscores.

```gs2
local myVariable = 5;
local _private = "hidden";
local camelCase = true;
```

#### Literals

```gs2
// Numbers
local integer = 42;
local float = 3.14;
local hex = 0xFF;

// Strings
local text = "Hello, World!";
local withEscape = "Line 1\nLine 2";

// Booleans
local flag = true;
local noFlag = false;
```

### Types

| Type | Description | Example |
|------|-------------|---------|
| `number` | Numeric values (int/float) | `42`, `3.14` |
| `string` | Text values | `"hello"` |
| `bool` | true/false | `true`, `false` |
| `object` | Complex structures | `{x: 10, y: 20}` |
| `array` | Ordered lists | `[1, 2, 3]` |

### Variables

#### Local Variables

Scoped to the current function block.

```gs2
function onCreated() {
    local count = 0;
    local name = "Player";
    // count and name are local to onCreated
}
```

#### Object Properties (this.*)

Persist across function calls within the same script.

```gs2
function onCreated() {
    this.counter = 0;  // Persists
}

function onTimeout() {
    this.counter++;     // Accessible here
    echo(this.counter);
}
```

### Operators

#### Arithmetic

| Operator | Description |
|----------|-------------|
| `+` | Addition |
| `-` | Subtraction |
| `*` | Multiplication |
| `/` | Division |
| `%` | Modulo |
| `^` | Power |

#### Comparison

| Operator | Description |
|----------|-------------|
| `==` | Equal |
| `!=` | Not equal |
| `<` | Less than |
| `>` | Greater than |
| `<=` | Less or equal |
| `>=` | Greater or equal |

#### Logical

| Operator | Description |
|----------|-------------|
| `&&` | AND |
| `\|\|` | OR |
| `!` | NOT |

#### String

| Operator | Description |
|----------|-------------|
| `+` | Concatenation |
| `@` | Substring/index |

### Control Flow

#### If Statements

```gs2
if (condition) {
    // code
} else if (otherCondition) {
    // code
} else {
    // code
}
```

#### Loops

```gs2
// While loop
while (condition) {
    // code
}

// For loop
for (local i = 0; i < 10; i++) {
    echo(i);
}

// Foreach
for (local item : collection) {
    echo(item);
}
```

#### Switch

```gs2
switch (value) {
    case 1:
        // code
        break;
    case 2:
        // code
        break;
    default:
        // code
}
```

### Functions

#### Declaration

```gs2
function myFunction(param1, param2) {
    local result = param1 + param2;
    return result;
}
```

#### Calling

```gs2
local sum = myFunction(5, 10);
myFunction();  // No arguments
```

#### Arrow Functions

```gs2
local add = (a, b) => a + b;
```

### Event Handlers

Event handlers are special functions called by the Graal engine.

```gs2
// Script initialization
function onCreated() {
    echo("Script loaded");
}

// Player enters the level
function onPlayerEnters() {
    echo(player.name @ " joined");
}

// Player leaves the level
function onPlayerLeaves() {
    echo(player.name @ " left");
}

// Player sends a message
function onPlayerChats() {
    echo(player.chat);
}

// Timer event (set with setTimer)
function onTimeout() {
    echo("Timer fired");
}

// Player triggers a NPC
function onActionNPC() {
    echo("NPC clicked");
}
```

### Built-in Objects

#### player

Represents the triggering player.

| Property | Type | Description |
|----------|------|-------------|
| `name` | string | Player name |
| `id` | number | Player account ID |
| `x` | number | X coordinate |
| `y` | number | Y coordinate |
| `chat` | string | Last chat message |
| `account` | string | Account name |

#### this

The script object itself.

```gs2
function onCreated() {
    this.storedValue = 100;
    this.timeout = setTimer(10, "onTimeout");
}
```

#### GraalControl

Engine control functions.

```gs
GraalControl.login();  // Trigger login
```

### Built-in Functions

#### Output

```gs2
echo("Hello");           // Output to console
echo("Value: " @ value); // String concatenation
```

#### Timers

```gs2
// Set a timer (seconds, function name)
local timerId = setTimer(5, "onTimeout");

// Kill a timer
killTimer(timerId);

// Kill all timers
killTimer();
```

#### Player Manipulation

```gs2
player.chat = "Hello!";
player.x = 50;
player.y = 50;
player dir = 2;  // Direction: 0=up, 1=right, 2=down, 3=left
```

#### Weapons

```gs2
player.addweapon("weapon_name");
player.remweapon("weapon_name");
```

---

## Bytecode Format

### Header Structure

```
+------------------+
| Magic (4 bytes)  | = 0x00000001
+------------------+
| Flags (4 bytes)  | Reserved
+------------------+
| Script Body      | Variable length
+------------------+
```

### Instruction Encoding

Each instruction consists of:

- **Opcode** (1 byte): The operation to perform
- **Arguments**: Variable length depending on opcode

### Common Opcodes

| Opcode | Name | Description |
|--------|------|-------------|
| 0x00 | NOP | No operation |
| 0x01 | PUSH | Push value to stack |
| 0x02 | POP | Pop from stack |
| 0x03 | LOAD | Load variable |
| 0x04 | STORE | Store to variable |
| 0x05 | CALL | Call function |
| 0x06 | RET | Return from function |
| 0x07 | JMP | Jump to address |
| 0x08 | JZ | Jump if zero |
| 0x29 | STRING | Push string constant |

---

## Compiler Internals

### Architecture

```
GS2 Source
    │
    ▼
┌─────────┐
│  Lexer  │ (logos) → Tokens
└─────────┘
    │
    ▼
┌─────────┐
│ Parser  │ (LALRPOP) → AST
└─────────┘
    │
    ▼
┌─────────┐
│ Encoder │ → Bytecode
└─────────┘
```

### Lexer (logos)

The tokenizer converts GS2 source code into a stream of tokens.

```rust
// Token types
enum Token {
    Identifier(String),
    Number(f64),
    StringLiteral(String),
    // ... operators, keywords, etc.
}
```

### Parser

The parser builds an Abstract Syntax Tree (AST) from the token stream.

```rust
// AST nodes
enum Stmt {
    Function(String, Vec<Stmt>),
    If(Expr, Vec<Stmt>, Option<Vec<Stmt>>),
    While(Expr, Vec<Stmt>),
    // ... etc
}

enum Expr {
    Binary(Box<Expr>, Op, Box<Expr>),
    Call(String, Vec<Expr>),
    // ... etc
}
```

---

## Injection System

### Memory Layout

GS2 strings are represented as:

```
Reference: 8 bytes (pointer to data)
Data:
  - Length: u32 (4 bytes)
  - Value: u32 (4 bytes) - always 100 for GS2 strings
  - Bytes: variable length
  - Null: 1 byte
```

### Function Offsets

#### Graal V6

| Function | Offset | Convention |
|----------|--------|------------|
| Constructor | 0x195770 | thiscall |
| SetScript | 0x196290 | thiscall |

#### Graal Worlds

| Function | Offset | Convention |
|----------|--------|------------|
| Constructor | 0x9A340 | cdecl |
| SetScript | 0x9EDE0 | cdecl |

### Injection Process

1. **Generate Frida script** with correct client offsets
2. **Allocate memory** in target process
3. **Create GS2 string structures** for variable name and bytecode
4. **Call constructor** to create TGraalVar instance
5. **Call SetScript** to inject bytecode
6. **Wait for execution** (script runs immediately after SetScript)

---

## API Reference

### frida-bridge

```rust
use frida_bridge::{FridaInjector, ClientType};

// Create injector
let injector = FridaInjector::new(ClientType::GraalWorlds);

// Generate script
let script = injector.generate_injection_script(
    "00 01 02 FF",  // bytecode hex
    "."            // variable name
);

// Inject (async)
let result = injector.inject(&bytecode, ".").await?;
```

### gs2-compiler

```rust
use gs2_compiler::Compiler;

// Compile script
let compiler = Compiler::new();
let bytecode = compiler.compile(source_code)?;

// Get hex representation
let hex = compiler.bytecode_to_hex(&bytecode);
```

---

## Appendix

### Error Codes

| Code | Description |
|------|-------------|
| E001 | Syntax error |
| E002 | Undefined variable |
| E003 | Type mismatch |
| E004 | Stack overflow |
| E005 | Missing event handler |

### Version History

| Version | Date | Changes |
|---------|------|---------|
| 0.1.0 | 2025-01 | Initial release |
| 0.2.0 | 2025-01 | Add async injection |

### Support

For issues and questions:
- GitHub: https://github.com/vinvicta/GInjector/issues
- Documentation: https://github.com/vinvicta/GInjector/wiki
