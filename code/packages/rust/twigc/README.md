# twigc — Twig Compiler CLI

`twigc` is the command-line compiler driver for the Twig programming language
(TW05-R / LANG73).  It wraps the full multi-file compilation pipeline — module
resolution, Phase 3.5 type-checking, IIR compilation, and interpreter execution
— in a single binary.

## Usage

```bash
twigc [OPTIONS] <file.tw>
```

### Options

| Flag | Description |
|------|-------------|
| `--check` | Type-check only.  Exit 0 on success, 1 on type errors in `(typed strict)` modules. |
| `--emit=iir` | Compile to IIR and print a human-readable listing to stdout. |
| `--search-path=<DIR>` | Add DIR to the module search path (repeatable). |
| `-h`, `--help` | Print help. |
| `-V`, `--version` | Print version. |

Default (no flags): compile and run via `twig-vm`; print the integer return
value of `main()` to stdout.

## Exit codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Type error in a `(typed strict)` module |
| 2 | Compilation error (parse error, missing import, …) |
| 3 | Runtime trap from the interpreter |
| 4 | Usage error (bad flags, missing file) |

## Examples

```bash
# Type-check only — useful in CI:
twigc --check src/main.tw

# Dump IIR for debugging:
twigc --emit=iir src/main.tw

# Compile and run:
twigc src/main.tw

# Multi-module program with an explicit search path:
twigc --search-path=lib src/main.tw
```

## How it works

```
<file.tw>
   │
   ▼  twig-module-driver::compile_module_tree
Phase 1: recursive import discovery
Phase 2: cycle detection
Phase 3: extern name collection
Phase 3.5: topological type-check  ← LANG72 / TW05-Q
Phase 4: per-module IIR compilation
Phase 5: IIR linking
   │
   ▼  twig-vm::run  (default mode only)
IIRModule → interpreter → i64 result
```

`--check` stops after Phase 5 (compilation only).
`--emit=iir` runs Phase 1–5 and formats the resulting `IIRModule`.
Default mode runs all phases plus the interpreter.

## Library API

The `twigc` crate also exposes a library:

```rust
use twigc::{twigc_check, twigc_emit_iir, twigc_run};
use std::path::Path;

// Type-check only
twigc_check(Path::new("main.tw"), &[]).unwrap();

// Emit IIR text
let iir = twigc_emit_iir(Path::new("main.tw"), &[]).unwrap();

// Compile and run
let result = twigc_run(Path::new("main.tw"), &[]).unwrap();
println!("→ {result}");
```

## In the stack

```
twigc (this crate)
├── twig-module-driver  — multi-file resolution + type-check
│   ├── twig-parser     — lexer, parser, AST
│   ├── twig-ir-compiler — IIR code generation
│   └── twig-type-checker — static type checking
└── twig-vm             — interpreter execution
```

## Changelog

See [CHANGELOG.md](CHANGELOG.md).
