# twig-clr-compiler

Twig → CLR (CIL bytecode + PE/CLI assembly) compiler.

This is the first non-Python target for Twig.  Source compiles to a
real `.dll`-shaped PE assembly that runs on
[`clr-vm-simulator`](../clr-vm-simulator) (and, in principle, on
`mono` / `dotnet` since the assembly format is standard ECMA-335).

See [TW02 spec](../../../specs/TW02-twig-clr-compiler.md) for the
v1 surface and the multi-spec roadmap (TW03: BEAM, TW04: JVM).

## Architecture

```
Twig source
   ↓  parse_twig + extract_program        (twig package)
typed AST
   ↓  twig_clr_compiler.compile_to_ir     (this package)
IrProgram
   ↓  ir-optimizer
IrProgram (optimised)
   ↓  lower_ir_to_cil_bytecode            (ir-to-cil-bytecode)
CIL bytecode
   ↓  write_cli_assembly                  (cli-assembly-writer)
PE/CLI assembly bytes
   ↓  run_clr_entry_point                 (clr-vm-simulator)
program return value
```

## V1 surface (this commit)

- Integer literals, booleans
- Arithmetic: `+`, `-`, `*`, `/`
- Comparison: `=`, `<`, `>`
- Control flow: `if`, `let`, `begin`
- Top-level expression returns its value

## NOT in v1 (deferred to follow-on commits / future PRs)

- `define` (top-level value or function bindings)
- `lambda` and closures
- `cons` / `car` / `cdr` / symbols / `nil`
- `print` and other I/O
- Tail-call optimisation

## Quick start

```python
from twig_clr_compiler import compile_source, run_source

# Compile to a PE assembly:
result = compile_source("(+ 1 2)")
print(len(result.assembly_bytes), "bytes")

# Compile and run:
exec_result = run_source("(if (= 1 1) 100 200)")
print(exec_result.vm_result.return_value)  # 100
```
