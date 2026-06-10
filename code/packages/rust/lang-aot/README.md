# lang-aot

Multi-language AOT driver — compile **Twig, Nib, Brainfuck, Dartmouth
BASIC, Oct, and McCarthy Lisp** to native executables through the shared
LANG VM chain.  (McCarthy Lisp is wired as of L3a — scalar programs run
end-to-end natively; symbol/cons backend support is L3b.)  As of L3b-3a-3c,
`compile_source_to_wasm` also compiles McCarthy **cons** programs to a runnable
WasmGC module — `(CAR (CONS 7 9))` → `7` on the in-repo `wasm-runtime` (integer
atoms boxed as `i31ref`, the cons cell a `$LispyPair` struct) — and the McCarthy
predicates, `COND`, and symbols: `pair?`/`ATOM` (`(ATOM 5)` → 1; L3b-3a-4b),
`EQ` atom equality (`(EQ 5 5)` → 1; L3b-3a-4c), `COND` with lisp-truthiness
(`(COND (0 7) (5 9))` → 7 — `0` is truthy; L3b-3a-4d), and symbols
(`(EQ 'A 'A)` → T, `(EQ 'A 'B)` → nil; W1), and `LAMBDA`/`LABEL`/recursion
(`((LAMBDA (X) X) 5)` → 5; a recursive `LABEL` walks a list; W2). The **full
McCarthy core** — cons, `ATOM`, `EQ`, `COND`, symbols, and lambda/label/recursion
(F1–F7) — now runs on the wasm backend.

`compile_source_to_jvm` is the second managed target: scalar McCarthy runs on the
in-repo `jvm-simulator` (`42` → 42; W3a), **cons** runs on a real JVM —
`(CAR (CONS 7 9))` → 7 (W3b) — and now `ATOM`/`EQ`/`COND` too: `(ATOM 5)` → 1,
`(EQ 5 5)` → 1, `(COND ((EQ 1 1) 7) (5 9))` → 7 (W4). It runs the *same* structural
passes as the wasm path; the JVM backend lowers the backend-agnostic
`box`/`unbox`/`alloc`/`field_*` + `pair?`/`not`/`equal?` to `Integer`/`Object[]` +
`instanceof`/`ixor`/`if_icmpeq` (where wasm uses `i31ref`/`$LispyPair`/`ref.test`).
**Symbols** run too (W5a): `(EQ 'X 'X)` → 1 — their interned ids (`2²⁹`) load via
the JVM `ldc` constant-pool path. And `LAMBDA`/`LABEL`/recursion (W5b):
`((LAMBDA (X) X) 5)` → 5, a recursive `LABEL` → 99. **The JVM backend is now
McCarthy-complete (F1–F7)** — the second managed backend after WASM.

`compile_source_to_cil_artifact` is the third managed target: scalar McCarthy
emits CIL that **runs** on the in-repo `clr-simulator` (`42` → 42; W6a), and
**cons** too — `(CAR (CONS 7 9))` → 7 (W6b, after the simulator gained an
object/reference value model). It runs the *same* structural passes as the
wasm/JVM paths; the CLR backend lowers the backend-agnostic
`box`/`unbox`/`alloc`/`field_*` to `box [int32]`/`unbox.any` + `object[]` cells.
McCarthy's predicates run too (W7, F3–F5): `(ATOM 7)`→1, `(EQ 7 7)`→1,
`(COND …)` — `pair?`→`isinst object[]`, `not`→`x^1`, `equal?`→`unbox; unbox; ceq`.
The remaining CLR symbols + lambda (F6–F7) are W8.

`compile_source_to_beam` is the fourth managed target and the first on the
**Erlang VM** (W9a): scalar McCarthy emits a `.beam` that **runs** on a real
`erl` (OTP), `42` → 42. BEAM uses the native Erlang-terms value model
(integers/atoms/list cells), so its cons/symbol/lambda lowering (W9+) is its own.

## Stack position

```
<lang> source
   │
   ▼ <lang>-iir-compiler                  (per-language frontend)
interpreter_ir::IIRModule                  ← lingua franca
   │
   ▼ twig_aot::compile_module_to_*_executable
       (x86_64-backend / aarch64-backend  →  elf/pe/macho_object  →
        system linker)
   │
   ▼
native executable
```

The point of this crate is the **dispatch layer** at the top: pick the
right frontend based on the input file's extension or an explicit
`--lang` flag, then hand the resulting `IIRModule` to twig-aot for the
rest of the chain.

