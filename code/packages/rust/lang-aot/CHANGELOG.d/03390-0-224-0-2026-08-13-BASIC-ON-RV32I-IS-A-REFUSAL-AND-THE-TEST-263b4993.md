## 0.224.0 - 2026-08-13 - BASIC on RV32I is a refusal, and the test now says so

`end_to_end_smoke.rs`'s `end_to_end_basic_print_emits_riscv32_bin_via_lang_aot`
was red on `origin/main`:

```
unexpected BASIC -> RV32I error: riscv32: riscv-backend: unsupported RV32I scalar type "f64"
```

Two things met here, and only one of them was a bug.

**The f64 is correct.** Dartmouth BASIC has exactly one numeric type and it is
REAL. After the BA7 floating-point conversion
(`code/specs/lang-full-ba7-floating-point.md`) even `10 PRINT 42` carries the
double `42.0`, not the integer 42 — `main` emits `const_f64 42.0` and calls
`__basic_print_real(x : f64)`. The sibling LLVM test in this same file has
pinned that shape all along (`call i64 @__basic_print_real(double ...)`).
Nothing is over-widening an integer.

**RV32I genuinely cannot carry it.** RV32I is the RISC-V base *integer* ISA:
32 integer registers, no `f0`..`f31` bank, no `fadd.d`. Floating point lives
in the optional `F`/`D` extensions (RV32F/RV32D). So the backend's refusal was
the right answer.

The bug was the expectation. The test asserted a `.bin` and only tolerated
failure by substring-matching a hand-maintained list of "lowering gap"
messages — a list that named `UnsupportedOp`/`UnsupportedType` but not the
Display text a type refusal actually prints. A message reword flips such a
test between "skipped" and "failed" without anything real changing, and while
it was silently skipping it proved nothing at all.

- Replaced it with `basic_print_refuses_riscv32_because_basic_numbers_are_f64`,
  which pins the refusal itself: it names the target, the function (`main`),
  the CIR op (`const_f64`), the type (`f64`), explains that RV32I has no
  floating-point registers, names RV32F/RV32D as the extensions that would
  carry it, and asserts no partial `.bin` is left behind. If a soft-float pass
  ever lands, this test fails loudly and gets rewritten into the byte-pinning
  test it used to pretend to be.
- `compile_file_to_riscv32_bin` now names the offending function in the error.
  Its doc comment always promised "the message names the function and op", but
  the mapping threw the name away — and a BASIC module is `main` plus the whole
  injected `__basic_print_*` runtime, so an unqualified message left the reader
  guessing. Needs `riscv-backend` 0.2.0, which supplies the op and the reason.

Not done, deliberately: nothing truncates `42.0` to an integer to make the
test emit bytes. A loud refusal beats a silent wrong answer.

