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
procedures with `value` parameters**, **switches** (computed goto), and
literal string output:

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

Since **LANG-FULL AL6** a variable may be declared **`own`** (`own integer n`),
giving it *static lifetime*: it is allocated once and retains its value across
every call of the enclosing block/procedure (ALGOL 60 §5.2.5). It reuses the
same module-global storage — keyed by a per-procedure-unique slot so two
procedures' `own n` stay independent — and is **not** re-zeroed on entry, so it
accumulates: `own integer n; n := n + d` called three times with `d = 1` yields
`1`, `2`, `3`. Runs on all 7 backends.

Since **LANG-FULL AL8** the **standard function `abs`** (ALGOL 60 §3.2.4) is
built in: `abs(E)` is the absolute value of `E`, keeping its type
(`integer`→`integer`, `real`→`real`). It is resolved by name — a program may
still redeclare its own `procedure abs`, which then wins — and lowers inline to
`if E < 0 then -E else E` (a compare against zero, then a negated or
pass-through move into one result slot), so it **runs on all 7 backends**;
`abs(0 - 42)` ⇒ `42`. **`sign`** is the second (algol-iir-compiler 0.9.0):
`sign(E)` is `+1`/`-1`/`0` for a positive/negative/zero operand and, unlike
`abs`, always yields an **`integer`** (`sign(-2.5)` is the integer `-1`). It
lowers the same way — `if E > 0 then 1 else if E < 0 then -1 else 0` — and
also runs on every backend; `43 + sign(0 - 1)` ⇒ `42`.

`entier(E)` (ALGOL 60 §3.2.5) is the largest **integer** not greater than the
**real** `E` — floor, rounding toward −∞: `entier(2.7)` ⇒ `2`, `entier(-2.7)` ⇒
`-3` (not `-2`). It lowers to a single E8 `real_to_int_floor` IIR op (the floor
and the real→integer narrowing fused into the primitive), so every backend emits
its native floor-then-convert. A `real` argument is required (it is specifically
the real→integer floor). The remaining standard functions (`sqrt`/`sin`/`cos`/…)
are not implemented yet.

Since **LANG-FULL AL4** the implementation-defined output procedures `print`
and `output` are recognised in statement position when they are not user
declared procedures. String literal actuals lower to shared E4 `str_const` +
`print_str`, so `begin print('HI') end` writes `HI` on all seven LANG backends.
Literal-backed scalar variables now use the same shape: `string s; s := 'HI'`
materialises `s` with `str_const`, and `print(s)` consumes that direct slot.
Literal-backed scalar copies now reuse E4 `str_concat` with an empty suffix:
`string s, t; s := 'OK'; t := s; print(t)` writes `OK`, and `s := 'NO'`
after the copy does not change the copied `t` slot. Multi-argument output over
literal-backed scalar string variables also stays on the same path:
`string s, t; s := 'O'; t := 'K'; output(s, t)` emits ordered `print_str`
calls and writes `OK`. Literal-backed scalar string predicates now lower through
the shared E4 comparison ops too: `s = 'OK'` / `s != 'NO'` use `str_eq` plus a
typed zero comparison, while `s < 'BETA'` / `'BETA' > s` use `str_cmp` plus the
corresponding typed zero comparison before the normal ALGOL conditional branch.
Initialized scalar locals now also carry runtime string procedure results:
`string s; s := pick(1); if s < 'LO' then ...; print(s)` uses the shared
runtime `str_concat`/`str_cmp`/`print_str` path. Reads before assignment still
fail closed. `string array A[1:2]` uses the same `array<str>` substrate as
other LANG frontends; its elements can be written from literals or initialized
scalar strings, then read for lexical comparison or output. Captured/`own`
strings remain unsupported.

`real` values lower to the IIR `f64` type and run across the established LANG
backends. `2.5 * 2.0`, `7.0 / 2.0`, and real comparisons execute as IEEE-754
doubles. When a real is required, an `integer` operand widens through the
shared `int_to_real` IIR conversion: mixed numeric arithmetic and comparisons,
`/`, real assignments/array elements/formals, and the real standard functions
all accept integer inputs. `div` and `mod` remain integer-only.

One-dimensional **arrays** lower and run on **all seven standard backends**
(LANG-FULL enabler E5 / AL2). `integer array A[1:10]` (and `real array`) becomes an
`alloc_array` sized at run time from the bounds (`upper - lower + 1`, so dynamic
bounds `A[lo:hi]` work); `A[i]` reads/writes become bounds-checked `array_get`/
`array_set` with the index translated to the IIR's 0-based form `i - lower`. An
out-of-range subscript traps at run time. Procedures can capture an enclosing
array: its handle and declared lower-bound/stride metadata are stored in typed
module globals, so a captured subscript keeps the declaration's index space.
`own integer array A[lo:hi]` has static lifetime too: its bounds and backing
storage are initialized on the first procedure call and persist across later
calls.

Scalar `string` captures use typed module globals, so a procedure can write
and read a string declared by its enclosing block. `own string` initializes to
the empty string on its first procedure call and retains subsequent assignments
across calls. A captured string still requires assignment before its first
read, just like a local string.

Proper procedures now lower as side-effecting IIR `void` functions when called
in statement position. They can write enclosing scalar or array globals and use
the same literal-backed output path as typed procedures; using a proper procedure in
value position is a clean type error because it has no return value.

Unsupported ALGOL 60 features — non-numeric/non-string arrays, arrays as procedure
parameters, dynamic string variables, by-name (non-`value`) parameters,
parameterless-procedure calls (a bare name parses as a variable), and
conditional/nested switch-list elements — return explicit compiler errors.
