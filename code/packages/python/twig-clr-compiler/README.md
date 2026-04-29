# twig-clr-compiler

End-to-end Twig source → PE/CLI assembly (a real `.exe`) → real
`dotnet` execution.  The CLR-side counterpart to
[`twig-jvm-compiler`](../twig-jvm-compiler/) and
[`twig-beam-compiler`](../twig-beam-compiler/) — **completing the
Twig real-runtime trilogy across JVM, CLR, and BEAM.**

## Pipeline

```
Twig source
    ↓ parse + extract       (twig package)
typed AST
    ↓ Compiler.compile()    (this module — Twig → compiler-IR)
IrProgram
    ↓ ir-optimizer          (constant folding, DCE)
IrProgram (optimised)
    ↓ lower_ir_to_cil_bytecode  (ir-to-cil-bytecode)
CILProgramArtifact
    ↓ write_cli_assembly        (cli-assembly-writer, CLR01-conformant)
.exe bytes
    ↓ dotnet <name>.exe         (real .NET runtime, net9.0)
program output
```

## V1 surface

The first iteration is intentionally narrow — exactly what
`ir-to-cil-bytecode` supports plus the ``main()`` boilerplate
the CLR runtime expects:

- Integer literals (positive integers in v1)
- Binary arithmetic: `+`, `-`, `*`, `/`
- Single-binding `let`
- `begin` for sequencing

The value of the program's last top-level expression becomes the
process exit code (matching how a `static int Main()` C# program
returns to the shell).

## Quick start

```python
from twig_clr_compiler import run_source

result = run_source("(* 6 7)", assembly_name="Answer42")
print(result.returncode)   # 42 — value flows through Main's return
```

## Out of scope for v1

- **Top-level `define`**: requires multi-method lowering on top
  of `ir-to-cil-bytecode`'s entry-point assumption.  v2.
- **Recursion**: requires the `CALL` IR-op to plumb through
  `ir-to-cil-bytecode` end-to-end with a real-`dotnet`-loadable
  entry shape.  v2.
- **`if` outside trivial cases**: needs branching in the lowering
  + verifier-friendly stack discipline.  v2.
- **Closures, cons, lists, symbols**: cross-backend heap work
  (TW02.5 / TW03 spec coordination).
- **`SYSCALL` / `Console.WriteLine`**: process exit code is the
  v1 result channel; explicit I/O is v2.

## Sister packages

- [`twig-jvm-compiler`](../twig-jvm-compiler/) — JVM target.
- [`twig-beam-compiler`](../twig-beam-compiler/) — BEAM target.
- [`twig`](../twig/) — frontend + `vm-core` interpreter.
- [`cli-assembly-writer`](../cli-assembly-writer/) — emits
  CLR01-conformant `.exe` bytes.
- [`ir-to-cil-bytecode`](../ir-to-cil-bytecode/) — IR → CIL
  method bodies.

## Why this matters

Per the Twig vision: "compile Twig to JVM, CLR and BEAM VM
bytecode such that those engines can also run them."  This
package is the third leg of that trilogy.  The shape of the
public API (`compile_to_ir`, `compile_source`, `run_source`,
`dotnet_available`) deliberately mirrors `twig-jvm-compiler` and
`twig-beam-compiler` so all three real-runtime targets feel
identical to drive.
