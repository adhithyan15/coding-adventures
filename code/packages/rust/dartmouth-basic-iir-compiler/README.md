# `dartmouth-basic-iir-compiler`

Compiles 1964 Dartmouth BASIC source to
[`interpreter_ir::IIRModule`](../interpreter-ir) so the LANG VM AOT
chain can produce native Linux / Windows / macOS executables from
`.bas` files.

## Why a separate crate?

`dartmouth-basic-ir-compiler` (the existing crate) targets the GE-225
simulator's custom `compiler_ir::IrProgram` IR — meaningful on the
historical hardware but not pluggable into the LANG VM AOT chain.
PL05 introduces this *new* crate that emits `IIRModule` directly so
BASIC programs get the same Linux / Windows / macOS native pipeline
Twig and Nib enjoy.

Both crates can coexist: pick the historical GE-225 path with
`dartmouth-basic-ir-compiler`, or the LANG VM AOT path with
`dartmouth-basic-iir-compiler` (this crate).

## Usage

```rust
use dartmouth_basic_iir_compiler::compile_source;

let module = compile_source(
    "10 PRINT 42\n20 END\n",
    "hello",
).expect("compile ok");
assert_eq!(module.functions[0].name, "main");
```

Pipe the resulting `IIRModule` into
`twig_aot::compile_module_to_{linux,windows,macos}_executable` (or
just use `lang-aot foo.bas`, which does the routing for you).

## V1 scope

Integer-only programs.  Floats truncate to i64; strings, GOSUB,
arrays, and READ/DATA are deferred to V2.  See
[CHANGELOG.md](CHANGELOG.md) for the full table.

`LET`, `PRINT`, `IF … THEN <line>`, `GOTO`, `FOR`/`NEXT`, and `DEF FN`
single-line user functions lower to the shared IIR and RUN on
native / LLVM / WASM / JVM / CLR / VM / JIT.  LANG-FULL BA0 fixed the comparison
operand-width hint that had broken control flow on LLVM/WASM; BA5 added
`DEF FNx(X) = expr` (lowered to a sibling `IIRFunction` + `call`, like ALGOL's
value procedures).  A `DEF` body may reference only its own parameter — global
access from inside a function needs enabler E6 (see
`code/specs/LANG-FULL-IMPLEMENTATION.md`).

## Spec

[`code/specs/PL05-dartmouth-basic-iir-compiler.md`](../../../specs/PL05-dartmouth-basic-iir-compiler.md).
