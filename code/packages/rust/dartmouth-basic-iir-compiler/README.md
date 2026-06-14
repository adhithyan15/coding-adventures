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
arrays, READ/DATA, and DEF are deferred to V2.  See
[CHANGELOG.md](CHANGELOG.md) for the full table.

## Spec

[`code/specs/PL05-dartmouth-basic-iir-compiler.md`](../../../specs/PL05-dartmouth-basic-iir-compiler.md).
