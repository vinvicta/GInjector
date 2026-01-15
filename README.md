# GInjector

> A professional GS2 (Graal Script 2) development environment with integrated bytecode compiler and Frida-based injection for Graal Online clients.

[![Build Status](https://img.shields.io/badge/build-passing-brightgreen)](https://github.com/vinvicta/GInjector)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue)](LICENSE)

## Overview

GInjector is a modern, cross-platform IDE for GS2 scripting that combines:

- **Syntax-aware code editor** with tabbed interface
- **Native GS2 bytecode compiler** written in Rust
- **Frida-powered injection** for runtime script deployment
- **Support for Graal V6 and Graal Worlds clients**

The application provides a streamlined workflow for writing, compiling, and injecting GS2 scripts without leaving the IDE.

## Features

| Feature | Description |
|---------|-------------|
| **Script Editor** | Multi-tab editor with monospace font, undo/redo support |
| **Live Compilation** | Compile GS2 to bytecode with error reporting |
| **Client Detection** | Auto-detect running Graal clients |
| **Background Injection** | Non-blocking Frida injection via background threads |
| **Real-time Dashboard** | Status monitoring for Frida, process, and bytecode |
| **Cross-platform** | Windows, Linux, and macOS support |

## Screenshots

```
┌─────────────────────────────────────────────────────────────────────────────────────────┐
│  GInjector - GS2 Development Environment                                  [_][□][×]     │
├─────────────────────────────────────────────────────────────────────────────────────────┤
│                                                                                         │
│  ┌─────────────────────────────────────┐  ┌──────────────────┐  ┌─────────────────┐    │
│  │  Script Editor                      │  │  Dashboard       │  │  Actions        │    │
│  │                                     │  │                  │  │                 │    │
│  │  function onCreated() {             │  │  Frida: ✓        │  │  [Compile]      │    │
│  │    echo("Welcome to GInjector!");   │  │  Process: ✓      │  │  [Inject]       │    │
│  │    player.chat = "Hello!";          │  │  Client: V6      │  │  [Save]         │    │
│  │  }                                  │  │  Script: ✓       │  │                 │    │
│  │                                     │  │  Bytes: 247      │  │                 │    │
│  │                                     │  │                  │  │                 │    │
│  └─────────────────────────────────────┘  └──────────────────┘  └─────────────────┘    │
│                                                                                         │
│  ┌─────────────────────────────────────────────────────────────────────────────────┐    │
│  │  Log Window                                                                      │    │
│  │  > [12:34:05] GInjector started                                                 │    │
│  │  > [12:34:06] Compiled successfully: 247 bytes                                  │    │
│  │  > [12:34:10] Injecting into Graal.exe...                                       │    │
│  │  > [12:34:21] SCRIPT INJECTED                                                   │    │
│  └─────────────────────────────────────────────────────────────────────────────────┘    │
│                                                                                         │
└─────────────────────────────────────────────────────────────────────────────────────────┘
│  Untitled.gs2 | Target: Graal.exe | Ln 4, Col 12 | Frida Ready | Ctrl+I to Inject      │
└─────────────────────────────────────────────────────────────────────────────────────────┘
```

## Table of Contents

- [Installation](#installation)
- [Quick Start](#quick-start)
- [Configuration](#configuration)
- [Client Reference](#client-reference)
- [GS2 Language](#gs2-language-reference)
- [Injection Architecture](#injection-architecture)
- [Development](#development)
- [Troubleshooting](#troubleshooting)
- [License](#license)

---

## Installation

### Prerequisites

| Requirement | Version | Notes |
|-------------|---------|-------|
| **Rust** | 1.70+ | For building from source |
| **Frida CLI** | 16.0+ | Required for injection |
| **Graal Client** | V6 or Worlds | Target process must be running |

### Installing Frida

```bash
# Using pip (recommended)
pip install frida-tools

# Or using npm
npm install -g frida

# Verify installation
frida --version
```

### Building from Source

```bash
# Clone the repository
git clone https://github.com/vinvicta/GInjector.git
cd GInjector

# Build release binary
cargo build --release

# The executable will be at:
# - Linux/macOS: target/release/ginjector
# - Windows: target/release/ginjector.exe
```

### Pre-built Binaries

Pre-built binaries are available on the [Releases](https://github.com/vinvicta/GInjector/releases) page.

---

## Quick Start

### 1. Launch GInjector

```bash
./target/release/ginjector
```

### 2. Write Your Script

```gs2
function onCreated() {
    echo("GInjector is running!");

    // Player joined event
    function onPlayerEnters() {
        echo("Player " + player.name + " joined!");
    }
}
```

### 3. Compile

Press `Ctrl+B` or `F5` to compile your script. The bytecode size will be displayed in the dashboard.

### 4. Inject

1. Start your Graal client (V6 or Worlds)
2. Press `Ctrl+I` to inject the compiled bytecode
3. The script runs immediately in the client

### Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `Ctrl+S` | Save current script |
| `Ctrl+O` | Open file dialog |
| `Ctrl+B` / `F5` | Compile script |
| `Ctrl+I` | Inject bytecode |
| `Ctrl+T` | New tab |
| `Ctrl+W` | Close current tab |
| `Ctrl+Q` | Quit application |

---

## Configuration

GInjector uses a TOML configuration file. Create `config.toml` in the application directory:

```toml
# Client Configuration
[client]
# Target client: "graalv6" or "graalworlds"
type = "graalv6"

# Default variable name for injection (optional)
# Graal V6 defaults to: "VarName"
# Graal Worlds defaults to: "."
# variable_name = "MyCustomVar"

[editor]
# Editor settings
font_size = 14
tab_width = 4
show_line_numbers = true

[theme]
# UI colors (hex format)
background = "#1a1b26"
foreground = "#a9b1d6"
accent = "#7aa2f7"

[injection]
# Injection settings
timeout_seconds = 30
wait_before_inject = 10  # Seconds to wait for client init
```

---

## Client Reference

### Graal V6

| Property | Value |
|----------|-------|
| **Executable** | `Graal.exe` |
| **Packed** | Yes (Themida) |
| **Constructor** | `0x195770` |
| **SetScript** | `0x196290` |
| **Convention** | `thiscall` |
| **Magic Check** | `0x17da90` → `157876074` |
| **Default Var** | `VarName` |

> **Note:** Graal V6 is packed with Themida. Offsets apply to unpacked versions only.

### Graal Worlds

| Property | Value |
|----------|-------|
| **Executable** | `Worlds.exe` |
| **Packed** | No |
| **Constructor** | `0x9A340` |
| **SetScript** | `0x9EDE0` |
| **Convention** | `cdecl` |
| **Magic Check** | N/A |
| **Default Var** | `.` |

---

## GS2 Language Reference

### Basic Syntax

```gs2
// Event handlers
function onCreated() {
    // Runs when script is loaded
}

function onPlayerEnters() {
    // Runs when a player enters the area
}

function onPlayerChats() {
    // Runs when a player sends a message
}
```

### Variables

```gs2
// Local variables
local x = 5;
local text = "Hello";

// Global variables (persist across events)
this.counter = 0;
```

### Built-in Objects

| Object | Description |
|--------|-------------|
| `player` | The triggering player |
| `this` | The script object itself |
| `GraalControl` | Engine control functions |

### Events

| Event | Trigger |
|-------|---------|
| `onCreated()` | Script initialized |
| `onPlayerEnters()` | Player entered level |
| `onPlayerLeaves()` | Player left level |
| `onPlayerChats()` | Player sent message |
| `onTimeout()` | Timer triggered |

For a complete GS2 reference, see [DOCUMENTATION.md](DOCUMENTATION.md).

---

## Injection Architecture

### Workflow Diagram

```
┌─────────────┐      ┌──────────────┐      ┌─────────────┐
│   GS2       │ ──►  │   Compiler   │ ──►  │  Bytecode   │
│   Source    │      │  (Rust)      │      │  (Vec<u8>)  │
└─────────────┘      └──────────────┘      └─────────────┘
                                                   │
                                                   ▼
┌─────────────┐      ┌──────────────┐      ┌─────────────┐
│   Graal     │ ◄──  │    Frida     │ ◄─── │   Script    │
│   Client    │      │   Injection  │      │  Generator  │
└─────────────┘      └──────────────┘      └─────────────┘
```

### Memory Layout

GInjector creates GS2-compatible string structures in target memory:

```
GS2 String Structure (in-process):
┌──────────────────────────────────────────────┐
│  Reference Pointer (8 bytes)                 │ ──┐
│  Points to data structure below              │   │
└──────────────────────────────────────────────┘   │
                                                  │
┌──────────────────────────────────────────────┘   │
│ Data Structure:                                 │
│ ┌──────────────────────────────────────────┐   │
│ │ Length (u32)    │ Value (u32)           │   │
│ │ 4 bytes         │ 4 bytes (=100)        │   │
│ ├──────────────────────────────────────────┤   │
│ │ String bytes...                         │   │
│ │ (variable length)                        │   │
│ ├──────────────────────────────────────────┤   │
│ │ Null terminator (1 byte)                 │   │
│ └──────────────────────────────────────────┘   │
└──────────────────────────────────────────────────┘
```

### TGraalVar Injection

The tool calls two native functions:

1. **TGraalVar::TGraalVar** - Constructor
   - Allocates variable object
   - Sets variable name

2. **TGraalVar::SetScript** - Script loader
   - Compiles bytecode into executable script
   - Attaches to variable

---

## Development

### Project Structure

```
ginjector/
├── src/
│   ├── main.rs           # Application entry point
│   ├── app.rs            # Main app state and UI
│   └── config.rs         # Configuration management
├── frida-bridge/         # Frida integration crate
│   └── src/lib.rs        # Injection logic
├── gs2-compiler/         # GS2 compiler crate
│   └── src/
│       ├── lib.rs        # Compiler interface
│       ├── parser/       # Language parser
│       └── opcode/       # Bytecode definitions
├── docs/                 # Additional documentation
├── tests/                # Integration tests
└── Cargo.toml           # Workspace config
```

### Building

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Run tests
cargo test

# Run with logging
RUST_LOG=debug cargo run
```

### Code Style

```bash
# Format code
cargo fmt

# Run linter
cargo clippy -- -W warnings
```

---

## Troubleshooting

### Frida Not Detected

**Symptoms:** Dashboard shows "Frida: ✗"

**Solutions:**
1. Verify Frida installation: `frida --version`
2. Check PATH includes Frida binary location
3. Reinstall Frida: `pip install --upgrade frida-tools`

### Injection Fails

**Symptoms:** "Injection failed" in logs

**Solutions:**
1. Ensure Graal client is running before injecting
2. Verify correct client type is selected (V6 vs Worlds)
3. Run GInjector with elevated permissions (sudo/admin)
4. Check for antivirus interference with Frida

### GUI Freezes During Injection

**Symptoms:** Application becomes unresponsive

**Solutions:**
1. This is normal for the 10-second wait period
2. The UI remains responsive with background threading
3. Check logs for completion message

### Compilation Errors

**Symptoms:** "Compilation failed" message

**Solutions:**
1. Check GS2 syntax for errors
2. Review log panel for specific error messages
3. Ensure all event functions are properly closed

### Themida Packed Clients

**Symptoms:** Injection crashes Graal V6 client

**Explanation:** Graal V6 uses Themida packing which obfuscates memory

**Solutions:**
1. Use an unpacked version of Graal.exe for development
2. Target Graal Worlds instead (unpacked)
3. Update offsets for packed version (requires reverse engineering)

---

## Documentation

- [DOCUMENTATION.md](DOCUMENTATION.md) - Complete GS2 language reference
- [ARCHITECTURE.md](docs/ARCHITECTURE.md) - Internal design documentation
- [CONTRIBUTING.md](CONTRIBUTING.md) - Contribution guidelines

---

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

```
MIT License

Copyright (c) 2025 vinvicta

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction...
```

---

## Disclaimer

This tool is provided for educational and research purposes only. Using Frida to inject code into running processes may violate the Terms of Service of some applications. Users are responsible for ensuring their use complies with applicable laws and terms. The author assumes no liability for misuse.

---

## Resources

| Resource | Link |
|----------|------|
| GS2 Language | [Graal Scripts](https://graalonline.net/Creation/Dev/GScript) |
| Frida Docs | [frida.re/docs](https://frida.re/docs/) |
| GS2 Parser | [GitHub](https://github.com/xtjoeytx/gs2-parser) |
| egui Framework | [docs.rs/egui](https://docs.rs/egui/) |

---

## Author

**vinvicta**

- GitHub: [@vinvicta](https://github.com/vinvicta)

---

## Acknowledgments

- The Graal community for GS2 language documentation
- Frida developers for the excellent instrumentation framework
- egui developers for the simple and effective GUI library
