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
#[derive(Debug)]
enum Expect {
    /// The process exit code (an expression language's returned value, `& 0xFF`).
    Exit(i32),
    /// A trimmed stdout string (an I/O language's printed output).
    Stdout(&'static str),
    /// The program must fail closed at runtime (for example, a bounds trap).
    Trap,
}

/// The observed outcome from a backend that was present and successfully built
/// the program.
#[derive(Debug)]
enum RunResult {
    Completed { code: Option<i32>, stdout: String },
    Trapped,
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
mod common;

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
    // Twig — **E6d-1: TW3-core dynamic `cons`/`car`/`cdr` on the code-gen backends.**
    // `(car (cons 42 0))` allocates a heap cons pair `(42 . 0)` and reads its head.
    // The Twig frontend emits `call_builtin "cons"/"car" [any]`; the shared
    // `iir-builtin-lowering` passes — `lower_heap_builtins` (cons→`alloc`+`field_store`;
    // car→`field_load[0]`) then `lower_dyn_repr_structural` (use-site boxing to the
    // uniform `ref<any>` value) — run for EVERY language, so Twig's first genuinely
    // dynamic value lowers to the exact heap-object family McCarthy Lisp already runs
    // on all five code-gen backends (WASM `anyref`+`$LispyPair`, JVM `Object[]`, CLR
    // `object[]`, LLVM tagged-i64 + `__dyn_*` runtime, native).  The entry
    // result (a boxed `42`) is unboxed to the process exit code.  The generic Vm/Jit
    // columns run `vm-core` typed IIR, which has no `ref<any>`/`alloc`, so dynamic
    // Twig is proven on the code-gen columns (which cross-check each other + the
    // known result); `twig-vm` is the interpreter reference off-matrix.
    Prog {
        lang: Language::Twig,
        ext: "twig",
        src: "(car (cons 42 0))",
        expect: Expect::Exit(42),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Twig — E6d-1: nested cons proves multi-cell pointer chasing.
    // `(car (cdr (cons 1 (cons 42 0))))` = car(cdr(`(1 . (42 . 0))`)) =
    // car(`(42 . 0)`) = 42 — two `cons` allocations, a `cdr` then a `car`.
    Prog {
        lang: Language::Twig,
        ext: "twig",
        src: "(car (cdr (cons 1 (cons 42 0))))",
        expect: Expect::Exit(42),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Twig — **E6d-2: dynamic integer arithmetic over `any`, all 5 code-gen
    // backends.** `(+ (car (cons 41 0)) 1)` forces `+` over a boxed operand —
    // `car`'s result is a `ref<any>` lisp value, not a machine int — which the
    // typed backends have no opcode for. The shared `lower_dynamic_arith` pass
    // expands it to `unbox (car …) → 41 ; add 41 1 → 42 ; box 42`.
    //   • **Structural** (E6d-2a): WASM `i31ref` / JVM `Integer` / CLR
    //     boxed-int32 lower the generic `unbox`/`add`/`box` ops directly, the
    //     same way they lower `cons`.
    //   • **Tagged-i64** (E6d-2b): NativeAot + LLVM have no `box`/`unbox` opcode —
    //     a tagged word is `n<<3`. `lower_box_unbox_to_runtime_calls` rewrites the
    //     generic ops to `dyn_box_int`/`dyn_unbox_int` runtime calls, which the
    //     backends dispatch to `__dyn_box_int`/`__dyn_unbox_int` in
    //     `dynval_runtime.c`. (The native AOT path also gains `lower_dynamic_arith`
    //     here — `prepare_module_for_aot` did not run it before.)
    // Exit 42 on every backend. `Vm`/`Jit` run typed `vm-core` IIR (`twig-vm` is
    // the off-matrix dynamic reference).
    Prog {
        lang: Language::Twig,
        ext: "twig",
        src: "(+ (car (cons 41 0)) 1)",
        expect: Expect::Exit(42),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Twig — **E6d-3a: the `list` constructor on the code-gen backends.**
    // `list` is pure sugar over `cons`: `(list a b c)` = `(cons a (cons b
    // (cons c nil)))`. The shared `iir-builtin-lowering` `desugar_list_in_function`
    // pass (at the head of `lower_heap_builtins` *and* `lower_heap_builtins_runtime`)
    // expands `call_builtin "list"` into a nil `const` + a right-to-left `cons`
    // chain, so the whole list rides the exact heap path E6d-1 proved — no new
    // backend op, no allowlist entry (the `list` builtin is gone before the
    // backend sees it). `(car (list 42 1 2))` reads the head element ⇒ 42.
    Prog {
        lang: Language::Twig,
        ext: "twig",
        src: "(car (list 42 1 2))",
        expect: Expect::Exit(42),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Twig — E6d-3a: `list` + `cdr` traversal reaches the second element.
    // `(car (cdr (list 1 42 3)))` = car(cdr(`(1 42 3)`)) = car(`(42 3)`) = 42,
    // proving the desugared cons chain links correctly across cells.
    Prog {
        lang: Language::Twig,
        ext: "twig",
        src: "(car (cdr (list 1 42 3)))",
        expect: Expect::Exit(42),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Twig — **E6d-3b: the `length` list operation on the code-gen backends.**
    // Unlike the `list` *constructor* (E6d-3a, a straight-line cons desugar),
    // `length` *walks* the cons chain, so `iir-builtin-lowering::lower_list_ops`
    // rewrites `call_builtin "length" lst` to a call to a synthesized recursive
    // helper `__dyn_list_length(lst) = if null?(lst) then 0 else 1 + length(cdr(lst))`
    // injected into the module. The helper is a *proper lisp function* (returns a
    // boxed `ref<any>`; its `+` is the E6d-2 dynamic add), so it rides `null?`/`cdr`
    // (E6d-1) + dynamic arithmetic (E6d-2) — nothing new lowers. This also required
    // fixing the WASM nil const: `const 0 : ref<LispyPair>` now emits `ref.null`
    // (it was `i32.const 0`, so `is_null` never detected the list terminator and the
    // walk overran) — aligning WASM with CLR's existing `ldnull` for nil.
    // `(+ (length (list 1 2 3)) 39)` = 3 + 39 = 42, proving `length` composes with
    // dynamic arithmetic and returns a genuine lisp value.
    Prog {
        lang: Language::Twig,
        ext: "twig",
        src: "(+ (length (list 1 2 3)) 39)",
        expect: Expect::Exit(42),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Twig — E6d-3b: `null?` on the empty list `(list)` (a bare nil) is #t → exit 1,
    // the direct regression guard for the WASM nil-const `ref.null` fix.
    Prog {
        lang: Language::Twig,
        ext: "twig",
        src: "(null? (list))",
        expect: Expect::Exit(1),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Twig — **E6d-3b: the `list-ref` list operation on the code-gen backends.**
    // Like `length`, `list-ref` *walks* the cons chain, so `lower_list_ops`
    // rewrites `call_builtin "list-ref" lst n` to a call to a synthesized
    // recursive helper `__dyn_list_ref(lst, n) = if n==0 then car(lst) else
    // list-ref(cdr(lst), n-1)` injected into the module. The index is a boxed
    // lisp value at the call boundary (the uniform-anyref convention boxes every
    // lisp-call argument), so the helper unboxes it once; the index test/decrement
    // are then typed `cmp_eq : bool` (feeding `jmp_if_false`) and `sub : i64`,
    // re-boxed for the recursive call — nothing new lowers. The *return* is a
    // `car` result (a lisp value). `(list-ref (list 10 20 42) 2)` = 42.
    Prog {
        lang: Language::Twig,
        ext: "twig",
        src: "(list-ref (list 10 20 42) 2)",
        expect: Expect::Exit(42),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Twig — **E6d-3b: the `append` list operation on the code-gen backends.**
    // `append` *rebuilds* the first list in front of the second, so `lower_list_ops`
    // rewrites `call_builtin "append" a b` to a synthesized recursive helper
    // `__dyn_list_append(a, b) = if null?(a) then b else cons(car(a), append(cdr(a), b))`.
    // Unlike `list-ref` there is no index — both args are lisp lists and every value
    // it touches is already a reference, so no unbox/box; its one new op vs
    // length/list-ref is the `cons` in the recursive arm (the E6d-1 heap builtin,
    // lowered for the injected helper too). `(append (list 1 42) (list 3))` builds
    // `(1 42 3)`; `(car (cdr …))` = 42, proving the rebuilt spine is a real cons chain.
    Prog {
        lang: Language::Twig,
        ext: "twig",
        src: "(car (cdr (append (list 1 42) (list 3))))",
        expect: Expect::Exit(42),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Twig — **E6d-3b: the `reverse` list operation on the code-gen backends.**
    // `reverse` is lowered by `lower_list_ops` to a nil-seeded call to a synthesized
    // *tail-recursive accumulator* helper:
    //   reverse(a) = __dyn_list_reverse(a, nil)
    //   __dyn_list_reverse(a, acc) = if null?(a) then acc
    //                                else __dyn_list_reverse(cdr(a), cons(car(a), acc))
    // Consing each element onto the accumulator's front reverses the order. The
    // call site seeds `acc` with a `const 0 : ref<LispyPair>` nil (the `list`-desugar
    // sentinel); the recursion reuses null?/car/cdr/cons — nothing new lowers.
    // `(reverse (list 1 2 42))` = `(42 2 1)`; `car` = 42.
    Prog {
        lang: Language::Twig,
        ext: "twig",
        src: "(car (reverse (list 1 2 42)))",
        expect: Expect::Exit(42),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Twig — **E6d-3b: the `assoc` list operation on the code-gen backends** (the
    // last E6d-3b op). `assoc` searches an association list (a list of `(k . v)`
    // cons pairs) for a key: `lower_list_ops` rewrites `call_builtin "assoc" key
    // alist` to a synthesized recursive helper
    //   __dyn_list_assoc(key, alist) = if null?(alist) then nil
    //     else if key == car(car(alist)) then car(alist) else assoc(key, cdr(alist))
    // V1 keys are integers: the key test unboxes both keys to i64 and compares with
    // a typed `cmp_eq` (feeding jmp_if_false), since `equal?` lowers unevenly across
    // the managed/runtime paths (symbol keys arrive with E6d-4). `(assoc 2 alist)`
    // over `((1 . 10) (2 . 42) (3 . 30))` finds `(2 . 42)`; `cdr` = 42.
    Prog {
        lang: Language::Twig,
        ext: "twig",
        src: "(cdr (assoc 2 (list (cons 1 10) (cons 2 42) (cons 3 30))))",
        expect: Expect::Exit(42),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Twig — E6d-3b: `assoc` of an ABSENT key returns nil, so `null?` of the result
    // is #t → exit 1 — the direct guard for the not-found (nil base-case) branch.
    Prog {
        lang: Language::Twig,
        ext: "twig",
        src: "(null? (assoc 9 (list (cons 1 10) (cons 2 20))))",
        expect: Expect::Exit(1),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Twig — **E6d-4: symbols / quote on the code-gen backends.** A quote literal
    // `'a` (or `(quote a)`) now lowers to `const Var("a") : symbol` — the same
    // interned-const form McCarthy Lisp emits — instead of the runtime `make_symbol`
    // string path. The shared `intern_symbols` (native) / `intern_symbols_structural`
    // (managed) passes assign each distinct name one module-wide id in a reserved
    // high range, so `equal?` on symbols is bit-equality with no new value type.
    // `(equal? 'a 'a)` = #t → exit 1.
    Prog {
        lang: Language::Twig,
        ext: "twig",
        src: "(equal? 'a 'a)",
        expect: Expect::Exit(1),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr],
    },
    // Twig — E6d-4: two DISTINCT symbols are not `equal?` (different interned ids),
    // so `(equal? 'a 'b)` = #f → exit 0 — the discriminating half of the proof.
    Prog {
        lang: Language::Twig,
        ext: "twig",
        src: "(equal? 'a 'b)",
        expect: Expect::Exit(0),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr],
    },
    // Twig — **E6d-5: records (TW6 part 1) on the code-gen backends.** A `(record
    // Name (f : T) …)` erases to a constructor `Name(…)` that builds a cons chain
    // (typed `alloc [ref<LispyPair>]` + `field_store`) and accessors `name-f(r)` =
    // `car(cdr^i(r))` (typed `field_load`) — the E6d-1 heap substrate, so records
    // ride the same proven cons/car/cdr path with no new value type. `(Point 42 7)`
    // builds `(42 . (7 . nil))`; `(point-x …)` = `car` = 42, proving construction
    // + field access round-trip end-to-end.
    //
    // Shipping this also fixed a latent WASM-runtime bug: struct field counts were
    // registered by per-function count, over-counting when functions share a
    // signature (a record emits a constructor + N same-shape accessors + a
    // predicate), so the `$LispyPair` field-count landed at the wrong type index
    // and every `struct.set` trapped "field 0 out of range" (wasm-runtime 0.4.0).
    //
    // **`Vm` + `Jit` too** (vm-core 0.17.0): the generic VM now runs the E6d heap
    // ops (`alloc`/`field_store`/`field_load`) on its bounds-checked array heap, so
    // records execute on the interpreter columns as well — all seven engines. (The
    // union cells stay code-gen-only for now: `match` tests the nil sentinel via
    // `is_null`, and on the generic VM a nil `Int(0)` is not yet distinguishable
    // from the first object handle `0`; records never dereference nil, so they are
    // sound. A nil-handle disambiguation for unions on the VM is a follow-up.)
    Prog {
        lang: Language::Twig,
        ext: "twig",
        src: "(record Point (x : int) (y : int)) (point-x (Point 42 7))",
        expect: Expect::Exit(42),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Twig — E6d-5: the SECOND field of a record (accessor walks one `cdr` then
    // `car`), proving the cons-chain offset is right. `(point-y (Point 7 42))` = 42.
    Prog {
        lang: Language::Twig,
        ext: "twig",
        src: "(record Point (x : int) (y : int)) (point-y (Point 7 42))",
        expect: Expect::Exit(42),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Twig — **E6d-6: unions / `match` (TW6 part 2).** A `(union Name (Variant …) …)`
    // erases to integer-tagged constructors (a cons `(tag . fields…)`) and `match`
    // dispatches by comparing the scrutinee's tag (`car`) to each variant's integer
    // tag, binding fields via `car(cdr^i)`. The tag compare uses the E6d-1 heap
    // substrate + dynamic `=`; two fixes made it run on the managed backends: the
    // match tag const is typed `i64` (not `any`, which caused a bogus `unbox`), and
    // `dyn_repr_structural` branches a boxed-bool `jmp_if_false` on its raw truth
    // value (a boxed `#f` is a non-nil i31, which the nil-truthiness wrap mis-read
    // as true → every arm matched). `(match (Some 42) …)` binds `v = 42`.
    //
    // **E6d-6b — runs on the tagged backends too (NativeAot/Llvm).** `match` reads
    // a variant's tag + fields back as *boxed* `DynValue`s (`field_load` result →
    // dynamic `=` on the tag; the bound field flows into an `any` context that
    // `unbox`es). The constructor `emit_union_def` now stores them boxed — a `box`
    // op on the tag const and on each field before the `field_store`. On the tagged
    // backends (`any` = raw i64) that `box` is the `n<<3` that makes the later
    // `unbox` recover the value (previously a raw `42` gave `unbox(42)=5`, a raw tag
    // `1` gave `unbox(1)=0` ⇒ `None` never matched); on the structural backends
    // (`anyref`/`Integer`) `box` of an already-boxed field is the identity, so their
    // round-trip is unchanged.
    //
    // **E6d-6c — completes the CLR column, so union `match` runs on ALL FIVE
    // code-gen backends.** Two CLR-only fixes: (1) `iir-to-cil-bytecode` emits a
    // special-char method name (`Some?`, `point-x`) as an ILAsm single-quoted
    // identifier `'Some?'` (the CIL twin of iir-to-llvm's `llvm_fn_ident`) — the CIL
    // grammar rejects `?`/`-` bare, so the union predicate previously would not even
    // assemble; (2) the CLR `box` op is now the identity when its source is already
    // a reference (`object`/`object[]`) — an E6d-6b union field arrives boxed at the
    // call boundary, so `box System.Int32` on it boxed the *pointer* (`box(object
    // 42)` → a truncated handle, not 42). Run-verified exit 42 on all five columns.
    //
    // **`Vm` + `Jit` too** (vm-core 0.18.0): the generic VM runs union `match` via
    // the E6d heap ops (records PR) plus `box` (the identity there) and the dynamic
    // `=`/`+`/`-`/`*` builtins the tag-test and arms use — so union `match` runs on
    // **all seven engines**. (`match` here needs no `is_null`, so no nil-handle
    // disambiguation is required; that is only for list `null?`.)
    Prog {
        lang: Language::Twig,
        ext: "twig",
        src: "(union Opt (Some (v : int)) (None)) (match (Some 42) ((Some v) v) ((None) 0))",
        expect: Expect::Exit(42),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Twig — E6d-6: matching the SECOND variant (`None`) proves the tag dispatch
    // actually discriminates — the boxed-bool branch takes the right arm, not
    // always the first. `(match (None) ((Some v) v) ((None) 42))` = 42. This is the
    // cell the raw-tag bug broke on the tagged backends (`unbox(raw 1)=0`); E6d-6b's
    // boxed tag fixes it. Runs on all seven engines (Clr via E6d-6c, Vm/Jit above).
    Prog {
        lang: Language::Twig,
        ext: "twig",
        src: "(union Opt (Some (v : int)) (None)) (match (None) ((Some v) v) ((None) 42))",
        expect: Expect::Exit(42),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Twig — **E6d-8: dynamic globals on the code-gen backends.** A value global
    // `g` that is *forward-referenced* (read inside `f` before its `define`) is
    // emitted as `call_builtin "global_get"/"global_set"` (the dynamic `any`-typed
    // global path, vs the typed-local-slot form a non-forward define gets). The
    // shared `lower_global_io` pass rewrites those to `global_load`/`global_store`
    // — the typed-global ops every backend accepts — but the managed pipelines
    // (WASM/JVM/CLR/BEAM) + the LLVM pipeline never ran it (only native `twig-aot`
    // did), so a dynamic global reached the backend as an unsupported `call_builtin`.
    // Adding `lower_global_io` to those pipelines makes the set+get roundtrip work:
    // `main` sets `g = 42` (`global_store`), `f` reads it (`global_load`), `(f)` = 42.
    // (A dynamic global flowing into *dynamic arithmetic* still needs the global slot
    // widened to a boxed `any` — a follow-up; here the roundtrip value is returned
    // directly.)
    Prog {
        lang: Language::Twig,
        ext: "twig",
        src: "(define (f) g) (define g 42) (f)",
        expect: Expect::Exit(42),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr],
    },
    // Twig — **E6d-7: closures (TW5) on all 5 code-gen backends** (the last E6
    // backend gap). `((lambda (x) (+ x 1)) 41)` allocates a closure and applies
    // it → 42. JVM/CLR run it via `long[]`/`object[]` dispatch arrays; NativeAot/
    // LLVM via the C runtime; **WASM** — which had no closure model — via
    // `iir-builtin-lowering::lower_closures_to_heap` (E6d-7a): the closure lowers
    // to a cons-chain `(box(idx) . (caps…))` and a synthesized `__dyn_call_closure`
    // dispatcher (a `cmp_eq` chain over statically-known bodies → direct `call`),
    // reusing the E6d-1 heap substrate — no new WasmGC `funcref`/`call_indirect`.
    //
    // **`Vm` + `Jit` too**: the same `lower_closures_to_heap` + `lower_heap_builtins`
    // passes the code-gen pipelines run are applied on the VM/JIT compile path
    // (`lower_dynamic_for_generic_engine`), so a closure lowers to the cons-chain
    // object + dispatcher over ops the generic VM now runs (`alloc`/`field_load`/
    // `box`/dynamic `=`/`+`). Closures run on **all seven engines**.
    Prog {
        lang: Language::Twig,
        ext: "twig",
        src: "((lambda (x) (+ x 1)) 41)",
        expect: Expect::Exit(42),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Twig — E6d-7: a **capturing** closure. The outer lambda returns an inner
    // one that captures `x`; applying it threads the captured 40 + the arg 2.
    // `(((lambda (x) (lambda (y) (+ x y))) 40) 2)` → 42. Two lambda bodies get
    // distinct dispatch indices in the synthesized dispatcher. Runs on all seven
    // engines (Vm/Jit via the closure/heap passes on the generic-engine path).
    Prog {
        lang: Language::Twig,
        ext: "twig",
        src: "(((lambda (x) (lambda (y) (+ x y))) 40) 2)",
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
    // Twig — E4 literal `string-ref`. The front-end emits `str_const` plus a
    // typed integer index and `str_index`; ASCII keeps the byte-oriented E4
    // result aligned with JVM/CLR host string char indexing for this foothold.
    Prog {
        lang: Language::Twig,
        ext: "twig",
        src: "(string-ref \"ABC\" 1)",
        expect: Expect::Exit(66),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Twig — E4 literal `string-ref` out-of-bounds trap. This proves the same
    // runtime fail-closed contract on every backend: native/LLVM lower the
    // compile-known OOB literal to a trap path, WASM/VM/JIT use their shared bounds
    // checks, and JVM/CLR surface their managed string index exceptions.
    Prog {
        lang: Language::Twig,
        ext: "twig",
        src: "(string-ref \"ABC\" 3)",
        expect: Expect::Trap,
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
    // Twig — E4 named string values. Non-escaping top-level string `define`s
    // now stay in `main` as typed `str_const` registers, so shared string ops can
    // consume them without the dynamic `global_set`/`global_get` path. This
    // proves non-literal `str_concat` feeding `str_len` while staying within the
    // immutable top-level value subset; reassigned string variables remain a
    // separate E4/BA4 frontend slice.
    Prog {
        lang: Language::Twig,
        ext: "twig",
        src: "(define a \"AB\") (define b \"CDE\") (string-length (string-append a b))",
        expect: Expect::Exit(5),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Twig — E4 named string equality driving control flow. The `string=?`
    // result is the shared i64 boolean consumed by the existing `if` lowering,
    // which makes the observable value depend on the string operation rather
    // than on a folded top-level constant.
    Prog {
        lang: Language::Twig,
        ext: "twig",
        src: "(define s \"HELLO\") (if (string=? s \"HELLO\") 42 0)",
        expect: Expect::Exit(42),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Twig — E4 named string indexing. This reuses the landed in-bounds
    // `str_index` backend support but proves the source string can be a named
    // top-level value rather than only a direct literal at the call site.
    Prog {
        lang: Language::Twig,
        ext: "twig",
        src: "(define s \"ABC\") (string-ref s 2)",
        expect: Expect::Exit(67),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Twig — E4 lexical string locals. A `let` string binding now materialises
    // directly as a typed `str_const` register, and a local integer binding can
    // feed the `str_index` index operand. This proves local string slots across
    // the same all-seven E4 path without claiming captured/reassigned strings.
    Prog {
        lang: Language::Twig,
        ext: "twig",
        src: "(let ((s \"ABC\") (i 2)) (string-ref s i))",
        expect: Expect::Exit(67),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Twig — E4 lexical `let*` string locals. The sequential-binding form uses
    // the same typed `str_const` local slot path as `let`, and `str_len`
    // observes it without falling back to dynamic `string-length`.
    Prog {
        lang: Language::Twig,
        ext: "twig",
        src: "(let* ((s \"HELLO\")) (string-length s))",
        expect: Expect::Exit(5),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Twig — E4 lexical string locals can drive equality control flow too.
    // Two local string slots feed `str_eq`, and the resulting i64 boolean flows
    // into the existing `if` branch shape.
    Prog {
        lang: Language::Twig,
        ext: "twig",
        src: "(let ((s \"OK\") (t \"OK\")) (if (string=? s t) 42 0))",
        expect: Expect::Exit(42),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Twig — E4 lexical string locals feeding concat. This takes the local
    // string path beyond indexing: two `let` string slots feed `str_concat`,
    // then `str_len` observes the result. It proves local non-literal string
    // operands without claiming captured or reassigned string variables.
    Prog {
        lang: Language::Twig,
        ext: "twig",
        src: "(let ((a \"AB\") (b \"CDE\")) (string-length (string-append a b)))",
        expect: Expect::Exit(5),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Twig — E4 `str_concat` feeding `str_index`. The prior local-string proof
    // observed a concat result with `str_len`; this row makes the byte-indexing
    // contract consume that same temporary string value. `AB` + `CDE` = `ABCDE`,
    // and index 3 is byte `D` (68).
    Prog {
        lang: Language::Twig,
        ext: "twig",
        src: "(let ((a \"AB\") (b \"CDE\") (i 3)) (string-ref (string-append a b) i))",
        expect: Expect::Exit(68),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Twig — E4 `str_len` computing a `str_index` operand. This keeps
    // `string-length` on the shared `str_len` path, lowers `(- len 1)` as typed
    // integer arithmetic, and feeds the computed register into `str_index`.
    // `ABCDE` length 5 minus 1 is index 4, byte `E` (69).
    Prog {
        lang: Language::Twig,
        ext: "twig",
        src: "(let ((s \"ABCDE\")) (string-ref s (- (string-length s) 1)))",
        expect: Expect::Exit(69),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Twig — E4 `substring` feeding `string-ref`. This proves the shared
    // `str_slice` op produces a string value that all seven proven columns can
    // consume with the existing byte-indexing contract. `substring` 1..4 is
    // `BCD`, and index 1 is byte `C` (67).
    Prog {
        lang: Language::Twig,
        ext: "twig",
        src: "(let ((s \"ABCDE\")) (string-ref (substring s 1 4) 1))",
        expect: Expect::Exit(67),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Twig — E4 `str_cmp` driving lexical string predicates. The frontend lowers
    // `string<?`/`string>?` to shared `str_cmp` followed by typed comparison
    // against zero, so every proven column observes the same byte ordering.
    Prog {
        lang: Language::Twig,
        ext: "twig",
        src: "(if (string<? \"ALPHA\" \"BETA\") (if (string>? \"BETA\" \"ALPHA\") 42 0) 0)",
        expect: Expect::Exit(42),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Twig — E4 string ops inside a direct top-level function. The function body
    // lowers `(string-length "HELLO")` to typed `str_const` + `str_len`, and the
    // direct `(strlen)` call now carries the function's `i64` return type instead
    // of falling back to `any`.
    Prog {
        lang: Language::Twig,
        ext: "twig",
        src: "(define (strlen) (string-length \"HELLO\")) (strlen)",
        expect: Expect::Exit(5),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Twig — E4 string ops over an annotated top-level function parameter. The
    // bare `str` annotation gives the compiler enough static evidence to stamp
    // the parameter as `str`, so `string-length s` lowers to `str_len` instead of
    // the dynamic builtin path.  NativeAot uses the LANG-STR-RT runtime: the
    // caller builds a length-prefixed `[i64 len][bytes...]` heap buffer and the
    // callee emits `field_load s, 0 → len` to read it.  Wasm now runs too: the
    // caller promotes the `str_const` literal to a `[i32 len][bytes]` runtime block
    // before passing it, so the callee's runtime `str_len` reads the header (see
    // iir-to-wasm `collect_runtime_str_vars`).  Llvm runs too: a `str_const` literal
    // (tracked as its `{i64 len,[N×i8]}` global pointer) passed as a call argument is
    // `ptrtoint`'d to an i64 handle first (mirror of the `ret` path), so the callee's
    // runtime `str_len` reads the length header via `inttoptr`+`load`. All 7 backends.
    Prog {
        lang: Language::Twig,
        ext: "twig",
        src: "(define (strlen (s : str)) (string-length s)) (strlen \"HELLO\")",
        expect: Expect::Exit(5),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Twig — E4 string ops over an unannotated top-level function parameter
    // with direct-call evidence from `main`. The direct `(strlen "HELLO")`
    // call gives the compiler enough static evidence to stamp `s` as `str`
    // without creating refinement annotations.  NativeAot now uses the
    // LANG-STR-RT runtime (`field_load s, 0 → len`) — see cell above.
    // Wasm now runs via the `str_const`→runtime-block call-argument promotion; Llvm
    // via the `ptrtoint` of the literal's global pointer at the call site. All 7.
    Prog {
        lang: Language::Twig,
        ext: "twig",
        src: "(define (strlen s) (string-length s)) (strlen \"HELLO\")",
        expect: Expect::Exit(5),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Twig — E4 string ops over an unannotated top-level function parameter
    // with direct-call evidence from a static string expression actual. The
    // actual materialises through `str_concat` + `str_slice`, then feeds the
    // inferred `str` parameter without creating refinement annotations.
    // NativeAot: caller folds str_concat+str_slice to a literal-backed buffer
    // (LANG-STR-RT layout), callee reads length via `field_load x, 0 → len`.
    // Llvm now runs: the folded `substring` result is a `@__twig_str` global, which
    // `lower_call` `ptrtoint`s to an i64 handle before the call (same as `str_const`).
    // Wasm now runs: the folded `str_slice` result is promoted to a length-prefixed
    // runtime block `[i32 len][bytes]` (its dest is used as a call arg), so the callee
    // reads a real header instead of the first sliced byte (`'H'`=72). All 7.
    Prog {
        lang: Language::Twig,
        ext: "twig",
        src: "(define (strlen x) (string-length x)) (strlen (substring (string-append \"HE\" \"LLO!\") 0 5))",
        expect: Expect::Exit(5),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Twig — E4 string equality over multiple unannotated top-level function
    // parameters inferred from one direct call. The first actual is literal,
    // the second is a static `str_concat` expression, so both slots are stamped
    // `str` and the function body lowers `string=? a b` through `str_eq`.
    // NativeAot: runtime `str_eq` parameters lower to `call_builtin "str_eq"` →
    // `__twig_str_eq(a, b)` which compares the LANG-STR-RT length headers then
    // memcmp's the byte data.
    // Llvm now runs: `lower_str_eq` gained a runtime path — when an operand isn't a
    // compile-time literal (here both are params), it calls `@__twig_str_eq(i64, i64)`
    // over the two handles (the caller `ptrtoint`s the literal-args' globals first).
    // Wasm now runs: `str_eq` over runtime handles calls the self-contained in-module
    // `$__str_eq(i32,i32)->i32` helper (header-length check + byte-compare loop); the
    // literal args are promoted to `[i32 len][bytes]` blocks so both present a header.
    // This is the last lang-full string-tail cell — all 3 now run on all 7 backends.
    Prog {
        lang: Language::Twig,
        ext: "twig",
        src: "(define (same a b) (if (string=? a b) 42 0)) (same \"OK\" (string-append \"O\" \"K\"))",
        expect: Expect::Exit(42),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Twig — E4 string ops over an unannotated top-level function parameter
    // with direct-call evidence from a non-escaping top-level string value. The
    // named actual stays in `main` as a typed `str` register.  NativeAot: the
    // global string `s` is built as a LANG-STR-RT buffer in `main`; the callee
    // reads the length via `field_load x, 0 → len`.
    // Wasm now runs via the `str_const`→runtime-block call-argument promotion; Llvm
    // via the `ptrtoint` of the literal's global pointer at the call site. All 7.
    Prog {
        lang: Language::Twig,
        ext: "twig",
        src: "(define s \"HELLO\") (define (strlen x) (string-length x)) (strlen s)",
        expect: Expect::Exit(5),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Twig — E4 string ops over an unannotated top-level function parameter
    // with direct-call evidence from a lexical string local in `main`. The
    // `let` binding keeps `s` as a typed `str` register at the call site.
    // NativeAot: let-bound string is a LANG-STR-RT buffer; callee reads length
    // via `field_load x, 0 → len`.
    // Wasm now runs via the `str_const`→runtime-block call-argument promotion; Llvm
    // via the `ptrtoint` of the literal's global pointer at the call site. All 7.
    Prog {
        lang: Language::Twig,
        ext: "twig",
        src: "(define (strlen x) (string-length x)) (let ((s \"HELLO\")) (strlen s))",
        expect: Expect::Exit(5),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Twig — E4 string ops over an unannotated top-level function parameter
    // with direct-call evidence from a derived sequential `let*` string local
    // in `main`. The second binding sees the first as static string evidence,
    // materialises `b` through `str_concat`, and keeps `(strlen b)` typed.
    // NativeAot: `main`'s lowering folds `str_concat` to a LANG-STR-RT buffer;
    // callee reads length via `field_load x, 0 → len`.
    // Llvm now runs: the folded `str_concat` result is a `@__twig_str` global, which
    // `lower_call` `ptrtoint`s to an i64 handle before the call (same as `str_const`).
    // Wasm now runs: the folded `str_concat` result is promoted to a length-prefixed
    // runtime block `[i32 len][bytes]` (its dest is used as a call arg), so the callee
    // reads a real header instead of the first byte (`'H'`=72). All 7.
    Prog {
        lang: Language::Twig,
        ext: "twig",
        src: "(define (strlen x) (string-length x)) (let* ((a \"HE\") (b (string-append a \"LLO\"))) (strlen b))",
        expect: Expect::Exit(5),
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
    // Nib — logical NOT (`!`, LANG-FULL N9). `1 == 2` is false, so
    // `!(1 == 2)` must be true and take the 42 branch. The old passthrough
    // behavior would return 0 here.
    Prog {
        lang: Language::Nib,
        ext: "nib",
        src: "fn main() -> u8 { if !(1 == 2) { return 42; } return 0; }",
        expect: Expect::Exit(42),
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
    // Nib — module-scoped `static` globals (LANG-FULL N8). The counter starts
    // at 40, a separate function increments the same module global twice, and
    // `main` reads back 42. A plain per-function register would lose the shared
    // state. The frontend lowers the initializer/read/write to the shared E6
    // `global_store`/`global_load` substrate that every backend already runs.
    Prog {
        lang: Language::Nib,
        ext: "nib",
        src: "static counter: u8 = 40; \
              fn bump(step: u8) -> u8 { counter = counter + step; return counter; } \
              fn main() -> u8 { let a: u8 = bump(1); let b: u8 = bump(1); return counter; }",
        expect: Expect::Exit(42),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Nib — const/static expression folding (LANG-FULL N10). `BASE` folds
    // `6 * 7` at compile time, then the static initializer folds `BASE + 0`
    // before seeding the shared module global. No runtime arithmetic is needed
    // for the initializer, but every backend still observes the global value.
    Prog {
        lang: Language::Nib,
        ext: "nib",
        src: "const BASE: u8 = 6 * 7; static counter: u8 = BASE + 0; fn main() -> u8 { return counter; }",
        expect: Expect::Exit(42),
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
    // Oct — logical NOT (`!`) produces a clean boolean 0/1 (LANG-FULL O-!).
    // `1 == 2` is false, so `!(1 == 2)` is true and prints 42. The old lowering
    // reused bitwise `not`: `not 0` produced `-1`, which branch truthiness treated
    // as true but did NOT materialise a clean Oct bool value. The new lowering uses
    // the same portable branch substrate as O1 short-circuiting.
    Prog {
        lang: Language::Oct,
        ext: "oct",
        src: "fn main() { if !(1 == 2) { out(1, 42); } else { out(1, 0); } }",
        expect: Expect::Stdout("42"),
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
    // ALGOL 60 — the exponentiation operator `↑` (§3.3.4, spelled `^`; LANG-FULL
    // AL-pow).  A **nonnegative integer-literal exponent** unrolls to repeated
    // multiplication and keeps the base's type, so `2 ^ 5` is the *integer* 32 —
    // exactly `mul`/`imul` the code-gen backends already run (no new IIR op).
    // `↑` binds tighter than `*`, so `10 + 2 ^ 5` = `10 + 32` = 42.  (The
    // `real ↑ real` shape lowers to the `f64_pow` op BASIC's BA-pow already
    // proved on every backend; this cell exercises the integer-unroll path,
    // which stays i64 end-to-end.)  Runs on **all 7 backends**.  Exit 42.
    Prog {
        lang: Language::Algol60,
        ext: "alg",
        src: "begin integer result; result := 10 + 2 ^ 5 end",
        expect: Expect::Exit(42),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // ALGOL 60 — literal string output (LANG-FULL AL4 on E4). ALGOL leaves I/O
    // implementation-defined, so this frontend recognises undeclared statement
    // calls named `print`/`output` as standard output procedures. The narrow
    // foothold is deliberately literal-only: `print('HI')` lowers to E4
    // `str_const` + `print_str`, exactly the same shared string op pair BASIC
    // string `PRINT` already proved. That makes stdout the observable result on
    // native-AOT / LLVM / WASM / JVM / CLR / VM / JIT without adding any
    // ALGOL-specific backend hooks.
    Prog {
        lang: Language::Algol60,
        ext: "alg",
        src: "begin print('HI') end",
        expect: Expect::Stdout("HI"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // ALGOL 60 — string procedures (LANG-FULL E4-dyn payoff, E4d-AL). The first
    // E4-dyn *frontend* feature: a `string procedure` returns a runtime string.
    // Here the result is chosen by control flow inside the body
    // (`if n > 0 then pick := 'HI' else pick := 'LO'`), so `pick`'s result slot
    // is a genuinely runtime, branch-selected string — the exact shape the
    // E4-dyn foothold proved on all seven backends. The call site `print(pick(1))`
    // evaluates the call to a runtime string handle and prints it (a new general
    // string-expression path in the ALGOL output lowering). This exercises the
    // whole chain end-to-end: string-procedure declaration + call + runtime-string
    // return + print. It runs on the columns that carry a runtime string arriving
    // as a **call result / return value** (not just a branch-selected local slot,
    // which is all the E4-dyn foothold exercised):
    //   * NativeAot — twig-aot: a call-result str is absent from the compile-time
    //     `strings` map, so `print_str` reads the length header at run time.
    //   * Llvm (E4d-2b) — `str` maps to an i64 handle at function boundaries, and
    //     `print_str`/`str_len` read the header at run time for ANY string without
    //     a compile-time length (call result / return value / param), not only a
    //     promoted slot. Verified via clang.
    //   * Wasm (E4d-3b) — `str` already types as an i32 handle at boundaries; the
    //     validator now accepts `str` on `call`/`ret`, and `print_str`/`str_len`
    //     read the length via `i32.load` for any non-foldable string. Verified
    //     in-process via wasm-runtime.
    //   * Jvm (E4d-JVM) — a `str` value is a `java.lang.String`; the validator now
    //     accepts `str` on `call`/`ret`. Verified on real java.
    //   * Clr — a `str` value is a `System.String`; it already accepted `str`
    //     call/ret and lowered the returned string. Verified via the CLR simulator.
    //   * VM / JIT — tagged values: a returned string is printed like any value.
    // A `string procedure` — the first E4-dyn frontend feature — now runs on ALL
    // SEVEN backends. Stdin-free; N=1>0 → `HI`.
    Prog {
        lang: Language::Algol60,
        ext: "alg",
        src: "begin string procedure pick(n); value n; integer n; \
                  if n > 0 then pick := 'HI' else pick := 'LO'; \
              print(pick(1)) end",
        expect: Expect::Stdout("HI"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // ALGOL 60 — scalar string variables in the current AL4 foothold. A
    // `string` scalar may be assigned from a literal, which emits `str_const`
    // directly to the variable slot; `print(s)` is accepted only because that
    // slot is literal-backed. This deliberately avoids dynamic string copies or
    // captured string globals while still proving source-level string variables
    // through the same E4 `print_str` path on all seven backends.
    Prog {
        lang: Language::Algol60,
        ext: "alg",
        src: "begin string s; s := 'HI'; print(s) end",
        expect: Expect::Stdout("HI"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // ALGOL 60 — the implementation-defined `output` spelling follows the same
    // AL4 path as `print`: a literal-backed string slot consumed by E4
    // `print_str`. This row proves the alias rather than only the `print`
    // spelling, without widening into dynamic strings.
    Prog {
        lang: Language::Algol60,
        ext: "alg",
        src: "begin string s; s := 'OK'; output(s) end",
        expect: Expect::Stdout("OK"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // ALGOL 60 — `output` preserves multiple literal-backed scalar string
    // actuals. This proves the AL4 standard-output alias can consume two E4
    // string slots in source order without adding separators or using dynamic
    // procedure calls.
    Prog {
        lang: Language::Algol60,
        ext: "alg",
        src: "begin string s, t; s := 'O'; t := 'K'; output(s, t) end",
        expect: Expect::Stdout("OK"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // ALGOL 60 — literal-backed scalar string copy. This keeps AL4 inside the
    // same immutable E4 shape BASIC now uses: `t := s` lowers through
    // `str_concat` with an empty suffix, then `print(t)` consumes the target
    // string slot.
    Prog {
        lang: Language::Algol60,
        ext: "alg",
        src: "begin string s, t; s := 'OK'; t := s; print(t) end",
        expect: Expect::Stdout("OK"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // ALGOL 60 — copied scalar string slots are snapshots. Reassigning the
    // source after `t := s` rematerializes `s` but must not change `t`, keeping
    // the AL4 copy foothold inside immutable E4 string values.
    Prog {
        lang: Language::Algol60,
        ext: "alg",
        src: "begin string s, t; s := 'OK'; t := s; s := 'NO'; print(t) end",
        expect: Expect::Stdout("OK"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // ALGOL 60 — literal-backed scalar string predicates. AL4 now lowers
    // comparisons over string literals and literal-backed scalar variables
    // through the shared E4 `str_eq` / `str_cmp` ops, then compares their
    // integer results against typed zero before the normal `if` branch consumes
    // the boolean. This row proves equality, inequality, and both ordering
    // operand orders without widening into captured strings, arrays, or dynamic
    // string storage.
    Prog {
        lang: Language::Algol60,
        ext: "alg",
        src: "begin string s; s := 'ALPHA'; if (s = 'ALPHA' and s != 'OMEGA') and (s < 'BETA' and 'BETA' > s) then print('OK') else print('BAD') end",
        expect: Expect::Stdout("OK"),
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
    // ALGOL 60 — the `sqrt` **standard function** (§3.2.4, LANG-FULL AL8-sqrt).
    // `sqrt(E)` computes the IEEE-754 square root of a `real` expression.  Unlike
    // `abs`/`sign` (which lower to compare+branch) and `entier` (which lowers to
    // `real_to_int_floor`), `sqrt` lowers to the new **`f64_sqrt`** IIR op — a
    // single-argument f64→f64 primitive that every backend emits in its native idiom:
    // LLVM `@llvm.sqrt.f64` intrinsic, WASM `f64.sqrt` (opcode 0x9F, MVP), JVM
    // `invokestatic java/lang/Math.sqrt:(D)D`, CLR
    // `call float64 [System.Runtime]System.Math::Sqrt(float64)`, native aarch64
    // `FSQRT Dd,Dn`, native x86_64 `SQRTSD xmm0,xmm0`.  The JIT falls back to the
    // VM handler (`f64::sqrt()`) via the `_f64`-suffix fallback.  The proof program
    // computes `sqrt(49.0)` = 7.0, converts to integer via `entier`, and exits with
    // that value: exit 7 on **all 7 backends**.
    Prog {
        lang: Language::Algol60,
        ext: "alg",
        src: "begin real r; integer result; r := sqrt(49.0); \
               result := entier(r) end",
        expect: Expect::Exit(7),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // ALGOL 60 — **AL8-trig**: `cos` standard function (§3.2.4).
    // `cos(0.0)` = 1.0 exactly in IEEE-754 double.  `entier(1.0) + 41` = 42.
    // Every backend calls the platform libm / runtime: WASM resolves `env.__cos`
    // to Rust `f64::cos`; LLVM emits `@llvm.cos.f64`; JVM `Math.cos`; CLR
    // `System.Math.Cos`; native aarch64/x86_64 `BL cos` / `call cos`; VM/JIT
    // via the `f64_cos` dispatch handler.
    Prog {
        lang: Language::Algol60,
        ext: "alg",
        src: "begin real r; integer result; r := cos(0.0); \
               result := entier(r) + 41 end",
        expect: Expect::Exit(42),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // ALGOL 60 — **AL8-trig**: `exp` standard function (§3.2.4).
    // `exp(0.0)` = 1.0 exactly.  `entier(1.0) + 41` = 42.
    Prog {
        lang: Language::Algol60,
        ext: "alg",
        src: "begin real r; integer result; r := exp(0.0); \
               result := entier(r) + 41 end",
        expect: Expect::Exit(42),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // ALGOL 60 — **AL8-trig**: `sin` standard function (§3.2.4).
    // `sin(0.0)` = 0.0 exactly.  `entier(0.0 + 42.0)` = 42.
    Prog {
        lang: Language::Algol60,
        ext: "alg",
        src: "begin real r; integer result; r := sin(0.0); \
               result := entier(r + 42.0) end",
        expect: Expect::Exit(42),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // ALGOL 60 — **AL8-trig**: `ln` standard function (§3.2.4).
    // `ln(1.0)` = 0.0 exactly.  `entier(0.0 + 42.0)` = 42.
    // Note: ALGOL 60 §3.2.4 calls it `ln` (natural logarithm); every backend
    // maps this to libm `log` / `Math.log` / `@llvm.log.f64` etc.
    Prog {
        lang: Language::Algol60,
        ext: "alg",
        src: "begin real r; integer result; r := ln(1.0); \
               result := entier(r + 42.0) end",
        expect: Expect::Exit(42),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // ALGOL 60 — **AL8-arctan**: `arctan` standard function (§3.2.4).
    // `arctan(0.0)` = 0.0 exactly (arctan of 0 is 0).
    // `entier(0.0 + 42.0)` = 42.
    // Every backend calls libm `atan` / `Math.atan` / `@atan` etc.
    Prog {
        lang: Language::Algol60,
        ext: "alg",
        src: "begin real r; integer result; r := arctan(0.0); \
               result := entier(r + 42.0) end",
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
    // compiles + runs via `clang` → exit 55. **NativeAot and WASM run it too**: the
    // for-loop lowers to the same generic `alloc_array`/`array_get`/`array_set` +
    // integer relation/branch ops the straight-line E5 cell (below) already proved on
    // both — no backend-specific for-loop path exists, so once the LLVM guard-width
    // bug was fixed there was nothing left blocking native/wasm. This cell now runs on
    // all 7 backends (the loop is a pure control-flow composition over ops all backends
    // already lower).
    Prog {
        lang: Language::Algol60,
        ext: "alg",
        src: "begin integer array A[1:5]; integer i, result; \
               for i := 1 step 1 until 5 do A[i] := i * i; \
               result := 0; \
               for i := 1 step 1 until 5 do result := result + A[i] end",
        expect: Expect::Exit(55),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
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
    // ALGOL 60 — a **string-typed value parameter** (LANG-FULL AL4-str-params).
    // `integer procedure echo(s); value s; string s; print(s)` declares a
    // procedure that accepts a string by value and prints it.  The call
    // `echo('HELLO')` passes the literal through the shared `call` IIR op;
    // inside `echo`, `s` is a `str`-typed IIR parameter already seeded into
    // `literal_string_slots`, so `print(s)` lowers to `print_str s` on all
    // backends — the same path ALGOL literal-string `print` uses (AL4).
    // The return value (implicitly 0, the integer default) is discarded.
    // NativeAot, Llvm, and Wasm run it too now: the E4-dyn runtime-string work
    // (E4d-2b LLVM / E4d-3b WASM / E4d-4 native) gave `print_str` a RUNTIME path
    // that reads the length from the `[len][bytes]` block header at run time for
    // any string lacking a compile-time-length entry — which is exactly a string
    // parameter `s` (it receives its i64/i32 handle at the call site and is absent
    // from the `strings`/`str_lens`/string-local maps). The literal actual
    // `'HELLO'` is passed as that handle across the shared `call`. All 7 backends.
    Prog {
        lang: Language::Algol60,
        ext: "alg",
        src: "begin integer procedure echo(s); value s; string s; print(s); \
               echo('HELLO') end",
        expect: Expect::Stdout("HELLO"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // ALGOL 60 — a named string variable passed to a string parameter (AL4-str-params).
    // The outer block declares `string msg`, initialises it with a literal, then
    // calls `say(msg)`.  This proves the call site can pass a *named* E4 string
    // slot (not just an inline literal) as a `str`-typed actual argument, and
    // the callee's `print(s)` still lowers to `print_str` on all managed
    // backends.
    // NativeAot, Llvm, and Wasm run it too now (same E4-dyn runtime `print_str`
    // path as `echo` above): the callee's `s` param carries a runtime string
    // handle and `print_str` reads its header at run time. The extra wrinkle here
    // — the actual is a *named* outer-block string slot `msg` (not an inline
    // literal) — makes no difference: the call still passes `msg`'s handle. All 7.
    Prog {
        lang: Language::Algol60,
        ext: "alg",
        src: "begin string msg; \
               integer procedure say(s); value s; string s; print(s); \
               msg := 'HI'; say(msg) end",
        expect: Expect::Stdout("HI"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // ALGOL 60 — *`real` array* (LANG-FULL AL9-a — real-typed E5 arrays).  The E5
    // array substrate (`alloc_array`/`array_set`/`array_get`) uses the IIR
    // type_hint to decide the element width: `f64` instead of `i64`.  BASIC's BA3
    // cells already proved `array<f64>` reaches all 7 backends, but this is the
    // first ALGOL 60 real-array matrix cell — exercising `real array A[lo:hi]`
    // syntax, the ALGOL lower-bound subtraction on access, and f64-typed element
    // stores/loads through `entier` back to an integer exit code.
    //
    // `A[1] := 40.0; A[3] := 2.0; result := entier(A[1] + A[3])` ⇒ `entier(42.0)`
    // = 42 ⇒ exit 42 on all 7 backends.  Exercises a *non-contiguous* pair of
    // f64 slots (index 1 and 3 with a gap at 2) so the element-offset arithmetic is
    // non-trivial.
    Prog {
        lang: Language::Algol60,
        ext: "alg",
        src: "begin real array A[1:3]; integer result; \
               A[1] := 40.0; A[3] := 2.0; result := entier(A[1] + A[3]) end",
        expect: Expect::Exit(42),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // ALGOL 60 — *`real` procedure with a real value parameter* (LANG-FULL AL9-b).
    // Proves that the `call`/`ret` pathway carries `f64` arguments and return values
    // correctly across all 7 backends.  The function type is `(f64) -> f64`; the
    // call site lowers to a `call square(x_slot)` with a real (f64) argument, and
    // the callee emits `ret square` after `square := x * x` (an f64 multiply).
    //
    // `square(6.5)` = 6.5 × 6.5 = 42.25; `entier(42.25)` = 42 (floor, not trunc —
    // same direction here since 42.25 > 0) ⇒ exit 42 on all 7 backends.  The
    // non-integer argument (6.5) and non-integer intermediate result (42.25) both
    // exercise the f64 parameter-passing path independently of the E3 scalar
    // arithmetic cells, which use only whole-valued reals.
    Prog {
        lang: Language::Algol60,
        ext: "alg",
        src: "begin real procedure square(x); value x; real x; square := x * x; \
               integer result; result := entier(square(6.5)) end",        expect: Expect::Exit(42),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // ALGOL 60 — *recursive procedure* (LANG-FULL AL12). The classic factorial
    // function calls itself via `fact(n - 1)`, proving that the call-and-return
    // mechanism lowers through every backend's `call`/`ret` path recursively.
    // ALGOL 60 report §5.4.2: a typed procedure may appear in its own body.
    // `fact(3)` = 3 × 2 × 1 = 6; three levels of recursion are enough to
    // distinguish correct recursion from a loop or memoised result.  The IIR
    // compiles to a `call fact [sub(n,1)]` instruction inside the `fact`
    // function; every backend already handled cross-function `call` for the
    // non-recursive procedure cells.  Exit 6.
    Prog {
        lang: Language::Algol60,
        ext: "alg",
        src: "begin integer result; \
               integer procedure fact(n); value n; integer n; \
               if n < 2 then fact := 1 else fact := n * fact(n - 1); \
               result := fact(3) end",
        expect: Expect::Exit(6),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // ALGOL 60 — *`goto` loop* (LANG-FULL AL12). Labels + `goto` are ALGOL 60
    // primitives (report §4.7): `label:` defines a label in the current block;
    // `goto label` transfers control unconditionally.  Here the loop adds 7
    // each iteration until `x >= 42`: x goes 7 → 14 → 21 → 28 → 35 → 42 (6
    // iters), then the `if x < 42` is false and control falls through.
    // `result := x` = 42.  The IIR lowering emits the label as a block and
    // `jmp` / `jmp_if_true` for the conditional — the same skeleton every
    // backend already lowers for `for` and `if`.  Exit 42.
    Prog {
        lang: Language::Algol60,
        ext: "alg",
        src: "begin integer x, result; x := 0; \
               loop: x := x + 7; if x < 42 then goto loop; result := x end",
        expect: Expect::Exit(42),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // ALGOL 60 — *`for … while`* (LANG-FULL AL11). ALGOL 60 report §4.6.4:
    // a `for` element of the form `expr while cond` re-evaluates the
    // expression first, then tests the condition; if false the whole `for`
    // exits.  Here `i := i + 6 while i <= 36` advances `i` by 6 each
    // iteration; the body captures the stepped value into `result`.
    // Trace: i=6 (6<=36 → result=12), 12 (→18), 18 (→24), 24 (→30),
    // 30 (→36), 36 (→42), 42 (42<=36 false, exit) → result=42.
    // The `emit_for_while` IIR lowering emits a loop label, the expression
    // code, a `jmp_if_false` to the exit, the body, and an unconditional
    // `jmp` back.  All 7 backends lower this loop shape already (it's the
    // same `jmp`/`jmp_if_*`/`label` skeleton as the `step-until` loop).
    Prog {
        lang: Language::Algol60,
        ext: "alg",
        src: "begin integer i, result; i := 0; \
               for i := i + 6 while i <= 36 do result := i + 6 end",
        expect: Expect::Exit(42),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // ALGOL 60 — real-valued procedure (LANG-FULL AL13). `scale(x)` is a `real
    // procedure` that multiplies its argument by 6.0 and assigns the product to
    // the implicit return slot `scale`.  The caller applies the standard function
    // `entier` (§3.2.4) to floor the f64 result to an integer exit code.
    //
    // This proves three things simultaneously:
    //   1. A `real` return slot is declared for the procedure name (same path as
    //      `integer` / `boolean` procedures; the compiler seeds it with `0.0`).
    //   2. `emit_call_common` returns `ExprValue { ty: ScalarType::Real }` for a
    //      real-returning call, so `emit_entier` sees the right type without any
    //      special-case code.
    //   3. All 7 backends can propagate an f64 through a `call` instruction and
    //      hand it to `real_to_int_floor` in the calling function.
    //
    // scale(7.0) = 7.0 × 6.0 = 42.0; entier(42.0) = 42.
    Prog {
        lang: Language::Algol60,
        ext: "alg",
        src: "begin real procedure scale(x); value x; real x; scale := x * 6.0; \
               integer result; result := entier(scale(7.0)) end",        expect: Expect::Exit(42),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // ALGOL 60 — *boolean variable* (LANG-FULL AL10). `boolean b` declares a
    // slot at type_hint `"bool"` (LLVM `i1`, JVM/CLR `int32 0/1`, VM
    // `Value::Bool`).  `b := true` lowers to `const _t0 = 1 / mov b, _t0`
    // (both at `"bool"` type_hint).  The `if b` condition forwards the slot
    // directly to `jmp_if_true`; since `b`'s env entry is the literal `1` and
    // the instr's type_hint maps to `i1`, LLVM branches without a redundant
    // trunc.  Every other backend already handles `bool`-typed conditions for
    // comparison results; this cell proves a *named, mutable variable* of
    // declared type `boolean` reaches every backend's branch op correctly.
    // ALGOL 60 report §5.1: `boolean` is a primitive scalar type on equal
    // footing with `integer` and `real`.  Exit 42.
    Prog {
        lang: Language::Algol60,
        ext: "alg",
        src: "begin boolean b; integer result; b := true; \
               if b then result := 42 else result := 0 end",
        expect: Expect::Exit(42),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // ALGOL 60 — *boolean `not`* (LANG-FULL AL10).  The unary `not` op
    // flips a `false` slot to `true`.  In LLVM it lowers to `xor i1 %b, 1`;
    // in WASM to `i32.eqz` on the i32 bool representation; in JVM to a
    // `ifeq`/`goto` pattern; in CLR to `ldc.i4.0` + `ceq`; in the VM to
    // the `not` dispatch arm.  `b := false; if not b` → the then-arm fires →
    // exit 42.  Together with the cell above this proves both `true` and
    // `false` literals, direct boolean variable use, and logical negation.
    Prog {
        lang: Language::Algol60,
        ext: "alg",
        src: "begin boolean b; integer result; b := false; \
               if not b then result := 42 else result := 0 end",
        expect: Expect::Exit(42),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // ALGOL 60 — *boolean `and` + `not` compound* (LANG-FULL AL10). Two
    // boolean variables; the compound condition `a and (not b)` with
    // `a = true`, `b = false` evaluates to `true and true = true`.  Exercises
    // the two-operand `and` IIR op (LLVM `and i1`, WASM `i32.and`, JVM/CLR
    // integer AND, VM `and`), wired up after a `not`-inverted sub-expression.
    // Proves compound boolean algebra over named variables works end-to-end
    // on all 7 backends.  Exit 42.
    Prog {
        lang: Language::Algol60,
        ext: "alg",
        src: "begin boolean a, b; integer result; a := true; b := false; \
               if a and (not b) then result := 42 else result := 0 end",
        expect: Expect::Exit(42),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // ALGOL 60 — *nested block variable shadowing* (LANG-FULL AL12). ALGOL 60
    // report §2.7: each `begin … end` is a block with its own declarations;
    // inner declarations shadow outer ones.  This program nests three blocks,
    // each declaring its own `integer x` and `boolean flag`.  The compiler
    // disambiguates them by scope; the IIR uses unique slot names so every
    // backend sees plain SSA/register values with no aliasing.
    // Trace: outer x=1, flag=true; middle x=10, flag=false; inner x=31;
    // `if not flag` uses middle's flag=false → not false=true → result:=31;
    // after inner: result := 31 + middle-x(10) = 41;
    // after middle: `if flag` uses outer flag=true → result := 41 + outer-x(1)
    // = 42.  Exit 42.
    Prog {
        lang: Language::Algol60,
        ext: "alg",
        src: "begin integer x, result; boolean flag; x := 1; flag := true; result := 0; \
               begin integer x; boolean flag; x := 10; flag := false; \
               begin integer x; x := 31; \
               if not flag then result := x else result := 1 end; \
               result := result + x end; \
               if flag then result := result + x else result := 0 end",
        expect: Expect::Exit(42),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // ALGOL 60 — *single-value `for` element* (LANG-FULL AL11). The ALGOL
    // 60 for-list `for i := expr do body` with a single literal value
    // executes the body exactly once with `i = expr`.  The `emit_for_value`
    // IIR lowering emits a single `mov` + body block — no loop at all.
    // `for i := 2 do result := 40 + i` → result = 42.  Proves the
    // degenerate single-element list path on all 7 backends.
    Prog {
        lang: Language::Algol60,
        ext: "alg",
        src: "begin integer i, result; for i := 2 do result := 40 + i end",
        expect: Expect::Exit(42),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // ALGOL 60 — *multi-element `for` list* (LANG-FULL AL11). The for-list
    // `1 step 1 until 3, 10, i + 1 while i < 13` sequences three kinds of
    // for-element in a single `for` head: (1) `step-until` (i=1,2,3),
    // (2) a single literal value (i=10), (3) `while` (i=11,12).
    // Sum = 1+2+3+10+11+12 = 39; `result := result + 3` brings it to 42.
    // The multi-element lowering emits each element's control-flow block in
    // sequence; on exit of one the next element's init code runs.  No backend
    // needed a change — the loop skeleton already existed.  Exit 42.
    Prog {
        lang: Language::Algol60,
        ext: "alg",
        src: "begin integer i, result; i := 0; result := 0; \
               for i := 1 step 1 until 3, 10, i + 1 while i < 13 do \
               result := result + i; result := result + 3 end",
        expect: Expect::Exit(42),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // ALGOL 60 — *two-dimensional integer array* (LANG-FULL AL-multidim).
    // `integer array M[1:2, 1:2]` declares a 2×2 matrix in ALGOL 60 report
    // §5.2.1 syntax.  The `algol-iir-compiler` (v0.23.0) lowers it to a
    // **single** flat `alloc_array` of length 4 (= 2×2): subscript `M[i, j]`
    // translates to the 0-based flat index `(i − 1)*2 + (j − 1)` using the
    // row-major formula `flat = Σ_d (sub[d] − lower[d]) * stride[d]` where
    // `stride[last] = 1` (the multiply is elided) and outer strides are
    // computed right-to-left during declaration.  The program stores four
    // constants: `M[1,1]=10; M[1,2]=20; M[2,1]=5; M[2,2]=7`, then reads
    // them all back: `result := M[1,1] + M[1,2] + M[2,1] + M[2,2]` = 42.
    // The emitted IIR contains only `alloc_array`/`array_set`/`array_get`
    // with flat indices — **no backend change required** — so the cell runs on
    // **all 7 backends** identically to E5 straight-line arrays.  Exit 42.
    Prog {
        lang: Language::Algol60,
        ext: "alg",
        src: "begin integer array M[1:2, 1:2]; integer result; \
               M[1, 1] := 10; M[1, 2] := 20; M[2, 1] := 5; M[2, 2] := 7; \
               result := M[1, 1] + M[1, 2] + M[2, 1] + M[2, 2] end",
        expect: Expect::Exit(42),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // ALGOL 60 — *two-dimensional **real** array* (LANG-FULL AL-multidim-real).
    // Identical multidim flat-index machinery as the integer cell above, but the
    // element type is `real` (f64).  The `algol-iir-compiler` records `elem_ty =
    // Real` at declaration, so `alloc_array`/`array_set`/`array_get` carry the
    // fractional doubles on the same 8-byte slots that E5 1-D real arrays use —
    // only the *index* computation `(i−1)*2 + (j−1)` is multidim.  Four fractional
    // cells are stored: `M[1,1]=10.25; M[1,2]=10.25; M[2,1]=10.75; M[2,2]=10.75`,
    // summed with the f64 `add` path to `42.0`, then floored to an integer exit
    // code via the E8 `entier` (`real_to_int_floor`) conversion.  This proves f64
    // multidim elements — the follow-up flagged when AL-multidim first landed —
    // run on **all 7 backends** with no backend change.  Exit 42.
    Prog {
        lang: Language::Algol60,
        ext: "alg",
        src: "begin real array M[1:2, 1:2]; real sum; integer result; \
               M[1, 1] := 10.25; M[1, 2] := 10.25; \
               M[2, 1] := 10.75; M[2, 2] := 10.75; \
               sum := M[1, 1] + M[1, 2] + M[2, 1] + M[2, 2]; \
               result := entier(sum) end",
        expect: Expect::Exit(42),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // ALGOL 60 — *three-dimensional integer array* (LANG-FULL AL-multidim-3D).
    // Proves the multidim lowering is genuinely **N-dimensional**, not hardcoded
    // to 2-D.  For `M[1:2, 1:2, 1:2]` the `algol-iir-compiler` computes strides
    // right-to-left at declaration: `stride[2] = 1` (elided), `stride[1] = size[2]
    // = 2`, `stride[0] = size[1]*stride[1] = 4`.  Subscript `M[i,j,k]` lowers to
    // the flat 0-based index `(i−1)*4 + (j−1)*2 + (k−1)` — a single flat
    // `alloc_array` of length 8.  Three corner cells with **distinct** flat
    // indices are stored to exercise every stride: `M[1,1,1]=6` (flat 0),
    // `M[2,1,1]=16` (flat 4, proves stride[0]), `M[1,2,1]=20` (flat 2, proves
    // stride[1]); summed = 42.  The emitted IIR is still only `alloc_array`/
    // `array_set`/`array_get` with flat indices — **no backend change** — so it
    // runs on **all 7 backends** identically to the 1-D/2-D cells.  Exit 42.
    Prog {
        lang: Language::Algol60,
        ext: "alg",
        src: "begin integer array M[1:2, 1:2, 1:2]; integer result; \
               M[1, 1, 1] := 6; M[2, 1, 1] := 16; M[1, 2, 1] := 20; \
               result := M[1, 1, 1] + M[2, 1, 1] + M[1, 2, 1] end",
        expect: Expect::Exit(42),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // ALGOL 60 — *2-D array with arbitrary lower bounds* (LANG-FULL
    // AL-multidim-bounds).  Unlike BASIC's fixed 0-based arrays, ALGOL arrays
    // carry an explicit lower bound per dimension (`[lo:hi]`), so each subscript
    // is translated to `sub − lower` *before* the row-major stride is applied:
    // `flat = Σ_d (sub[d] − lower[d]) * stride[d]`.  This cell uses a **negative**
    // lower bound on one axis and a non-zero one on the other: `M[-1:1, 2:3]` has
    // sizes `(3, 2)`, strides `[2, 1]`, so `M[i,j]` → `(i−(−1))*2 + (j−2)`.
    // Stores `M[-1,2]=40` (flat `0*2+0 = 0`) and `M[1,3]=2` (flat `2*2+1 = 5`,
    // the last of 6 cells — proving both the lower-bound subtraction and the
    // stride), then `M[-1,2] + M[1,3]` = 42.  The negative bound is written
    // `0-1` since ALGOL number literals are unsigned.  Still only `alloc_array`/
    // `array_set`/`array_get` with a flat index — **no backend change** — so it
    // runs on **all 7 backends**.  Exit 42.
    Prog {
        lang: Language::Algol60,
        ext: "alg",
        src: "begin integer array M[0-1:1, 2:3]; integer result; \
               M[0-1, 2] := 40; M[1, 3] := 2; \
               result := M[0-1, 2] + M[1, 3] end",
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
    // Dartmouth BASIC — exponentiation with an integer-valued literal exponent
    // (LANG-FULL BA-^). General `^` still needs a math runtime for variable or
    // fractional exponents; this proof lowers `6 ^ 2` to repeated f64 `mul`,
    // then adds 6 and prints 42 on every backend.
    Prog {
        lang: Language::DartmouthBasic,
        ext: "bas",
        src: "10 PRINT 6 ^ 2 + 6\n20 END\n",
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
    // Dartmouth BASIC — BA4 string variables. The frontend accepts `$`-suffixed
    // names, lowers `LET A$ = "HI"` directly into a safe typed `str_const` slot,
    // and `PRINT A$` reuses the same `print_str` path proven by literal output.
    // This is deliberately still the literal-backed scalar slice: string arrays,
    // INPUT, and richer string expressions remain follow-ups.
    Prog {
        lang: Language::DartmouthBasic,
        ext: "bas",
        src: "10 LET A$ = \"HI\"\n20 PRINT A$\n30 END\n",
        expect: Expect::Stdout("HI"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Dartmouth BASIC — BA4 literal string reassignment. Re-emitting
    // `str_const` into the same backend-facing slot makes the most recent
    // literal assignment observable through `PRINT A$` without widening into
    // string-to-string copies or dynamic byte-string storage.
    Prog {
        lang: Language::DartmouthBasic,
        ext: "bas",
        src: "10 LET A$ = \"NO\"\n20 LET A$ = \"OK\"\n30 PRINT A$\n40 END\n",
        expect: Expect::Stdout("OK"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Dartmouth BASIC — BA4 literal string concatenation. The frontend lowers
    // `"O" + "K"` to E4 `str_const` + `str_concat`, stores the result in the
    // same safe string slot, and `PRINT A$` proves the resulting value.
    Prog {
        lang: Language::DartmouthBasic,
        ext: "bas",
        src: "10 LET A$ = \"O\" + \"K\"\n20 PRINT A$\n30 END\n",
        expect: Expect::Stdout("OK"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Dartmouth BASIC — BA4 literal-backed scalar string copy. `B$ = A$`
    // lowers to E4 `str_concat` with an empty suffix, proving immutable copy
    // semantics without a new string-copy opcode or dynamic string storage.
    Prog {
        lang: Language::DartmouthBasic,
        ext: "bas",
        src: "10 LET A$ = \"OK\"\n20 LET B$ = A$\n30 PRINT B$\n40 END\n",
        expect: Expect::Stdout("OK"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Dartmouth BASIC — BA4 multi-item string PRINT. `;` keeps adjacent output,
    // so two scalar string slots are consumed by ordered E4 `print_str` calls
    // without using concat or numeric formatting helpers.
    Prog {
        lang: Language::DartmouthBasic,
        ext: "bas",
        src: "10 LET A$ = \"O\"\n20 LET B$ = \"K\"\n30 PRINT A$; B$\n40 END\n",
        expect: Expect::Stdout("OK"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Dartmouth BASIC — BA4 string PRINT composes with BA2 comma separators.
    // The comma path emits a single `putchar(' ')` between the ordered E4
    // `print_str` calls, proving separators stay language-neutral and do not
    // route string items through numeric formatting.
    Prog {
        lang: Language::DartmouthBasic,
        ext: "bas",
        src: "10 LET A$ = \"O\"\n20 LET B$ = \"K\"\n30 PRINT A$, B$\n40 END\n",
        expect: Expect::Stdout("O K"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Dartmouth BASIC — BA4 string expression in PRINT. This keeps concat out
    // of an assignment target and proves `PRINT` can consume a temporary E4
    // string expression result directly.
    Prog {
        lang: Language::DartmouthBasic,
        ext: "bas",
        src: "10 LET A$ = \"O\"\n20 PRINT A$ + \"K\"\n30 END\n",
        expect: Expect::Stdout("OK"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Dartmouth BASIC — BA4 variable-variable string expression in PRINT.
    // This proves `PRINT` can consume a temporary E4 `str_concat` result when
    // both concat operands are scalar string slots, not only a slot + literal.
    Prog {
        lang: Language::DartmouthBasic,
        ext: "bas",
        src: "10 LET A$ = \"O\"\n20 LET B$ = \"K\"\n30 PRINT A$ + B$\n40 END\n",
        expect: Expect::Stdout("OK"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Dartmouth BASIC — BA4 string expression in IF equality. This proves the
    // relation path can consume a temporary E4 string expression result before
    // `str_eq` drives the existing line-control branch.
    Prog {
        lang: Language::DartmouthBasic,
        ext: "bas",
        src: "10 LET A$ = \"O\"\n20 IF A$ + \"K\" = \"OK\" THEN 50\n30 PRINT \"BAD\"\n40 END\n50 PRINT \"OK\"\n60 END\n",
        expect: Expect::Stdout("OK"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Dartmouth BASIC — BA4 variable-variable string expression in IF
    // equality. This extends the equality branch proof beyond `A$ + literal`:
    // both concat operands are scalar string slots, the temporary feeds E4
    // `str_eq`, and `jmp_if_true` takes the line-control target.
    Prog {
        lang: Language::DartmouthBasic,
        ext: "bas",
        src: "10 LET A$ = \"O\"\n20 LET B$ = \"K\"\n30 IF A$ + B$ = \"OK\" THEN 60\n40 PRINT \"BAD\"\n50 END\n60 PRINT \"OK\"\n70 END\n",
        expect: Expect::Stdout("OK"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Dartmouth BASIC — BA4 string expression in IF inequality. This composes
    // the variable-variable concat proof with the `<>` branch path: E4
    // `str_eq` is still the only compare op, but the line-control jump is
    // `jmp_if_false`, so stdout proves the false-equality path is taken.
    Prog {
        lang: Language::DartmouthBasic,
        ext: "bas",
        src: "10 LET A$ = \"O\"\n20 LET B$ = \"K\"\n30 IF A$ + B$ <> \"NO\" THEN 60\n40 PRINT \"BAD\"\n50 END\n60 PRINT \"OK\"\n70 END\n",
        expect: Expect::Stdout("OK"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Dartmouth BASIC — BA4 string expression assignment with a variable
    // operand. This proves a non-literal concat can be stored in another safe
    // scalar string slot before printing.
    Prog {
        lang: Language::DartmouthBasic,
        ext: "bas",
        src: "10 LET A$ = \"O\"\n20 LET B$ = A$ + \"K\"\n30 PRINT B$\n40 END\n",
        expect: Expect::Stdout("OK"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Dartmouth BASIC — BA4 chained string expression assignment. This takes
    // the variable-backed concat proof beyond two operands: `A$ + "B" + "C"`
    // lowers left-to-right through two E4 `str_concat` ops, and the final
    // concat stores directly in the target scalar string slot.
    Prog {
        lang: Language::DartmouthBasic,
        ext: "bas",
        src: "10 LET A$ = \"A\"\n20 LET B$ = A$ + \"B\" + \"C\"\n30 PRINT B$\n40 END\n",
        expect: Expect::Stdout("ABC"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Dartmouth BASIC — BA4 string equality drives control flow. `IF A$ = "Y"`
    // lowers to shared E4 `str_eq`, then the existing branch machinery chooses
    // the target line. The false path prints BAD and stops; the true path prints
    // OK, so stdout proves both the variable/string comparison and the branch.
    Prog {
        lang: Language::DartmouthBasic,
        ext: "bas",
        src: "10 LET A$ = \"Y\"\n20 IF A$ = \"Y\" THEN 50\n30 PRINT \"BAD\"\n40 END\n50 PRINT \"OK\"\n60 END\n",
        expect: Expect::Stdout("OK"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Dartmouth BASIC — BA4 copied string slots in control flow. This composes
    // the scalar copy foothold with `str_eq` over two string slots, proving
    // variable-to-variable equality rather than only slot-vs-literal equality.
    Prog {
        lang: Language::DartmouthBasic,
        ext: "bas",
        src: "10 LET A$ = \"OK\"\n20 LET B$ = A$\n30 IF B$ = A$ THEN 60\n40 PRINT \"BAD\"\n50 END\n60 PRINT \"OK\"\n70 END\n",
        expect: Expect::Stdout("OK"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Dartmouth BASIC — BA4 string inequality drives control flow too. The
    // frontend reuses E4 `str_eq` but targets the THEN line with `jmp_if_false`,
    // proving the standard `<>` relop without adding a new string-compare op.
    Prog {
        lang: Language::DartmouthBasic,
        ext: "bas",
        src: "10 LET A$ = \"N\"\n20 IF A$ <> \"Y\" THEN 50\n30 PRINT \"BAD\"\n40 END\n50 PRINT \"OK\"\n60 END\n",
        expect: Expect::Stdout("OK"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Dartmouth BASIC — BA4 lexical string ordering drives control flow. The
    // frontend lowers `$` string `<` / `>` relops through E4 `str_cmp`, compares
    // the ordering result with zero using typed `cmp_lt` / `cmp_gt`, and then
    // uses the existing line-control `jmp_if_true` path.
    Prog {
        lang: Language::DartmouthBasic,
        ext: "bas",
        src: "10 LET A$ = \"ALPHA\"\n20 IF A$ < \"BETA\" THEN 40\n30 END\n40 IF \"BETA\" > A$ THEN 60\n50 END\n60 PRINT \"OK\"\n70 END\n",
        expect: Expect::Stdout("OK"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Dartmouth BASIC — `FOR`/`NEXT` loop with an accumulator (LANG-FULL BA0). Sums
    // 1..5 into S and prints 15. FOR/NEXT lowers to `cmp_le`, which the WASM and LLVM
    // backends could not run correctly until this slice (LLVM compared at `i1` width;
    // the BASIC compiler now emits the `i64` operand type — see its CHANGELOG). Until
    // now BASIC loops executed only on the VM/JIT; this RUNS a real FOR loop on the
    // code-gen backends.
    //
    // BA-JVM-1 is now resolved: `iir-to-jvm-class-file` ≥ 0.13.3 correctly generates
    // StackMapTable frames for backward branches (loops) combined with `print_i64` calls.
    // JVM is included in the backends list above.
    Prog {
        lang: Language::DartmouthBasic,
        ext: "bas",
        src: "10 LET S = 0\n20 FOR I = 1 TO 5\n30 LET S = S + I\n40 NEXT I\n50 PRINT S\n60 END\n",
        expect: Expect::Stdout("15"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Dartmouth BASIC — `FOR … STEP` with step size 2 (LANG-FULL BA-step). The
    // `STEP` clause stores its value into a dedicated `_for_<n>_step` IIR slot
    // (a `const 2` at `"i64"` type_hint); each `NEXT I` adds `step` to `I`
    // before re-testing `I <= limit`.  Here I iterates 1 → 3 → 5 → (7 > 5
    // exits); S accumulates 1+3+5 = 9.  Without STEP the loop would run 5
    // iterations summing to 15 — the differing output distinguishes STEP-2 from
    // default STEP-1.  Integer STEP keeps S on the `i64` track so `PRINT S`
    // emits "9" via the integer helper on every backend.
    Prog {
        lang: Language::DartmouthBasic,
        ext: "bas",
        src: "10 LET S = 0\n20 FOR I = 1 TO 5 STEP 2\n30 LET S = S + I\n40 NEXT I\n50 PRINT S\n60 END\n",
        expect: Expect::Stdout("9"),
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
    // Dartmouth BASIC — *two-dimensional real arrays* (LANG-FULL BA-DIM-2D,
    // enabler **E5**).  `DIM A(1,2)` declares a 2×3 matrix (0-based inclusive:
    // rows `A(0..1, …)`, cols `A(…, 0..2)`), lowered to a **single** flat
    // `alloc_array` of `(1+1)*(2+1) = 6` `f64` elements.  A subscript `A(i,j)`
    // folds through the row-major strides recorded at `DIM` — `stride = [3, 1]`
    // — to the flat 0-based index `i*3 + j` (a `const 3` + `mul` + `add`), so no
    // new IIR op and no backend change: it runs on the same E5 `alloc_array`/
    // `array_set`/`array_get` substrate every backend already supports.  Stores
    // `A(0,0)=40` (flat 0) and `A(1,2)=2` (flat 5, the last cell — proving the
    // row stride), then `PRINT A(0,0) + A(1,2)` reads both back ⇒ prints 42.
    // Straight-line (no loop), so — like the BA3 1-D array cell — the JVM
    // loop+print StackMapTable follow-up doesn't apply and all 7 backends run.
    Prog {
        lang: Language::DartmouthBasic,
        ext: "bas",
        src: "10 DIM A(1,2)\n20 LET A(0,0) = 40\n30 LET A(1,2) = 2\n40 PRINT A(0,0) + A(1,2)\n50 END\n",
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
    // Dartmouth BASIC — `SQR` (square root) built-in (LANG-FULL BA-builtins).
    // SQR(X) lowers to the f64_sqrt IIR op (same hardware instruction that
    // ALGOL sqrt uses).  SQR(49) is exactly 7.0 — a whole-valued real — so
    // BA7's formatter prints `7` with no decimal point.
    Prog {
        lang: Language::DartmouthBasic,
        ext: "bas",
        src: "10 PRINT SQR(49)\n20 END\n",
        expect: Expect::Stdout("7"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Dartmouth BASIC — `INT` (floor) built-in (LANG-FULL BA-builtins).
    // INT(X) = ⌊X⌋, returned as a real.  Lowers to real_to_int_floor +
    // int_to_real (both E8 ops).  INT(3.7) → 3.0, printed as `3`.
    Prog {
        lang: Language::DartmouthBasic,
        ext: "bas",
        src: "10 PRINT INT(3.7)\n20 END\n",
        expect: Expect::Stdout("3"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Dartmouth BASIC — `ABS` (absolute value) built-in (LANG-FULL BA-builtins).
    // ABS(X) is lowered inline: if X < 0 then −X else X (store-per-branch,
    // same pattern as ALGOL abs).  ABS(-42) → 42.0, printed as `42`.
    Prog {
        lang: Language::DartmouthBasic,
        ext: "bas",
        src: "10 PRINT ABS(-42)\n20 END\n",
        expect: Expect::Stdout("42"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Dartmouth BASIC — `SGN` (signum) built-in (LANG-FULL BA-builtins).
    // SGN(X) = 1.0 if X > 0, −1.0 if X < 0, 0.0 if X = 0.  Lowered
    // inline as a 3-way conditional.  SGN(-5) → −1.0, printed as `-1`.
    Prog {
        lang: Language::DartmouthBasic,
        ext: "bas",
        src: "10 PRINT SGN(-5)\n20 END\n",
        expect: Expect::Stdout("-1"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Dartmouth BASIC — `ATN` (arctangent) built-in (LANG-FULL BA-arctan).
    // ATN(X) lowers to the f64_atan IIR op which calls libm `atan` on every
    // backend.  ATN(0) = 0.0 exactly in IEEE-754 double; BA7's formatter
    // prints whole-valued reals without a decimal point, so output is `0`.
    Prog {
        lang: Language::DartmouthBasic,
        ext: "bas",
        src: "10 PRINT ATN(0)\n20 END\n",
        expect: Expect::Stdout("0"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Dartmouth BASIC — `TAN` (tangent) built-in (LANG-FULL BA-arctan).
    // TAN(X) lowers to the f64_tan IIR op which calls libm `tan` on every
    // backend.  TAN(0) = 0.0 exactly; output is `0`.
    Prog {
        lang: Language::DartmouthBasic,
        ext: "bas",
        src: "10 PRINT TAN(0)\n20 END\n",
        expect: Expect::Stdout("0"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Dartmouth BASIC — general `^` exponentiation via f64_pow IIR op (LANG-FULL BA-pow).
    // 4 ^ 0.5 = pow(4.0, 0.5) = 2.0 exactly; printed as "2" by __basic_print_real
    // (no decimal point when fractional part is zero).  Non-integer exponent exercises
    // the new runtime pow path; the literal-integer fast path stays for whole-number
    // exponents so this cell is the minimal proof of the general case.
    Prog {
        lang: Language::DartmouthBasic,
        ext: "bas",
        src: "10 PRINT 4 ^ 0.5\n20 END\n",
        expect: Expect::Stdout("2"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Dartmouth BASIC — scalar `INPUT` statement (LANG-FULL BA-INPUT). `INPUT X`
    // reads one line from stdin, parses it as an integer, and stores the result in
    // the variable `X`.  The frontend lowers this to `call_builtin "input_i64"` —
    // a new IIR builtin wired in this slice to all 7 backends:
    //   • native / LLVM: `__twig_input_i64()` in `twig_runtime.c` (already present)
    //   • WASM: `env.__input_i64()` host import (new `InputI64Func` here)
    //   • JVM: `env.BasicRuntime.readLong()J` (new method in BASIC_RUNTIME_JAVA)
    //   • CLR: `Console.ReadLine()` + `Int32.Parse()` in iir-to-cil-bytecode
    //   • VM/JIT: registered closures below (same pattern as `getchar`)
    // Stdin "42\n" → X = 42 → PRINT X ⇒ "42".  The newline-terminated input
    // proves the line-draining logic works correctly on every backend.
    Prog {
        lang: Language::DartmouthBasic,
        ext: "bas",
        src: "10 INPUT X\n20 PRINT X\n30 END\n",
        expect: Expect::Stdout("42"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Dartmouth BASIC — two sequential `INPUT` reads, then arithmetic (BA-INPUT).
    // This proves that two independent calls to `input_i64` drain successive lines
    // from stdin: A reads `10`, B reads `32`, PRINT A + B ⇒ `42`.  On the WASM
    // and VM/JIT columns the shared stdin buffer must advance past the first `\n`
    // before the second read — a single-byte drainer (`getchar`) would fail here.
    Prog {
        lang: Language::DartmouthBasic,
        ext: "bas",
        src: "10 INPUT A\n20 INPUT B\n30 PRINT A + B\n40 END\n",
        expect: Expect::Stdout("42"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Dartmouth BASIC — a *runtime (non-foldable) string* chosen by control flow
    // (LANG-FULL E4-dyn foothold). `INPUT N` reads an integer at run time, so
    // which string literal `A$` holds at line 60 — `"HI"` (N>0) or `"LO"` — is
    // **not known at compile time**: `A$` is a genuinely dynamic string value,
    // unlike every prior E4 cell where the compiler could fold the string to a
    // constant. This is the first matrix proof of a runtime string, and it runs
    // today on the already-dynamic columns — VM/JIT (tagged value), JVM
    // (`java.lang.String` local), CLR (`System.String` local) — which carry a
    // reassigned-across-branches string slot natively, plus the two static
    // backends whose runtime heap-string lowering has landed: Llvm (E4d-2) and
    // Wasm (E4d-3), and NativeAot (E4d-4, aarch64 + x86_64). On the native
    // columns `str_const` already builds a `[i64 len][bytes]` heap buffer and
    // stores its address in the variable's stack slot (`mov dest = buf`); the
    // E4d-4 fix keeps a branch-selected string OUT of `twig-aot`'s compile-time
    // literal map so `print_str` reads the length from the buffer header at run
    // time (`field_load` + `__twig_print_string`) rather than folding one branch's
    // constant. Each E4-dyn backend PR
    // (E4d-2 LLVM → E4d-3 WASM → E4d-4 native) extends THIS cell's `backends`
    // list once its runtime string lowering lands. On WASM (E4d-3) a
    // branch-selected string variable carries an i32 **handle** = the offset of
    // a length-prefixed block `[i32 len][bytes]` in linear memory; `print_str`
    // reads the length back with `i32.load` and calls `env.__print_str(ptr,len)`
    // — so the value is genuinely chosen at run time, not folded. Stdin `1` →
    // N=1>0 → `A$="HI"` → prints `HI`.
    Prog {
        lang: Language::DartmouthBasic,
        ext: "bas",
        src: "10 INPUT N\n20 IF N > 0 THEN 50\n30 LET A$ = \"LO\"\n40 GOTO 60\n50 LET A$ = \"HI\"\n60 PRINT A$\n70 END\n",
        expect: Expect::Stdout("HI"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Dartmouth BASIC — **string** `INPUT A$` (LANG-FULL E4-dyn, BA string INPUT
    // foothold). Unlike the numeric `INPUT X` (which parses the line to an i64) and
    // the foothold above (which reads a *number* and merely *selects* between two
    // compile-time string literals), this reads the whole stdin line **as the
    // string value itself**: `A$` holds bytes that never appear anywhere in the
    // program source, so the compiler cannot fold it. The frontend lowers `INPUT A$`
    // to a new `call_builtin "input_str"` (a `str`-typed sibling of `input_i64`),
    // then `mov`s the runtime handle into the `$`-variable's string slot; `PRINT A$`
    // consumes that slot through the shared E4 `print_str` op. This is the first
    // matrix proof that a runtime string can *originate at the input boundary*.
    // It runs on the dynamic VM/JIT columns (a tagged `Value::Str` read from the
    // shared stdin buffer by the registered `input_str` closure) and the two
    // **managed** columns whose `str` is already a host string object:
    //   • JVM — `env.BasicRuntime.readLine()Ljava/lang/String;` returns a
    //     `java.lang.String`, which is exactly how `str` is carried on the JVM
    //     (`iir_type_to_jvm("str") = Ref`); the reference `astore`s into the slot.
    //   • CLR — `System.Console.ReadLine()` returns a `System.String`, which is the
    //     CLR representation of a `str` local; it stores straight in.
    // The managed columns (JVM/CLR) needed no new value-model — only a read-a-line
    // host primitive returning the backend's native string. The **static** columns
    // (NativeAot, Llvm) carry a `str` as an i64 **handle** — the base address of a
    // length-prefixed `[i64 len][bytes]` heap block (the same repr `str_eq` /
    // `print_str` already read) — so they gain a host primitive that BUILDS such a
    // block from the input line:
    //   • NativeAot — `__twig_input_str()` in `twig_runtime.c` reads a line and
    //     returns a handle to a fresh `alloc_bytes` block; the aarch64/x86_64
    //     backends add it to their `V1_BUILTINS` table (0-arg / returns-i64, the
    //     exact shape of `input_i64` — the returned pointer rides x0/RAX), so NO
    //     new codegen. `print_str` reads the header length at run time.
    //   • Llvm — the same `@__twig_input_str()` from the AOT archive; `iir-to-llvm`
    //     lowers `call_builtin "input_str"` to `call i64 @__twig_input_str()` and
    //     carries the handle as an i64 (str→i64 at boundaries, E4d-2b).
    // WASM (E4d): `env.__input_str(block, max)` writes the whole `[i32 len][bytes]`
    // block into linear memory; iir-to-wasm bump-allocates the block from
    // `__array_bump` and passes its base. With this column BASIC string `INPUT A$`
    // runs on **all seven backends**. Stdin "OK\n" → A$ = "OK" → prints "OK".
    Prog {
        lang: Language::DartmouthBasic,
        ext: "bas",
        src: "10 INPUT A$\n20 PRINT A$\n30 END\n",
        expect: Expect::Stdout("OK"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // BA runtime string CONCAT — `str_concat` over two operands that are *both* read
    // from `INPUT`, so neither carries any compile-time string metadata (no data-segment
    // literal, no known length). Every earlier `str_concat` cell had at least one operand
    // the compiler could fold to a literal; here the concatenation can only happen at run
    // time. Stdin "OK\n!\n" → A$="OK", B$="!", and `PRINT A$ + B$` ⇒ "OK!".
    //
    // The four columns whose `str_concat` is *already* a runtime operation — no new
    // lowering needed:
    //   • Vm / Jit — a `str` is a tagged `Value::Str`; concat allocates a fresh tagged
    //     string from the two operands' bytes, wholly at run time.
    //   • Jvm — the two `str` locals are `java.lang.String` references (the `astore`d
    //     results of `readLine()`); `str_concat` lowers to a `String` concat that builds
    //     a new `String` from them — the operands need no compile-time identity.
    //   • Clr — the same story with `System.String::Concat(string, string)`.
    // The two **static** columns gain a genuine runtime-operand path in this cell — their
    // `str_concat` was literal-fold-only before, so both now route a non-foldable concat
    // to the runtime helper `__twig_str_concat(a, b)` (in `twig_runtime.c`), which reads
    // both `[i64 len][bytes]` headers and returns a handle to a fresh joined block:
    //   • NativeAot — twig-aot keeps the both-literal fold, but emits `call_builtin
    //     "str_concat"` (2-arg / returns-i64) when an operand isn't foldable; the
    //     aarch64/x86_64 backends add `str_concat` to `V1_BUILTINS` (same shape as
    //     `str_eq`, so NO new codegen — the handles ride x0/x1 | RDI/RSI, result in
    //     x0/RAX). aarch64 run-verified locally; x86_64 on CI.
    //   • Llvm — `iir-to-llvm` keeps the literal fold, but lowers a runtime concat to
    //     `%r = call i64 @__twig_str_concat(i64 a, i64 b)` against the AOT archive symbol
    //     (str→i64 handle, E4d-2b), storing the result as a runtime string. Via clang.
    //   • Wasm — no libc and no host helper: the concat happens entirely *in* wasm.
    //     `iir-to-wasm` bump-allocates a `[i32 len][bytes]` block from `__array_bump`,
    //     writes the `i32` length header, and splices both operands' bytes with two
    //     `memory.copy` (bulk-memory `0xFC 0x0A`) instructions — no scratch locals (each
    //     operand length is re-read from its header with `i32.load`). The in-repo
    //     `wasm-execution` interpreter gained a `0xFC` decoder + `LinearMemory::copy`
    //     to execute it. With this column runtime `str_concat` runs on **all seven
    //     backends**.
    Prog {
        lang: Language::DartmouthBasic,
        ext: "bas",
        src: "10 INPUT A$\n20 INPUT B$\n30 PRINT A$ + B$\n40 END\n",
        expect: Expect::Stdout("OK!"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // Dartmouth BASIC — **string arrays** (LANG-FULL E4-dyn, work item
    // E4d-BA-arr).  `DIM A$(2)` allocates an `array<str>`: the E5 length-
    // prefixed aggregate substrate carrying an E4-dyn runtime string *handle*
    // per element instead of an `f64`.  `A$(0)`/`A$(1)` are assigned string
    // literals through a `str`-typed `array_set`, and `PRINT A$(0) + A$(1)`
    // reads them back through two `str`-typed `array_get`s and concatenates —
    // so the printed `OK` (not `OO`/`KK`) proves the two element slots are
    // distinct and the handles survive a store→load round-trip through the
    // aggregate.  Runs on **all seven backends**, each with its native
    // representation of a `str` element:
    //   • **VM/JIT** — a tagged `Value::Str` array element.
    //   • **WASM** — a 4-byte i32 handle per element (`i32.store`/`i32.load`, the
    //     E4d-BA-arr `wasm_array_elem` branch + folded-literal→array_set promotion).
    //   • **LLVM** — an 8-byte i64 handle per element (`str`→`i64`); `array_set`
    //     `ptrtoint`s a folded literal's global to the i64 handle.
    //   • **NativeAot** — an 8-byte handle (address of the `[i64 len][bytes]` block);
    //     `native_array_elem_size` accepts `str` as an 8-byte element on x86_64/aarch64.
    //   • **JVM** — a `java.lang.String[]` (`anewarray` + `aaload`/`aastore`).
    //   • **CLR** — a `System.String[]` (`newarr` + `ldelem.ref`/`stelem.ref`).
    Prog {
        lang: Language::DartmouthBasic,
        ext: "bas",
        src: "10 DIM A$(2)\n20 LET A$(0) = \"O\"\n30 LET A$(1) = \"K\"\n\
               40 PRINT A$(0) + A$(1)\n50 END\n",
        expect: Expect::Stdout("OK"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // COBOL-60 — literal `DISPLAY` (PL09 step 4, the `cobol-iir-compiler` minimal
    // slice). A four-division program whose PROCEDURE DIVISION `DISPLAY`s a string
    // literal lowers to the shared E4 `str_const` + `print_str` op pair (then a
    // `putchar('\n')` record terminator) — exactly the string-output substrate
    // Dartmouth BASIC and ALGOL 60 already prove on every backend. So COBOL's
    // first matrix cell is stdout on all seven columns with no COBOL-specific
    // backend hooks. The source is carded into the fixed 80-column format
    // (6 sequence columns + indicator, code from column 8). Stdout is trimmed,
    // so the trailing newline is immaterial.
    Prog {
        lang: Language::Cobol60,
        ext: "cob",
        src: "000000 IDENTIFICATION DIVISION.\n\
               000000 PROGRAM-ID. HELLO.\n\
               000000 PROCEDURE DIVISION.\n\
               000000 MAIN.\n\
               000000     DISPLAY \"HELLO\".\n\
               000000     STOP RUN.",
        expect: Expect::Stdout("HELLO"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // COBOL-60 — `MOVE` a numeric literal into a PICTURE-typed item, then
    // `DISPLAY` the item (PL09 step 4). This is the cell that proves the
    // *data model*: `01 N PIC 9(5)` is a fixed-width numeric field, so `MOVE 42`
    // stores the zero-filled image `00042` (not `42`) and `DISPLAY N` shows those
    // five digits. Because this rung has no arithmetic, the compiler formats the
    // literal into its picture image at compile time (reusing cobol-runtime's own
    // `move_into_numeric`) and emits it as one `str_const` — so, like the literal
    // cell, it is the shared string-output substrate on all seven backends. The
    // leading zeros survive stdout trimming (only surrounding whitespace is cut),
    // so `00042` is positive proof the field reshaped the value.
    Prog {
        lang: Language::Cobol60,
        ext: "cob",
        src: "000000 IDENTIFICATION DIVISION.\n\
               000000 PROGRAM-ID. P.\n\
               000000 DATA DIVISION.\n\
               000000 WORKING-STORAGE SECTION.\n\
               000000 01  N  PIC 9(5).\n\
               000000 PROCEDURE DIVISION.\n\
               000000 MAIN.\n\
               000000     MOVE 42 TO N.\n\
               000000     DISPLAY N.\n\
               000000     STOP RUN.",
        expect: Expect::Stdout("00042"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // COBOL-60 — integer arithmetic (PL09 step 4, PR2). Numeric items are now
    // scaled `i64` slots, so `ADD`/`MULTIPLY`/`SUBTRACT` lower to native `add` /
    // `mul` / `sub` on the slot, the result reduced to the field (magnitude, low
    // `int_digits` digits kept), and `DISPLAY` renders the slot through the
    // fixed-width digit helper. Here R starts 0: `+7 = 7`, `×3 = 21`, `−1 = 20`,
    // shown as the two-digit field `20`. This proves the value survives the
    // store→load round-trip through the slot across three verbs, on every backend
    // that runs the shared integer-arithmetic + digit-print substrate.
    Prog {
        lang: Language::Cobol60,
        ext: "cob",
        src: "000000 IDENTIFICATION DIVISION.\n\
               000000 PROGRAM-ID. P.\n\
               000000 DATA DIVISION.\n\
               000000 WORKING-STORAGE SECTION.\n\
               000000 01  R  PIC 9(2) VALUE 0.\n\
               000000 PROCEDURE DIVISION.\n\
               000000 MAIN.\n\
               000000     ADD 7 TO R.\n\
               000000     MULTIPLY 3 BY R.\n\
               000000     SUBTRACT 1 FROM R.\n\
               000000     DISPLAY R.\n\
               000000     STOP RUN.",
        expect: Expect::Stdout("20"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // COBOL-60 — scaled-decimal ADD (PL09 step 4, PR3). `R PIC 9(2)V99` holds a
    // value scaled by 2, so it starts 1.50 (slot `150`). `ADD 2.25 TO R` aligns
    // the implied point — the literal folds to `225` at the same scale — sums to
    // `375`, and the receiver renders those four digits with no point: `0375`
    // (= 3.75). This proves the implied-point alignment lowers to plain `i64`
    // `add` on the scaled slots, on every backend running the shared substrate.
    Prog {
        lang: Language::Cobol60,
        ext: "cob",
        src: "000000 IDENTIFICATION DIVISION.\n\
               000000 PROGRAM-ID. P.\n\
               000000 DATA DIVISION.\n\
               000000 WORKING-STORAGE SECTION.\n\
               000000 01  R  PIC 9(2)V99 VALUE 1.5.\n\
               000000 PROCEDURE DIVISION.\n\
               000000 MAIN.\n\
               000000     ADD 2.25 TO R.\n\
               000000     DISPLAY R.\n\
               000000     STOP RUN.",
        expect: Expect::Stdout("0375"),
        backends: &[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit],
    },
    // COBOL-60 — IF / ELSE with a relational condition (PL09 step 4, PR4). The
    // condition lowers to a `cmp_gt` on the aligned values; `jmp_if_false` skips
    // the then-branch to the else. Here N=5 > 3 → the then-branch runs, printing
    // `BIG`. This proves the three-way branch (cmp / conditional jump / labels)
    // lowers correctly on every backend that runs the shared control-flow + print
    // substrate.
    Prog {
        lang: Language::Cobol60,
        ext: "cob",
        src: "000000 IDENTIFICATION DIVISION.\n\
               000000 PROGRAM-ID. P.\n\
               000000 DATA DIVISION.\n\
               000000 WORKING-STORAGE SECTION.\n\
               000000 01  N  PIC 9(3) VALUE 5.\n\
               000000 PROCEDURE DIVISION.\n\
               000000 MAIN.\n\
               000000     IF N GREATER 3 DISPLAY \"BIG\" ELSE DISPLAY \"SMALL\".\n\
               000000     STOP RUN.",
        expect: Expect::Stdout("BIG"),
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

/// Drain one newline-terminated line from a shared in-process stdin buffer and
/// return it as an owned `String` (the terminating `\n` is consumed, not kept).
/// This is the string sibling of the inline `input_i64` line-drain: BASIC
/// `INPUT A$` reads a *whole line as the string value*, so — unlike `input_i64` —
/// there is no `parse`/`trim` step. An empty or drained buffer yields `""` (EOF).
/// Shared by the VM and both JIT tiers' `input_str` closures so all three
/// registration sites agree byte-for-byte.
fn drain_stdin_line(
    buf: &std::sync::Arc<std::sync::Mutex<std::collections::VecDeque<u8>>>,
) -> String {
    let mut b = buf.lock().expect("lang-matrix stdin buffer poisoned");
    let mut line = Vec::new();
    loop {
        match b.pop_front() {
            None | Some(b'\n') => break,
            Some(byte) => line.push(byte),
        }
    }
    String::from_utf8_lossy(&line).into_owned()
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
        // BA-INPUT: single INPUT — reads "42\n", PRINT X ⇒ "42"
        (Language::DartmouthBasic, "10 INPUT X\n20 PRINT X\n30 END\n") => b"42\n",
        // BA-INPUT: two INPUTs — reads "10\n32\n", PRINT A + B ⇒ "42"
        (Language::DartmouthBasic, "10 INPUT A\n20 INPUT B\n30 PRINT A + B\n40 END\n") => b"10\n32\n",
        // E4-dyn foothold: `INPUT N` = 1 (>0) selects the `"HI"` branch for `A$`.
        (Language::DartmouthBasic, "10 INPUT N\n20 IF N > 0 THEN 50\n30 LET A$ = \"LO\"\n40 GOTO 60\n50 LET A$ = \"HI\"\n60 PRINT A$\n70 END\n") => b"1\n",
        // BA string INPUT: `INPUT A$` reads the whole line "OK" as the string value.
        (Language::DartmouthBasic, "10 INPUT A$\n20 PRINT A$\n30 END\n") => b"OK\n",
        // BA runtime string concat: two INPUT lines feed `str_concat` → "OK!".
        (Language::DartmouthBasic, "10 INPUT A$\n20 INPUT B$\n30 PRINT A$ + B$\n40 END\n") => b"OK\n!\n",
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
fn run_native(p: &Prog) -> Option<RunResult> {
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
    if matches!(&p.expect, Expect::Trap) && !out.status.success() {
        return Some(RunResult::Trapped);
    }
    Some(RunResult::Completed { code: out.status.code(), stdout })
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
    "#include <stdio.h>\n#include <stdint.h>\n#include <stdlib.h>\n#include <string.h>\n\
void __print_i64(int64_t x){printf(\"%lld\\n\",(long long)x);}\n\
void __print_str(const char* p,int64_t len){if(len>0){fwrite(p,1,(size_t)len,stdout);}}\n\
int64_t __twig_input_i64(void){char buf[64];int i=0;int c;\
while((c=getchar())!=EOF&&c!='\\n'&&i<63){buf[i++]=(char)c;}buf[i]=0;\
long long v=0;sscanf(buf,\"%lld\",&v);return (int64_t)v;}\n\
int64_t __twig_input_str(void){char buf[4096];int i=0;int c;\
while((c=getchar())!=EOF&&c!='\\n'&&i<4095){buf[i++]=(char)c;}\
int64_t* h=(int64_t*)malloc(8+(size_t)i);if(!h)return 0;\
h[0]=(int64_t)i;if(i>0)memcpy((char*)h+8,buf,(size_t)i);\
return (int64_t)(intptr_t)h;}\n\
int64_t __twig_str_concat(int64_t a,int64_t b){\
int64_t la=a?*(int64_t*)(intptr_t)a:0;int64_t lb=b?*(int64_t*)(intptr_t)b:0;\
if(la<0)la=0;if(lb<0)lb=0;\
int64_t* h=(int64_t*)malloc(8+(size_t)(la+lb));if(!h)return 0;\
h[0]=la+lb;\
if(la>0)memcpy((char*)h+8,(const char*)(intptr_t)a+8,(size_t)la);\
if(lb>0)memcpy((char*)h+8+la,(const char*)(intptr_t)b+8,(size_t)lb);\
return (int64_t)(intptr_t)h;}\n\
int64_t __twig_str_eq(int64_t a,int64_t b){\
int64_t la=a?*(int64_t*)(intptr_t)a:0;int64_t lb=b?*(int64_t*)(intptr_t)b:0;\
if(la!=lb)return 0;if(la<=0)return 1;\
return memcmp((const char*)(intptr_t)a+8,(const char*)(intptr_t)b+8,(size_t)la)==0?1:0;}\n";

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
fn run_llvm(p: &Prog) -> Option<RunResult> {
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
    // Link the generic I/O runtime iff the program actually uses print or input.
    if ll.contains("@__print_i64") || ll.contains("@__print_str")
        || ll.contains("@__twig_input_i64") || ll.contains("@__twig_input_str")
        || ll.contains("@__twig_str_concat") || ll.contains("@__twig_str_eq") {
        let rt_path = dir.path().join("rt.c");
        std::fs::write(&rt_path, PRINT_RUNTIME_C).ok()?;
        cmd.arg("-x").arg("c").arg(&rt_path);
    }
    // Link the tagged-value lisp runtime iff the program calls a `__dyn_*`
    // primitive (cons/car/box_int/… — McCarthy Lisp + Twig dynamic values,
    // E6d-2b). `dynval_runtime.c` implements the tagged-word model and calls the
    // conservative GC, which now lives in the `gc-core-capi` staticlib (twig_gc.c
    // was retired in #118b-2b); `twig_runtime.c` supplies any I/O the runtime
    // itself needs. The two C files come from the crate's runtime dir; the GC
    // archive (+ its system libs) is supplied by `common::gc_link_args`.
    if ll.contains("@__dyn_") {
        let rt = |name: &str| {
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../twig-aot/runtime").join(name)
        };
        cmd.arg("-x").arg("none")
            .arg(rt("dynval_runtime.c"))
            .args(common::gc_link_args())
            .arg(rt("twig_runtime.c"));
    }
    let built = cmd.arg("-x").arg("none").arg("-o").arg(&exe).output().ok()?;
    if !built.status.success() {
        return None;
    }
    // Same stdin wiring as `run_native`: a Brainfuck `,` reads libc `getchar` from the
    // process stdin; empty for every other program.
    let out = output_with_stdin(Command::new(&exe), program_stdin(p))?;
    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if matches!(&p.expect, Expect::Trap) && !out.status.success() {
        return Some(RunResult::Trapped);
    }
    Some(RunResult::Completed { code: out.status.code(), stdout })
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

/// `env.__pow(f64 base, f64 exp) -> f64` — libm `pow` for WASM modules that
/// emit the `f64_pow` IIR op (BA-pow: BASIC general `^` exponentiation).
/// Two f64 arguments in; one f64 result out.  Rust `f64::powf` matches libm
/// IEEE-754 semantics on all tier-1 platforms.
struct PowFunc;

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

/// `env.__input_i64() -> i64` — WASM host import for BASIC `INPUT X`.
/// Drains the stdin buffer line-by-line: reads bytes up to (and including) the
/// next `\n`, parses the trimmed ASCII decimal as an i64, and returns the value.
/// An empty or exhausted buffer returns 0 (matches `__twig_input_i64` EOF behaviour).
struct InputI64Func {
    input: std::sync::Arc<std::sync::Mutex<std::collections::VecDeque<u8>>>,
}

impl wasm_execution::HostFunction for InputI64Func {
    fn func_type(&self) -> &wasm_types::FuncType {
        static FT: std::sync::LazyLock<wasm_types::FuncType> =
            std::sync::LazyLock::new(|| wasm_types::FuncType {
                params: vec![],
                results: vec![wasm_types::ValueType::I64],
            });
        &FT
    }

    fn call(
        &self,
        _args: &[wasm_execution::WasmValue],
        _memory: Option<&mut wasm_execution::LinearMemory>,
    ) -> Result<Vec<wasm_execution::WasmValue>, wasm_execution::TrapError> {
        let mut buf = self
            .input
            .lock()
            .expect("lang-matrix wasm stdin buffer poisoned");
        let mut line = Vec::new();
        loop {
            match buf.pop_front() {
                None | Some(b'\n') => break,
                Some(b) => line.push(b),
            }
        }
        let s = String::from_utf8_lossy(&line);
        let v: i64 = s.trim().parse().unwrap_or(0);
        Ok(vec![wasm_execution::WasmValue::I64(v)])
    }
}

/// `env.__input_str(i32 block, i32 max) -> ()` — WASM host import for BASIC string
/// `INPUT A$` (E4-dyn). Drains one line from the stdin buffer (bytes up to the next
/// `\n`, newline consumed but not stored) and writes the WHOLE runtime-string block
/// `[i32 len][bytes]` into linear memory at `block`: an `i32` length header
/// (`store_i32`) then the bytes (`store_i32_8`). The length is capped at `max` (the
/// codegen reserved a `[i32 len][max bytes]` region); a longer line is truncated —
/// the V1 permissive contract. The handle the module keeps is `block`; `print_str`
/// reads the length back with `i32.load` at it. The wasm sibling of the native
/// `__twig_input_str` C helper.
struct InputStrFunc {
    input: std::sync::Arc<std::sync::Mutex<std::collections::VecDeque<u8>>>,
}

impl wasm_execution::HostFunction for InputStrFunc {
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
        let block = args.first()
            .ok_or_else(|| wasm_execution::TrapError::new("__input_str: missing block ptr"))?
            .as_i32()
            .map_err(|e| wasm_execution::TrapError::new(e.message))?;
        let max = args.get(1)
            .ok_or_else(|| wasm_execution::TrapError::new("__input_str: missing max"))?
            .as_i32()
            .map_err(|e| wasm_execution::TrapError::new(e.message))?;
        if block < 0 || max < 0 {
            return Err(wasm_execution::TrapError::new("__input_str: negative block/max"));
        }
        let memory = memory
            .ok_or_else(|| wasm_execution::TrapError::new("__input_str: no linear memory"))?;
        let base = usize::try_from(block)
            .map_err(|_| wasm_execution::TrapError::new("__input_str: block overflow"))?;
        // Drain one line from the shared stdin buffer.
        let mut line = Vec::new();
        {
            let mut buf = self.input.lock().expect("lang-matrix wasm stdin buffer poisoned");
            loop {
                match buf.pop_front() {
                    None | Some(b'\n') => break,
                    Some(b) => line.push(b),
                }
            }
        }
        // Cap at `max` (the codegen reserved [i32 len][max bytes]); truncate a longer line.
        let len = line.len().min(max as usize);
        // Length header (i32) then the bytes.
        memory.store_i32(base, len as i32)?;
        for (i, &b) in line.iter().take(len).enumerate() {
            memory.store_i32_8(base + 4 + i, b as i32)?;
        }
        Ok(vec![])
    }
}

// AL8 transcendentals — stateless `HostFunction` adapters for `env.__sin`,
// `env.__cos`, `env.__ln`, `env.__exp`.  Each takes one f64 and returns one f64,
// delegating to Rust's `f64::*` methods (which call the platform libm).
// `__ln` wraps `f64::ln` (natural log) matching ALGOL 60 semantics; the WASM
// import uses `__ln` rather than `__log` to signal intent at the ABI boundary.

struct SinFunc;
impl wasm_execution::HostFunction for SinFunc {
    fn func_type(&self) -> &wasm_types::FuncType {
        static FT: std::sync::LazyLock<wasm_types::FuncType> =
            std::sync::LazyLock::new(|| wasm_types::FuncType {
                params: vec![wasm_types::ValueType::F64],
                results: vec![wasm_types::ValueType::F64],
            });
        &FT
    }
    fn call(
        &self,
        args: &[wasm_execution::WasmValue],
        _memory: Option<&mut wasm_execution::LinearMemory>,
    ) -> Result<Vec<wasm_execution::WasmValue>, wasm_execution::TrapError> {
        let x = args.first()
            .ok_or_else(|| wasm_execution::TrapError::new("__sin: missing argument"))?
            .as_f64()
            .map_err(|e| wasm_execution::TrapError::new(e.message))?;
        Ok(vec![wasm_execution::WasmValue::F64(x.sin())])
    }
}

struct CosFunc;
impl wasm_execution::HostFunction for CosFunc {
    fn func_type(&self) -> &wasm_types::FuncType {
        static FT: std::sync::LazyLock<wasm_types::FuncType> =
            std::sync::LazyLock::new(|| wasm_types::FuncType {
                params: vec![wasm_types::ValueType::F64],
                results: vec![wasm_types::ValueType::F64],
            });
        &FT
    }
    fn call(
        &self,
        args: &[wasm_execution::WasmValue],
        _memory: Option<&mut wasm_execution::LinearMemory>,
    ) -> Result<Vec<wasm_execution::WasmValue>, wasm_execution::TrapError> {
        let x = args.first()
            .ok_or_else(|| wasm_execution::TrapError::new("__cos: missing argument"))?
            .as_f64()
            .map_err(|e| wasm_execution::TrapError::new(e.message))?;
        Ok(vec![wasm_execution::WasmValue::F64(x.cos())])
    }
}

struct LnFunc;
impl wasm_execution::HostFunction for LnFunc {
    fn func_type(&self) -> &wasm_types::FuncType {
        static FT: std::sync::LazyLock<wasm_types::FuncType> =
            std::sync::LazyLock::new(|| wasm_types::FuncType {
                params: vec![wasm_types::ValueType::F64],
                results: vec![wasm_types::ValueType::F64],
            });
        &FT
    }
    fn call(
        &self,
        args: &[wasm_execution::WasmValue],
        _memory: Option<&mut wasm_execution::LinearMemory>,
    ) -> Result<Vec<wasm_execution::WasmValue>, wasm_execution::TrapError> {
        let x = args.first()
            .ok_or_else(|| wasm_execution::TrapError::new("__ln: missing argument"))?
            .as_f64()
            .map_err(|e| wasm_execution::TrapError::new(e.message))?;
        Ok(vec![wasm_execution::WasmValue::F64(x.ln())])
    }
}

struct ExpFunc;
impl wasm_execution::HostFunction for ExpFunc {
    fn func_type(&self) -> &wasm_types::FuncType {
        static FT: std::sync::LazyLock<wasm_types::FuncType> =
            std::sync::LazyLock::new(|| wasm_types::FuncType {
                params: vec![wasm_types::ValueType::F64],
                results: vec![wasm_types::ValueType::F64],
            });
        &FT
    }
    fn call(
        &self,
        args: &[wasm_execution::WasmValue],
        _memory: Option<&mut wasm_execution::LinearMemory>,
    ) -> Result<Vec<wasm_execution::WasmValue>, wasm_execution::TrapError> {
        let x = args.first()
            .ok_or_else(|| wasm_execution::TrapError::new("__exp: missing argument"))?
            .as_f64()
            .map_err(|e| wasm_execution::TrapError::new(e.message))?;
        Ok(vec![wasm_execution::WasmValue::F64(x.exp())])
    }
}

impl wasm_execution::HostFunction for PowFunc {
    fn func_type(&self) -> &wasm_types::FuncType {
        static FT: std::sync::LazyLock<wasm_types::FuncType> =
            std::sync::LazyLock::new(|| wasm_types::FuncType {
                params: vec![wasm_types::ValueType::F64, wasm_types::ValueType::F64],
                results: vec![wasm_types::ValueType::F64],
            });
        &FT
    }

    fn call(
        &self,
        args: &[wasm_execution::WasmValue],
        _memory: Option<&mut wasm_execution::LinearMemory>,
    ) -> Result<Vec<wasm_execution::WasmValue>, wasm_execution::TrapError> {
        let base = match args.first() {
            Some(wasm_execution::WasmValue::F64(v)) => *v,
            _ => return Err(wasm_execution::TrapError::new("pow: arg 0 not f64")),
        };
        let exp_ = match args.get(1) {
            Some(wasm_execution::WasmValue::F64(v)) => *v,
            _ => return Err(wasm_execution::TrapError::new("pow: arg 1 not f64")),
        };
        Ok(vec![wasm_execution::WasmValue::F64(base.powf(exp_))])
    }
}

struct AtanFunc;
impl wasm_execution::HostFunction for AtanFunc {
    fn func_type(&self) -> &wasm_types::FuncType {
        static FT: std::sync::LazyLock<wasm_types::FuncType> =
            std::sync::LazyLock::new(|| wasm_types::FuncType {
                params: vec![wasm_types::ValueType::F64],
                results: vec![wasm_types::ValueType::F64],
            });
        &FT
    }
    fn call(
        &self,
        args: &[wasm_execution::WasmValue],
        _memory: Option<&mut wasm_execution::LinearMemory>,
    ) -> Result<Vec<wasm_execution::WasmValue>, wasm_execution::TrapError> {
        let x = args.first()
            .ok_or_else(|| wasm_execution::TrapError::new("__atan: missing argument"))?
            .as_f64()
            .map_err(|e| wasm_execution::TrapError::new(e.message))?;
        Ok(vec![wasm_execution::WasmValue::F64(x.atan())])
    }
}

struct TanFunc;
impl wasm_execution::HostFunction for TanFunc {
    fn func_type(&self) -> &wasm_types::FuncType {
        static FT: std::sync::LazyLock<wasm_types::FuncType> =
            std::sync::LazyLock::new(|| wasm_types::FuncType {
                params: vec![wasm_types::ValueType::F64],
                results: vec![wasm_types::ValueType::F64],
            });
        &FT
    }
    fn call(
        &self,
        args: &[wasm_execution::WasmValue],
        _memory: Option<&mut wasm_execution::LinearMemory>,
    ) -> Result<Vec<wasm_execution::WasmValue>, wasm_execution::TrapError> {
        let x = args.first()
            .ok_or_else(|| wasm_execution::TrapError::new("__tan: missing argument"))?
            .as_f64()
            .map_err(|e| wasm_execution::TrapError::new(e.message))?;
        Ok(vec![wasm_execution::WasmValue::F64(x.tan())])
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
            // BA-INPUT: `env.__input_i64` reads a full line from the stdin buffer
            // and parses it as an i64; used by BASIC `INPUT X`.
            ("env", "__input_i64") => Some(Box::new(InputI64Func {
                input: std::sync::Arc::clone(&self.input),
            })),
            // E4-dyn: `env.__input_str` reads a line and writes a `[i32 len][bytes]`
            // block into linear memory; used by BASIC string `INPUT A$`.
            ("env", "__input_str") => Some(Box::new(InputStrFunc {
                input: std::sync::Arc::clone(&self.input),
            })),
            // AL8 transcendentals: env.__sin/cos/ln/exp are f64→f64 host imports.
            ("env", "__sin")  => Some(Box::new(SinFunc)),
            ("env", "__cos")  => Some(Box::new(CosFunc)),
            ("env", "__ln")   => Some(Box::new(LnFunc)),
            ("env", "__exp")  => Some(Box::new(ExpFunc)),
            // AL8-arctan: env.__atan/tan are f64→f64 host imports.
            ("env", "__atan") => Some(Box::new(AtanFunc)),
            ("env", "__tan")  => Some(Box::new(TanFunc)),
            ("env", "__pow")  => Some(Box::new(PowFunc)),
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
fn run_wasm(p: &Prog) -> Option<RunResult> {
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
    let result = match rt.load_and_run(&wasm, "main", &[]) {
        Ok(result) => result,
        Err(_) if matches!(&p.expect, Expect::Trap) => return Some(RunResult::Trapped),
        Err(_) => return None,
    };
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
    Some(RunResult::Completed { code, stdout })
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
    "package env; public final class BasicRuntime { \
public static void println(long x){ System.out.println(x); } \
public static long readLong(){ try { \
java.io.InputStream in = System.in; \
StringBuilder sb = new StringBuilder(); \
int c; \
while((c = in.read()) != -1 && c != '\\n'){ sb.append((char)c); } \
String s = sb.toString().trim(); \
if(s.isEmpty()) return 0L; \
return Long.parseLong(s); \
} catch(Exception e){ return 0L; } } \
public static String readLine(){ try { \
java.io.InputStream in = System.in; \
StringBuilder sb = new StringBuilder(); \
int c; \
while((c = in.read()) != -1 && c != '\\n'){ sb.append((char)c); } \
return sb.toString(); \
} catch(Exception e){ return \"\"; } } }";

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
fn run_jvm(p: &Prog) -> Option<RunResult> {
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
    let prints = matches!(&p.expect, Expect::Stdout(_));
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
        // Pick the host class(es) the program's I/O lowers to. Brainfuck's `.`/`,`
        // and — since BA2 — Dartmouth BASIC's `PRINT` lower to `putchar`
        // (`invokestatic env/BFRuntime.putchar(I)V`), so both use `env.BFRuntime`.
        // Dartmouth BASIC's `INPUT X` (BA-INPUT) lowers to `invokestatic
        // env/BasicRuntime.readLong()J`, so DartmouthBasic additionally needs
        // `BasicRuntime.java` on the classpath alongside `BFRuntime.java`.
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
        // Dartmouth BASIC programs that use INPUT need BasicRuntime.readLong()J.
        // Compile BasicRuntime.java alongside BFRuntime.java only when the program
        // actually has an INPUT statement; other programs use only BFRuntime (putchar).
        if p.lang == Language::DartmouthBasic && p.src.contains("INPUT") {
            let basic_src = dir.path().join("BasicRuntime.java");
            std::fs::write(&basic_src, BASIC_RUNTIME_JAVA).ok()?;
            let built = Command::new("javac").arg("-d").arg(dir.path()).arg(&basic_src).output().ok()?;
            if !built.status.success() {
                return None;
            }
        }
    }
    // A Brainfuck `,` reads `env.BFRuntime.getchar()` → `System.in`, so pipe the
    // program's stdin to the `java` process; empty for every other program.
    let mut java = Command::new("java");
    java.arg("-cp").arg(dir.path()).arg("Main");
    let out = output_with_stdin(java, program_stdin(p))?;
    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if matches!(&p.expect, Expect::Trap) && !out.status.success() {
        return Some(RunResult::Trapped);
    }
    if prints {
        // The program wrote its result to stdout via `env.BasicRuntime.println`.
        Some(RunResult::Completed { code: out.status.code(), stdout })
    } else {
        // The launcher printed the entry method's result; parse it as the program's
        // value (matching the exit-code convention of the other columns).
        Some(RunResult::Completed { code: stdout.parse::<i32>().ok(), stdout: String::new() })
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
fn run_clr(p: &Prog) -> Option<RunResult> {
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
        if matches!(&p.expect, Expect::Trap) {
            return Some(RunResult::Trapped);
        }
        return None;
    }
    // Whatever the program wrote to `Console`: for an expression language that's the
    // launcher's `Console.WriteLine` of the entry's `int` result (parsed as the value,
    // matching the exit-code convention); for an I/O language (Dartmouth BASIC) it's
    // the `PRINT` output captured directly. Return both — `assert_cell` picks the one
    // the program's `Expect` cares about.
    let printed = String::from_utf8_lossy(&out.stdout).trim().to_string();
    Some(RunResult::Completed { code: printed.parse::<i32>().ok(), stdout: printed })
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
/// Apply the shared IIR-lowering passes the code-gen backends run, so the generic
/// register VM / JIT execute the same **dynamic** Twig features they do.
/// `lower_closures_to_heap` rewrites `alloc_closure`/`call_closure` to a cons-chain
/// object + a synthesized dispatcher; `lower_heap_builtins` rewrites the
/// `cons`/`car`/`cdr`/`null?` builtins to the structural `alloc`/`field_load`/
/// `is_null` ops the VM runs; `lower_global_io` rewrites dynamic
/// `global_get`/`global_set` to typed `global_load`/`global_store`. Without this the
/// VM sees the raw frontend `alloc_closure`/`cons`-builtin IIR it cannot dispatch.
/// The passes are no-ops on programs without those constructs (every non-Twig /
/// non-dynamic cell), matching how the code-gen pipelines already run them for
/// every language.
fn lower_dynamic_for_generic_engine(module: &mut interpreter_ir::IIRModule) {
    iir_builtin_lowering::lower_global_io(module);
    iir_builtin_lowering::lower_closures_to_heap(module);
    iir_builtin_lowering::lower_heap_builtins(module);
}

fn run_vm(p: &Prog) -> Option<RunResult> {
    use std::sync::{Arc, Mutex};
    use vm_core::core::VMCore;
    use vm_core::value::Value;

    let mut module = lang_aot::compile_source_to_iir(p.lang, p.src, "main").ok()?;
    lower_dynamic_for_generic_engine(&mut module);
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
    // BA-INPUT: `input_i64` reads a full line from the stdin buffer and parses it
    // as an i64. Mirrors `env.__input_i64` in the WASM column and `__twig_input_i64`
    // in the native/LLVM column. Returns 0 on empty/drained buffer (EOF semantics).
    let input = Arc::clone(&stdin_buf);
    vm.builtins_mut().register("input_i64", move |_args: &[Value]| {
        let mut buf = input.lock().expect("lang-matrix VM stdin buffer poisoned");
        let mut line = Vec::new();
        loop {
            match buf.pop_front() {
                None | Some(b'\n') => break,
                Some(b) => line.push(b),
            }
        }
        let s = String::from_utf8_lossy(&line);
        let v: i64 = s.trim().parse().unwrap_or(0);
        Ok(Value::Int(v))
    });
    // BA string INPUT (E4-dyn): `input_str` reads a whole line and returns it as a
    // runtime `Value::Str` — the string sibling of `input_i64`. The value comes from
    // stdin, so it is not foldable at compile time; `PRINT A$` prints it via `print_str`.
    let input = Arc::clone(&stdin_buf);
    vm.builtins_mut().register("input_str", move |_args: &[Value]| {
        Ok(Value::Str(drain_stdin_line(&input)))
    });

    let result = match vm.execute(&mut module, &entry, &[]) {
        Ok(result) => result,
        Err(_) if matches!(&p.expect, Expect::Trap) => return Some(RunResult::Trapped),
        Err(_) => return None,
    };

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
    Some(RunResult::Completed { code, stdout })
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
fn run_jit(p: &Prog) -> Option<RunResult> {
    use std::sync::{Arc, Mutex};
    use jit_core::core::JITCore;
    use jit_core::GenericCirJit;
    use vm_core::core::VMCore;
    use vm_core::value::Value;

    let mut module = lang_aot::compile_source_to_iir(p.lang, p.src, "main").ok()?;
    lower_dynamic_for_generic_engine(&mut module);
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
    let input = Arc::clone(&stdin_buf);
    vm.builtins_mut().register("input_i64", move |_args: &[Value]| {
        let mut buf = input.lock().expect("lang-matrix JIT stdin buffer poisoned");
        let mut line = Vec::new();
        loop {
            match buf.pop_front() {
                None | Some(b'\n') => break,
                Some(b) => line.push(b),
            }
        }
        let s = String::from_utf8_lossy(&line);
        let v: i64 = s.trim().parse().unwrap_or(0);
        Ok(Value::Int(v))
    });
    let input = Arc::clone(&stdin_buf);
    vm.builtins_mut().register("input_str", move |_args: &[Value]| {
        Ok(Value::Str(drain_stdin_line(&input)))
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
    let input = Arc::clone(&stdin_buf);
    backend.register_builtin("input_i64", move |_args: &[Value]| {
        let mut buf = input.lock().expect("lang-matrix JIT stdin buffer poisoned");
        let mut line = Vec::new();
        loop {
            match buf.pop_front() {
                None | Some(b'\n') => break,
                Some(b) => line.push(b),
            }
        }
        let s = String::from_utf8_lossy(&line);
        let v: i64 = s.trim().parse().unwrap_or(0);
        Value::Int(v)
    });
    let input = Arc::clone(&stdin_buf);
    backend.register_builtin("input_str", move |_args: &[Value]| {
        Value::Str(drain_stdin_line(&input))
    });

    // `JITCore::new` takes `&mut vm` only to thread thresholds — it does not hold the
    // borrow, so `execute_with_jit` can re-borrow `vm` for the interpreter tier.
    let mut jit = JITCore::new(&mut vm, Box::new(backend));
    let result = match jit.execute_with_jit(&mut vm, &mut module, &entry, &[]) {
        Ok(result) => result,
        Err(_) if matches!(&p.expect, Expect::Trap) => return Some(RunResult::Trapped),
        Err(_) => return None,
    };

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
    Some(RunResult::Completed { code, stdout })
}

/// Dispatch a program to a backend runner. `None` = the backend's toolchain is
/// unavailable on this host (skip, like the W16 external-tool backends).
fn run(backend: Backend, p: &Prog) -> Option<RunResult> {
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
fn assert_cell(backend: Backend, p: &Prog, result: RunResult) {
    match (&p.expect, result) {
        (Expect::Exit(n), RunResult::Completed { code, stdout }) => assert_eq!(
            code,
            Some(*n),
            "{backend:?} {:?}: expected exit {n}, got {code:?} (stdout {stdout:?})",
            p.lang
        ),
        (Expect::Stdout(s), RunResult::Completed { stdout, .. }) => assert_eq!(
            stdout, *s,
            "{backend:?} {:?}: expected stdout {s:?}, got {stdout:?}",
            p.lang
        ),
        (Expect::Trap, RunResult::Trapped) => {}
        (expect, other) => panic!(
            "{backend:?} {:?}: expected {expect:?}, got {other:?}",
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
            let Some(result) = run(backend, p) else {
                continue;
            };
            assert_cell(backend, p, result);
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
                "native-AOT present but failed to run {:?}: {}",
                p.lang,
                p.src
            );
        }
    }
    // LLVM: when clang is present every LLVM-tagged program must run.
    if clang_ok() {
        for p in PROGRAMS.iter().filter(|p| p.backends.contains(&Llvm)) {
            assert!(
                run_llvm(p).is_some(),
                "clang present but LLVM failed to run {:?}: {}",
                p.lang,
                p.src
            );
        }
    }
    // WASM: the runtime is in-process (always present), so every WASM-tagged program
    // must run — no host gate.
    for p in PROGRAMS.iter().filter(|p| p.backends.contains(&Wasm)) {
        assert!(
            run_wasm(p).is_some(),
            "in-process wasm-runtime failed to run {:?}: {}",
            p.lang,
            p.src
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
                "dotnet+ilasm present but CLR failed to run {:?}: {}",
                p.lang,
                p.src
            );
        }
    }
}










// ═══════════════════════════════════════════════════════════════════════════
// AOT00 T7 — conformance-at-scale: generative DIFFERENTIAL testing
// ═══════════════════════════════════════════════════════════════════════════
//
// The hand-written cells above each pin one (program, backend) result. This is
// the complementary safety net the roadmap (AOT00 §5.2, track T7) sequences
// next: GENERATE random well-formed programs and assert every available engine
// AGREES. It is the mechanism that auto-detects the cross-backend disagreements
// the E6d union work found one at a time — a tagged-vs-boxed mismatch would
// surface here as "vm=X, llvm=Y" on some random program, with no cell authored
// for it.
//
// Seed slice: random `u8` expression trees over `+ & | ^` (total — no
// div-by-zero — and wrapping mod 256 identically on every engine). The in-process
// VM is the reference oracle; every other engine present is cross-checked against
// it. Fast in-process engines (WASM/JIT) run on EVERY program; the slower
// toolchain engines (native/LLVM/CLR, one process spawn per program) run on a
// deterministic sample so the test stays quick. Absent toolchains skip.

/// Deterministic zero-dep PRNG (xorshift64) — reproducible, so any failure
/// replays from the fixed seed.
fn xorshift(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

/// A random total `u8` expression: a literal, or `(a OP b)` over `+ & | ^`.
/// Depth-capped so the emitted source stays small (no parser-depth blowup).
fn gen_u8_expr(state: &mut u64, depth: usize) -> String {
    if depth >= 4 || (depth > 0 && xorshift(state).is_multiple_of(3)) {
        return (xorshift(state) % 256).to_string();
    }
    let op = ["+", "&", "|", "^"][(xorshift(state) % 4) as usize];
    let a = gen_u8_expr(state, depth + 1);
    let b = gen_u8_expr(state, depth + 1);
    format!("({a} {op} {b})")
}

/// Process exit code from an engine's result (None if the engine was absent or
/// the program trapped).
fn exit_code(r: Option<RunResult>) -> Option<i32> {
    match r {
        Some(RunResult::Completed { code, .. }) => code,
        _ => None,
    }
}

#[test]
fn t7_differential_random_u8_expressions_agree() {
    const SEED: u64 = 0x9E37_79B9_7F4A_7C15;
    const N: usize = 160;
    const TOOLCHAIN_EVERY: usize = 10; // sample the slow (process-spawning) engines
    let mut state = SEED;
    let mut cross_checks = 0usize;
    for i in 0..N {
        let expr = gen_u8_expr(&mut state, 0);
        let src: &'static str =
            Box::leak(format!("fn main() -> u8 {{ return {expr}; }}").into_boxed_str());
        let p = Prog { lang: Language::Nib, ext: "nib", src, expect: Expect::Exit(0), backends: &[] };

        // The observable is the `u8` program's LOW BYTE. Every engine that reports
        // via a process exit code is already truncated to 8 bits by the OS
        // (`300 & 0xFF = 44`); `run_clr` reports the launcher's printed `int32`
        // (full width), so normalise every result to `& 0xFF` to compare the same
        // observable. (That CLR's *un-narrowed* `u8` return is 300 not 44 — the
        // declared width isn't masked before `ret`, only hidden by exit-code
        // truncation elsewhere — is a real conformance finding this harness
        // surfaced; tracked separately, out of scope for the low-byte differential.)
        let obs = |r: Option<RunResult>| exit_code(r).map(|c| c & 0xFF);
        let Some(want) = obs(run_vm(&p)) else {
            panic!("VM (reference oracle) failed to run generated program: {src:?}");
        };

        let mut engines: Vec<(&str, Option<i32>)> =
            vec![("wasm", obs(run_wasm(&p))), ("jit", obs(run_jit(&p)))];
        if i.is_multiple_of(TOOLCHAIN_EVERY) {
            engines.push(("native", obs(run_native(&p))));
            engines.push(("llvm", obs(run_llvm(&p))));
            engines.push(("clr", obs(run_clr(&p))));
        }
        for (engine, got) in engines {
            if let Some(got) = got {
                assert_eq!(
                    got, want,
                    "T7 differential disagreement [{engine}] on {src:?}: vm={want}, {engine}={got}"
                );
                cross_checks += 1;
            }
        }
    }
    eprintln!("T7 differential: {cross_checks} cross-engine agreements over {N} random programs");
    // WASM + JIT are always in-process → at least 2 checks per program.
    assert!(cross_checks >= 2 * N, "expected >= {} cross-checks, got {cross_checks}", 2 * N);
}



// ── AOT00 T7 — full-value differential over BASIC `PRINT` (stdout channel) ────
//
// The u8 differential above compares an 8-bit exit code — enough to catch a
// low-byte disagreement, but blind to the upper bits (a u8 `200+100` reads 44 on
// every engine only because the OS truncates the exit code; the full value 300 is
// unobservable there). Dartmouth BASIC's `PRINT` reports the **full** integer on
// stdout, so this differential compares whole `i64` values — negatives and large
// products included — across every engine. Strictly stronger coverage: a
// full-value disagreement (not just a low-byte one) fails loudly.

/// A random total `i64` `PRINT` expression over `+ - *`. Literals are `0..=16`
/// and depth is capped at 3 (≤ 8 leaves), so an all-`*` tree is at most
/// `16^8 ≈ 4.3e9` — comfortably inside `i64`, never overflowing. No division, so
/// it is total.
fn gen_basic_expr(state: &mut u64, depth: usize) -> String {
    if depth >= 3 || (depth > 0 && xorshift(state).is_multiple_of(3)) {
        return (xorshift(state) % 17).to_string();
    }
    let op = ["+", "-", "*"][(xorshift(state) % 3) as usize];
    let a = gen_basic_expr(state, depth + 1);
    let b = gen_basic_expr(state, depth + 1);
    format!("({a} {op} {b})")
}

/// The trimmed stdout of an engine's result (None if absent/trapped).
fn stdout_of(r: Option<RunResult>) -> Option<String> {
    match r {
        Some(RunResult::Completed { stdout, .. }) => Some(stdout.trim().to_string()),
        _ => None,
    }
}

#[test]
fn t7_differential_random_basic_print_agree() {
    const SEED: u64 = 0x2545_F491_4F6C_DD1D;
    const N: usize = 160;
    const TOOLCHAIN_EVERY: usize = 10;
    let mut state = SEED;
    let mut cross_checks = 0usize;
    for i in 0..N {
        let expr = gen_basic_expr(&mut state, 0);
        let src: &'static str =
            Box::leak(format!("10 PRINT {expr}\n20 END\n").into_boxed_str());
        let p = Prog {
            lang: Language::DartmouthBasic,
            ext: "bas",
            src,
            expect: Expect::Stdout(""),
            backends: &[],
        };

        let Some(want) = stdout_of(run_vm(&p)) else {
            panic!("VM (reference oracle) failed to run generated program: {src:?}");
        };

        let mut engines: Vec<(&str, Option<String>)> =
            vec![("wasm", stdout_of(run_wasm(&p))), ("jit", stdout_of(run_jit(&p)))];
        if i.is_multiple_of(TOOLCHAIN_EVERY) {
            engines.push(("native", stdout_of(run_native(&p))));
            engines.push(("llvm", stdout_of(run_llvm(&p))));
            engines.push(("clr", stdout_of(run_clr(&p))));
        }
        for (engine, got) in engines {
            if let Some(got) = got {
                assert_eq!(
                    got, want,
                    "T7 full-value differential disagreement [{engine}] on {src:?}: vm={want:?}, {engine}={got:?}"
                );
                cross_checks += 1;
            }
        }
    }
    eprintln!("T7 BASIC-print differential: {cross_checks} full-value cross-engine agreements over {N} programs");
    assert!(cross_checks >= 2 * N, "expected >= {} cross-checks, got {cross_checks}", 2 * N);
}


// ── AOT00 T7 — control-flow differential: BASIC `IF … THEN`/`GOTO` ───────────
//
// The two arithmetic differentials above exercise only straight-line evaluation.
// This one exercises the **comparison ops + conditional branch + `GOTO`** codegen
// — the paths where cross-backend disagreements are most likely (boolean
// representation, branch-condition polarity: exactly the class of the E6d-6
// boxed-bool `jmp_if_false` bug). A random comparison picks between two `PRINT`
// arms, so the printed value witnesses *both* the comparison result and that the
// right branch was taken, compared as a full `i64` across every engine.

/// A random BASIC program: `IF <a> <relop> <b> THEN <print d> ELSE <print c>`,
/// laid out with a `GOTO` (Dartmouth `IF` only takes a target line). `a`/`b`/`c`/`d`
/// are the §3b `+ - *` expression trees; `<relop>` ranges over all six.
fn gen_basic_if_program(state: &mut u64) -> String {
    let relop = ["=", "<>", "<", ">", "<=", ">="][(xorshift(state) % 6) as usize];
    let a = gen_basic_expr(state, 0);
    let b = gen_basic_expr(state, 0);
    let c = gen_basic_expr(state, 0);
    let d = gen_basic_expr(state, 0);
    format!("10 IF {a} {relop} {b} THEN 40\n20 PRINT {c}\n30 GOTO 50\n40 PRINT {d}\n50 END\n")
}

#[test]
fn t7_differential_random_basic_conditionals_agree() {
    const SEED: u64 = 0x1234_5678_9ABC_DEF1;
    const N: usize = 120;
    const TOOLCHAIN_EVERY: usize = 15;
    let mut state = SEED;
    let mut cross_checks = 0usize;
    for i in 0..N {
        let src: &'static str = Box::leak(gen_basic_if_program(&mut state).into_boxed_str());
        let p = Prog {
            lang: Language::DartmouthBasic,
            ext: "bas",
            src,
            expect: Expect::Stdout(""),
            backends: &[],
        };

        let Some(want) = stdout_of(run_vm(&p)) else {
            panic!("VM (reference oracle) failed to run generated program: {src:?}");
        };

        let mut engines: Vec<(&str, Option<String>)> =
            vec![("wasm", stdout_of(run_wasm(&p))), ("jit", stdout_of(run_jit(&p)))];
        if i.is_multiple_of(TOOLCHAIN_EVERY) {
            engines.push(("native", stdout_of(run_native(&p))));
            engines.push(("llvm", stdout_of(run_llvm(&p))));
            engines.push(("clr", stdout_of(run_clr(&p))));
        }
        for (engine, got) in engines {
            if let Some(got) = got {
                assert_eq!(
                    got, want,
                    "T7 conditional differential disagreement [{engine}] on {src:?}: vm={want:?}, {engine}={got:?}"
                );
                cross_checks += 1;
            }
        }
    }
    eprintln!("T7 conditional differential: {cross_checks} cross-engine agreements over {N} branch programs");
    assert!(cross_checks >= 2 * N, "expected >= {} cross-checks, got {cross_checks}", 2 * N);
}







// ── AOT00 T7 — loop differential: BASIC `FOR … NEXT` accumulator ─────────────
//
// The arithmetic and `IF` differentials exercise straight-line and single-branch
// code; neither touches a **loop back-edge** — a distinct codegen path (the loop
// header/latch, the counter increment + bound test, a mutated accumulator across
// iterations) and a classic source of cross-backend divergence (off-by-one bounds,
// STEP handling, `NEXT` target). This generates `FOR I = 1 TO n` accumulator
// programs and compares the printed sum across every engine.

/// A random total loop-body expression over `+ - *`, whose leaves are the loop
/// counter `I` or a literal `0..=6`. Depth ≤ 2 (≤ 4 leaves), and the driver caps
/// the trip count, so the accumulated sum stays far inside `i64` — no overflow.
fn gen_loop_body_expr(state: &mut u64, depth: usize) -> String {
    if depth >= 2 || (depth > 0 && xorshift(state).is_multiple_of(3)) {
        // Leaf: the counter `I` a third of the time, else a small literal.
        return if xorshift(state).is_multiple_of(3) {
            "I".to_string()
        } else {
            (xorshift(state) % 7).to_string()
        };
    }
    let op = ["+", "-", "*"][(xorshift(state) % 3) as usize];
    let a = gen_loop_body_expr(state, depth + 1);
    let b = gen_loop_body_expr(state, depth + 1);
    format!("({a} {op} {b})")
}

/// A random accumulator loop: `S := 0; for I in 1..=n { S := S + <body(I)> }; print S`.
fn gen_basic_for_program(state: &mut u64) -> String {
    let n = 2 + (xorshift(state) % 5); // trip count 2..=6
    let body = gen_loop_body_expr(state, 0);
    format!("10 LET S = 0\n20 FOR I = 1 TO {n}\n30 LET S = S + {body}\n40 NEXT I\n50 PRINT S\n60 END\n")
}

#[test]
fn t7_differential_random_basic_loops_agree() {
    const SEED: u64 = 0x9E37_79B9_1234_5678;
    const N: usize = 120;
    const TOOLCHAIN_EVERY: usize = 15;
    let mut state = SEED;
    let mut cross_checks = 0usize;
    for i in 0..N {
        let src: &'static str = Box::leak(gen_basic_for_program(&mut state).into_boxed_str());
        let p = Prog {
            lang: Language::DartmouthBasic,
            ext: "bas",
            src,
            expect: Expect::Stdout(""),
            backends: &[],
        };
        let Some(want) = stdout_of(run_vm(&p)) else {
            panic!("VM (reference oracle) failed to run generated program: {src:?}");
        };
        let mut engines: Vec<(&str, Option<String>)> =
            vec![("wasm", stdout_of(run_wasm(&p))), ("jit", stdout_of(run_jit(&p)))];
        if i.is_multiple_of(TOOLCHAIN_EVERY) {
            engines.push(("native", stdout_of(run_native(&p))));
            engines.push(("llvm", stdout_of(run_llvm(&p))));
            engines.push(("clr", stdout_of(run_clr(&p))));
        }
        for (engine, got) in engines {
            if let Some(got) = got {
                assert_eq!(
                    got, want,
                    "T7 loop differential disagreement [{engine}] on {src:?}: vm={want:?}, {engine}={got:?}"
                );
                cross_checks += 1;
            }
        }
    }
    eprintln!("T7 loop differential: {cross_checks} cross-engine agreements over {N} loop programs");
    assert!(cross_checks >= 2 * N, "expected >= {} cross-checks, got {cross_checks}", 2 * N);
}
