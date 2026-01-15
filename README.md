# GraalHax

A Terminal User Interface (TUI) development environment for GS2 (Graal Script 2) with integrated compilation and bytecode injection via Frida.

## Features

- **Script Editor** - Vim-like keybindings for writing GS2 scripts
- **Built-in Compiler** - Integrated GS2 compiler (ported from C++ to Rust)
- **Client Toggle** - Switch between Graal V6 and Graal Worlds clients
- **Frida Injection** - Native bytecode injection without external scripts
- **Real-time Status** - Dashboard showing connection, compilation, and bytecode info
- **Multiple Tabs** - Edit multiple GS2 scripts simultaneously

## Screenshots

```
+------------------------------------------------------------------------------------------+
|  Menu Bar: [File] [Edit] [Build] [Tools]                                                  |
+------------------------------------------------------------------------------------------+
|  Dashboard (Status)                                                                       |
|  +----------+  +----------+  +----------+  +----------+  +----------+                     |
|  | Graal V6 |  | Frida    |  | Script   |  | Bytecode |  | Toggle   |                     |
|  |          |  | Attached |  | Compiled |  | 247 bytes|  | ^T=Client|                     |
|  +----------+  +----------+  +----------+  +----------+  +----------+                     |
+------------------------------------------------------------------------------------------+
|  +-------------------------------+  +--------------------------------------------------+  |
|  |   Script Editor               |  |  Log Window                                       |  |
|  |   (vim-like keybindings)      |  |  > Compilation successful                          |  |
|  |                               |  |  > Bytecode: 247 bytes                            |  |
|  |   function onCreated() {      |  |  [14:32:05] Script injected to Graal.exe          |  |
|  |     echo("Hello World");      |  |  [14:32:07] onCreated triggered                   |  |
|  |   }                          |  |                                                   |  |
|  +-------------------------------+  +--------------------------------------------------+  |
+------------------------------------------------------------------------------------------+
|  Bytecode Preview (hex)                                                                   |
|  00 00 00 01 00 00 00 04 00 00 00 00 00 00 00 02 00 00 00 29 ...                      |
+------------------------------------------------------------------------------------------+
|  Status Bar: [NORMAL] | Ln 12, Col 8 | weapon.gs2 | Target: Graal.exe | Ctrl+I to Inject   |
+------------------------------------------------------------------------------------------+
```

## Requirements

- **Rust** 1.70+ - For building the TUI
- **Frida CLI** - For bytecode injection
  - Install from: https://frida.re/docs/installation/
- **Target Graal Client** - Graal V6 or Graal Worlds running

## Building

### Linux/macOS
```bash
git clone https://github.com/yourusername/graalhax.git
cd graalhax
cargo build --release
```

### Windows
```bash
git clone https://github.com/yourusername/graalhax.git
cd graalhax
cargo build --release
```

The executable will be at `target/release/graalhax` (Linux/macOS) or `target/release/graalhax.exe` (Windows).

## Usage

### Basic Workflow

1. **Launch the TUI**
   ```bash
   ./target/release/graalhax
   ```

2. **Write your GS2 script**
   - Press `i` to enter insert mode
   - Type your GS2 code
   - Press `Esc` to return to normal mode

3. **Compile the script**
   - Press `Ctrl+B` to compile
   - Check the log window for compilation results

4. **Inject into the running client**
   - Make sure your Graal client is running
   - Press `Ctrl+I` to inject the bytecode
   - Watch the logs for injection status

### Keybindings

| Key | Action |
|-----|--------|
| **Movement** | |
| `↑` | Move cursor up |
| `↓` | Move cursor down |
| `←` | Move cursor left |
| `→` | Move cursor right |
| **Modes** | |
| `i` or `Enter` | Enter insert mode |
| `Esc` | Return to normal mode |
| **Editing** | |
| `Enter` | Insert newline |
| `Backspace` | Delete character before cursor |
| `Delete` | Delete character at cursor |
| `Tab` | Insert tab |
| **Files** | |
| `Ctrl+S` | Save current tab |
| `Ctrl+O` | Open file |
| `Ctrl+N` | New tab |
| `Ctrl+W` | Close tab |
| `Tab` | Next tab |
| `Shift+Tab` | Previous tab |
| **Build** | |
| `Ctrl+B` | Compile script |
| `Ctrl+I` | Inject bytecode |
| **Client** | |
| `Ctrl+T` | Toggle client (V6 ↔ Worlds) |
| **Other** | |
| `Ctrl+Q` | Quit |
| `:` | Open command prompt |

