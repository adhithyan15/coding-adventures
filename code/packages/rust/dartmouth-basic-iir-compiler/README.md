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

Integer-only programs.  Floats truncate to i64 (BA7 makes the value model `f64`);
strings are deferred to LANG77.  See [CHANGELOG.md](CHANGELOG.md) for the full table.

`LET`, `PRINT`, `IF … THEN <line>`, `GOTO`, `FOR`/`NEXT`, `DEF FN`
single-line user functions, `DIM` arrays, `READ`/`DATA`/`RESTORE`, and
`GOSUB`/`RETURN` lower to the shared IIR and
RUN on native / LLVM / WASM / JVM / CLR / VM / JIT.  **BA1** added unstructured
`GOSUB`/`RETURN`: a return-address `array<i64>` stack + an AL5 computed-`goto`
inside `main` (no new backend op), so the same `RETURN` resumes at the
dynamically most-recent `GOSUB` — runs on six backends (the wasm computed-`goto`
CFG is a tracked iir-to-wasm follow-up, BA1-WASM).  LANG-FULL BA0 fixed the
comparison operand-width hint that had broken control flow on LLVM/WASM; BA5
added `DEF FNx(X) = expr` (lowered to a sibling `IIRFunction` + `call`, like
ALGOL's value procedures); **BA3** added one-dimensional integer arrays —
`DIM A(n)` → `alloc_array` (0-based and inclusive, so `n + 1` elements),
`LET A(i) = e` → `array_set`, and `A(i)` in an expression → `array_get`, the
same shared array ops ALGOL's E5 arrays run on every backend; **BA6** added
`READ`/`DATA`/`RESTORE` on top of that array substrate — a pre-pass gathers all
`DATA` integers into a pool materialised at the top of `main` as an `array<i64>`
plus a read-pointer register, `READ` does `array_get pool, ptr` + `ptr := ptr +
1`, and `RESTORE` resets the pointer.  A `DEF` body may reference only its own
parameter — global access from inside a function needs enabler E6 (see
`code/specs/LANG-FULL-IMPLEMENTATION.md`).  **BA2** made `PRINT` print several
items on one line: each numeric item lowers to a `call __basic_print_int` — a
synthetic recursive helper that renders digits one at a time via the universal
`putchar` builtin — so `;` joins items tightly (`PRINT 4; 2` ⇒ `42`), `,`
inserts a space (`PRINT 4, 2` ⇒ `4 2`), and a trailing separator suppresses the
line-ending newline.  Because the helpers reuse only ops every backend already
runs, BA2 needed **zero** backend changes.  (String `PRINT` items still wait for
LANG77 / E4; `,` is a single space rather than a true 14-column print zone — a
documented, deferred approximation.)

## Spec

[`code/specs/PL05-dartmouth-basic-iir-compiler.md`](../../../specs/PL05-dartmouth-basic-iir-compiler.md).
