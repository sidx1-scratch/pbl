# PBL — Polar Bear Language (v0.4.0)

PBL is a compiled programming language featuring Python-inspired simplified syntax backed by Rust's performance, type safety, and ecosystem capabilities.

PBL compiles source code into Rust, managing project setups and delegating binary compilation to Cargo and the Rust toolchain.

---

## Key Features

- **Python-inspired readability**: Concise syntax without trailing semicolons.
- **Rust capabilities & safety**: Backed directly by Rust primitives and compiler guarantees.
- **Direct Crates.io support**: Native declarations for external crates and features (`crate <name> = "<version>"`).
- **Rust interoperability**: Embedded `rust { ... }` blocks for raw Rust features or complex libraries.
- **Systems programming groundwork**: Built-in support for `no_std` targets and custom compilation triples.
- **Helpful compiler diagnostics**: Span-tracked syntax and semantic error reports.

---

## Compilation Architecture

```text
PBL Source (*.pbl)
       |
     Lexer (Tokenization, Span Tracking)
       |
     Parser (AST Construction, Precedence)
       |
  Semantic Analysis (Scope & Type Checking)
       |
  Rust Code Generator (PBL -> Idiomatic Rust)
       |
  Cargo Integration (Project Generation, Cargo.toml)
       |
     Cargo / rustc
       |
  Native Executable
```

---

## Installation & Build Instructions

### Prerequisites

- Rust toolchain (`rustc`, `cargo` 1.80+ recommended).

### Building the PBL Compiler

```bash
git clone <repo-url> pbl
cd pbl
cargo build --release
```

The resulting compiler binary will be located at `target/release/pbl`.

---

## CLI Usage

```text
pbl <command> [options] <file>
```

### Commands

| Command | Description |
|---|---|
| `pbl run <file>` | Compile and immediately execute the PBL program. |
| `pbl build <file>` | Compile into a native executable binary. |
| `pbl compile <file>` | Generate the Rust project without running Cargo build. |
| `pbl check <file>` | Validate syntax, semantics, and Cargo checking without linking. |
| `pbl clean [file]` | Clean generated build directories (`.pbl_build`). |
| `pbl version` | Print compiler version information. |
| `pbl help` | Display CLI usage details. |

### Options

- `--release`: Build with Cargo release optimizations.
- `--output <path>`: Specify custom binary output destination.
- `--target <triple>`: Set cross-compilation target (e.g. `x86_64-unknown-none`).
- `--no-std`: Force `#![no_std]` code generation.
- `--verbose`: Enable detailed pipeline logs.
- `--keep-generated`: Retain generated Cargo project in `.pbl_build/`.

---

## Language Syntax Guide

### 1. Functions & Entry Point

```pbl
fn greet(name: str) {
    println("Hello " + name)
}

fn add(a: i32, b: i32) -> i32 {
    return a + b
}

fn main() {
    greet("Polar Bear")
    let sum = add(5, 10)
    println(sum)
}
```

### 2. Variables & Mutability

Immutable bindings use `let`, while mutable bindings use `mut`:

```pbl
let animal: str = "Polar Bear"
let age: i32 = 11

mut score: i32 = 0
score = score + 1
score += 5
```

Type annotations are optional when inferrable.

### 3. Supported Types

| PBL Type | Rust Target |
|---|---|
| `str` | `String` / `&str` |
| `i32`, `i64`, `i128` | Standard signed integers |
| `u8`, `u32`, `u64`, `usize` | Standard unsigned integers |
| `f32`, `f64` | Floating-point numbers |
| `bool` | `bool` |
| `char` | `char` |
| `[T]` | `Vec<T>` |
| `?T` | `Option<T>` |
| `(A, B)` | Tuples |
| `&T`, `&mut T` | References |

### 4. Control Flow

```pbl
// Conditionals
if score > 100 {
    println("High score!")
} else if score > 50 {
    println("Good score")
} else {
    println("Keep trying")
}

// While loops
mut count = 0
while count < 5 {
    println(count)
    count += 1
}

// For loops
let items = [10, 20, 30]
for item in items {
    println(item)
}
```

### 5. Structs & Enums

```pbl
struct PolarBear {
    name: str,
    age: i32
}

enum Animal {
    Bear,
    Penguin,
    Fox
}

fn main() {
    let bear = PolarBear { name: "Nanuk", age: 11 }
    println(bear.name)
}
```

### 6. Crate Ecosystem Integration

PBL allows declaring external Cargo crates directly in source code:

```pbl
crate rand = "0.9"

use rand

fn main() {
    println("External crate integration active")
}
```

Features can be specified with `features`:

```pbl
crate serde = "1" features ["derive"]
```

### 7. Rust Interoperability

Direct Rust code can be embedded using `rust { ... }` blocks:

```pbl
fn main() {
    rust {
        use std::collections::HashMap;

        let mut map = HashMap::new();
        map.insert("ice", 100);
        for (k, v) in &map {
            println!("{}: {}", k, v);
        }
    }
}
```

### 8. Systems Programming & `no_std`

Low-level systems programming target groundwork:

```pbl
project {
    mode: no_std
}

rust {
    #![no_main]

    use core::panic::PanicInfo;

    #[panic_handler]
    fn panic(_info: &PanicInfo) -> ! {
        loop {}
    }
}
```

Compile with:
```bash
pbl compile kernel.pbl --target x86_64-unknown-none --no-std
```

---

## Project Structure

```text
pbl/
├── Cargo.toml
├── README.md
├── LICENSE
├── src/
│   ├── main.rs         # CLI binary entry point
│   ├── lib.rs          # Library root
│   ├── cli.rs          # CLI argument parser & execution router
│   ├── ast/            # AST data structures & Span definition
│   ├── lexer/          # Token kinds and source tokenizer
│   ├── parser/         # Pratt-parser and AST constructor
│   ├── semantic/       # Scope, symbol, and type validation
│   ├── codegen/        # Rust source code generation
│   ├── project/        # Cargo.toml generator and Cargo command runner
│   ├── compiler/       # Orchestration pipeline
│   └── errors.rs       # Diagnostic error types and formatting
├── examples/           # Validated PBL example code
└── tests/              # Compiler test suites
```

---

## License

This project is licensed under the Apache License, Version 2.0. See [LICENSE](file:///home/sidx1/pbl/LICENSE) for details.
