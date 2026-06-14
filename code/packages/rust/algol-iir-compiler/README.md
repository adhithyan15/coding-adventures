# algol-iir-compiler

Rust frontend for compiling a conservative ALGOL 60 scalar subset into the
shared LANG VM `interpreter_ir::IIRModule`.

This crate intentionally lives on the Rust LANG VM chain:

```text
ALGOL source -> algol-lexer/parser -> algol-iir-compiler -> IIRModule
  -> vm-core / jit-core / aot-core / iir-to-wasm / iir-to-jvm / iir-to-cil
  -> iir-to-beam / iir-to-llvm
```

The first slice supports scalar `integer` and `boolean` programs with
assignments, integer arithmetic (`+`, `-`, `*`, `div`, `mod`), comparisons,
`if`/`else`, compound statements, labels, `goto`,
`for i := a step k until b do ...`, **typed procedures with `value`
parameters**, and **switches** (computed goto):

```algol
begin
  integer result;
  integer procedure sq(x); value x; integer x;
    sq := x * x;
  switch jump := first, second, third;
  integer i;
  i := 3;
  goto jump[i];               comment selects the 3rd label;
  first:  result := 1; goto done;
  second: result := 2; goto done;
  third:  result := sq(7);    comment result = 49;
  done:
end
```

A typed procedure lowers to a sibling `IIRFunction` (here `sq(x: i64) -> i64`)
and a call becomes an IIR `call`, so procedures run on every backend exactly
like any other function. A `switch` is a named jump table: `goto s[i]` selects
the i-th (1-based) target by a portable `index == k ? jmp Lk` chain; an
out-of-range subscript falls through. Conditional designators
(`goto if b then L1 else L2`) are also supported. Procedures may be called
before they are textually declared and may recurse; procedure bodies see their
own value parameters but not enclosing-block variables (lexically flat for now).

Unsupported ALGOL 60 features, including arrays, strings, reals, `own`
variables, proper (void) procedures, by-name (non-`value`) parameters,
non-local access from a procedure body, and conditional/nested switch-list
elements, return explicit compiler errors.
