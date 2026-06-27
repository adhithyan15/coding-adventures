//! # Cross-language platform matrix — LANG-PLATFORM-MATRIX (LM0 foundation).
//!
//! The generalization of the McCarthy W16 capstone (`conformance.rs`) from one
//! reference language to **every language frontend in the repo**. Each language has
//! a small battery of programs with a known result; each backend is a runner gated
//! on its toolchain; every `(program, backend)` cell is asserted **by running**.
//!
//! The harness is a `Backend`-keyed grid: each `Prog` lists the backends a slice has
//! **proven** run it, and `matrix_every_proven_cell_agrees` runs every such cell on
//! its real toolchain and asserts the known result (skipping a cell whose tool is
//! absent, failing loudly when present-but-wrong). Columns so far:
//!
//! * **native-AOT** (LM0) — source → shared IIR → host object → system linker → run.
//!   Uniformly green: all six languages.
//! * **LLVM** (Phase L) — source → textual `.ll` (`iir-to-llvm`) → real `clang` → run.
//!   Green for Twig / Nib / Oct / ALGOL 60 (exit code) and Dartmouth BASIC (stdout —
//!   the `.ll`'s `@__print_i64` is satisfied by a generic print runtime). Brainfuck
//!   deferred (the i64-slot-model mismatch — see the spec's Deferred section).
//! * **WASM** (Phase W) — source → wasm bytes (`iir-to-wasm`) → the in-process
//!   `wasm-runtime`. Green for the expression languages Twig / Nib / Oct / ALGOL 60
//!   (exit code from `main`'s wasm result) and Dartmouth BASIC (stdout — a `PrintHost`
//!   resolves the `env.__print_i64` import and captures the printed value). Brainfuck
//!   pends the tape ops — its own follow-up.
//! * **JVM** (Phase J) — source → `JvmClassFile` (`iir-to-jvm-class-file`) → real
//!   `java`. The W16 wrapper-launcher pattern: a `main([Ljava/lang/String;)V` launcher
//!   invokes the entry method and either `System.out.println`s its result (the
//!   expression languages Twig / Nib / Oct / ALGOL 60, parsed back) or discards it
//!   while `env.BasicRuntime` — compiled with `javac` onto the classpath — handles the
//!   output (Dartmouth BASIC, whose `print_i64` lowers to `env/BasicRuntime.println`).
//!   Green for all five non-Brainfuck languages; Brainfuck pends the tape ops.
//! * **CLR** (Phase C) — source → textual `.il` (`iir-to-cil-bytecode`) → real `ilasm`
//!   → real `dotnet`, the CLR-real path. An expression program's entry
//!   `Console.WriteLine`s its `int` result (parsed); Dartmouth BASIC's `PRINT` lowers
//!   to `Console.WriteLine(int32)` and the launcher discards (not re-prints) the entry
//!   result, so the harness captures `Console`. Green for Twig / Nib / Oct / ALGOL 60
//!   (this needed the CIL backend to grow integer arithmetic + the comparison opcodes)
//!   and Dartmouth BASIC; Brainfuck pends the tape ops.
//!
//! The remaining work is the Deferred items — Brainfuck-on-LLVM/WASM/JVM/CLR, and the
//! McCarthy-specialized VM and JIT (op-coverage work). See
//! `code/specs/LANG-PLATFORM-MATRIX.md`.
//!
//! ## Two result kinds
//!
//! * **Expression languages** (Twig, Nib, Oct, ALGOL 60) return an integer — the
//!   process **exit code** (`& 0xFF` via the C runtime's `exit()`). Oct's `main` is
//!   void, so it exits `0`; the program still proves the whole chain runs.
//! * **I/O languages** (Brainfuck, Dartmouth BASIC) produce their result on
//!   **stdout** (`putchar` / `PRINT`), so the harness captures and compares stdout.

use lang_aot::Language;
use std::process::Command;

/// A non-BEAM backend the matrix proves languages on. Each new column the campaign
/// lands adds a variant here and a `run` arm; each `Prog` lists the backends it is
/// **proven** to run on (so a cell is only asserted once a slice has verified it).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Backend {
    /// Source → IIR → host object → system linker → run (LM0). General code gen.
    NativeAot,
    /// Source → textual `.ll` (`iir-to-llvm`) → real `clang` → run (Phase L).
    Llvm,
    /// Source → wasm bytes (`iir-to-wasm`) → in-process `wasm-runtime` (Phase W).
    Wasm,
    /// Source → `JvmClassFile` (`iir-to-jvm-class-file`) → real `java` (Phase J).
    /// The W16 wrapper-launcher pattern: a `main([Ljava/lang/String;)V` launcher is
    /// injected to invoke the entry method and `System.out.println` its `int` result.
    Jvm,
    /// Source → textual `.il` (`iir-to-cil-bytecode`) → real `ilasm` → real `dotnet`
    /// (Phase C, the CLR-real path). The emitted entry `Console.WriteLine`s its `int`
    /// result, which the harness parses (mirrors the McCarthy CLR-real chapter).
    Clr,
    /// Source → IIR (`compile_source_to_iir`) → the **generic register VM**
    /// (`vm_core::VMCore`) interpreting the shared IIR directly (Phase V). This is the
    /// execution-time analog of the code-gen backends: `VMCore` consumes the same
    /// `IIRModule` every other backend does — its instruction dispatch already covers
    /// arithmetic / comparison / bitwise / control-flow / memory / `call_builtin`, so a
    /// scalar language runs with **zero** VM-specific code, exactly the way a future
    /// Ruby/JS frontend would. (McCarthy lisp keeps its own `LispyValue` VM — the
    /// matrix's six languages are all scalar, so they share this one.) In-process, so
    /// no host gate; the I/O languages' `print_i64`/`putchar` are registered builtins.
    Vm,
    /// Source → IIR (`compile_source_to_iir`) → the **generic JIT** (`jit_core::JITCore`
    /// driving the language-agnostic `GenericCirJit` backend) over the shared IIR
    /// (Phase I). `execute_with_jit` eagerly compiles every fully-typed function to JIT
    /// bytecode and installs a native handler, falling back to the `VMCore` interpreter
    /// for anything the backend can't yet lower — so a program runs *through the JIT
    /// pipeline* and produces the same observable result. Like `Vm`, this consumes the
    /// same `IIRModule` as every other backend with **zero** language-specific code: a
    /// compiled function reads its arguments because `GenericCirJit` pre-binds parameters
    /// to registers `0..n` and `JITCore` seeds them from the call args (the generic
    /// register-VM/JIT design a future Ruby/JS frontend reuses unchanged). In-process, so
    /// no host gate; the I/O builtins are registered on both the VM and the JIT backend.
    Jit,
}