## Client Configuration

### Graal V6
| Property | Value |
|----------|-------|
| Module | `Graal.exe` |
| Constructor Offset | `0x195770` |
| SetScript Offset | `0x196290` |
| Calling Convention | `thiscall` |
| Magic Check | `0x17da90` = `157876074` |
| Default Variable | `VarName` |

### Graal Worlds
| Property | Value |
|----------|-------|
| Module | `Graal3DEngine.dll` |
| Constructor Offset | `0x9A340` |
| SetScript Offset | `0x9EDE0` |
| Calling Convention | `cdecl` |
| Magic Check | None |
| Default Variable | `.` |

## Configuration

Create a `config.toml` file in the same directory as the executable:

```toml
# GS2 Compiler path (optional)
gs2_compiler_path = "./gs2-parser/bin/gs2test"

# Client type: "graalv6" or "graalworlds"
client_type = "graalv6"

# Override default variable name (optional)
# default_variable_name = "MyVar"

[editor]
line_numbers = true
tab_width = 4
use_spaces = true

[theme]
background = "#1a1b26"
foreground = "#a9b1d6"
primary = "#7aa2f7"
secondary = "#bb9af7"
error = "#f7768e"
warning = "#e0af68"
success = "#9ece6a"
```

## Architecture

```
graalhax/
├── src/
│   ├── main.rs              # Entry point, TUI initialization
│   ├── app.rs               # Application state, editor logic
│   ├── config.rs            # Configuration management
│   └── ui/
│       └── mod.rs           # UI rendering (5-panel layout)
├── frida-bridge/            # Frida injection library
│   └── src/
│       └── lib.rs           # Native injection logic
├── gs2-compiler/            # GS2 compiler (WIP)
│   └── src/
│       ├── parser/
│       │   └── lexer.rs     # Logos-based tokenizer
│       └── opcode/
│           └── mod.rs       # Opcode definitions
└── Cargo.toml               # Workspace configuration
```

## Injection Method

GraalHax injects GS2 bytecode by:

1. **Compiling** GS2 source code to bytecode
2. **Generating** a Frida script with the correct client offsets
3. **Creating** GS2 string structures in target process memory
4. **Calling** `TGraalVar` constructor and `SetScript` methods

### Memory Layout

The GS2 string structure created in memory:

```
+--------+--------+
|  Ref   | Data   |
+--------+--------+
   |         |
   v         v
+------+  +-----+-----+-----+-----+-----+-----+
| ptr  |  | len | val | ...bytes... | \0 |
+------+  +-----+-----+-----+-----+-----+-----+
   8B        4B    4B    len+1      1B
```

## Development

### Project Status

- [x] TUI Framework (ratatui)
- [x] Editor with vim-like keybindings
- [x] Client type toggle
- [x] Native Frida injection
- [x] GS2 Lexer (logos)
- [x] Opcode definitions
- [ ] LALRPOP grammar for parsing
- [ ] AST nodes
- [ ] Bytecode encoder
- [ ] Full compiler integration

### Running Tests

```bash
cargo test
```

### Code Coverage

```bash
cargo install cargo-tarpaulin
cargo tarpaulin --out Html
```

## Troubleshooting

### Frida not detected
- Ensure Frida CLI is installed: `frida --version`
- Check that Frida is in your PATH

### Injection failed
- Ensure the Graal client is running
- Check that you've selected the correct client type (Ctrl+T)
- Try running the client as administrator/sudo

### Compilation errors
- The GS2 compiler is still being ported from C++
- Use the external `gs2-parser` C++ compiler for now

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/AmazingFeature`)
3. Commit your changes (`git commit -m 'Add some AmazingFeature'`)
4. Push to the branch (`git push origin feature/AmazingFeature`)
5. Open a Pull Request

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- **GS2 Parser** - Original C++ GS2 compiler by the Graal community
- **Frida** - Dynamic instrumentation framework
- **ratatui** - Rust TUI library
- **logos** - Rust lexer generator

## Disclaimer

This tool is for educational purposes only. Using Frida to inject code into running processes may violate the Terms of Service of some games and applications. Use responsibly and at your own risk.

## Resources

- [GS2 Language Reference](https://www.graalonline.com/)
- [Frida Documentation](https://frida.re/docs/)
- [ratatui Documentation](https://docs.rs/ratatui/)
