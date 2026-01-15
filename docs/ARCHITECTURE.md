# GInjector Architecture

Internal design documentation for developers.

## System Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                         GInjector                               │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐         │
│  │    GUI       │  │  Compiler    │  │   Frida      │         │
│  │   (egui)     │  │  (gs2-comp)  │  │  (frida-br)  │         │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘         │
│         │                 │                  │                  │
│         └─────────────────┴──────────────────┘                  │
│                           │                                      │
│                           ▼                                      │
│                   ┌───────────────┐                             │
│                   │  Application  │                             │
│                   │    State      │                             │
│                   └───────────────┘                             │
└─────────────────────────────────────────────────────────────────┘
```

## Components

### 1. GUI Layer (`src/`)

**Responsibility:** User interface and interaction

**Files:**
- `main.rs` - Application entry point, eframe setup
- `app.rs` - Main application state, egui App trait
- `config.rs` - Configuration management

**Key Types:**

```rust
pub struct GraalHaxApp {
    tabs: Vec<ScriptTab>,
    logs: Vec<LogEntry>,
    client_type: ClientType,
    compiled_bytecode: Option<Vec<u8>>,
    // ...
}
```

**Threading Model:**

The UI runs on the main thread. Background operations use:

1. **Status polling thread** - Checks Frida/process availability every 2 seconds
2. **Injection thread** - Spawns per injection, non-blocking

Communication via `mpsc::channel`:

```
Background Thread          Main Thread (UI)
     │                          ▲
     │ try_send()               │ try_recv()
     └──────────────────────────┘
          mpsc::channel
```

### 2. Compiler (`gs2-compiler/`)

**Responsibility:** GS2 source → Bytecode

**Structure:**

```
gs2-compiler/
├── lib.rs           # Public API
├── parser/
│   ├── lexer.rs     # Tokenization (logos)
│   └── parser.rs    # AST generation
└── opcode/
    ├── mod.rs       # Opcode definitions
    └── encoder.rs   # Bytecode encoding
```

**API:**

```rust
pub struct Compiler {
    // Compiler state
}

impl Compiler {
    pub fn new() -> Self;
    pub fn compile(&mut self, source: &str) -> Result<Vec<u8>, Error>;
    pub fn bytecode_to_hex(&self, bytecode: &[u8]) -> String;
}
```

### 3. Frida Bridge (`frida-bridge/`)

**Responsibility:** Frida script generation and execution

**Structure:**

```
frida-bridge/
└── lib.rs           # Injection logic
```

**Key Types:**

```rust
pub enum ClientType {
    GraalV6,
    GraalWorlds,
}

pub struct FridaInjector {
    client_type: ClientType,
}

impl FridaInjector {
    pub fn new(client_type: ClientType) -> Self;
    pub fn generate_injection_script(&self, bytecode_hex: &str, var_name: &str) -> String;
    pub fn inject(&self, bytecode: &[u8], variable_name: &str) -> Result<String, String>;
}
```

## Data Flow

### Compilation Flow

```
User Input (GS2)
       │
       ▼
┌──────────────┐
│  Editor Tab  │
└──────┬───────┘
       │ content
       ▼
┌──────────────┐
│  Compiler    │
│  .compile()  │
└──────┬───────┘
       │ Vec<u8>
       ▼
┌──────────────┐
│  Hex Encode  │
└──────┬───────┘
       │ String
       ▼
┌──────────────┐
│ Store in     │
│ app.bytecode │
└──────────────┘
```

### Injection Flow

```
Inject Click
       │
       ▼
┌──────────────┐
│  Verify      │
│  Bytecode    │
└──────┬───────┘
       │
       ▼
┌──────────────┐
│  Generate    │
│  Frida Script│
└──────┬───────┘
       │
       ▼
┌──────────────┐
│  Write to    │
│  Temp File   │
└──────┬───────┘
       │
       ▼
┌──────────────┐
│  Spawn       │
│  Thread      │
└──────┬───────┘
       │
       ▼
┌──────────────┐
│  Run Frida   │
│  CLI         │
└──────┬───────┘
       │
       ▼
┌──────────────┐
│  Send Result │
│  via Channel │
└──────────────┘
```

## Memory Management

### Script Storage

Each tab stores full source code in memory:

```
Vec<ScriptTab>
    └── content: String  // Full source
```

### Bytecode Storage

Compiled bytecode stored once:

```
compiled_bytecode: Option<Vec<u8>>
```

### Temporary Files

- Frida scripts written to temp directory
- Cleaned up after injection (on error, kept for debugging)

## Concurrency

### Thread 1: Main Thread (UI)

- egui event loop
- State updates
- Channel polling

### Thread 2: Status Polling

```rust
loop {
    let frida = check_frida();
    let process = check_process();
    tx.send((frida, process))?;
    sleep(Duration::from_secs(2));
}
```

### Thread N: Injection Workers

Spawned per injection, exits after completion:

```rust
thread::spawn(|| {
    let result = run_frida();
    tx.send(result);
});
```

## State Machine

```
┌─────────┐     compile      ┌──────────────┐
│  Idle   │ ───────────────► │  Compiled    │
└─────────┘                   └──────────────┘
     ▲                              │
     │              inject          │ result
     └──────────────────────────────┘
          (injection in progress)
```

## Security Considerations

1. **Temp Files:** Written with restrictive permissions
2. **Frida Scripts:** Generated dynamically, never user-supplied
3. **Process Injection:** Requires target process to be running
4. **No Network:** All operations are local

## Extension Points

### Adding New Clients

```rust
// In ClientType enum
pub enum ClientType {
    GraalV6,
    GraalWorlds,
    // Add new client here
    GraalClassic,
}

// Add offset methods
impl ClientType {
    pub fn tgralvar_constructor_offset(&self) -> usize {
        match self {
            ClientType::GraalClassic => 0xXXXXX,
            // ...
        }
    }
}
```

### Custom Opcodes

```rust
// In gs2-compiler/src/opcode/mod.rs
pub enum Opcode {
    // Existing...
    Custom(0xFF),  // Add custom opcode
}
```

## Performance

| Operation | Time | Notes |
|-----------|------|-------|
| Compile | <10ms | Simple scripts |
| Inject | 10-30s | Includes 10s wait for init |
| UI Render | 16ms | 60 FPS target |

## Dependencies

```
ginjector
├── eframe 0.29        # GUI framework
├── egui 0.29          # UI widgets
├── tokio 1.40         # Async runtime
├── serde 1.0          # Serialization
├── logos 0.14         # Lexer generator
└── anyhow 1.0         # Error handling
```