/// The known, backend-independent observable result of a conformance program.
enum Expect {
    /// The process exit code (an expression language's returned value, `& 0xFF`).
    Exit(i32),
    /// A trimmed stdout string (an I/O language's printed output).
    Stdout(&'static str),
}

/// One conformance program: a language, a source-file extension, the source, the
/// result it must produce, and the backends a slice has **proven** run it.
struct Prog {
    lang: Language,
    ext: &'static str,
    src: &'static str,
    expect: Expect,
    backends: &'static [Backend],
}

use Backend::{Clr, Jit, Jvm, Llvm, NativeAot, Vm, Wasm};

/// The real-CoreCLR helpers (`find_ilasm`, the NuGet-cache assembler search) are
/// shared with the McCarthy CLR-real chapter; `#[path]`-include the module so we
/// reuse `find_ilasm` rather than duplicating the cache walk. Only `find_ilasm` is
/// called from here (the module's own runner is McCarthy-specific dead code).
#[path = "clr_support/mod.rs"]
mod clr_support;

/// The cross-language battery. Each program is deliberately tiny but exercises real
/// computation (arithmetic, calls, comparisons, loops, I/O) — not just constants —
/// so a backend that merely emits a literal would not pass.
const PROGRAMS: &[Prog] = &[
    // Twig — the original AOT language; a bare expression is the whole program.
    Prog { lang: Language::Twig, ext: "twig", src: "42", expect: Expect::Exit(42), backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit] },
    // Twig — *variadic* arithmetic (`(+ 10 20 12)` = 42).  Scheme's `+`/`-`/`*`/`/`
    // are n-ary; `twig-ir-compiler` folds an all-`i64` arithmetic call into a
    // left-associated chain of typed binary CIR ops (`r1 = add 10,20; r2 = add
    // r1,12`).  Before this (TW1), only the binary `(+ a b)` form lowered to a
    // typed `add`; three-or-more-argument calls fell back to `call_builtin "+"`
    // (`type_hint = "any"`), which every code-gen backend validator rejects — so
    // this is the first variadic Twig arithmetic to run anywhere but the dynamic
    // path.  Runs across native / LLVM / WASM / JVM / CLR / VM / JIT.
    Prog {
        lang: Language::Twig,
        ext: "twig",
        src: "(+ 10 20 12)",
        expect: Expect::Exit(42),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Twig — E4 literal `string-length`. The compiler lowers
    // `(string-length "HELLO")` to shared `str_const` + `str_len`, avoiding the
    // dynamic `call_builtin "string-length"` path that codegen validators reject.
    // Native AOT folds the direct literal to a normal integer const; LLVM and
    // WASM use their literal side tables; JVM/CLR call their managed
    // `String.length` / `String.Length` APIs. The VM/JIT use vm-core's byte-count
    // reference implementation. ASCII keeps managed char length equal to E4 byte
    // length for this foothold.
    Prog {
        lang: Language::Twig,
        ext: "twig",
        src: "(string-length \"HELLO\")",
        expect: Expect::Exit(5),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Twig — E4 literal `string-append` feeding `string-length`. This exercises
    // the shared `str_concat` op while staying on the direct-literal metadata
    // path: static backends carry the concatenated bytes as compile-time
    // metadata, and JVM/CLR use their host `String` concat APIs.
    Prog {
        lang: Language::Twig,
        ext: "twig",
        src: "(string-length (string-append \"AB\" \"CDE\"))",
        expect: Expect::Exit(5),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Twig — E4 literal `string=?`. Like the literal length row, this stays on
    // the direct `str_const` + `str_eq` path so every codegen backend can prove
    // observable string equality without taking on full dynamic byte-string
    // algebra yet.
    Prog {
        lang: Language::Twig,
        ext: "twig",
        src: "(string=? \"HELLO\" \"HELLO\")",
        expect: Expect::Exit(1),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Twig — *top-level value `define`* read from `main` (`(define x 40) (define
    // y 2) (+ x y)` = 42).  A value define previously lowered to
    // `call_builtin "global_set"` (and reads to `global_get`), `type_hint =
    // "any"` — rejected by every code-gen backend validator, so top-level
    // constants ran only on the VM.  TW2: a value define that is **not captured
    // by a lambda** (read only from `main`) now keeps its statically-typed value
    // in a `main` register, and reads return that register — so `x`/`y` are
    // plain `i64` consts and `(+ x y)` is a typed `add`.  No `call_builtin`
    // survives, so it runs across native / LLVM / WASM / JVM / CLR / VM / JIT.
    // (A value captured by a closure stays on the host global table, unchanged.)
    Prog {
        lang: Language::Twig,
        ext: "twig",
        src: "(define x 40) (define y 2) (+ x y)",
        expect: Expect::Exit(42),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Nib — typed functions: define `double`, call it, return the result. Greened on
    // WASM in LM-W Nib by completing the i64 materialization: `nib_ty_str` and the
    // un-annotated-literal fallback now emit `i64` (not `u8`), so the const argument
    // `21` matches the `i64` parameter the strict WASM backend expects.
    Prog {
        lang: Language::Nib,
        ext: "nib",
        src: "fn double(x: u8) -> u8 { return x + x; } fn main() -> u8 { return double(21); }",
        expect: Expect::Exit(42),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Nib — multiplication (LANG-FULL N1). `*` lowers to the shared IIR `mul`; the
    // multiplicative level binds tighter than additive (so `2 + 3 * 4` is `2 + (3*4)`).
    // Executed on every backend — the anti-smoke-test guardrail: proven by RUNNING, not
    // validating/encoding. `6 * 7` exits 42.
    Prog {
        lang: Language::Nib,
        ext: "nib",
        src: "fn main() -> u8 { return 6 * 7; }",
        expect: Expect::Exit(42),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Nib — division (LANG-FULL N1). `/` lowers to the shared IIR `div`. `84 / 2` exits 42.
    Prog {
        lang: Language::Nib,
        ext: "nib",
        src: "fn main() -> u8 { return 84 / 2; }",
        expect: Expect::Exit(42),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Nib — `for` loop (LANG-FULL N2). Desugars to the canonical counter loop; the range
    // `1 .. 6` is exclusive (`i = 1,2,3,4,5`), summing the loop variable into a local
    // accumulator → 15. Reassigning the loop counter + a local each iteration is the same
    // slot-mutation shape every backend already lowers for Brainfuck's pointer. Executed
    // on every backend. (The accumulator is a `let` local, not a parameter: the IIR-to-LLVM
    // backend allocas locals but keeps params in SSA, so reassigning a *parameter* in a
    // loop is a separate backend limitation, out of scope for N2.)
    Prog {
        lang: Language::Nib,
        ext: "nib",
        src: "fn main() -> u4 { let s: u4 = 0; for i: u4 in 1 .. 6 { s = s + i; } return s; }",
        expect: Expect::Exit(15),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Nib — nested `for` loops (LANG-FULL N2). 3 × 2 = 6 body executions; proves distinct
    // loop labels and nested counter reassignment lower correctly on every backend.
    Prog {
        lang: Language::Nib,
        ext: "nib",
        src: "fn main() -> u4 { let s: u4 = 0; \
               for i: u4 in 0 .. 3 { for j: u4 in 0 .. 2 { s = s + 1; } } return s; }",
        expect: Expect::Exit(6),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Nib — reassigning a *function parameter* inside a loop (LLVM first-class fix).
    // `acc` is a parameter accumulated across the loop (`acc = acc + 6`, 7 times) → 42.
    // The IIR-to-LLVM backend previously kept params in SSA and only allocated locals,
    // so a reassigned param silently dropped its update; now a promoted param is copied
    // into an i64 stack slot at entry. Executed on every backend to prove parity.
    Prog {
        lang: Language::Nib,
        ext: "nib",
        src: "fn run(acc: u8) -> u8 { for i: u8 in 0 .. 7 { acc = acc + 6; } return acc; } \
               fn main() -> u8 { return run(0); }",
        expect: Expect::Exit(42),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Nib — bitwise AND / OR / XOR (LANG-FULL N3). `& | ^` lower to the shared IIR
    // `and`/`or`/`xor`. Executed on every backend: `12 & 10` = 0b1100 & 0b1010 = 0b1000 = 8.
    Prog {
        lang: Language::Nib,
        ext: "nib",
        src: "fn main() -> u8 { return 12 & 10; }",
        expect: Expect::Exit(8),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // `12 | 3` = 0b1100 | 0b0011 = 0b1111 = 15.
    Prog {
        lang: Language::Nib,
        ext: "nib",
        src: "fn main() -> u8 { return 12 | 3; }",
        expect: Expect::Exit(15),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // `6 ^ 5` = 0b110 ^ 0b101 = 0b011 = 3.
    Prog {
        lang: Language::Nib,
        ext: "nib",
        src: "fn main() -> u8 { return 6 ^ 5; }",
        expect: Expect::Exit(3),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Nib — `&&` SHORT-CIRCUIT (LANG-FULL N4). The left `1 == 2` is false, so the right
    // operand `84 / 0 == 0` must NOT be evaluated — if it were, the division by zero would
    // trap. The program returning 7 (not crashing, not 9) on every backend is positive
    // proof the RHS was skipped. (`&&`/`||` lower to a result slot + jmp_if_false branches.)
    Prog {
        lang: Language::Nib,
        ext: "nib",
        src: "fn main() -> u8 { if 1 == 2 && 84 / 0 == 0 { return 9; } return 7; }",
        expect: Expect::Exit(7),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Nib — `||` SHORT-CIRCUIT (LANG-FULL N4). The left `1 == 1` is true, so `84 / 0 == 0`
    // must NOT be evaluated. Returns 7 on every backend ⇒ the RHS was skipped.
    Prog {
        lang: Language::Nib,
        ext: "nib",
        src: "fn main() -> u8 { if 1 == 1 || 84 / 0 == 0 { return 7; } return 9; }",
        expect: Expect::Exit(7),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Nib — `&&` true path: both sides true ⇒ the `if` is taken ⇒ 1.
    Prog {
        lang: Language::Nib,
        ext: "nib",
        src: "fn main() -> u8 { if 1 == 1 && 2 == 2 { return 1; } return 0; }",
        expect: Expect::Exit(1),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Nib — module-scoped `const` (LANG-FULL N5). A top-level `const N: u8 = 42;` is folded
    // to its literal at each use, so referencing it in `main` needs no runtime storage and
    // runs on every backend.
    Prog {
        lang: Language::Nib,
        ext: "nib",
        src: "const N: u8 = 42; fn main() -> u8 { return N; }",
        expect: Expect::Exit(42),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Nib — multiple consts used in arithmetic: `30 + 12` = 42.
    Prog {
        lang: Language::Nib,
        ext: "nib",
        src: "const A: u8 = 30; const B: u8 = 12; fn main() -> u8 { return A + B; }",
        expect: Expect::Exit(42),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Nib — u8 WRAP (LANG-FULL E2 / N6). `200u8 + 100u8` overflows the byte and must
    // wrap mod-256 to `44`. The exit code is itself `& 0xFF`, so a bare `return 200+100`
    // could NOT distinguish a wrapped 44 from an unwrapped 300 (both exit 44) — instead we
    // compare the in-register value: with wrap `x == 44` is true (→1), without wrap
    // `300 == 44` is false (→0). Returning 1 on every backend proves the add wrapped
    // BEFORE the comparison. This exercises the whole E2 stack: the Nib frontend emits a
    // `u8` type_hint on the add (bidirectional typing), and each backend masks the result
    // — vm-core/jit-core/wasm/jvm/cil by value-mask, LLVM by `and i64`, native-AOT by the
    // aarch64/x86_64 `and #mask`.
    Prog {
        lang: Language::Nib,
        ext: "nib",
        src: "fn main() -> u8 { let x: u8 = 200 + 100; if x == 44 { return 1; } return 0; }",
        expect: Expect::Exit(1),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Nib — u8 wrap is width-correct, not "any mask": `6 * 7 = 42` must stay 42 (it fits a
    // byte), proving the mask is mod-256 not a blanket truncation. `6` and `7` are typed
    // `u8` from the `-> u8` return context (bidirectional typing), so the product is masked
    // at 0xFF and 42 < 256 is unchanged. (Guards the regression where magnitude-based
    // literal typing made `6*7` a `u4` and wrapped it to `42 & 0xF = 10`.)
    Prog {
        lang: Language::Nib,
        ext: "nib",
        src: "fn main() -> u8 { return 6 * 7; }",
        expect: Expect::Exit(42),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Nib — `+%` WRAPPING add (LANG-FULL N7). `200u8 +% 100` discards the carry →
    // `44`. The comparison (`x == 44`) distinguishes a wrapped 44 from an unwrapped
    // 300 (whose exit-code low byte would also be 44). `+%` lowers to the same
    // narrow-typed `add` as `+`, which the E2 backend mask wraps.
    Prog {
        lang: Language::Nib,
        ext: "nib",
        src: "fn main() -> u8 { let x: u8 = 200 +% 100; if x == 44 { return 1; } return 0; }",
        expect: Expect::Exit(1),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Nib — `+?` SATURATING add (LANG-FULL N7). `200u8 +? 100` clamps at the u8 max
    // → `255` (NOT 44, the wrapping result). `+?` lowers to a *wide* add + a clamp
    // branch (`min(sum, 255)`), exercising add/const/cmp_gt/jmp_if_false/mov/label
    // together on every code-gen backend.
    Prog {
        lang: Language::Nib,
        ext: "nib",
        src: "fn main() -> u8 { let x: u8 = 200 +? 100; if x == 255 { return 1; } return 0; }",
        expect: Expect::Exit(1),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Nib — `+?` saturating at the u4 max: `15u4 +? 1` clamps to `15` (the nibble
    // max), not the wrapping `0`.
    Prog {
        lang: Language::Nib,
        ext: "nib",
        src: "fn main() -> u4 { let a: u4 = 15 +? 1; if a == 15 { return 1; } return 0; }",
        expect: Expect::Exit(1),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Nib — `+?` that does NOT overflow returns the plain sum (`3 +? 4 = 7`),
    // proving the clamp branch only fires on overflow.
    Prog {
        lang: Language::Nib,
        ext: "nib",
        src: "fn main() -> u8 { let x: u8 = 3 +? 4; if x == 7 { return 1; } return 0; }",
        expect: Expect::Exit(1),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Nib — bitwise NOT (`~`), u8 (LANG-FULL N3). `~0` flips all bits; masked to the
    // u8 width (the E2 value-mask) it is `255` (`-1 & 0xFF`), NOT the i64 all-ones.
    // `compile_unary` lowers `~` to the shared IIR `not` op with a `u8` type_hint;
    // `iir-to-llvm` 0.12.0 grew the `not` op (the last backend that lacked it), so this
    // now runs on every backend. The `if x == 255` guard distinguishes the masked
    // complement from an unmasked `not 0` (`-1`, which would NOT equal 255 → exit 0).
    Prog {
        lang: Language::Nib,
        ext: "nib",
        src: "fn main() -> u8 { let x: u8 = ~0; if x == 255 { return 1; } return 0; }",
        expect: Expect::Exit(1),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Nib — bitwise NOT (`~`), u4 (LANG-FULL N3). `~15` on a nibble: 15 = 0b1111, so
    // its complement masked to 4 bits is `0`. Proves the `not` mask is width-correct
    // (a u8 or i64 mask would leave 0xF0/-16, not 0), distinct from the u8 case above.
    Prog {
        lang: Language::Nib,
        ext: "nib",
        src: "fn main() -> u4 { let x: u4 = ~15; if x == 0 { return 1; } return 0; }",
        expect: Expect::Exit(1),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Oct — `let` + `if` + comparison; `main` is void so the process exits 0.
    Prog {
        lang: Language::Oct,
        ext: "oct",
        src: "fn main() { let x: u8 = 1; if x == 1 { let y: u8 = 2; } else { let z: u8 = 3; } }",
        expect: Expect::Exit(0),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Oct — the `out` intrinsic prints to stdout (LANG-FULL O-OUT). The 8008 writes a
    // value to an I/O port; on the general backends all ports collapse to stdout via
    // `call_builtin "print_i64"`. This is Oct's FIRST observable output — until now an
    // Oct program could only exit 0, so no Oct result could be checked by running.
    // `out(1, 200)` prints 200.
    Prog {
        lang: Language::Oct,
        ext: "oct",
        src: "fn main() { out(1, 200); }",
        expect: Expect::Stdout("200"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Oct — `out` of a computed value: `100 + 100` = 200 printed. Proves Oct arithmetic
    // produces the right result *observably* (not just "ran and exited 0").
    Prog {
        lang: Language::Oct,
        ext: "oct",
        src: "fn main() { out(1, 100 + 100); }",
        expect: Expect::Stdout("200"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Oct — `&&` SHORT-CIRCUIT, PROVEN observably (LANG-FULL O1). `side()` prints 5 and
    // returns 1; it sits in the right operand of `&&`. The left `1 == 2` is false, so a
    // correct short-circuit must NOT call `side()` — output is just `9` (the else branch).
    // The OLD eager-bitwise lowering called `side()` unconditionally → it would print `5`
    // first (`5`,`9`). So stdout == "9" is positive proof the RHS was skipped. (JVM
    // excluded — branch + `print_i64`, the BA-JVM-1 StackMapTable follow-up.)
    Prog {
        lang: Language::Oct,
        ext: "oct",
        src: "fn side() -> u8 { out(1, 5); return 1; } \
               fn main() { if 1 == 2 && side() == 1 { out(1, 1); } else { out(1, 9); } }",
        expect: Expect::Stdout("9"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Oct — `||` SHORT-CIRCUIT, PROVEN observably. `1 == 1` is true, so `side()` (in the
    // right operand) must be skipped → output `7`. Eager would print `5` then `7`.
    Prog {
        lang: Language::Oct,
        ext: "oct",
        src: "fn side() -> u8 { out(1, 5); return 1; } \
               fn main() { if 1 == 1 || side() == 1 { out(1, 7); } else { out(1, 9); } }",
        expect: Expect::Stdout("7"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Oct — bitwise NOT (`~`) masks to the u8 width (LANG-FULL O2). Oct's only integer
    // type is `u8` (the 8008 byte), so `~0` flips 8 bits → `255` (`-1 & 0xFF`), NOT the
    // i64 all-ones. `oct-iir-compiler` 0.7.0 emits the `not` op with a `u8` type_hint;
    // every backend masks it (the E2 value-mask), the same path Nib N3-`~` proved. The
    // `out` prints the value, so stdout is the direct observable proof. (An unmasked
    // `~0` would print `-1`, not `255`.)
    Prog {
        lang: Language::Oct,
        ext: "oct",
        src: "fn main() { out(1, ~0); }",
        expect: Expect::Stdout("255"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Oct — u8 arithmetic WRAPS modulo 256 (LANG-FULL O2). The grammar specifies Oct
    // addition wraps mod-2⁸; `200 + 100 = 300` wraps to `44`. Until O2 the result rode an
    // unmasked i64 slot and printed `300`. Now the `add` carries the `u8` hint and every
    // backend masks it. Distinct from the existing `100 + 100 = 200` Oct program, which
    // does NOT overflow — this one proves the wrap actually fires.
    Prog {
        lang: Language::Oct,
        ext: "oct",
        src: "fn main() { out(1, 200 + 100); }",
        expect: Expect::Stdout("44"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Oct — `static` module GLOBAL, shared across functions (LANG-FULL O3). Until now
    // Oct's top-level `static` was silently dropped at IIR-gen; `oct-iir-compiler` 0.8.0
    // lowers it to the IIR module-global ops (`global_load`/`global_store`, LANG32 — the
    // same path ALGOL's enclosing-block scalars use for E6). `counter` is initialised to
    // 40 once at the top of `main`, then `bump()` — a SEPARATE function — increments the
    // shared global twice, and `main` prints it: `42`. This proves three things at once,
    // observably: (1) the initialiser ran, (2) a write in one function is visible in
    // another (it's ONE global, not a per-function register — a register model would
    // print 40), and (3) the global survives across the two `bump` calls. Runs on all 7
    // backends, each materialising the global natively (LLVM `@__twig_global_N`, a JVM/CLR
    // `static` field, a WASM module global, the native `_twig_globals` slot, the VM/JIT
    // name-keyed map, the BEAM process dict) — no backend learned anything Oct-specific.
    Prog {
        lang: Language::Oct,
        ext: "oct",
        src: "static counter: u8 = 40; \
               fn bump() { counter = counter + 1; } \
               fn main() { bump(); bump(); out(1, counter); }",
        expect: Expect::Stdout("42"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // ALGOL 60 — a begin/end block with real integer arithmetic (`17 mod 5` = 2).
    Prog {
        lang: Language::Algol60,
        ext: "alg",
        src: "begin integer result; result := 17 mod 5 end",
        expect: Expect::Exit(2),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // ALGOL 60 — the `abs` **standard function** (§3.2.4, LANG-FULL AL8). `abs`
    // is built into the language, not a user procedure, so `algol-iir-compiler`
    // resolves it by name and lowers `abs(0 - 42)` inline to the value of
    // `if (0-42) < 0 then -(0-42) else (0-42)` — a `cmp_lt` against zero, then a
    // `jmp_if_false` choosing between a negated and a pass-through `mov` into one
    // result slot (the store-per-branch shape the conditional-expression lowering
    // already runs on every backend). No backend learns anything about `abs`: it
    // is compare + branch + subtract in the shared IIR, so |−42| = 42 ⇒ exit 42
    // on native-AOT / LLVM / WASM / JVM / CLR / VM / JIT alike.
    Prog {
        lang: Language::Algol60,
        ext: "alg",
        src: "begin integer result; result := abs(0 - 42) end",
        expect: Expect::Exit(42),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // ALGOL 60 — the `sign` **standard function** (§3.2.4, LANG-FULL AL8). Like
    // `abs`, `sign` is built in and resolved by name; it lowers to the nested
    // conditional `if E > 0 then 1 else if E < 0 then -1 else 0` — three `i64`
    // constants moved into one result slot (store-per-branch, no phi). `sign`
    // always returns an `integer` regardless of operand type. `sign(0 - 1) = -1`,
    // so `43 + sign(0 - 1)` = 42 ⇒ exit 42 — exercising the negative branch on
    // native-AOT / LLVM / WASM / JVM / CLR / VM / JIT.
    Prog {
        lang: Language::Algol60,
        ext: "alg",
        src: "begin integer result; result := 43 + sign(0 - 1) end",
        expect: Expect::Exit(42),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // ALGOL 60 — the `entier` **standard function** (§3.2.5, LANG-FULL E8 + AL-entier).
    // `entier(E)` is the largest *integer* not greater than the *real* `E` — floor,
    // rounding toward −∞ (NOT trunc toward zero): `entier(2.7)` = 2, `entier(−2.7)`
    // = −3.  Unlike `abs`/`sign` (which lower to compare+branch over existing ops),
    // `entier` lowers to a single **E8 `real_to_int_floor`** IIR conversion op — the
    // floor and the real→integer narrowing fused into one primitive that every
    // backend emits in its native idiom: LLVM `@llvm.floor.f64`+`fptosi`, WASM
    // `f64.floor`+`i32.trunc_f64_s`, JVM `Math.floor`+`d2i`, CLR `Math::Floor`+
    // `conv.ovf.i4`, native aarch64 `frintm`+`fcvtzs`, native x86_64 `roundsd …,1`+
    // `cvttsd2si`.  The program builds a *negative* real (`0.0 − 2.7 = −2.7`) so the
    // observable distinguishes floor from trunc: `45 + entier(−2.7)` = `45 + (−3)` =
    // 42 (trunc would give `45 + (−2)` = 43).  This RUNS the E8 floor-conversion op
    // end-to-end on **all 7 backends** — the proof that closes the E8 conversions arc.
    Prog {
        lang: Language::Algol60,
        ext: "alg",
        src: "begin real r; integer result; r := 0.0 - 2.7; \
               result := 45 + entier(r) end",
        expect: Expect::Exit(42),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // ALGOL 60 — a *typed procedure with a value parameter* (`integer procedure
    // sq(x); value x; integer x; sq := x*x`) called from the main block:
    // `result := sq(7)` ⇒ exit 49.  `algol-iir-compiler` lowers the procedure
    // out of line into a second `IIRFunction sq(x:i64) -> i64` (its body assigns
    // the result to the in-scope `sq` slot and `ret`s it), and the call site to
    // a `call` whose `srcs[0]` names `sq` and whose remaining `srcs` carry the
    // argument slots.  Every backend already iterates `module.functions` and
    // resolves a same-module `call` by name, so the procedure runs everywhere:
    // native-AOT/LLVM emit a real `call @sq`, WASM a `call $sq`, the JVM a static
    // `invokestatic Program.sq(J)J`, the CLR a `call int64 Program::sq(int64)`,
    // and the VM/JIT push a frame for `sq`.  No backend learned anything about
    // ALGOL — procedures are just functions + calls in the shared IIR.
    Prog {
        lang: Language::Algol60,
        ext: "alg",
        src: "begin integer result; integer procedure sq(x); value x; integer x; \
               sq := x * x; result := sq(7) end",
        expect: Expect::Exit(49),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // ALGOL 60 — a procedure that *reads and writes a variable from the enclosing
    // block* (LANG-FULL enabler **E6**, layer 1 — typed module globals).
    // `counter` is declared in the outer block and accessed by both `incr` and the
    // block, so `algol-iir-compiler`'s E6 capture analysis materialises it as a
    // typed module **global**: `incr` reads it (`global_load "counter"`), adds its
    // value parameter, writes it back (`global_store "counter"`), and returns it;
    // the block seeds `counter := 40` (another `global_store`) then `result :=
    // incr(2)` ⇒ 42.  The global is a value a register frame couldn't carry — it
    // outlives `incr`'s call and is shared across the two `IIRFunction`s — which
    // is the whole point of E6.  (The procedure is named `incr`, not `add`: `add`
    // is a CIL opcode, so an unquoted `call …::add(int32)` won't assemble — a
    // pre-existing CLR identifier-quoting limitation, orthogonal to E6.)
    //
    // **This is the E6-layer-1 completion proof: it RUNS on all 7 backends**, each
    // realising the shared global in its own native idiom — VM/JIT a name-keyed
    // map; LLVM a `@__twig_global_N = internal global i64`; the JVM/CLR a `static
    // long`/`int64` field (`getstatic`/`putstatic`, `ldsfld`/`stsfld`); BEAM the
    // process dictionary; WASM a module mutable `global`; native a `_twig_globals`
    // data slot.  No backend learned anything ALGOL-specific — `global_load`/
    // `global_store` are shared IIR ops every backend grew (this PR is the last
    // brick: the producer that finally emits them from real typed source).
    Prog {
        lang: Language::Algol60,
        ext: "alg",
        src: "begin integer counter, result; \
               integer procedure incr(x); value x; integer x; \
                  incr := counter := counter + x; \
               counter := 40; \
               result := incr(2) end",
        expect: Expect::Exit(42),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // ALGOL 60 — an `own` variable: static lifetime (LANG-FULL **AL6**). `bump`
    // declares `own integer n` inside its body; ALGOL 60 §5.2.5 says an `own`
    // variable is allocated once and *retains its value across calls*. The
    // frontend lowers it to a module **global** (the E6 substrate —
    // `global_load`/`global_store`), keyed by a per-procedure-unique slot, and
    // crucially does NOT re-`const`-zero it on entry (that would destroy
    // persistence). Three calls accumulate on the one cell: `bump(1)` ⇒ 1,
    // `bump(1)` ⇒ 2, `bump(1)` ⇒ 3, so `result := 1 + 2 + 3 = 6`. A non-`own`
    // local would reset to 0 each call → `1 + 1 + 1 = 3`, so **6 is positive
    // proof of static lifetime**. Runs on all 7 backends, each persisting the
    // global in its native idiom (same realisations as the E6 proof above); the
    // JVM/CLR store the i32-concretized `integer` in a 64-bit field and narrow
    // on load (the `l2i`/`conv.i4` path the E6 matrix proof established).
    Prog {
        lang: Language::Algol60,
        ext: "alg",
        src: "begin integer result; \
               integer procedure bump(d); value d; integer d; \
                  begin own integer n; n := n + d; bump := n end; \
               result := bump(1) + bump(1) + bump(1) end",
        expect: Expect::Exit(6),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // ALGOL 60 — a *switch* (computed goto) + the integer comparison that drives
    // it.  `switch s := a1, a2, a3; … goto s[i]` selects the i-th label by a
    // 1-based linear `index == k ? jmp Lk` chain (portable jmp/jmp_if_false/label
    // subset).  With `i := 3` control reaches `a3` ⇒ exit 49.  The index test
    // (`i == k`) is also the first ALGOL comparison exercised on a code-gen
    // backend: `algol-iir-compiler` now emits `cmp_*` with the **operand** width
    // (`i64`), not the `bool` result width — emitting `bool` made LLVM compare at
    // 1-bit `i1` (`3 == 1` → `1 == 1` → true → wrong target), the same latent
    // truncation the BASIC BA0 fix found. `s[3]` is chosen specifically because
    // an i1 compare would mis-select the first arm, so the cell proves the fix.
    Prog {
        lang: Language::Algol60,
        ext: "alg",
        src: "begin integer result; switch s := a1, a2, a3; integer i; i := 3; \
               goto s[i]; a1: result := 1; goto done; a2: result := 2; goto done; \
               a3: result := 49; done: end",
        expect: Expect::Exit(49),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // ALGOL 60 — *real (f64) arithmetic* + a real comparison (LANG-FULL AL1 /
    // enabler E3, phase 1).  `r := 2.5 * 2.0` computes in IEEE-754 double
    // (`5.0`), then `if r = 5.0` folds a real equality to the integer exit code
    // 42.  This RUNS real multiplication and an `f64` comparison end-to-end:
    // `algol-iir-compiler` lowers `real` to the IIR's `f64` type and
    // `2.5`/`2.0`/`5.0` to `Operand::Float`, and the runtime computes in `f64`.
    // The exit code stays an integer, so no float *printing* is needed to verify
    // (the comparison fold is the observable).
    //
    // **Backends:** ALL 7 (**E3 COMPLETE**). VM/JIT carry a real tagged value
    // model; **LLVM** uses `iir-to-llvm`'s `double` stack slots; **WASM** needed
    // no change (typed-local model); **JVM** uses `iir-to-jvm-class-file`'s
    // `CONSTANT_Double` pool + `dcmpl`/`dcmpg`; **CLR** uses `iir-to-cil-bytecode`'s
    // `float64` locals + `ldc.r8`; and **NativeAot** uses the direct backends'
    // FP codegen — `aarch64-backend` (`fadd`/`fcmp`+`cset`, executed on this Mac)
    // and `x86_64-backend` (SSE2 `addsd`/`ucomisd`+`setcc`, executed on the
    // Linux-x86 CI runner). `run_native` compiles for the host arch, so this cell
    // exercises aarch64 locally and x86_64 in CI.
    Prog {
        lang: Language::Algol60,
        ext: "alg",
        src: "begin real r; integer result; r := 2.5 * 2.0; \
               if r = 5.0 then result := 42 else result := 0 end",
        expect: Expect::Exit(42),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // ALGOL 60 — *real division* (`/`) + an *ordered* real comparison (E3).
    // `r := 7.0 / 2.0` is true division (`3.5`, not integer `div`'s `3`), and
    // `if r < 4.0` exercises `f64` ordered comparison (LLVM `fcmp olt` / WASM
    // `f64.lt` / JVM `dcmpl`+`ifge`) ⇒ exit 1.  Same LLVM+WASM+JVM+VM+JIT slice as
    // above (native + CLR pend E3-native / E3-clr).
    Prog {
        lang: Language::Algol60,
        ext: "alg",
        src: "begin real r; integer result; r := 7.0 / 2.0; \
               if r < 4.0 then result := 1 else result := 0 end",
        expect: Expect::Exit(1),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // ALGOL 60 — *one-dimensional integer arrays* (LANG-FULL enabler E5, AL2).
    // `integer array A[1:5]` lowers to an IIR `alloc_array`; the first `for` loop
    // fills `A[i] := i*i` (an `array_set` with the 0-based index `i - 1`), the
    // second sums `result := result + A[i]` (an `array_get`) ⇒ 1+4+9+16+25 = 55.
    // This RUNS the four E5 array ops end-to-end: `algol-iir-compiler` lowers the
    // declaration, subscripted stores, and subscripted loads, and `vm-core`
    // executes them on its bounds-checked `Vec<Vec<Value>>` heap.
    //
    // **Backends:** VM + JIT + **JVM** + **CLR** (E5 PR-1/2/3a/3b). The reference
    // interpreter (`vm-core` 0.7.0) implements the array ops; the JIT cold-interprets
    // them. On the **JVM** (`iir-to-jvm-class-file`) the array is a real `int[]`:
    // `alloc_array`→`newarray T_INT`, `array_set`→`iastore`, `array_get`→`iaload`,
    // with the JVM's native bounds check (OOB → `ArrayIndexOutOfBoundsException`).
    // On the **CLR** (`iir-to-cil-bytecode` textual `.il`) it's a real `int32[]`:
    // `alloc_array`→`newarr System.Int32`, `array_set`→`stelem.i4`, `array_get`→
    // `ldelem.i4`, `array_len`→`ldlen`+`conv.i4`, with CoreCLR's native bounds check
    // (OOB → `System.IndexOutOfRangeException`). Both managed runtimes give E5's trap
    // for free. The handle is a reference local. On **LLVM** (`iir-to-llvm`) it now
    // runs too: the ALGOL `for`-loop guard formerly emitted an `i1`-typed `icmp`
    // over `i64` operands (it tagged the comparison with the boolean *result* type
    // instead of the operand width) — `clang` rejected that IR, so this cell was VM/
    // JIT/JVM/CLR only. Fixed in `algol-iir-compiler` (the guard now compares at
    // `i64`, like every other relation), and the `for`-loop sum-of-squares now
    // compiles + runs via `clang` → exit 55. (NativeAot/WASM lower arrays but the
    // for-loop path is the LLVM-specific cmp lowering this exercises.)
    Prog {
        lang: Language::Algol60,
        ext: "alg",
        src: "begin integer array A[1:5]; integer i, result; \
               for i := 1 step 1 until 5 do A[i] := i * i; \
               result := 0; \
               for i := 1 step 1 until 5 do result := result + A[i] end",
        expect: Expect::Exit(55),
        backends: &[Vm, Jit, Jvm, Clr, Llvm],
    },
    // ALGOL 60 — *straight-line* 1-D integer array (LANG-FULL E5, the **LLVM**
    // static-array proof — PR-4a). `A[1] := 40; A[3] := 2; result := A[1] + A[3]`
    // ⇒ 42, exercising `alloc_array`/`array_set`/`array_get` with no loop. On
    // **LLVM** (`iir-to-llvm`) this is the *static* array model: a length-prefixed
    // `@calloc` block `[i64 len][elems…]` with the handle pointing at the payload;
    // each `array_set`/`array_get` emits an **explicit** `icmp uge idx, len` and a
    // `br` to a `call void @llvm.trap()` block on out-of-range (the native target
    // has no managed runtime to bounds-check for it), then a typed `getelementptr`
    // + `load`/`store`. `clang` compiles the `.ll` and runs it → exit 42. On **WASM**
    // (`iir-to-wasm`, PR-4b) it is the same static model in **linear memory**: a
    // `__array_bump` global hands each `alloc_array` a fresh `[i64 len][elems…]`
    // region; `array_get`/`array_set` emit `idx >=u len` → `if … unreachable` (the
    // wasm trap) then an `i64.load`/`i64.store` at `wrap(handle)+idx*8` offset 8.
    // The in-repo `wasm-runtime` interpreter executes it → exit 42. On **NativeAot**
    // (`x86_64-backend` / `aarch64-backend`, PR-4c) it is the same static model in
    // raw machine code: `alloc_array` calls the shared `__twig_alloc_bytes` for an
    // `8 + count*8` block and writes the length header; `array_get`/`array_set` emit
    // an explicit unsigned `cmp` + branch (`jb`/`b.lo`) over a `ud2`/`udf` **trap**,
    // then a base+idx*8 load/store at offset 8. `run_native` builds a real exe and
    // runs it → exit 42 (aarch64 on this Apple Silicon host; x86_64 on the Linux CI
    // runner). **This completes E5 across all 7 backends.** Runs on every already-
    // supported array backend too (VM/JIT/JVM/CLR), all straight-line.
    Prog {
        lang: Language::Algol60,
        ext: "alg",
        src: "begin integer array A[1:3]; integer result; \
               A[1] := 40; A[3] := 2; result := A[1] + A[3] end",
        expect: Expect::Exit(42),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Brainfuck — build 65 on the tape and `putchar` it: prints `A`.
    // `lower_brainfuck_for_aot` widens the BF cell/ptr registers to `i64` (byte width
    // survives only at the tape boundary) for every code-gen backend. On LLVM (LM-L)
    // `iir-to-llvm` (v0.9.0) lowers the tape ops to `@calloc`/`getelementptr i8`+`zext`/
    // `trunc`+`store` + libc `putchar`/`getchar`. On WASM (LM-W) `iir-to-wasm` (v0.13.0)
    // lowers them over linear memory: `alloc_bytes`→base offset 0, `load_byte`→
    // `i32.load8_u`+`i64.extend_i32_u`, `store_byte`→`i32.wrap_i64`+`i32.store8`, and
    // `putchar`/`getchar`→ the `env.putchar`/`env.getchar` host imports `run_wasm`'s
    // `PutcharFunc` resolves (capturing raw bytes → stdout `A`, not the decimal `65`).
    // On JVM (LM-J) `iir-to-jvm-class-file` lowers the tape to a static `byte[] __tape`
    // (`getstatic … __tape : [B` + `baload`/`bastore`) and `.`/`,` to `invokestatic
    // env/BFRuntime.putchar(I)V`/`getchar()I`; `run_jvm` compiles the `env.BFRuntime`
    // host class with `javac` and captures its `System.out.write` bytes (→ `A`).
    // On CLR (LM-C) `iir-to-cil-bytecode`'s textual `.il` lowers the tape to an
    // `unsigned int8[]` local (`newarr [System.Runtime]System.Byte`, `ldelem.u1`/
    // `stelem.i1`) and `.` to `Console::Write(char)` (so `.` of 65 writes `A`); the
    // `Run()` launcher discards the entry result (the `putchar` side effect is the
    // output). `run_clr` assembles with real `ilasm` and runs on real `dotnet`.
    // On the VM (Phase V), `vm-core`'s dispatch grew the byte-tape ops `alloc_bytes`/
    // `load_byte`/`store_byte` over its flat `memory` (a cell is `memory[base+idx]`,
    // `store_byte` masks to a byte for the 8-bit wrap); `.` is the registered
    // `putchar` builtin capturing bytes (→ `A`). No per-language VM code.
    Prog {
        lang: Language::Brainfuck,
        ext: "bf",
        src: "++++++++[>++++++++<-]>+.",
        expect: Expect::Stdout("A"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Brainfuck — a NESTED-loop, multi-output program (LANG-FULL B1). The classic
    // multiply-by-repeated-addition idiom `[>[->+>+<<]>>[-<<+>>]<<<-]` is a loop *inside*
    // a loop that moves data across four cells: it computes 8 × 9 = 72 (`H`), printed by
    // the first `.`; then `-------` brings the cell to 65 (`A`), printed by the second.
    // The matrix's other BF cell is a single loop printing one char — this proves the
    // backends lower **nested loops + multi-cell pointer movement + multiple `putchar`s**,
    // not just one loop. Output: "HA".
    Prog {
        lang: Language::Brainfuck,
        ext: "bf",
        src: "++++++++>+++++++++<[>[->+>+<<]>>[-<<+>>]<<<-]>>.-------.",
        expect: Expect::Stdout("HA"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Brainfuck — two sequential loops + two outputs (LANG-FULL B1). Builds 80 in a loop,
    // `-` → 79 (`O`), prints; `----` → 75 (`K`), prints. Distinct loop labels + multi-output.
    Prog {
        lang: Language::Brainfuck,
        ext: "bf",
        src: "++++++++[>++++++++++<-]>-.----.",
        expect: Expect::Stdout("OK"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Brainfuck — STDIN (LANG-FULL B1-stdin). The matrix proved every backend can *write*
    // output (`.`); these two prove every backend can *read* input (`,`). `,+.` reads one
    // byte from real stdin, `+` increments it, and `.` prints the result — so the output
    // depends on BOTH the input and a computation on it (not a constant, not a bare echo):
    // input "A" (65) → output "B" (66). The harness feeds "A" to the process stdin on the
    // four subprocess backends (libc `getchar` / `System.in` / `Console.Read`) and to the
    // in-process `getchar` buffer on WASM/VM/JIT — see `program_stdin` + `output_with_stdin`.
    //
    // It reads EXACTLY as many bytes as supplied and never reads past EOF, so it terminates
    // identically on every backend *regardless* of the EOF convention — which still differs
    // (JVM's BFRuntime and the VM/JIT return 0; libc `getchar`, `Console.Read` and the wasm
    // host return -1 → the cell wraps to 255). The classic cat `,[.,]` would loop forever on
    // the -1 backends, so normalising EOF across backends is a separate item; these programs
    // sidestep it by construction.
    Prog {
        lang: Language::Brainfuck,
        ext: "bf",
        src: ",+.",
        expect: Expect::Stdout("B"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Brainfuck — multi-byte STDIN echo (LANG-FULL B1-stdin). `,.,.` reads a byte and prints
    // it, twice; with input "Hi" it echoes "Hi". Proves *repeated* reads advance through the
    // input stream on every backend (the second `,` must see 'i', not 'H' again). Like `,+.`
    // it reads exactly the supplied bytes — no EOF-gated loop — so it terminates everywhere.
    Prog {
        lang: Language::Brainfuck,
        ext: "bf",
        src: ",.,.",
        expect: Expect::Stdout("Hi"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Brainfuck — the canonical `cat` (LANG-FULL B1-eof). `,[.,]` reads a byte and, while
    // it is non-zero, prints it and reads the next — echoing stdin until end-of-input. The
    // loop terminates ONLY when `,` returns 0, which is what `getchar` must yield at EOF.
    // The `,+.`/`,.,.` programs above read exactly their input and never hit EOF; this one
    // reads PAST the input, so it exercises the EOF convention directly. It was the deferred
    // half of B1-stdin: backends disagreed on EOF (JVM/VM/JIT → 0, libc/Console/wasm → -1 →
    // cell 255), so cat looped forever on the -1 backends. `brainfuck-iir-compiler` now
    // clamps a negative `,` result to 0 in the shared IIR (`getchar` read at i64, `cmp_lt
    // 0` + branch), so EOF is 0 on EVERY backend and cat echoes "Hi" then halts.
    Prog {
        lang: Language::Brainfuck,
        ext: "bf",
        src: ",[.,]",
        expect: Expect::Stdout("Hi"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Dartmouth BASIC — `PRINT 42` writes `42` to stdout. BA7-1b makes even the
    // integer-spelled literal a scalar `f64`, so this baseline cell now runs through
    // `__basic_print_real` and the shared f64 backend tracks. The helper's current
    // whole-valued contract truncates with E8 `real_to_int_trunc`, then reuses BA2's
    // digit printer, keeping the observable output identical on all 7 backends.
    Prog {
        lang: Language::DartmouthBasic,
        ext: "bas",
        src: "10 PRINT 42\n20 END\n",
        expect: Expect::Stdout("42"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Dartmouth BASIC — E4/BA4 first string-PRINT proof. The frontend lowers a
    // string literal item to shared `str_const` + `print_str`, and the existing
    // BASIC PRINT machinery emits the trailing newline via `putchar`. Native AOT
    // rewrites the literal to `alloc_bytes` + `store_byte` + `print_string`;
    // LLVM emits a length-prefixed private constant and calls the generic
    // `__print_str` runtime with `(payload,len)`; WASM stores the literal bytes in
    // linear memory and calls `env.__print_str(ptr,len)`; JVM uses `ldc` +
    // `PrintStream.print(String)` and textual CIL uses `ldstr` +
    // `Console.Write(string)`, while richer byte-string operations stay outside
    // this slice.
    Prog {
        lang: Language::DartmouthBasic,
        ext: "bas",
        src: "10 PRINT \"HELLO\"\n20 END\n",
        expect: Expect::Stdout("HELLO"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Dartmouth BASIC — `FOR`/`NEXT` loop with an accumulator (LANG-FULL BA0). Sums
    // 1..5 into S and prints 15. FOR/NEXT lowers to `cmp_le`, which the WASM and LLVM
    // backends could not run correctly until this slice (LLVM compared at `i1` width;
    // the BASIC compiler now emits the `i64` operand type — see its CHANGELOG). Until
    // now BASIC loops executed only on the VM/JIT; this RUNS a real FOR loop on the
    // code-gen backends.
    //
    // JVM is excluded pending a separate fix: a backward branch (loop) combined with a
    // `print_i64` call after it trips the `iir-to-jvm-class-file` StackMapTable
    // generation (a forward-branch print — the IF program below — and a loop *without*
    // print — Nib's for-loops — both work on JVM; only the loop+print combination
    // fails). Tracked as roadmap item BA-JVM-1.
    Prog {
        lang: Language::DartmouthBasic,
        ext: "bas",
        src: "10 LET S = 0\n20 FOR I = 1 TO 5\n30 LET S = S + I\n40 NEXT I\n50 PRINT S\n60 END\n",
        expect: Expect::Stdout("15"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Dartmouth BASIC — `IF … THEN <line>` + `GOTO`-style jump (LANG-FULL BA0). `A > 5`
    // lowers to `cmp_gt` (one of the comparisons LLVM compared at the wrong width until
    // this slice's BASIC-compiler fix); the taken branch jumps to line 100 which prints
    // A (7). Proves conditional control flow runs on the code-gen backends, not just the
    // VM/JIT. (JVM excluded — same BA-JVM-1 StackMapTable follow-up as the FOR program:
    // a branch combined with a `print_i64` call trips JVM stack-frame generation.)
    Prog {
        lang: Language::DartmouthBasic,
        ext: "bas",
        src: "10 LET A = 7\n20 IF A > 5 THEN 100\n30 PRINT 0\n40 END\n100 PRINT A\n110 END\n",
        expect: Expect::Stdout("7"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Dartmouth BASIC — `DEF FN` user-defined function (LANG-FULL BA5). The
    // single-line definition `DEF FNS(X) = X * X` lowers to a *sibling*
    // `IIRFunction` named `FNS` (one `f64` parameter, body `mul X X; ret`),
    // and `PRINT FNS(7)` lowers to an IIR `call` — exactly the calling
    // convention ALGOL's value procedures (AL3) already run on every backend.
    // This RUNS a real cross-function call combined with `print_i64` output:
    // `FNS(7)` returns 49, which `main` prints. Until this slice BASIC had no
    // user functions at all (`DEF` was an `UnsupportedStatement`). Proves that
    // a `call` to a same-module function resolves and executes on the code-gen
    // backends, not just the VM/JIT.
    Prog {
        lang: Language::DartmouthBasic,
        ext: "bas",
        src: "10 DEF FNS(X) = X * X\n20 PRINT FNS(7)\n30 END\n",
        expect: Expect::Stdout("49"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Dartmouth BASIC — *one-dimensional real arrays* (LANG-FULL BA3 + BA7,
    // enabler
    // **E5**). `DIM A(3)` lowers to an IIR `alloc_array` (element count `3 + 1`,
    // since BASIC arrays are **0-based and inclusive**: `A(0)..A(3)`). `LET
    // A(1) := 40` / `A(2) := 2` are `array_set`s into `array<f64>` storage and
    // `PRINT A(1) + A(2)` reads them back with two `array_get`s ⇒ prints 42.
    // These are the *same* IIR
    // array ops ALGOL's E5 arrays emit, so BASIC arrays run on every backend E5
    // already supports — straight-line (no loop), so the JVM's BA-JVM-1
    // loop+print StackMapTable follow-up doesn't apply, and all 7 backends run:
    // the managed runtimes (JVM `int[]`/`iastore`/`iaload`, CLR `int32[]`/
    // `stelem`/`ldelem`) bounds-check natively, while the static backends
    // (LLVM/WASM/NativeAot) use the length-prefixed `[i64 len][elems…]` block
    // with an explicit bounds `cmp`+trap.  `dartmouth-basic-iir-compiler` lowers
    // `DIM`/subscripted-`LET`/subscripted-read; the subscript is used directly
    // as the 0-based index (no lower-bound subtraction, unlike ALGOL `[lo:hi]`).
    Prog {
        lang: Language::DartmouthBasic,
        ext: "bas",
        src: "10 DIM A(3)\n20 LET A(1) = 40\n30 LET A(2) = 2\n40 PRINT A(1) + A(2)\n50 END\n",
        expect: Expect::Stdout("42"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Dartmouth BASIC — *`READ` / `DATA` / `RESTORE`* (LANG-FULL BA6 + BA7). The
    // `DATA` pool is materialised once at the top of `main` as an `array<f64>`
    // (the same E5 array ops BA3 uses) plus an `__basic_data_ptr` register seeded to 0;
    // `READ` does `array_get pool, ptr` then `ptr := ptr + 1`, and `RESTORE` resets
    // `ptr := 0`. Here `DATA 21` is a one-value pool: `READ A` takes 21 and advances
    // the pointer; `RESTORE` rewinds it; `READ B` therefore takes 21 *again* — so
    // `PRINT A + B` ⇒ 42, observably proving sequential consumption AND the rewind
    // in one program. Straight-line (no loop), so it runs on all 7 backends exactly
    // like the BA3 array cell — no new IIR op (pure frontend lowering onto the E5
    // array substrate). A non-rewinding READ would read past the 1-element pool and
    // trap, so 42 also proves the pointer/RESTORE arithmetic is correct.
    Prog {
        lang: Language::DartmouthBasic,
        ext: "bas",
        src: "10 DATA 21\n20 READ A\n30 RESTORE\n40 READ B\n50 PRINT A + B\n60 END\n",
        expect: Expect::Stdout("42"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Dartmouth BASIC — *multi-item `PRINT` on one line* with a `;` separator and
    // a negative value (LANG-FULL BA2). `PRINT 0 - 12; 34` prints `-12` and `34`
    // back-to-back on a SINGLE line ⇒ `-1234`. This is the headline BA2 proof:
    // the old lowering emitted one `call_builtin "print_i64"` per item, and
    // `print_i64` appends a newline, so the two items would have landed on
    // separate lines (`-12⏎34`). BA2 replaces that with a character-level model —
    // each item lowers to a `call __basic_print_int`, a synthetic *recursive*
    // helper that emits digits one at a time through the universal `putchar`
    // builtin (the very same builtin Brainfuck's `.` uses), then the statement
    // emits its own trailing newline. So this one cell proves, observably and on
    // every backend at once: (1) two items share a line (`;` joins with nothing
    // between), (2) the recursion renders multi-digit numbers left-to-right
    // (`12`, `34` — not `21`, `43`), and (3) the sign path runs (`-`). Because it
    // reuses only ops the matrix already runs everywhere — `call` (the ALGOL
    // value-procedure ABI), integer `div`/`mul`/`sub`/`add`, `cmp_*`, and
    // `putchar` — BA2 needed ZERO backend changes; no runtime learned anything
    // BASIC-specific. Straight-line `main` (no loop), so the JVM BA-JVM-1
    // loop+print StackMapTable follow-up doesn't apply and all 7 backends run.
    Prog {
        lang: Language::DartmouthBasic,
        ext: "bas",
        src: "10 PRINT 0 - 12; 34\n20 END\n",
        expect: Expect::Stdout("-1234"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Dartmouth BASIC — multi-item `PRINT` with a `,` separator (LANG-FULL BA2).
    // Where `;` joins tightly, `,` inserts a separator space: `PRINT 5, 6` ⇒
    // `5 6`. (Historical Dartmouth BASIC tabs `,` to the next 14-column print
    // zone; that needs a run-time output-column counter and is deferred — a
    // single space is BA2's well-defined approximation, documented in the
    // dartmouth-basic-iir-compiler CHANGELOG.) The inner space (a `putchar(32)`
    // between the two `__basic_print_int` calls) survives the harness's
    // outer-trim, so `5 6` distinguishes `,` from `;` observably on all 7
    // backends.
    Prog {
        lang: Language::DartmouthBasic,
        ext: "bas",
        src: "10 PRINT 5, 6\n20 END\n",
        expect: Expect::Stdout("5 6"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Dartmouth BASIC — BA7-1 scalar real arithmetic. Decimal spellings (`6.0`,
    // `7.0`) stay on the same `f64` value path as integer-spelled literals; `*`
    // is an `f64` multiply, and `PRINT` routes through `__basic_print_real`.
    // This proof stays intentionally whole-valued: 6.0 * 7.0 => 42.
    Prog {
        lang: Language::DartmouthBasic,
        ext: "bas",
        src: "10 PRINT 6.0 * 7.0\n20 END\n",
        expect: Expect::Stdout("42"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Dartmouth BASIC — BA7-2a fixed-decimal fractional `PRINT`. The helper
    // handles ordinary fractional values without backend-specific code: an
    // integer part plus trimmed fractional digits (`3.14`), a magnitude below 1
    // with no leading zero (`.25`), and a negative fractional value (`-2.5`).
    // This cell keeps the ordinary fixed-decimal smoke; the next BA7 cell covers
    // six-significant-digit rounding and `E` notation on all 7 backends.
    Prog {
        lang: Language::DartmouthBasic,
        ext: "bas",
        src: "10 PRINT 3.14\n20 PRINT 1.0 / 4.0\n30 PRINT 0.0 - 2.5\n40 END\n",
        expect: Expect::Stdout("3.14\n.25\n-2.5"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Dartmouth BASIC — BA7-2b historical real formatting. The helper now
    // rounds to six significant digits (`1.234567` -> `1.23457`) and switches
    // to signed, two-digit `E` notation for large and small magnitudes, while
    // retaining the no-leading-zero rule below 1.
    Prog {
        lang: Language::DartmouthBasic,
        ext: "bas",
        src: "10 PRINT 1.234567\n20 PRINT 123456789\n30 PRINT 0.0001234567\n40 PRINT 1.0 / 4.0\n50 END\n",
        expect: Expect::Stdout("1.23457\n1.23457E+08\n1.23457E-04\n.25"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Dartmouth BASIC — BA7-3 real aggregate storage. Fractional `DATA` values
    // are materialised in an `array<f64>` pool, `READ A(0)` stores one into a
    // BASIC `array<f64>`, and `READ B` stores the next into a scalar. Printing
    // both proves fractional DATA survives the pool, array element storage, and
    // scalar READ path on every backend.
    Prog {
        lang: Language::DartmouthBasic,
        ext: "bas",
        src: "10 DIM A(1)\n20 DATA 3.14, 0.25\n30 READ A(0)\n40 READ B\n50 PRINT A(0)\n60 PRINT B\n70 END\n",
        expect: Expect::Stdout("3.14\n.25"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Dartmouth BASIC — *unstructured `GOSUB` / `RETURN`* (LANG-FULL BA1, enabler
    // **E7**). The headline proof that one `RETURN` resumes at the *dynamically
    // most-recent* `GOSUB`: line 100 is `GOSUB`'d twice and its single `RETURN`
    // (line 110) must come back to two DIFFERENT places. `GOSUB 100` (push site 0)
    // prints `9`; `RETURN` pops 0 and resumes at line 20 → `1`; `GOSUB 100` again
    // (push site 1) prints `9`; `RETURN` pops 1 and resumes at line 40 → END.
    // Output `919` (trailing `;` from BA2 keeps it on one line). A fixed return
    // label couldn't do this — only the runtime return-address stack can.
    // `dartmouth-basic-iir-compiler` lowers this INSIDE `main` with NO new backend
    // op: an E5 `array<i64>` return stack (`alloc_array`/`array_set`/`array_get`,
    // the same ops BA3/BA6 run on every backend) + the AL5 computed-`goto` chain
    // (`cmp_eq` + `jmp_if_true`). So BA1 runs on all 7 backends exactly like the
    // array cells. Straight control flow + the BA2 print helpers — both already
    // green on JVM, so no BA-JVM-1 caveat.
    Prog {
        lang: Language::DartmouthBasic,
        ext: "bas",
        src: "10 GOSUB 100\n20 PRINT 1;\n30 GOSUB 100\n40 END\n\
               100 PRINT 9;\n110 RETURN\n",
        expect: Expect::Stdout("919"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Dartmouth BASIC — *nested* `GOSUB` (LANG-FULL BA1). A subroutine that itself
    // `GOSUB`s a second one before returning — proves the **LIFO stack discipline**
    // across depth > 1, not just a single level. `GOSUB 100` prints `8`, then
    // (line 110) `GOSUB 200` prints `7`, whose `RETURN` (line 210) must resume at
    // line 120 → `6`, whose `RETURN` (line 130) must resume at line 20 → END.
    // Output `876`: the inner return goes to 120 and the outer to 20, which only a
    // proper stack (not a single saved address) produces.
    Prog {
        lang: Language::DartmouthBasic,
        ext: "bas",
        src: "10 GOSUB 100\n20 END\n100 PRINT 8;\n110 GOSUB 200\n\
               120 PRINT 6;\n130 RETURN\n200 PRINT 7;\n210 RETURN\n",
        expect: Expect::Stdout("876"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
];

/// Is a usable native linker present on this host? On Linux/macOS the AOT path uses
/// the always-present system linker; on Windows it needs a real MSVC/LLD/gcc linker.
fn native_linker_ok() -> bool {
    if cfg!(target_os = "windows") {
        // Mirror twig-aot's probe: confirm a genuine linker, not git-bash's `link`.
        let probes: &[(&str, &str, &[&str])] = &[
            ("link.exe", "", &["Microsoft", "Linker"]),
            ("lld-link.exe", "", &["LLD"]),
            ("gcc.exe", "--version", &["gcc"]),
        ];
        probes.iter().any(|(name, arg, markers)| {
            let mut cmd = Command::new(name);
            if !arg.is_empty() {
                cmd.arg(arg);
            }
            cmd.output()
                .map(|o| {
                    let banner = format!(
                        "{}{}",
                        String::from_utf8_lossy(&o.stdout),
                        String::from_utf8_lossy(&o.stderr)
                    );
                    markers.iter().all(|m| banner.contains(m))
                })
                .unwrap_or(false)
        })
    } else {
        cfg!(any(target_os = "linux", target_os = "macos"))
    }
}

/// Compile `p` to a native executable for the host OS. `None` when the host can't
/// produce a native exe (skip), so the suite degrades gracefully off Linux/macOS.
fn compile_native(src_path: &std::path::Path, exe: &std::path::Path, lang: Language) -> Option<()> {
    #[cfg(target_os = "linux")]
    {
        lang_aot::compile_file_to_linux_executable(src_path, exe, lang).ok()
    }
    #[cfg(target_os = "macos")]
    {
        lang_aot::compile_file_to_macos_executable(src_path, exe, lang).ok()
    }
    #[cfg(target_os = "windows")]
    {
        lang_aot::compile_file_to_windows_executable(src_path, exe, lang).ok()
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        let _ = (src_path, exe, lang);
        None
    }
}

/// The stdin bytes a Brainfuck `,` program reads. The matrix is otherwise stdin-free —
/// every program above supplies none, so `getchar` sees EOF (the prior behaviour) — so
/// only the explicit stdin programs name their input here, keyed on `(lang, src)`.
/// Keeping the input in a small side table rather than a new `Prog` field avoids
/// editing every one of the ~30 existing `Prog` literals (and the churn/merge conflicts
/// that would cause in a fast-moving array); the Brainfuck sources are unique, so the
/// match is unambiguous.
fn program_stdin(p: &Prog) -> &'static [u8] {
    match (p.lang, p.src) {
        (Language::Brainfuck, ",+.") => b"A",   // read 'A' (65), `+` → 66, print 'B'
        (Language::Brainfuck, ",.,.") => b"Hi", // read a byte and echo it, twice → "Hi"
        (Language::Brainfuck, ",[.,]") => b"Hi", // cat: echo until EOF (EOF → 0 halts) → "Hi"
        _ => b"",
    }
}

/// Run `cmd` feeding `input` to its stdin, returning the finished output. The four
/// subprocess backends (native / LLVM / JVM / CLR) read a Brainfuck `,` from their
/// real process stdin — libc `getchar` (native/LLVM), `System.in` (JVM's BFRuntime),
/// `Console.Read` (CLR) — so this writes the program's input and closes the pipe (→
/// EOF). `input` is empty for every non-stdin program, so `write_all(b"")` is a no-op
/// and behaviour is unchanged. A write error is deliberately ignored: a program that
/// exits before reading closes the read end, and a resulting broken pipe must not fail
/// an otherwise-correct run. The captured stdout/stderr are piped so they don't leak to
/// the test's own console.
fn output_with_stdin(mut cmd: Command, input: &[u8]) -> Option<std::process::Output> {
    use std::io::Write as _;
    cmd.stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let mut child = cmd.spawn().ok()?;
    if let Some(mut si) = child.stdin.take() {
        let _ = si.write_all(input); // `si` dropped at block end → stdin closes → EOF
    }
    child.wait_with_output().ok()
}

/// Native-AOT runner: write the source, compile to a host executable, run it, and
/// return `(exit_code, trimmed_stdout)`. `None` when native AOT is unavailable here.
///
/// The programs are fixed literals (no untrusted input), and each terminates by
/// construction — there is no unbounded loop or recursion in the harness itself.
/// The work happens in a fresh `tempfile::tempdir()` (a random, `0700`, auto-removed
/// directory) rather than a predictable `temp_dir()/<pid>` path, so a local attacker
/// cannot pre-create the directory or plant a symlink at `prog` and have the harness
/// execute substituted code in the compile→run window (CWE-377/367). The `_dir`
/// guard is held until after the executable runs so it is not removed early.
fn run_native(p: &Prog) -> Option<(Option<i32>, String)> {
    if !native_linker_ok() {
        return None;
    }
    let dir = tempfile::tempdir().ok()?;
    let src_path = dir.path().join(format!("prog.{}", p.ext));
    std::fs::write(&src_path, p.src).ok()?;
    let exe = dir.path().join("prog");
    compile_native(&src_path, &exe, p.lang)?;
    // Feed the program's stdin (a Brainfuck `,` reads it via libc `getchar`); empty for
    // every other program, so the prior no-stdin behaviour is unchanged.
    let out = output_with_stdin(Command::new(&exe), program_stdin(p))?;
    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
    Some((out.status.code(), stdout))
}

/// Is a usable `clang` present? Gates the LLVM column (skip when absent).
fn clang_ok() -> bool {
    Command::new("clang")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// A minimal C runtime providing the generic print primitives that `iir-to-llvm`
/// emits for I/O languages. `__print_i64` backs scalar BASIC `PRINT`, and
/// `__print_str` backs the E4 string literal-output foothold. It is not
/// language-specific: any IIR that references either symbol links it. Linked only
/// when the emitted `.ll` actually references one of the symbols, so the bare
/// expression-language programs still link a standalone `.ll`.
const PRINT_RUNTIME_C: &str =
    "#include <stdio.h>\n#include <stdint.h>\nvoid __print_i64(int64_t x){printf(\"%lld\\n\",(long long)x);}\nvoid __print_str(const char* p,int64_t len){if(len>0){fwrite(p,1,(size_t)len,stdout);}}\n";

/// LLVM runner: source → textual `.ll` (`iir-to-llvm`) → real `clang` → run, the
/// exact CLR-real/McCarthy strategy of handing symbolic code to the real toolchain.
/// `None` when `clang` is absent or the build fails (skip).
///
/// Handles both result kinds: the expression languages return an exit code from a
/// bare `.ll`; an I/O language (Dartmouth BASIC) emits `@__print_i64` or
/// `@__print_str`, so when the `.ll` references either symbol the generic
/// `PRINT_RUNTIME_C` is compiled in and the harness compares the program's
/// **stdout**.
///
/// Same temp-file hardening as `run_native`: a fresh `tempfile::tempdir()` whose
/// guard outlives the run, so the executed `prog` cannot be substituted (CWE-377/367).
fn run_llvm(p: &Prog) -> Option<(Option<i32>, String)> {
    if !clang_ok() {
        return None;
    }
    let triple = String::from_utf8(
        Command::new("clang").arg("-dumpmachine").output().ok()?.stdout,
    )
    .ok()?
    .trim()
    .to_string();
    let ll = lang_aot::compile_source_to_llvm_with_target(p.lang, p.src, "lm", &triple).ok()?;
    let dir = tempfile::tempdir().ok()?;
    let ll_path = dir.path().join("prog.ll");
    std::fs::write(&ll_path, &ll).ok()?;
    let exe = dir.path().join("prog");
    let mut cmd = Command::new("clang");
    cmd.arg("-x").arg("ir").arg(&ll_path);
    // Link the generic print runtime iff the program actually prints.
    if ll.contains("@__print_i64") || ll.contains("@__print_str") {
        let rt_path = dir.path().join("rt.c");
        std::fs::write(&rt_path, PRINT_RUNTIME_C).ok()?;
        cmd.arg("-x").arg("c").arg(&rt_path);
    }
    let built = cmd.arg("-x").arg("none").arg("-o").arg(&exe).output().ok()?;
    if !built.status.success() {
        return None;
    }
    // Same stdin wiring as `run_native`: a Brainfuck `,` reads libc `getchar` from the
    // process stdin; empty for every other program.
    let out = output_with_stdin(Command::new(&exe), program_stdin(p))?;
    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
    Some((out.status.code(), stdout))
}

/// The generic stdout primitive an I/O language's wasm emits. Dartmouth BASIC's
/// `PRINT` lowers to `call $__print_i64`, imported as `env.__print_i64 : (i64) -> ()`
/// — the wasm sibling of the LLVM column's `@__print_i64` C runtime, the JVM's
/// `BasicRuntime.println(J)V`, and the CLR's `Console.WriteLine(int64)`. It is *not*
/// language-specific: any IIR that prints an integer routes through this import.
///
/// `PrintFunc` is the host implementation of that import. Each call appends its single
/// `i64` argument to a shared capture buffer (`Arc<Mutex<Vec<i64>>>`) so the test can
/// read back exactly what the program printed. The function does no work proportional
/// to untrusted input — it pushes one integer and returns — so there is no DoS vector.
struct PrintFunc {
    captured: std::sync::Arc<std::sync::Mutex<Vec<i64>>>,
}

impl wasm_execution::HostFunction for PrintFunc {
    fn func_type(&self) -> &wasm_types::FuncType {
        // `(i64) -> ()`: one i64 in, nothing out. A `LazyLock` static gives the
        // `&FuncType` the trait must hand back a stable lifetime.
        static FT: std::sync::LazyLock<wasm_types::FuncType> =
            std::sync::LazyLock::new(|| wasm_types::FuncType {
                params: vec![wasm_types::ValueType::I64],
                results: vec![],
            });
        &FT
    }

    fn call(
        &self,
        args: &[wasm_execution::WasmValue],
        _memory: Option<&mut wasm_execution::LinearMemory>,
    ) -> Result<Vec<wasm_execution::WasmValue>, wasm_execution::TrapError> {
        let value = args
            .first()
            .ok_or_else(|| wasm_execution::TrapError::new("__print_i64: missing argument"))?
            .as_i64()
            .map_err(|e| wasm_execution::TrapError::new(e.message))?;
        self.captured
            .lock()
            .expect("lang-matrix print buffer poisoned")
            .push(value);
        Ok(vec![])
    }
}

/// Brainfuck's `.` lowers to `call $putchar`, imported as `env.putchar : (i32) -> ()`
/// (the wasm sibling of the LLVM column's libc `@putchar`). `PutcharFunc` is the host
/// implementation: each call appends the low byte of its i32 argument to a shared byte
/// buffer, so the test reads back the exact bytes the program wrote — Brainfuck's `.`
/// of cell value 65 produces the byte `A`, giving stdout `"A"` (NOT the decimal `"65"`
/// that `__print_i64` would). One byte pushed per call — no DoS vector.
struct PutcharFunc {
    bytes: std::sync::Arc<std::sync::Mutex<Vec<u8>>>,
}

impl wasm_execution::HostFunction for PutcharFunc {
    fn func_type(&self) -> &wasm_types::FuncType {
        static FT: std::sync::LazyLock<wasm_types::FuncType> =
            std::sync::LazyLock::new(|| wasm_types::FuncType {
                params: vec![wasm_types::ValueType::I32],
                results: vec![],
            });
        &FT
    }

    fn call(
        &self,
        args: &[wasm_execution::WasmValue],
        _memory: Option<&mut wasm_execution::LinearMemory>,
    ) -> Result<Vec<wasm_execution::WasmValue>, wasm_execution::TrapError> {
        let value = args
            .first()
            .ok_or_else(|| wasm_execution::TrapError::new("putchar: missing argument"))?
            .as_i32()
            .map_err(|e| wasm_execution::TrapError::new(e.message))?;
        self.bytes
            .lock()
            .expect("lang-matrix putchar buffer poisoned")
            .push((value & 0xFF) as u8);
        Ok(vec![])
    }
}

struct PrintStrFunc {
    bytes: std::sync::Arc<std::sync::Mutex<Vec<u8>>>,
}

impl wasm_execution::HostFunction for PrintStrFunc {
    fn func_type(&self) -> &wasm_types::FuncType {
        static FT: std::sync::LazyLock<wasm_types::FuncType> =
            std::sync::LazyLock::new(|| wasm_types::FuncType {
                params: vec![wasm_types::ValueType::I32, wasm_types::ValueType::I32],
                results: vec![],
            });
        &FT
    }

    fn call(
        &self,
        args: &[wasm_execution::WasmValue],
        memory: Option<&mut wasm_execution::LinearMemory>,
    ) -> Result<Vec<wasm_execution::WasmValue>, wasm_execution::TrapError> {
        let ptr = args
            .first()
            .ok_or_else(|| wasm_execution::TrapError::new("__print_str: missing ptr"))?
            .as_i32()
            .map_err(|e| wasm_execution::TrapError::new(e.message))?;
        let len = args
            .get(1)
            .ok_or_else(|| wasm_execution::TrapError::new("__print_str: missing len"))?
            .as_i32()
            .map_err(|e| wasm_execution::TrapError::new(e.message))?;
        if ptr < 0 || len < 0 {
            return Err(wasm_execution::TrapError::new("__print_str: negative ptr/len"));
        }
        let memory = memory
            .ok_or_else(|| wasm_execution::TrapError::new("__print_str: no linear memory"))?;
        let start = usize::try_from(ptr)
            .map_err(|_| wasm_execution::TrapError::new("__print_str: ptr overflow"))?;
        let len = usize::try_from(len)
            .map_err(|_| wasm_execution::TrapError::new("__print_str: len overflow"))?;
        let mut chunk = Vec::with_capacity(len);
        for offset in 0..len {
            chunk.push(memory.load_i32_8u(start + offset)? as u8);
        }
        self.bytes
            .lock()
            .expect("lang-matrix print_str buffer poisoned")
            .extend_from_slice(&chunk);
        Ok(vec![])
    }
}

/// Brainfuck's `,` lowers to `call $getchar`, imported as `env.getchar : () -> i32`
/// (the wasm sibling of libc `@getchar`). Each call pops the next byte from the program's
/// stdin buffer (seeded by `run_wasm` from `program_stdin`); when the buffer is drained it
/// returns `-1` (EOF) — the conventional Brainfuck "leave 255 on EOF" after the cell store
/// truncates it, matching the libc `getchar` column. A program with no stdin (every cell
/// but the B1-stdin ones) gets an empty buffer, so the first read is EOF, exactly the
/// previous behaviour.
struct GetcharFunc {
    input: std::sync::Arc<std::sync::Mutex<std::collections::VecDeque<u8>>>,
}

impl wasm_execution::HostFunction for GetcharFunc {
    fn func_type(&self) -> &wasm_types::FuncType {
        static FT: std::sync::LazyLock<wasm_types::FuncType> =
            std::sync::LazyLock::new(|| wasm_types::FuncType {
                params: vec![],
                results: vec![wasm_types::ValueType::I32],
            });
        &FT
    }

    fn call(
        &self,
        _args: &[wasm_execution::WasmValue],
        _memory: Option<&mut wasm_execution::LinearMemory>,
    ) -> Result<Vec<wasm_execution::WasmValue>, wasm_execution::TrapError> {
        let byte = self
            .input
            .lock()
            .expect("lang-matrix wasm stdin buffer poisoned")
            .pop_front();
        let code = byte.map(i32::from).unwrap_or(-1); // EOF → -1
        Ok(vec![wasm_execution::WasmValue::I32(code)])
    }
}

/// The host interface the matrix runs wasm under: it resolves the generic
/// `env.__print_i64` import to a `PrintFunc` (integer capture, for BASIC),
/// `env.__print_str` to a memory-reading byte capture, and the Brainfuck I/O
/// imports `env.putchar`/`env.getchar` to a `PutcharFunc` (byte capture) /
/// `GetcharFunc` (EOF). Everything else resolves to nothing (the expression languages
/// import no host functions, so the host is never consulted for them and behaviour is
/// identical to `WasmRuntime::new`).
struct PrintHost {
    captured: std::sync::Arc<std::sync::Mutex<Vec<i64>>>,
    bytes: std::sync::Arc<std::sync::Mutex<Vec<u8>>>,
    input: std::sync::Arc<std::sync::Mutex<std::collections::VecDeque<u8>>>,
}

impl wasm_execution::HostInterface for PrintHost {
    fn resolve_function(
        &self,
        module_name: &str,
        name: &str,
    ) -> Option<Box<dyn wasm_execution::HostFunction>> {
        match (module_name, name) {
            ("env", "__print_i64") => Some(Box::new(PrintFunc {
                captured: std::sync::Arc::clone(&self.captured),
            })),
            ("env", "__print_str") => Some(Box::new(PrintStrFunc {
                bytes: std::sync::Arc::clone(&self.bytes),
            })),
            ("env", "putchar") => Some(Box::new(PutcharFunc {
                bytes: std::sync::Arc::clone(&self.bytes),
            })),
            ("env", "getchar") => Some(Box::new(GetcharFunc {
                input: std::sync::Arc::clone(&self.input),
            })),
            _ => None,
        }
    }

    fn resolve_global(
        &self,
        _module_name: &str,
        _name: &str,
    ) -> Option<(wasm_types::GlobalType, wasm_execution::WasmValue)> {
        None
    }

    fn resolve_memory(
        &self,
        _module_name: &str,
        _name: &str,
    ) -> Option<wasm_execution::LinearMemory> {
        None
    }

    fn resolve_table(&self, _module_name: &str, _name: &str) -> Option<wasm_execution::Table> {
        None
    }
}

/// WASM runner: source → wasm bytes (`iir-to-wasm`) → the in-process `wasm-runtime`,
/// run under a `PrintHost` so an I/O language's `env.__print_i64` import resolves.
/// No external tool — the runtime is in-repo, so this always runs (returns `None`
/// only when the program fails to emit or the runtime can't load it). Handles both
/// result kinds: an expression language returns its value as `main`'s wasm result
/// (the `code`); an I/O language (Dartmouth BASIC) prints through `env.__print_i64`,
/// whose arguments the host captured into the buffer, joined as the program's stdout.
fn run_wasm(p: &Prog) -> Option<(Option<i32>, String)> {
    let wasm = lang_aot::compile_source_to_wasm(p.lang, p.src, "main").ok()?;
    let captured = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let byte_buf = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    // The program's stdin, drained by `env.getchar` (Brainfuck `,`); empty otherwise.
    let input = std::sync::Arc::new(std::sync::Mutex::new(
        program_stdin(p).iter().copied().collect::<std::collections::VecDeque<u8>>(),
    ));
    let host = PrintHost {
        captured: std::sync::Arc::clone(&captured),
        bytes: std::sync::Arc::clone(&byte_buf),
        input: std::sync::Arc::clone(&input),
    };
    let rt = wasm_runtime::WasmRuntime::with_host(Box::new(host));
    let result = rt.load_and_run(&wasm, "main", &[]).ok()?;
    // `main`'s single i64 result is the program's value (`& 0xFF` matches the exit
    // convention the native/LLVM columns use for the same programs).
    let code = result.first().copied().map(|v| (v as i32) & 0xFF);
    // stdout has two shapes: Brainfuck writes raw bytes via `env.putchar` (so `.` of 65
    // is the byte `A`); BASIC writes integers via `env.__print_i64` (one per call, joined
    // by newlines). A program uses one or the other, so prefer the byte stream when the
    // program wrote any. Expression languages print nothing → empty stdout.
    let printed_bytes = byte_buf.lock().expect("lang-matrix putchar buffer poisoned");
    let stdout = if printed_bytes.is_empty() {
        captured
            .lock()
            .expect("lang-matrix print buffer poisoned")
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        // `.trim()` to match the six sibling `run_*` functions (native/LLVM/CLR/
        // JVM/VM/JIT all trim): BASIC's BA2 `PRINT` ends each line with a
        // `putchar('\n')`, so the raw byte stream for `PRINT 42` is `"42\n"`.
        // Without trimming, the Wasm column alone disagreed with the others on
        // every BASIC `Stdout` cell (a latent inconsistency the putchar print
        // model surfaced). Inner newlines (multi-line output) are preserved.
        String::from_utf8_lossy(&printed_bytes).trim().to_string()
    };
    Some((code, stdout))
}

/// Is a usable `java` present? Gates the JVM column (skip when absent), exactly as
/// `clang_ok` gates LLVM and the W16 suite gates its external backends.
fn java_ok() -> bool {
    Command::new("java")
        .arg("-version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// A minimal `env.BasicRuntime` host class providing the `println(long)` primitive
/// that `iir-to-jvm-class-file` emits for Dartmouth BASIC's `PRINT` (`print_i64` →
/// `invokestatic env/BasicRuntime.println(J)V`). It is the JVM sibling of the wasm
/// column's `env.__print_i64` host import / the LLVM column's `@__print_i64` C
/// runtime, and is *not* language-specific: any IIR that prints an integer links it.
/// `run_jvm` compiles it with `javac` onto the classpath only when running an I/O
/// program, so the expression languages still run a standalone `Main.class`.
const BASIC_RUNTIME_JAVA: &str =
    "package env; public final class BasicRuntime { public static void println(long x){ System.out.println(x); } }";

/// The `env.BFRuntime` host class for Brainfuck (LANG-MATRIX LM-J). `iir-to-jvm-class-file`
/// lowers Brainfuck's tape to a static `byte[] __tape` field (`getstatic … __tape : [B` +
/// `baload`/`bastore`) and its `.`/`,` to `invokestatic env/BFRuntime.putchar(I)V` /
/// `getchar()I` — the JVM sibling of the LLVM column's libc `putchar`/`getchar` and the
/// wasm column's `env.putchar`/`env.getchar` host imports. `putchar` writes a raw byte to
/// stdout (so `.` of cell value 65 yields the byte `A`, not the decimal `65`); `getchar`
/// returns `0` at EOF (the matrix supplies no stdin). The 30 000-cell tape is zero-filled
/// by `new byte[30000]`, matching the `alloc_bytes` tape size.
const BF_RUNTIME_JAVA: &str = "package env; public final class BFRuntime { \
public static byte[] __tape = new byte[30000]; \
public static void putchar(int c){ System.out.write(c & 0xFF); System.out.flush(); } \
public static int getchar(){ try { int b = System.in.read(); return b < 0 ? 0 : b; } catch (java.io.IOException e) { return 0; } } }";

/// JVM runner: source → `JvmClassFile` (`iir-to-jvm-class-file`) → real `java`, the
/// W16 wrapper-launcher strategy generalized from McCarthy to **any** language.
///
/// `compile_source_to_jvm_class` emits a class `Main` whose entry method is `main`
/// with descriptor `()I` (an expression language's `int` result) or `()J` (an I/O
/// program left at its native `long` width — see `concretize_scalar_any_for_jvm`),
/// but a class run by `java` needs a `main([Ljava/lang/String;)V` entry point. So we
/// read the entry's real return descriptor and inject one of two launcher methods,
/// keyed on the program's result kind:
///
/// * **Expression language** (`Expect::Exit`) — print the entry method's result so
///   the harness can read it back:
///   ```text
///     getstatic  System.out         // : PrintStream
///     invokestatic Main.main()<R>   // : the program's result
///     invokevirtual println(<R>)V   // print it  (<R> = I or J)
///     return
///   ```
/// * **I/O language** (`Expect::Stdout`, Dartmouth BASIC) — the program writes its
///   own output through `env.BasicRuntime.println` as a side effect, so the launcher
///   merely runs `main` and **discards** its result (`pop` / `pop2`), and the host
///   class is compiled onto the classpath:
///   ```text
///     invokestatic Main.main()<R>   // runs the program (prints as a side effect)
///     pop / pop2                    // discard the (unused) return value
///     return
///   ```
///
/// (Two methods named `main` with different descriptors is legal — the JVM keys
/// methods on name **and** descriptor.) `None` when `java` (or, for I/O, `javac`) is
/// absent or any step fails (skip). We deliberately do **not** pass `-Xverify:none`:
/// the emitted bytecode is well-formed, so full verification is a *stronger* check
/// and rejects any malformed bytecode cleanly (a `VerifyError` → non-zero exit)
/// instead of executing it and SIGSEGV-crashing with an `hs_err_pid*.log` dump.
///
/// Security/termination: the program is a fixed literal (no untrusted input); the
/// emitted class + host source are written into a fresh `tempfile::tempdir()`
/// (random, `0700`, auto-removed) whose guard outlives the run, so neither the
/// executed `Main.class` nor the `javac`-compiled host can be substituted in the
/// write→run window (CWE-377/367); the class name is the constant `"Main"`, never
/// interpolated from input; and each program terminates by construction.
fn run_jvm(p: &Prog) -> Option<(Option<i32>, String)> {
    use iir_to_jvm_class_file::serialize_jvm_class_file;
    use jvm_class_file::{
        JvmCodeAttribute, JvmConstantPoolEntry, JvmMethodAttribute, JvmMethodInfo, ACC_PUBLIC,
        ACC_STATIC,
    };
    if !java_ok() {
        return None;
    }
    fn cp_append(cp: &mut Vec<Option<JvmConstantPoolEntry>>, e: JvmConstantPoolEntry) -> u16 {
        cp.push(Some(e));
        (cp.len() - 1) as u16
    }
    let prints = matches!(p.expect, Expect::Stdout(_));
    let mut class = lang_aot::compile_source_to_jvm_class(p.lang, p.src, "Main").ok()?;
    // The entry method's real return type — `I` (int) for the expression languages,
    // `J` (long) for a printing program. The launcher must match it exactly.
    let entry_desc = class.methods.iter().find(|m| m.name == "main")?.descriptor.clone();
    let ret = entry_desc.rsplit(')').next()?.to_string();

    // Build the constant-pool entries the launcher references. Always a self-ref to
    // `Main.main<entry_desc>`; for the print path also `System.out` and the matching
    // `println(<R>)V`.
    let (entry_ref, print_refs) = {
        let cp = &mut class.constant_pool;
        let print_refs = if prints {
            None
        } else {
            let sys_utf8 = cp_append(cp, JvmConstantPoolEntry::Utf8("java/lang/System".into()));
            let sys_class = cp_append(cp, JvmConstantPoolEntry::Class { name_index: sys_utf8 });
            let out_utf8 = cp_append(cp, JvmConstantPoolEntry::Utf8("out".into()));
            let ps_desc = cp_append(cp, JvmConstantPoolEntry::Utf8("Ljava/io/PrintStream;".into()));
            let out_nat = cp_append(
                cp,
                JvmConstantPoolEntry::NameAndType { name_index: out_utf8, descriptor_index: ps_desc },
            );
            let out_fieldref = cp_append(
                cp,
                JvmConstantPoolEntry::Fieldref { class_index: sys_class, name_and_type_index: out_nat },
            );
            let ps_utf8 = cp_append(cp, JvmConstantPoolEntry::Utf8("java/io/PrintStream".into()));
            let ps_class = cp_append(cp, JvmConstantPoolEntry::Class { name_index: ps_utf8 });
            let pln_utf8 = cp_append(cp, JvmConstantPoolEntry::Utf8("println".into()));
            let pln_desc = cp_append(cp, JvmConstantPoolEntry::Utf8(format!("({ret})V")));
            let pln_nat = cp_append(
                cp,
                JvmConstantPoolEntry::NameAndType { name_index: pln_utf8, descriptor_index: pln_desc },
            );
            let println_ref = cp_append(
                cp,
                JvmConstantPoolEntry::Methodref { class_index: ps_class, name_and_type_index: pln_nat },
            );
            Some((out_fieldref, println_ref))
        };
        let main_utf8 = cp_append(cp, JvmConstantPoolEntry::Utf8("Main".into()));
        let main_class = cp_append(cp, JvmConstantPoolEntry::Class { name_index: main_utf8 });
        let ent_name = cp_append(cp, JvmConstantPoolEntry::Utf8("main".into()));
        let ent_desc = cp_append(cp, JvmConstantPoolEntry::Utf8(entry_desc.clone()));
        let ent_nat = cp_append(
            cp,
            JvmConstantPoolEntry::NameAndType { name_index: ent_name, descriptor_index: ent_desc },
        );
        let entry_ref = cp_append(
            cp,
            JvmConstantPoolEntry::Methodref { class_index: main_class, name_and_type_index: ent_nat },
        );
        let _ = cp_append(cp, JvmConstantPoolEntry::Utf8("([Ljava/lang/String;)V".into()));
        (entry_ref, print_refs)
    };
    let [ent_hi, ent_lo] = entry_ref.to_be_bytes();
    let main_code = match print_refs {
        Some((out_fieldref, println_ref)) => {
            let [out_hi, out_lo] = out_fieldref.to_be_bytes();
            let [pln_hi, pln_lo] = println_ref.to_be_bytes();
            vec![
                0xB2, out_hi, out_lo, // getstatic System.out
                0xB8, ent_hi, ent_lo, // invokestatic Main.main()<R>
                0xB6, pln_hi, pln_lo, // invokevirtual println(<R>)V
                0xB1, // return
            ]
        }
        None => {
            // Discard the entry result: `pop2` for a wide (long/double) value, `pop`
            // for a single-slot value, nothing for a `void` entry.
            let discard: &[u8] = match ret.as_str() {
                "J" | "D" => &[0x58], // pop2
                "V" => &[],
                _ => &[0x57], // pop
            };
            let mut code = vec![0xB8, ent_hi, ent_lo]; // invokestatic Main.main()<R>
            code.extend_from_slice(discard);
            code.push(0xB1); // return
            code
        }
    };
    class.methods.push(JvmMethodInfo {
        access_flags: ACC_PUBLIC | ACC_STATIC,
        name: "main".into(),
        descriptor: "([Ljava/lang/String;)V".into(),
        attributes: vec![JvmMethodAttribute::Code(JvmCodeAttribute {
            name: "Code".into(),
            max_stack: 2,
            max_locals: 1,
            code: main_code,
            nested_attributes: vec![],
        })],
    });
    let bytes = serialize_jvm_class_file(&class);
    let dir = tempfile::tempdir().ok()?;
    std::fs::write(dir.path().join("Main.class"), &bytes).ok()?;
    // For an I/O program, compile the `env.BasicRuntime` host class onto the
    // classpath so its `println(J)V` resolves. `javac` ships with the JDK; if it is
    // somehow absent the cell skips gracefully (`None`).
    if prints {
        // Pick the host class the program's I/O lowers to. Both Brainfuck's
        // `.`/`,` and — since BA2 — Dartmouth BASIC's `PRINT` lower to the
        // generic `putchar` builtin (`invokestatic env/BFRuntime.putchar(I)V`),
        // so both use `env.BFRuntime`. (The legacy `env.BasicRuntime.println`
        // path is kept for any future language that lowers `print_i64`.)
        let (file, source) = if p.lang == Language::Brainfuck
            || p.lang == Language::DartmouthBasic
        {
            ("BFRuntime.java", BF_RUNTIME_JAVA)
        } else {
            ("BasicRuntime.java", BASIC_RUNTIME_JAVA)
        };
        let src = dir.path().join(file);
        std::fs::write(&src, source).ok()?;
        let built = Command::new("javac").arg("-d").arg(dir.path()).arg(&src).output().ok()?;
        if !built.status.success() {
            return None;
        }
    }
    // A Brainfuck `,` reads `env.BFRuntime.getchar()` → `System.in`, so pipe the
    // program's stdin to the `java` process; empty for every other program.
    let mut java = Command::new("java");
    java.arg("-cp").arg(dir.path()).arg("Main");
    let out = output_with_stdin(java, program_stdin(p))?;
    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if prints {
        // The program wrote its result to stdout via `env.BasicRuntime.println`.
        Some((out.status.code(), stdout))
    } else {
        // The launcher printed the entry method's result; parse it as the program's
        // value (matching the exit-code convention of the other columns).
        Some((stdout.parse::<i32>().ok(), String::new()))
    }
}

/// Is `dotnet` present? Together with `find_ilasm` this gates the CLR column.
fn dotnet_ok() -> bool {
    Command::new("dotnet")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// CLR runner: source → textual `.il` (`iir-to-cil-bytecode`) → real `ilasm` → real
/// `dotnet` — the CLR-real path of the McCarthy CLR chapter, generalized to any
/// language. `compile_source_to_cil_text` emits a `Main` whose entry computes the
/// program's value and `Console.WriteLine`s it, so (like the McCarthy runner) we
/// assemble the `.il` to a real PE with `ilasm -exe`, run it on `dotnet`, and parse
/// the printed integer. Gated on `dotnet` **and** a locatable `ilasm` (the assembler
/// ships only in a NuGet runtime pack — `clr_support::find_ilasm` walks the cache);
/// skips gracefully when either is absent.
///
/// Security/termination: the program is a fixed literal (no untrusted input); the
/// `.il`, the assembled `Main.dll` and its `runtimeconfig.json` are written into a
/// fresh `tempfile::tempdir()` (random, `0700`, auto-removed) whose guard outlives
/// the run, so the executed assembly cannot be substituted in the assemble→run
/// window (CWE-377/367); the class name is the constant `"Main"`, never from input;
/// and each program terminates by construction.
fn run_clr(p: &Prog) -> Option<(Option<i32>, String)> {
    if !dotnet_ok() {
        return None;
    }
    let ilasm = clr_support::find_ilasm()?;
    let il = lang_aot::compile_source_to_cil_text(p.lang, p.src, "Main").ok()?;
    let dir = tempfile::tempdir().ok()?;
    let il_path = dir.path().join("Main.il");
    std::fs::write(&il_path, &il).ok()?;
    let dll = dir.path().join("Main.dll");
    let asm = Command::new(&ilasm)
        .arg("-dll=false")
        .arg("-exe")
        .arg(format!("-output={}", dll.display()))
        .arg(&il_path)
        .output()
        .ok()?;
    if !asm.status.success() || !dll.exists() {
        return None;
    }
    // A `.runtimeconfig.json` is required to launch a framework-dependent assembly
    // on `dotnet`; pin it to the same net9.0 runtime the McCarthy CLR-real tests use.
    std::fs::write(
        dir.path().join("Main.runtimeconfig.json"),
        r#"{ "runtimeOptions": { "tfm": "net9.0", "framework": { "name": "Microsoft.NETCore.App", "version": "9.0.0" } } }"#,
    )
    .ok()?;
    // A Brainfuck `,` reads `Console.Read()` from the process stdin, so pipe the
    // program's stdin to the `dotnet` process; empty for every other program.
    let mut dn = Command::new("dotnet");
    dn.arg(&dll);
    let out = output_with_stdin(dn, program_stdin(p))?;
    if !out.status.success() {
        return None;
    }
    // Whatever the program wrote to `Console`: for an expression language that's the
    // launcher's `Console.WriteLine` of the entry's `int` result (parsed as the value,
    // matching the exit-code convention); for an I/O language (Dartmouth BASIC) it's
    // the `PRINT` output captured directly. Return both — `assert_cell` picks the one
    // the program's `Expect` cares about.
    let printed = String::from_utf8_lossy(&out.stdout).trim().to_string();
    Some((printed.parse::<i32>().ok(), printed))
}

/// VM runner: source → IIR (`compile_source_to_iir`) → the **generic register VM**
/// (`vm_core::VMCore`) interpreting the shared IIR directly (Phase V). This is the
/// in-process, run-anywhere analog of the code-gen columns: the *same* `IIRModule` the
/// LLVM/WASM/JVM/CLR backends compile is instead **interpreted** by `VMCore`, whose
/// instruction dispatch already covers the arithmetic / comparison / bitwise /
/// control-flow / memory / `call_builtin` ops every scalar language emits. There is no
/// per-language code here — a future Ruby/JS frontend that lowers to IIR would run the
/// same way. (McCarthy lisp uses its own `LispyValue` VM; the matrix's six languages are
/// all scalar, so they share this one.)
///
/// The I/O languages print through `call_builtin`, which `VMCore` dispatches to a
/// **registered builtin closure**: `print_i64` (Dartmouth BASIC's `PRINT`) appends its
/// integer argument to a capture buffer — the VM sibling of the wasm `PrintHost` import /
/// the LLVM `@__print_i64` C runtime / the JVM `BasicRuntime` / the CLR `Console.WriteLine`.
/// (`putchar`/`getchar`, for Brainfuck, are registered too but unused until the byte-tape
/// ops land on `VMCore` — Brainfuck-on-VM is the next slice.)
///
/// An expression language's `main` returns an `Int`, used as the exit code (`& 0xFF`, the
/// other columns' convention); an I/O language's stdout is the captured buffer. `None`
/// only if the program fails to compile or the VM errors — the VM is in-process, so a
/// tagged cell always runs (no host gate).
fn run_vm(p: &Prog) -> Option<(Option<i32>, String)> {
    use std::sync::{Arc, Mutex};
    use vm_core::core::VMCore;
    use vm_core::value::Value;

    let mut module = lang_aot::compile_source_to_iir(p.lang, p.src, "main").ok()?;
    let entry = module.entry_point.clone().unwrap_or_else(|| "main".to_string());

    let mut vm = VMCore::new();

    // Capture buffer for the I/O languages. `print_i64` (BASIC) appends one integer per
    // call, joined by newlines; `putchar` (Brainfuck) appends one byte. A program uses at
    // most one, so a single buffer + a byte buffer suffice; expression languages print
    // nothing. The closures push a bounded amount per call — no DoS vector.
    let printed_ints: Arc<Mutex<Vec<i64>>> = Arc::new(Mutex::new(Vec::new()));
    let printed_bytes: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));

    let ints = Arc::clone(&printed_ints);
    vm.builtins_mut().register("print_i64", move |args: &[Value]| {
        let n = args.first().and_then(|v| v.as_i64()).unwrap_or(0);
        ints.lock().expect("lang-matrix VM print buffer poisoned").push(n);
        Ok(Value::Null)
    });
    let bytes = Arc::clone(&printed_bytes);
    vm.builtins_mut().register("putchar", move |args: &[Value]| {
        let b = (args.first().and_then(|v| v.as_i64()).unwrap_or(0) & 0xFF) as u8;
        bytes.lock().expect("lang-matrix VM putchar buffer poisoned").push(b);
        Ok(Value::Null)
    });
    let bytes = Arc::clone(&printed_bytes);
    vm.builtins_mut().register("print_str", move |args: &[Value]| {
        let s = args.first().and_then(Value::as_str).unwrap_or("");
        bytes
            .lock()
            .expect("lang-matrix VM print_str buffer poisoned")
            .extend_from_slice(s.as_bytes());
        Ok(Value::Null)
    });
    // The program's stdin, drained one byte per `getchar` (Brainfuck `,`); empty for
    // every other program, so the first read is EOF → 0 (the prior behaviour).
    let stdin_buf: Arc<Mutex<std::collections::VecDeque<u8>>> =
        Arc::new(Mutex::new(program_stdin(p).iter().copied().collect()));
    let input = Arc::clone(&stdin_buf);
    vm.builtins_mut().register("getchar", move |_args: &[Value]| {
        // Pop the next stdin byte; EOF → 0 (BF convention) once the buffer is drained.
        let byte = input.lock().expect("lang-matrix VM stdin buffer poisoned").pop_front();
        Ok(Value::Int(byte.map(i64::from).unwrap_or(0)))
    });

    let result = vm.execute(&mut module, &entry, &[]).ok()?;

    // The exit code: an expression language's `main` returns an `Int`.
    let code = result.and_then(|v| v.as_i64()).map(|n| (n as i32) & 0xFF);
    // stdout: prefer the byte stream (Brainfuck `putchar`) when present, else the integer
    // stream (BASIC `print_i64`) joined by newlines; empty for the expression languages.
    let byte_buf = printed_bytes.lock().expect("lang-matrix VM putchar buffer poisoned");
    let stdout = if byte_buf.is_empty() {
        printed_ints
            .lock()
            .expect("lang-matrix VM print buffer poisoned")
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        // `.trim()` to match the subprocess columns: BA2 BASIC `PRINT` ends
        // each line with `putchar('\n')`, so the byte stream is e.g. `"42\n"`.
        String::from_utf8_lossy(&byte_buf).trim().to_string()
    };
    Some((code, stdout))
}

/// Run a program through the **generic JIT** and return `(exit_code, stdout)`.
///
/// This is the execution-time sibling of [`run_vm`]: same shared `IIRModule`, but
/// driven by `jit_core::JITCore` over the language-agnostic `GenericCirJit` backend
/// instead of the bare interpreter. `execute_with_jit` eagerly compiles every
/// fully-typed function to JIT bytecode (installing a native handler) and interprets
/// the rest, so the program runs *through the JIT pipeline*. A compiled function with
/// parameters — e.g. Nib's `double(x)` — reads its arguments because `GenericCirJit`
/// pre-binds params to registers `0..n` and `JITCore` seeds those registers from the
/// call args; that's the generic register-VM/JIT contract a future Ruby/JS frontend
/// reuses unchanged, with **zero** language-specific code here.
///
/// The I/O builtins must be registered on **both** the VM (the interpreter-fallback
/// path) and the `GenericCirJit` backend (the compiled path) so output is captured
/// regardless of which tier a given function lands on. Each closure appends a bounded
/// amount per call — no DoS vector.
fn run_jit(p: &Prog) -> Option<(Option<i32>, String)> {
    use std::sync::{Arc, Mutex};
    use jit_core::core::JITCore;
    use jit_core::GenericCirJit;
    use vm_core::core::VMCore;
    use vm_core::value::Value;

    let mut module = lang_aot::compile_source_to_iir(p.lang, p.src, "main").ok()?;
    let entry = module.entry_point.clone().unwrap_or_else(|| "main".to_string());

    let printed_ints: Arc<Mutex<Vec<i64>>> = Arc::new(Mutex::new(Vec::new()));
    let printed_bytes: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));

    // --- interpreter-fallback path: builtins on the VM (closures return Result) ---
    let mut vm = VMCore::new();
    let ints = Arc::clone(&printed_ints);
    vm.builtins_mut().register("print_i64", move |args: &[Value]| {
        let n = args.first().and_then(|v| v.as_i64()).unwrap_or(0);
        ints.lock().expect("lang-matrix JIT print buffer poisoned").push(n);
        Ok(Value::Null)
    });
    let bytes = Arc::clone(&printed_bytes);
    vm.builtins_mut().register("putchar", move |args: &[Value]| {
        let b = (args.first().and_then(|v| v.as_i64()).unwrap_or(0) & 0xFF) as u8;
        bytes.lock().expect("lang-matrix JIT putchar buffer poisoned").push(b);
        Ok(Value::Null)
    });
    let bytes = Arc::clone(&printed_bytes);
    vm.builtins_mut().register("print_str", move |args: &[Value]| {
        let s = args.first().and_then(Value::as_str).unwrap_or("");
        bytes
            .lock()
            .expect("lang-matrix JIT print_str buffer poisoned")
            .extend_from_slice(s.as_bytes());
        Ok(Value::Null)
    });
    // The program's stdin, shared by BOTH tiers' `getchar` (a function runs on one tier,
    // but sharing one buffer keeps the byte stream consistent whichever tier it lands
    // on); empty for every non-stdin program, so the first read is EOF → 0.
    let stdin_buf: Arc<Mutex<std::collections::VecDeque<u8>>> =
        Arc::new(Mutex::new(program_stdin(p).iter().copied().collect()));
    let input = Arc::clone(&stdin_buf);
    vm.builtins_mut().register("getchar", move |_args: &[Value]| {
        let byte = input.lock().expect("lang-matrix JIT stdin buffer poisoned").pop_front();
        Ok(Value::Int(byte.map(i64::from).unwrap_or(0)))
    });

    // --- compiled path: the same builtins on the JIT backend (closures return Value) ---
    let backend = GenericCirJit::new();
    let ints = Arc::clone(&printed_ints);
    backend.register_builtin("print_i64", move |args: &[Value]| {
        let n = args.first().and_then(|v| v.as_i64()).unwrap_or(0);
        ints.lock().expect("lang-matrix JIT print buffer poisoned").push(n);
        Value::Null
    });
    let bytes = Arc::clone(&printed_bytes);
    backend.register_builtin("putchar", move |args: &[Value]| {
        let b = (args.first().and_then(|v| v.as_i64()).unwrap_or(0) & 0xFF) as u8;
        bytes.lock().expect("lang-matrix JIT putchar buffer poisoned").push(b);
        Value::Null
    });
    let input = Arc::clone(&stdin_buf);
    backend.register_builtin("getchar", move |_args: &[Value]| {
        let byte = input.lock().expect("lang-matrix JIT stdin buffer poisoned").pop_front();
        Value::Int(byte.map(i64::from).unwrap_or(0))
    });

    // `JITCore::new` takes `&mut vm` only to thread thresholds — it does not hold the
    // borrow, so `execute_with_jit` can re-borrow `vm` for the interpreter tier.
    let mut jit = JITCore::new(&mut vm, Box::new(backend));
    let result = jit.execute_with_jit(&mut vm, &mut module, &entry, &[]).ok()?;

    // Exit code / stdout extraction is identical to `run_vm` — the JIT is observably
    // equivalent to the interpreter, which is the whole point of a JIT.
    let code = result.and_then(|v| v.as_i64()).map(|n| (n as i32) & 0xFF);
    let byte_buf = printed_bytes.lock().expect("lang-matrix JIT putchar buffer poisoned");
    let stdout = if byte_buf.is_empty() {
        printed_ints
            .lock()
            .expect("lang-matrix JIT print buffer poisoned")
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        // `.trim()` to match the subprocess columns: BA2 BASIC `PRINT` ends
        // each line with `putchar('\n')`, so the byte stream is e.g. `"42\n"`.
        String::from_utf8_lossy(&byte_buf).trim().to_string()
    };
    Some((code, stdout))
}

/// Dispatch a program to a backend runner. `None` = the backend's toolchain is
/// unavailable on this host (skip, like the W16 external-tool backends).
fn run(backend: Backend, p: &Prog) -> Option<(Option<i32>, String)> {
    match backend {
        Backend::NativeAot => run_native(p),
        Backend::Llvm => run_llvm(p),
        Backend::Wasm => run_wasm(p),
        Backend::Jvm => run_jvm(p),
        Backend::Clr => run_clr(p),
        Backend::Vm => run_vm(p),
        Backend::Jit => run_jit(p),
    }
}

/// Assert a single matrix cell agrees with the program's known result.
fn assert_cell(backend: Backend, p: &Prog, code: Option<i32>, stdout: &str) {
    match &p.expect {
        Expect::Exit(n) => assert_eq!(
            code,
            Some(*n),
            "{backend:?} {:?}: expected exit {n}, got {code:?} (stdout {stdout:?})",
            p.lang
        ),
        Expect::Stdout(s) => assert_eq!(
            stdout, *s,
            "{backend:?} {:?}: expected stdout {s:?}, got {stdout:?}",
            p.lang
        ),
    }
}

/// The capstone: every `(program, backend)` cell the campaign has **proven** runs
/// and agrees with the known result. A cell whose toolchain is absent skips
/// gracefully; a cell whose toolchain is present but disagrees fails loudly.
#[test]
fn matrix_every_proven_cell_agrees() {
    let mut ran = 0usize;
    for p in PROGRAMS {
        for &backend in p.backends {
            let Some((code, stdout)) = run(backend, p) else {
                continue;
            };
            assert_cell(backend, p, code, &stdout);
            ran += 1;
        }
    }
    eprintln!("lang-matrix: {ran} proven cells exercised");
}

/// Per-column floor: when a backend's toolchain IS present, every program tagged
/// for that backend MUST actually run — a proven cell silently skipping is a
/// regression, not a graceful absence.
#[test]
fn proven_columns_do_not_silently_skip() {
    // native-AOT: on a Linux/macOS host every native-tagged program must run.
    if cfg!(any(target_os = "linux", target_os = "macos")) {
        for p in PROGRAMS.iter().filter(|p| p.backends.contains(&NativeAot)) {
            assert!(
                run_native(p).is_some(),
                "native-AOT present but failed to run {:?}",
                p.lang
            );
        }
    }
    // LLVM: when clang is present every LLVM-tagged program must run.
    if clang_ok() {
        for p in PROGRAMS.iter().filter(|p| p.backends.contains(&Llvm)) {
            assert!(
                run_llvm(p).is_some(),
                "clang present but LLVM failed to run {:?}",
                p.lang
            );
        }
    }
    // WASM: the runtime is in-process (always present), so every WASM-tagged program
    // must run — no host gate.
    for p in PROGRAMS.iter().filter(|p| p.backends.contains(&Wasm)) {
        assert!(
            run_wasm(p).is_some(),
            "in-process wasm-runtime failed to run {:?}",
            p.lang
        );
    }
    // VM: the generic `vm_core::VMCore` is in-process (always present), so every
    // VM-tagged program must run — no host gate.
    for p in PROGRAMS.iter().filter(|p| p.backends.contains(&Vm)) {
        assert!(
            run_vm(p).is_some(),
            "in-process vm-core failed to run {:?}",
            p.lang
        );
    }
    // JIT: the generic `jit_core::JITCore` + `GenericCirJit` run in-process (always
    // present), so every JIT-tagged program must run — no host gate.
    for p in PROGRAMS.iter().filter(|p| p.backends.contains(&Jit)) {
        assert!(
            run_jit(p).is_some(),
            "in-process jit-core failed to run {:?}",
            p.lang
        );
    }
    // JVM: when `java` is present every JVM-tagged program must run.
    if java_ok() {
        for p in PROGRAMS.iter().filter(|p| p.backends.contains(&Jvm)) {
            assert!(
                run_jvm(p).is_some(),
                "java present but JVM failed to run {:?}",
                p.lang
            );
        }
    }
    // CLR: when `dotnet` + `ilasm` are present every CLR-tagged program must run.
    if dotnet_ok() && clr_support::find_ilasm().is_some() {
        for p in PROGRAMS.iter().filter(|p| p.backends.contains(&Clr)) {
            assert!(
                run_clr(p).is_some(),
                "dotnet+ilasm present but CLR failed to run {:?}",
                p.lang
            );
        }
    }
}
