# pipeline

**Layer 10 of the computing stack** — the orchestrator that ties the Rust
language-tooling packages into a single execution flow.

## What this package does

This crate runs source code through the VM path of the stack:

```text
Source -> Lexer -> Parser -> Compiler -> VM
```

Each stage is captured for inspection and visualization:

- `LexerStage` stores the token stream
- `ParserStage` stores the AST and a JSON-friendly AST view
- `CompilerStage` stores bytecode, constants, names, and instruction text
- `VMStage` stores execution traces, final variables, and captured output

The crate also exports pipeline runs as JSON in the shared report format used
by the Rust `html-renderer` package.

## Why the compiler and VM live here

The dedicated Rust `bytecode-compiler` and `virtual-machine` packages have not
been ported yet. Like the TypeScript package, this crate stays self-contained
for now by carrying the minimal compiler and VM implementations locally.

When those Rust packages exist, `pipeline` can switch to importing them.

## Usage

```rust
use pipeline::Pipeline;

let result = Pipeline::new().run("x = 1 + 2").unwrap();

assert_eq!(
    result.vm_stage.final_variables.get("x").unwrap().as_number(),
    Some(3.0)
);
```

## Current scope

This initial Rust port supports the VM execution path. The RISC-V and ARM
paths will follow once the Rust assembler and ISA simulators are available.
