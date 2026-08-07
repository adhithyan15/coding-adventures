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

BA7 scalar BASIC now follows Dartmouth's real numeric model: numeric literals
(including integer-spelled `42`), scalar variables, arithmetic, `DEF FN`,
`IF`, `FOR`, `PRINT`, `DATA`, and arrays lower as `f64`. Whole-valued real
output (`PRINT 6.0 * 7.0`) and ordinary fixed-decimal fractional output (`3.14`,
`.25`, `-2.5`), six-significant-digit rounding, and `E` notation run through the
shared E3/E8 backends. Integer `i64` remains explicit at structural boundaries:
line numbers, `DIM` bounds, array subscripts, DATA read pointers, and GOSUB
return stacks. Integer-valued literal exponentiation (`6 ^ 2`) lowers to
repeated `f64` multiplication and runs on all seven backends; variable,
fractional, negative, nested, and large exponents still need a runtime math
helper. String literal `PRINT`, literal-backed string variables
(`LET A$ = "HI"` / `PRINT A$`), literal reassignment
(`LET A$ = "NO"; LET A$ = "OK"; PRINT A$`), and `IF A$ = "Y"` / `IF A$ <> "Y"`
string branches now lower through the shared E4 path on all seven matrix
backends. Lexical string ordering in `IF` (`A$ < "B"` / `"B" > A$`) lowers
through E4 `str_cmp` plus typed zero comparisons on the same targets. Literal
`+` concatenation (`LET A$ = "O" + "K"; PRINT A$`) uses E4
`str_concat` on the same backends, and literal-backed scalar string copies
(`LET A$ = "OK"; LET B$ = A$; PRINT B$`) reuse that E4 path with an empty
suffix. Copied slots can participate in control flow too (`IF B$ = A$ THEN ...`).
Multi-item string printing (`PRINT A$; B$`) emits ordered `print_str` calls
without numeric formatting helpers, and comma-separated string printing
(`PRINT A$, B$`) reuses BA2's single-space separator between those E4 calls.
`PRINT A$ + "K"` also consumes a temporary E4 `str_concat` result directly, and
`PRINT A$ + B$` proves both concat operands can be scalar string slots in that
direct-print path. `IF A$ + "K" = "OK" THEN ...` feeds the same expression path
into `str_eq` line-control branching, `IF A$ + B$ = "OK"` proves
variable-variable concat on the equality branch path, and
`IF A$ + B$ <> "NO"` proves the sibling inequality branch path. `LET B$ = A$ + "K"; PRINT B$` stores the variable-backed
concat directly into another scalar string slot, and
`LET B$ = A$ + "B" + "C"; PRINT B$` proves chained left-associative concat
through repeated E4 `str_concat`. `INPUT A$` (E4-dyn) reads a whole line from
the host as a **runtime string** via `call_builtin "input_str"` — the compiler
cannot fold it, so `PRINT A$` prints whatever stdin supplied; it is proven on
**all seven backends** (native/LLVM/WASM/JVM/CLR/VM/JIT). **String arrays**
(E4d-BA-arr) lower `DIM A$(n)` to an `array<str>` — the same E5 aggregate as a
numeric array but carrying an E4-dyn string handle per element — so `A$(i) = s`
is a `str`-typed `array_set` and `A$(i)` reads a `str`-typed `array_get` that
feeds PRINT / `+` concat; part 1 runs on [Llvm, Wasm, Vm, Jit] with
NativeAot/JVM/CLR (native element-size + managed reference arrays) to follow.
Richer dynamic string expressions and string `READ`/`DATA` remain follow-ups.
See [CHANGELOG.md](CHANGELOG.md) for the full table.

`LET`, `PRINT`, `IF … THEN <line>`, `GOTO`, `FOR`/`NEXT`, `DEF FN`
single-line user functions, `DIM` arrays, `READ`/`DATA`/`RESTORE`, and
`GOSUB`/`RETURN` lower to the shared IIR and
RUN on native / LLVM / WASM / JVM / CLR / VM / JIT.  **BA1** added unstructured
`GOSUB`/`RETURN`: a return-address `array<i64>` stack + an AL5 computed-`goto`
inside `main` (no new backend op), so the same `RETURN` resumes at the
dynamically most-recent `GOSUB`; after the `iir-to-wasm` 0.18.0 dispatch-loop
fix, the two BA1 proof programs run on all 7 backends.  LANG-FULL BA0 fixed the
comparison operand-width hint that had broken control flow on LLVM/WASM; BA5
added `DEF FNx(X) = expr` (lowered to a sibling `IIRFunction` + `call`, like
ALGOL's value procedures); **BA3/BA7** now provide one-dimensional real arrays —
`DIM A(n)` → `alloc_array` with `array<f64>` elements (0-based and inclusive, so
`n + 1` elements), `LET A(i) = e` → `array_set`, and `A(i)` in an expression →
`array_get`, with subscripts explicitly truncated through E8; **BA6/BA7** added
`READ`/`DATA`/`RESTORE` on top of that array substrate — a pre-pass gathers all
finite `DATA` literals into a pool materialised at the top of `main` as an
`array<f64>` plus an `i64` read-pointer register, `READ` does `array_get pool,
ptr` + `ptr := ptr + 1`, and `RESTORE` resets the pointer.  A `DEF` body may reference only its own
parameter — global access from inside a function needs enabler E6 (see
`code/specs/LANG-FULL-IMPLEMENTATION.md`).  **BA2** made `PRINT` print several
items on one line: each numeric item lowers to a `call __basic_print_int` — a
synthetic recursive helper that renders digits one at a time via the universal
`putchar` builtin — so `;` joins items tightly (`PRINT 4; 2` ⇒ `42`), `,`
inserts a space (`PRINT 4, 2` ⇒ `4 2`), and a trailing separator suppresses the
line-ending newline.  Because the helpers reuse only ops every backend already
runs, BA2 needed **zero** backend changes.  String literal/scalar string
printing, literal concat, equality/inequality/order branches, and literal
reassignment now use E4 on all seven backends.
(`,` is a single space rather than a true 14-column print zone — a documented,
deferred approximation.)

**BA7-1/2/3 real values and historical real `PRINT`** makes BASIC values `f64`
even when the source spells them as integers. `PRINT` chooses
`__basic_print_real` for numeric items; that helper truncates whole-valued reals
via E8 `real_to_int_trunc`, delegates integer parts to the BA2 digit printer,
and emits rounded six-significant-digit decimal or `E` notation output. `DIM`/
array elements and `DATA`/`READ` now use `array<f64>` storage while indices and
read pointers remain `i64`. The matrix proofs for `PRINT 42`, `PRINT 6.0 * 7.0`
⇒ `42`, `3.14`/`.25`/`-2.5` fractional output, `1.23457E+08` / `1.23457E-04`
formatter output, and fractional `DATA` through an array + scalar `READ` run on
native / LLVM / WASM / JVM / CLR / VM / JIT without adding backend ops.

## Spec

[`code/specs/PL05-dartmouth-basic-iir-compiler.md`](../../../specs/PL05-dartmouth-basic-iir-compiler.md).