Besides native executables, `lang-aot` also exposes text/bytecode emit
pipelines — `compile_file_to_llvm_ir`, `compile_file_to_riscv32_bin`, …, and
(LANG77 / McCarthy L3b-3a) **`compile_source_to_wasm` / `compile_file_to_wasm`**.
The wasm path currently handles **scalar** programs (a polymorphic lisp
`"any"` value is concretised to `i64` for functions with no heap ops); a
scalar McCarthy `42` emits a `.wasm` whose `main` returns `i64 42`, verified by
running it on the in-repo `wasm-runtime`. Cons/symbol programs (the
boxed-`anyref` WasmGC value model) are a follow-up.

## CLI

```text
lang-aot <FILE> [-o <OUT>] [--lang <LANG>]
```

| Language | Extensions | Frontend crate | Status |
|---|---|---|---|
| Twig            | `.twig`         | `twig-ir-compiler`       | full |
| Nib             | `.nib`          | `nib-iir-compiler`       | full |
| Brainfuck       | `.bf`, `.b`    | `brainfuck-iir-compiler` + BF07 lowering pass | full — `lang-aot foo.bf` compiles end-to-end (cells live in a 30000-byte `alloc_bytes` tape; `load_mem`/`store_mem` are rewritten to `load_byte`/`store_byte` per LANG76) |
| Dartmouth BASIC | `.bas`, `.basic` | `dartmouth-basic-iir-compiler` | full — integer programs with LET / PRINT / INPUT / IF / GOTO / FOR / NEXT / END / REM compile end-to-end (PL05).  GOSUB / arrays / strings / DEF deferred to V2 |
| Oct             | `.oct`          | `oct-iir-compiler` (OCT02 phases 1–4) | full — integer subset compiles end-to-end (`fn`/`let`/`if`/`while`/`loop`/`break`, recursion).  8008 hardware intrinsics (`in`, `out`, `adc`, `sbb`, `rlc`, `rrc`, `ral`, `rar`, `carry`, `parity`) rejected cleanly with a pointer to the dedicated Intel-8008 simulator backend |
| McCarthy Lisp   | `.mcl`, `.lisp` | `mccarthy-lisp-iir-compiler` | **L3a** — the full Lisp 1.0 frontend (literals, `QUOTE`, `CONS`/`CAR`/`CDR`/`ATOM`/`EQ`, `COND`, `LAMBDA`/`LABEL` closures) produces an `IIRModule`, and **scalar** programs run end-to-end on the native AOT pipeline (`echo 42 > p.mcl; lang-aot p.mcl` → exits 42).  Symbol/cons-returning programs (e.g. `(CAR '(A B C))`) are accepted by the frontend but the native backend `BackendRefused`s them until the `lispy-runtime` value model is lowered into each backend (**L3b**) |

If `--lang` is omitted the language is inferred from the file
extension; unknown extensions get a "could not infer language" error
listing the recognised ones.

## Example

```bash
$ echo 'fn main() -> u8 { return 42; }' > hello.nib
$ lang-aot hello.nib
$ ./hello
$ echo $?
42
```

That ran a Nib source file all the way to a native executable on the
host — via the same `x86_64-backend` and `elf_object` / `pe_object` /
`macho_object` packagers Twig uses.

## Adding a new language

1. Build a `<lang>-iir-compiler` crate whose `compile_source(&str, &str)
   -> Result<interpreter_ir::IIRModule, _>` mirrors `nib-iir-compiler`
   or `brainfuck-iir-compiler`.
2. Add a variant to the [`Language`] enum and wire
   `compile_source_to_iir` to call your new frontend.
3. Add the file extension to `detect_language_from_path`.
4. Add a smoke test in `tests/end_to_end_smoke.rs` with a small program
   that compiles to a known exit code.

No backend changes needed — every frontend gets x86-64 Linux, x86-64
Windows, and ARM64 macOS for free.

## Limitations

- **Host-targets-host only.** Same V1 policy as `twig-aot`.  Use the
  `twig-aot --target= --emit-object` workflow for cross-OS object
  emission.
- **BF backend gap.** `brainfuck-iir-compiler` emits IR ops
  (`load_mem`, `store_mem`, `putchar`, `getchar`, …) that the x86_64
  and aarch64 AOT backends don't lower today.  The dispatch layer here
  correctly produces the IIR, but compilation to executable fails at
  the backend step.  Extending the backends to support these ops is a
  separate follow-up.
- **No `--target` / `--emit-object` flags yet.** `lang-aot`'s CLI is
  intentionally minimal in V1.  Cross-OS support will land alongside
  multi-language `--emit-object` once we have a story for cross-host
  runtime archives.
