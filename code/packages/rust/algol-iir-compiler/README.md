# algol-iir-compiler

Rust frontend for compiling a conservative ALGOL 60 scalar subset into the
shared LANG VM `interpreter_ir::IIRModule`.

This crate intentionally lives on the Rust LANG VM chain:

```text
ALGOL source -> algol-lexer/parser -> algol-iir-compiler -> IIRModule
  -> vm-core / jit-core / aot-core / iir-to-wasm / iir-to-jvm / iir-to-cil
  -> iir-to-beam / iir-to-llvm
```

The first slice supports scalar `integer`, `real`, and `boolean` programs with
assignments, integer arithmetic (`+`, `-`, `*`, `div`, `mod`), **real (f64)
arithmetic** (`+`, `-`, `*`, `/`), comparisons, `if`/`else`, compound
statements, labels, `goto`, `for i := a step k until b do ...`, **typed
procedures with `value` parameters**, and **switches** (computed goto):

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
before they are textually declared and may recurse. A procedure body sees its
own value parameters and, since **LANG-FULL E6**, may also **read and write a
scalar declared in an enclosing block**: such a shared scalar is materialised as
a typed module **global** (`global_load`/`global_store`) so the procedure and
the block share one cell — e.g. `integer procedure add(x); … add := counter :=
counter + x` over an enclosing `integer counter` runs across every backend.

`real` values lower to the IIR `f64` type and **run on the VM and JIT today**
(LANG-FULL AL1 / enabler E3 phase 1); `2.5 * 2.0`, `7.0 / 2.0`, and real
comparisons execute as IEEE-754 doubles. There is no implicit `integer`→`real`
coercion yet — mixing the two in one operator (or using `/` on integers) is a
clean type error. The five code-gen backends (LLVM/WASM/JVM/CLR/native) don't
execute f64 yet; see the `E3-*` follow-ups in
`code/specs/LANG-FULL-IMPLEMENTATION.md`.

One-dimensional **arrays** lower and **run on the VM and JIT today** (LANG-FULL
enabler E5 / AL2). `integer array A[1:10]` (and `real array`) becomes an
`alloc_array` sized at run time from the bounds (`upper - lower + 1`, so dynamic
bounds `A[lo:hi]` work); `A[i]` reads/writes become bounds-checked `array_get`/
`array_set` with the index translated to the IIR's 0-based form `i - lower`. An
out-of-range subscript traps at run time. The five code-gen backends don't lower
the array ops yet; see the `E5-*` follow-ups in
`code/specs/LANG-FULL-IMPLEMENTATION.md`.

Unsupported ALGOL 60 features — **multidimensional** and non-numeric arrays,
arrays as procedure parameters, strings, `own` variables, proper (void)
procedures, by-name (non-`value`) parameters, parameterless-procedure calls (a
bare name parses as a variable), enclosing-block **array** capture (only scalars
are globalised so far), and conditional/nested switch-list elements — return
explicit compiler
errors.
