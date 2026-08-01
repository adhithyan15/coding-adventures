# LANG-FULL-IMPLEMENTATION — every matrix language, fully implemented and *run* on every backend

## Why this campaign exists

The LANG-PLATFORM matrix (`LANG-PLATFORM-MATRIX.md`) proved the **plumbing**: six
frontends → shared `interpreter_ir::IIRModule` → seven backends → a runnable artifact.
But an honest audit (2026-06-13) found the green checkmarks rest on **one executed
program per language**, and each frontend is a **deliberate subset**:

| Language | What the matrix actually runs end-to-end on the code-gen backends | The subset gap |
|---|---|---|
| Twig | `42`, variadic arithmetic, top-level value `define`, and typed E4 string literal/named/local/function-call/annotated-parameter/direct-call-inferred-parameter proofs from literal, static expression, named, lexical, derived sequential `let*`, and multi-parameter actuals, including substring/index/equality and lexical ordering | rich Lisp frontend; lists/lambdas/dynamic globals/records/symbols still need E5/E6, and captured/reassigned/unobserved or conflicting parameter strings stay on the dynamic path |
| Nib | typed calls, `*`/`/`, `for`, bitwise, short-circuit logic, logical `!`, consts, const/static-expression folding, wrap/sat arithmetic, and module `static`s all run on all 7 backends | BCD semantics and Intel-4004 RAM mapping remain |
| Brainfuck | 1-loop "print A", nested-loop multiply (`"HA"`), two sequential loops (`"OK"`), stdin echo/transform, and canonical cat all run on all 7 backends | all 8 ops are cross-backend-proven by B1/B1-stdin/B1-eof; no current BF subset gap remains beyond adding more regression programs |
| Dartmouth BASIC | `PRINT 42`, `PRINT "HELLO"` on all 7 backends, `GOSUB`/`RETURN`, arrays, data, functions, scalar real arithmetic, historical real formatting | literal-backed string variables, literal reassignment, literal `+` concat, variable-backed and chained concat assignment, `PRINT`/`IF` string concat expressions, multi-item string `PRINT` with `;` and `,`, literal-backed scalar string copy, copied-slot string equality, and equality/inequality/lexical-ordering string branches ✅ (BA4/E4); integer-literal `^` ✅ (BA-^); string arrays/input and general runtime-math `^` remain; `FOR`/`NEXT`, `IF`/`GOTO`, `DEF FN` (BA5), `DIM` real arrays incl. multi-dimensional `DIM A(m,n)` (BA3/BA7/BA-DIM-2D), `READ`/`DATA`/`RESTORE` over real data (BA6/BA7), `GOSUB`/`RETURN` (BA1), and BA7 `f64` arithmetic/formatting all run on every backend |
| Oct | `let`/`if` | rejects **all 10 Intel-8008 intrinsics** (its raison d'être); `&&`/`||` short-circuit ✅ (O1), u8 wrap + `~` ✅ (O2), `static` module globals ✅ (O3), logical `!` ✅ (O-!); intrinsics remain |
| ALGOL 60 | `result := 17 mod 5` → 2 | `integer`/`real`/`boolean` scalars, typed procedures (including `real procedure` returning f64) ✅ (AL13, all 7 backends), nested procedures capturing scalar and array value formals ✅, switches including conditional/nested designators, rank-inferred array value parameters ✅ (1-D through N-D, including nested-procedure capture), N-dimensional integer & real arrays ✅ (AL-multidim / AL-multidim-real, all 7 backends), `boolean array` declarations and value formals ✅ (all 7 standard backends), `string array` ✅ (E4d-AL, all 7 standard backends), procedure capture of enclosing arrays with declared bounds ✅, `own` static-lifetime scalars, arrays, and strings ✅ (AL6, all 7 backends), `abs`/`sign`/`entier`/`sqrt`/`sin`/`cos`/`ln`/`exp`/`arctan` standard functions ✅ (AL8 + E8, all 7 backends), `↑` exponentiation ✅ (AL-pow, all 7 backends), string I/O plus initialized scalar locals carrying string-procedure results through runtime equality and lexical ordering ✅ (AL4/E4d-AL; also executed on BEAM's ASCII character-list subset); no call-by-name |

**AL-multidim-bool:** the seven-backend matrix executes a two-dimensional
boolean array through a rank-aware value formal with two non-unit lower bounds.
The checkerboard payload verifies both descriptor lower bounds and the outer
row-major stride after the procedure call.

**AL-multidim-string-capture:** the seven-backend matrix executes a nested
procedure that writes a two-dimensional string-array value formal through its
captured descriptor. Lexical ordering plus equality and inequality of separate
cells verifies the `array<str>` handle, both lower bounds, and outer row-major
stride survive the call and capture boundary.

**Goal of this campaign:** make every language a *full* implementation —
every construct in its grammar lowered to the shared IIR, running correctly on
**every backend except BEAM**, and **verified by RUNNING a real program that
exercises the feature on each code-gen backend** (not validated, not byte-encoded —
executed, output checked). McCarthy Lisp is the reference (52 executed e2e tests) and
the bar every other language must reach.

## Definition of done (per work item)

A feature is "done" only when ALL of:

1. **Lowered** — the Rust frontend (`<lang>-iir-compiler`) lowers the construct to the
   shared IIR (never a per-language backend hack).
2. **Runs on every code-gen backend** — native-AOT, LLVM, WASM, JVM, CLR — **and** the
   VM and JIT. Where a backend genuinely can't yet (missing IIR op), that gap becomes an
   explicit **enabler** work item (E-series below) and is closed first.
3. **Verified by RUNNING** — `lang-aot/tests/lang_matrix.rs` gains at least one new
   executed `Prog` exercising the feature, asserted across every backend the toolchain is
   present for. This is the anti-smoke-test guardrail: **no feature ships "validated" or
   "encoded" only.** Per-crate `jit_e2e`/`backend_compat` tests are kept but are not
   sufficient on their own.
4. **Tested** — frontend unit tests for the lowering + the executed matrix program;
   coverage stays well above 80%.
5. **Documented** — spec/CHANGELOG/README/version bumps for every crate touched; this
   roadmap's checklist updated; divergence from the language grammar called out.
6. **Reviewed** — `/security-review` sub-agent PASSED before push; clippy clean.

## Workflow (the loop)

One work item → one feature branch + worktree off `origin/main` → implement to the
definition of done → `/security-review` → PR (`feat(lang-full): …`) → babysit with a
recurring 3-minute timer (CI failures, merge conflicts, merged-check). **The user merges
(reviewing async); never self-merge.** On `state==MERGED`: clean up the worktree, then
loop to the next ☐ item. Safety valve: stop and ask on a genuine architectural fork
(flagged ⚠ below) or after 2 consecutive failures.

---

## Cross-cutting enablers (shared IIR + ALL backends)

Rich features need the IIR and every backend to grow. These are prerequisites shared by
multiple languages; close an enabler before the features that depend on it.

- **E1 — Multi-program execution battery (guardrail).** `lang_matrix.rs` already runs
  source→real-backend→execute for one `Prog` per language; this campaign adds many. No
  separate harness PR — every feature PR adds its executed `Prog`(s). *(Mechanism exists;
  enforced by the definition of done.)*
- **E2 — Integer width & wrap semantics.** ✅ **COMPLETE** (approach B — each backend
  masks narrow-typed arithmetic by `type_hint`, mirroring the byte-tape precedent). Model
  `u4`/`u8`/`u16`/`u32` wraparound (mod-2ⁿ) consistently across all backends, so Nib/Oct
  arithmetic and bitwise-NOT are *correct*, not "collapse to i64." (Brainfuck already proves
  the u8-tape pattern; this generalises it to register values.)
  - ✅ **vm-core** (1/6) — `mask_result(v, type_hint, u8_wrap)` masks every arithmetic /
    bitwise / shift result to the hint width; unit-tested (`200u8+100u8=44`, `~0u8=255`,
    `1u8<<8=0`, u4/u16/u32).
  - ✅ **jit-core** (2/6) — compiled tier emits a `MASK_WIDTH <reg> <bits>` opcode after a
    narrow `_u8`/`_u16`/`_u32` add/sub/mul/div/neg (`u4`/signed handled by the interpreter
    fallback); unit-tested. Matches vm-core's interpreter-tier mask.
  - ✅ **iir-to-wasm** (3/6) — masks a narrow `u4`/`u8`/`u16` arith/bitwise/neg/not result
    with `i32.const <mask>; i32.and` after the i32 op (`u32`/`i32` already wrap mod-2³² via
    the i32 op; `u4` newly mapped to i32). **Executed proof** on real `wasm-runtime`
    (`200u8+100u8=44`, `~0u8=255`, `1u8<<8=0`).
  - ✅ **iir-to-jvm-class-file** (4/6) — `emit_jvm_width_mask` appends `iconst/sipush/ldc
    <mask>; iand` after a narrow `u4`/`u8`/`u16` arith/bitwise/neg/not/shl `int` op (positive
    mask + `iand`, not `i2b`, to keep unsigned widths unsigned; `0xFFFF` via the constant
    pool). u32/i32 already wrap via the int op. Structural tests; executed JVM proof in the
    integration PR via the matrix's real-`java` run_jvm.
  - ✅ **iir-to-cil-bytecode** (5/6) — masks a narrow `u4`/`u8`/`u16` arith/bitwise/neg/not/shl
    result with `ldc.i4 <mask>; and` after the 32-bit CIL op, in **both** emitters (the `lower`
    bytecode builder via `emit_ldc_i4(mask); emit_and()`, and the textual `il_text` `.il` path)
    — so `200u8+100u8=44` and `~0u8=255`. u32/i32 already wrap mod-2³² via the 32-bit op; a
    positive mask + `and` (not `conv.u1`) keeps the unsigned widths unsigned. Structural tests
    on both emitters; executed CLR proof in the integration PR via the matrix's real-`dotnet`.
  - ✅ **Integration (N6)** — Nib emits narrow `type_hint`s for narrow-declared values + an
    **executed matrix proof** (`200u8+100u8=44`, `6*7=42`) across all 7 backends. (Nib `~`/N7
    and Oct O2 remain — separate language items below.) Wiring this up disproved the roadmap's
    earlier "LLVM already wraps natively (u8→i8)" assumption (never executed; false — every IIR
    value rides an i64 slot) AND surfaced that the three *op-typing* backends couldn't consume a
    narrow op over the operands a real frontend emits. Each was fixed before the frontend wiring:
    - ✅ **iir-to-llvm** (v0.11.0) — a narrow unsigned op computes at i64 then `and i64 …,
      <mask>` (u4/u8/u16/u32). Adds `u4` to the supported types. **Executed proof** on real
      `clang`: `200u8+100u8` → exit `44`. Matches the value-mask of the 5 register backends.
    - ✅ **lang-aot native codegen** (NativeAot — aarch64-backend 0.10.0 + x86_64-backend 0.12.0).
      The two *direct* native backends were never in E2's leg list and did **not** mask (their
      docs said so: "`add_u8 0xFF, 1` produces `0x100` … a future PR can add `and #mask`"), so
      `200u8+100u8` returned 300 on the `NativeAot` column. Now every narrow-unsigned op masks
      its result — aarch64 appends `mov X2,#mask; and X0,X0,X2`, x86_64 appends `movabs rcx,<mask>;
      and <dst>,rcx` (add/sub/mul/div/mod/and/or/xor/shl/shr/neg/not, for u4/u8/u16/u32; signed
      narrow + full-width untouched). **Executed proof on aarch64** — the generated ARM64 is
      installed via `jit-loader-macos` and *called*: `200u8+100u8=44`, `~0u8=255`, `1u8<<8=0`,
      `u32` mul wrap; x86_64 has structural mask tests + the lang-aot matrix on a Linux x86 runner.
    - ✅ **Stack-backend rework (compute-wide + mask).** Wiring Nib to emit narrow `type_hint`s
      surfaced that the 3 *stack* backends typed the masking op at the narrow width (wasm→i32,
      jvm→int, cil→i32), which **requires narrow-width operands**. Real frontends carry every
      `const`/`let` as `i64` (module uniformity) and put the narrow width only on the op, so a
      Nib `u8` add was an `i32.add` over `i64` operands → trap (`expected i32, got I64`). Their
      E2 unit tests never caught it (self-consistent narrow-width modules). Fix: narrow unsigned
      types ride the **i64 register model** (i64 op + i64-width AND mask), operand-agnostic like
      vm/jit/llvm/native.
      - ✅ **iir-to-wasm** (v0.15.0) — `uses_i64_register` selects `i64.*` ops; mask is
        `i64.const <mask>; i64.and` (now incl. `u32`). **Executed proof** on real `wasm-runtime`:
        `200i64 + 100i64 : u8 == 44`. Full matrix + wasm consumers green (no-op for i64 programs).
      - ✅ **iir-to-jvm-class-file** (v0.13.1) — narrow unsigned use the **int model**
        (`JvmType::Int`, `I` descriptor, `iadd`/`iand`, `sipush <mask>; iand`); `u4` newly
        recognised. *(v0.13.0 tried a long model like wasm; reverted — the JVM runs
        `lang_aot::concretize_scalar_any_for_jvm` which narrows scalar `i64`→`i32` before
        lowering, so the long model left the narrow op `long` while consts/return were `int`
        → unverifiable bytecode; the Nib u8 proof returned `None`.)* **Verified on real
        `java`**: the lowered `200u8+100u8` returns `44`. Regression test
        `e2_concretized_u8_shape_is_all_int` (post-concretize `const i32; add u8; ret i32` →
        `iand` mask, no `ladd`/`lreturn`). Full matrix + jvm consumers green.
      - ✅ **iir-to-cil-bytecode** (v0.20.1) — **no rework needed**: the CIL backend is
        *uniformly int32* (`cil_local_type` maps every scalar incl. `i64` to `int32`; `const`
        emits `ldc.i4`), so a frontend's i64 consts collapse to int32 and the existing
        `ldc.i4 <mask>; and` mask is already consistent — `200u8+100u8` lowers to all-int32 IL
        that wraps to `44`. Regression test `e2_u8_op_over_i64_operands_stays_int32` asserts
        no `int64`/`ldc.i8` leaks in. (Unlike wasm/jvm, which type the op and so needed the
        i64/long register model.)
    - ✅ **aot-core u4** (v0.2.2) — the native CIR pipeline (`infer`/`specialise`) didn't list
      `u4`, so a Nib `u4` op was refused before the aarch64/x86_64 backend mask could fire; added
      `u4` to both `ALLOWED_TYPES` sets + `numeric_rank`. *(Bundled with the Nib frontend PR.)*
    - ✅ **Nib frontend + matrix proof** (nib-type-checker 0.3.0, nib-iir-compiler 0.14.0,
      lang-aot 0.86.0) — `nib-type-checker` now does **bidirectional/context-directed** typing
      (let/assign/return/for/if thread the expected width, so `6*7` in a `u8` return context is
      `u8`, not magnitude-`u4`); `compile_binary_chain` emits the narrow `type_hint` on arith/
      bitwise ops (`i64` for cmp). **Executed cross-backend matrix proof**: `200u8+100u8` →
      `if x==44 {1} else {0}` returns **1**, and `6*7` returns **42**, on all 7 backends
      (native/LLVM/WASM/JVM/CLR/VM/JIT). N6 ✅. **N7** (`+%`/`+?`) ✅ (nib-iir-compiler 0.15.0).
      **N3-`~`** ✅ (nib-iir-compiler 0.16.0 lowers unary `~` → IIR `not` narrow-masked;
      `~0u8 == 255` / `~15u4 == 0` run on all 7 backends — needed `iir-to-llvm` 0.12.0's `not`
      op + `iir-to-cil-bytecode` 0.21.0's textual-`.il` `not` arm). **Oct O2-`~`** ✅ too
      (oct-iir-compiler 0.7.0 — Oct's single integer width makes every int op u8; needed a JVM
      long-model mask fix in iir-to-jvm-class-file 0.14.0). **E2 integration complete.**
- **E3 — Real / floating-point (`f64`).** ✅ **COMPLETE — every backend (VM/JIT + all 5 code-gen) executes f64.**
  End-to-end f64 arithmetic, comparison, and literals. Unlocks ALGOL reals and BASIC floats.
  - ✅ **vm-core** (0.6.0) — `add`/`sub`/`mul`/`div`/`neg` + ordered comparisons take a float
    track (`f64` result, IEEE div-by-zero → ±inf to match the code-gen backends' `fdiv`);
    integer programs unchanged. `cmp_eq`/`cmp_ne` already compared floats via `Value` equality.
  - ✅ **jit-core** — already executes f64 (its CIR carries `add_f64`/… and a float operand);
    confirmed by running the ALGOL real matrix proofs on the JIT.
  - ✅ **Frontend driver (AL1)** — ALGOL 60 `real` lowers to `f64`; two executed VM/JIT matrix
    proofs (real `*`+`=` → 42, real `/`+`<` → 1). *(BASIC floats / BA7 are a later driver.)*
  - ✅ **E3-codegen-slots** — **COMPLETE** (llvm + wasm + jvm all run reals). The shared problem:
    `iir-to-{llvm,jvm}` modelled every *variable slot* as a uniform `i64`/`long`, so an `f64`
    variable's store/load/compare was invalid; fix = per-slot float typing **+** a boolean (not
    operand-width) comparison-result type. (WASM was already fine — typed locals.)
    - ✅ **iir-to-llvm** (v0.13.0) — `collect_slot_types` gives an `f64` local an `alloca double`
      slot (`store/load double`); float `cmp_*` result `zext i1 → i64` (not the invalid
      `→ double`); `Operand::Float` rendered as LLVM's exact hex double `0x…` (Rust `{:e}` emitted
      `2e0`/`0e0`, which clang rejects). **Executed proof on real `clang`**: the two ALGOL real
      programs (exit 42, exit 1) run on the LLVM matrix column. (`f64` *params* reassigned across a
      back-edge still stay SSA — `param_slot_compatible` excludes floats — a separate unexercised case.)
    - ✅ **iir-to-wasm** (v0.15.1) — **no backend change needed.** Unlike LLVM/JVM's uniform
      slot model, WASM types each local individually (`hint_to_value_type("f64") = F64`) and
      selects `f64.mul`/`f64.eq`/`f64.lt` from the `f64` type_hint, so an `f64` variable already
      lived in an `F64` local. **Executed proof on `wasm-runtime`**: the two ALGOL real programs
      run on the WASM matrix column; added op-selection regression tests.
    - ✅ **iir-to-jvm-class-file** (v0.15.0) — f64 locals were already typed `Double` (`dstore`/
      `dload`, two slots) and arithmetic already used `dadd`/`dmul`; the two real gaps were
      (a) non-0/1 f64 *constants* emitted `ldc2_w #0` (the unused phantom slot) instead of a real
      `CONSTANT_Double` — fixed with `add_double` + `emit_dconst_cp`; (b) f64 *comparisons* fell
      into the integer `if_icmp` path (mis-reading a two-slot double as an int) — fixed with a
      `Double` branch emitting `dcmpl`/`dcmpg` + a unary `ifXX`. **Executed proof on real `java`**:
      both ALGOL real programs run on the JVM matrix column. **E3-codegen-slots COMPLETE.**
  - ✅ **E3-native** — both direct native backends. (`aot-core`'s `infer`/`specialise` already
    allow `f64`.)
    - ✅ **aarch64-backend** (v0.11.0, encoder v0.4.0) — `const_f64` materialises the bit pattern
      into the slot; `add/sub/mul/div_f64` use `ldr_d`/`fadd…`/`str_d` over `D0`/`D1`; `cmp_*_f64`
      use `fcmp` + `cset` with IEEE-ordered condition codes (`Lt`→`MI`, NaN → false). Added 7 FP
      encoders, **byte-verified vs the system assembler**. **Executed on real Apple-Silicon** via
      `jit-loader-macos` (`2.5*2.0`→`5.0` bits, `7.0/2.0`→`3.5`, all six comparisons).
    - ✅ **x86_64-backend** (v0.13.0, encoder v0.4.0) — mirrors the aarch64 lowering with SSE2:
      `const_f64` stores the bits; `add/sub/mul/div_f64` = `movsd`/`addsd…`/`movsd`; `cmp_*_f64` =
      `ucomisd` + `setcc` with operand-order + condition for IEEE-ordered semantics (`<`/`<=`
      reversed-operand `seta`/`setae`; `==` = `sete`&`setnp`; `!=` = `setne`|`setp`). Added SSE2
      encoders (`movsd`/`addsd`/`subsd`/`mulsd`/`divsd`/`ucomisd`), reg-reg **byte-verified vs the
      system assembler**. Structural exact-opcode tests; **executed on the lang-aot matrix
      `NativeAot` column on the Linux-x86 CI runner** (`run_native` compiles for the host arch — the
      same matrix cell runs aarch64 locally). **E3 COMPLETE — reals run on all 7 backends.**
  - ✅ **E3-clr** (iir-to-cil-bytecode v0.22.0) — the **textual `.il` emitter** (the real-CLR /
    `ilasm` matrix path) lowers `f64`: `cil_local_type` → `float64` locals, float consts →
    `ldc.r8` (exact LE bytes), comparison result forced to `int32` (CIL `ceq`/`cgt`/`clt` push
    an int even over `float64` operands). CIL `add`/`mul`/`ceq` are stack-type-overloaded → no
    opcode change. **Executed proof on real `ilasm` + `dotnet`**: both ALGOL real programs run
    on the CLR matrix column. (The structured **bytecode** emitter keeps its own f64 guard — a
    later follow-up; the real-CLR path is textual.)
- **E4 — Strings.** ◑ *IR + reference VM slice landed; see
  **[`lang-full-e4-strings.md`](lang-full-e4-strings.md)** for the full backend plan.*
  An IIR string value model (`str_const`, `str_len`, `str_index`, `str_concat`,
  `str_slice`, `str_eq`, `str_cmp`, `print_str`) + per-backend support, lowered to all 7 backends and
  verified by RUNNING (observable via **stdout**). A v1 string is an **immutable,
  length-counted byte buffer** — it reuses the E5 array substrate (length-prefixed flat
  buffer on the static backends; native `String` / managed `(array i8)` on the managed
  backends), so E4 is the *byte-aggregate sibling of E5*, not a new allocator. The one new
  host primitive is `__print_str`/`printStr` (the string sibling of `print_i64`). The VM now
  executes the shared ops directly (`tests/e4_strings.rs`), Dartmouth BASIC
  `PRINT "HELLO"` runs on all 7 backends, and Twig `(string-length "HELLO")`,
  `(string-ref "ABC" 1)`, `(string=? "HELLO" "HELLO")`, plus
  `(string-length (string-append "AB" "CDE"))` prove direct literal
  `str_len`/`str_index`/`str_eq`/`str_concat` on all 7 backends. Twig immutable
  top-level string value defines now also feed those same ops: named
  `str_concat`+`str_len`, `str_eq` driving an `if`, and named `str_index` all run
  on all 7 backends. Twig lexical `let` string locals now also run through E4:
  `(let ((s "ABC") (i 2)) (string-ref s i))` returns `67`, and
  `(let ((a "AB") (b "CDE")) (string-length (string-append a b)))` returns `5`,
  while `(let ((a "AB") (b "CDE") (i 3)) (string-ref (string-append a b) i))`
  returns `68` by feeding a concat temporary into `str_index`, and
  `(let ((s "ABCDE")) (string-ref s (- (string-length s) 1)))` returns `69`
  by feeding `str_len` through typed arithmetic into `str_index`, and
  `(let ((s "ABCDE")) (string-ref (substring s 1 4) 1))` returns `67` by
  feeding `str_slice` into `str_index`, and
  `(define (strlen) (string-length "HELLO")) (strlen)` returns `5` by
  preserving the top-level function's typed E4 string-op return through a
  direct `call`, and
  `(define (strlen (s : str)) (string-length s)) (strlen "HELLO")` returns `5`
  by using a bare `str` annotation to type a top-level function parameter and
  materialise the string argument through E4 string ops, and
  `(define (strlen s) (string-length s)) (strlen "HELLO")` returns `5` by using
  conservative `main`-level direct-call evidence to type an otherwise-unannotated
  top-level function parameter without synthesizing refinement annotations, and
  `(define (strlen x) (string-length x)) (strlen (substring (string-append
  "HE" "LLO!") 0 5))` returns `5` by accepting a static string expression
  actual as the same evidence, and
  `(define (same a b) (if (string=? a b) 42 0)) (same "OK"
  (string-append "O" "K"))` returns `42` by inferring multiple unannotated
  string parameters in one direct call and lowering parameter equality through
  `str_eq`, and
  `(define s "HELLO") (define (strlen x) (string-length x)) (strlen s)` returns
  `5` by accepting named, non-escaping top-level string actuals as the same
  evidence, and `(define (strlen x) (string-length x)) (let ((s "HELLO"))
  (strlen s))` returns `5` by accepting scoped lexical string actuals as the
  same evidence, and `(define (strlen x) (string-length x)) (let* ((a "HE")
  (b (string-append a "LLO"))) (strlen b))` returns `5` by accepting a derived
  sequential `let*` lexical string actual, on all 7 backends. The
  matrix now also proves the E4 bounds contract: `(string-ref "ABC" 3)` traps on
  native-AOT + LLVM + WASM + JVM + CLR + VM + JIT. WASM owns the literal-output
  shape with a linear-memory data segment + `env.__print_str(ptr,len)`, LLVM owns
  the static private `{len,bytes}` global + `@__print_str` shape, native AOT owns
  the heap-byte `alloc_bytes` + `store_byte` + `print_string` shape, and JVM/CLR
  own it with `ldc`/`ldstr` + `PrintStream.print(String)`/`Console.Write(string)`
  plus host string metadata/index calls. ALGOL now reuses the same output path:
  `begin print('HI') end` (and `output`) lowers literal actuals to `str_const` +
  `print_str` and runs on all 7 backends; `begin string s; s := 'HI'; print(s) end`
  proves literal-backed scalar string variables through the same path, and
  `begin string s, t; s := 'O'; t := 'K'; output(s, t) end` proves ordered
  multi-argument string output.
  ALGOL scalar string copies are snapshots (`begin string s, t; s := 'OK'; t := s;
  s := 'NO'; print(t) end` still prints `OK`). ALGOL literal-backed scalar string
  predicates lower through `str_eq`/`str_cmp` and run on every backend. BASIC also proves literal
  reassignment (`LET A$ = "NO"; LET A$ = "OK"; PRINT A$`) through the same slot,
  multi-item string `PRINT` (`PRINT A$; B$` and `PRINT A$, B$`), and copied-slot equality
  (`LET A$ = "OK"; LET B$ = A$; IF B$ = A$ THEN ...`) through `str_eq`, plus
  lexical string ordering branches through `str_cmp`.
  The runtime (non-literal) representation now carries ALGOL string-procedure
  results through initialized scalar locals (`str_concat` copy, `str_eq`, and
  `print_str`) across the seven standard backends and the BEAM ASCII
  character-list subset. Runtime lexical ordering now uses `str_cmp` across the
  seven standard backends and BEAM's character-list subset. Captured/`own`
  strings, string arrays, and Unicode-aware BEAM strings remain outside this slice. The fuller
  plan remains in **[`lang-full-e4-dyn-strings.md`](lang-full-e4-dyn-strings.md)**.
  Unlocks BASIC strings + string `PRINT` (BA4), ALGOL strings/I-O (AL4), Twig strings (TW4).
- **E5 — Arrays / linear aggregates.** ✅ **COMPLETE** *(PR-1..4c — runs on all 7 backends:
  VM, JIT, JVM, CLR, LLVM, WASM, native x86_64+aarch64).* An IIR
  array model (`alloc_array`/`array_len`/`array_get`/`array_set`, `array<T>` type hint,
  bounds-checked) that is **representation-agnostic** so it lowers to BOTH static-allocation
  (length-prefixed flat memory + explicit guard/trap on the native + LLVM backends, reusing the
  byte-tape allocator) AND garbage-collected targets (native managed arrays with native bounds
  checks on JVM/CLR/WasmGC). Bounds-checked from the start (OOB → trap). Full design + PR
  breakdown in **[`lang-full-e5-arrays.md`](lang-full-e5-arrays.md)**. Unlocks ALGOL arrays (AL2),
  BASIC `DIM` (BA3), Twig lists (TW3).
- **E6 — General `call_builtin` / closures / dynamic dispatch on code-gen backends.**
  The `call_builtin` allowlists + `type_hint="any"` rejection are why most of Twig
  runs only on the VM. **Design surveyed & written — the substrate already exists:**
  McCarthy Lisp's full cons/symbol/lambda/recursion suite runs on all 5 code-gen
  backends via the uniform boxed `ref<any>` value + two language-agnostic
  `iir-builtin-lowering` passes, and **Twig rides them too** — so this is a
  *catalog-extension*, not a from-scratch fork. Full design + PR breakdown in
  **[`lang-full-e6-dispatch.md`](lang-full-e6-dispatch.md)** (E6 layer 2).
  - **E6 layer 1 (typed module globals) — spec [`lang-full-e6-globals.md`](lang-full-e6-globals.md).**
    ✅ DONE. A function can read/write a typed scalar or array module global on
    all 7 backends (`global_load`/`global_store`). Scalars retain the original
    word slots; LLVM uses `ptr` and JVM/CLR use concrete reference fields for
    `array<T>`, so captured ALGOL arrays retain their handle and bounds across a
    call. Unblocks AL6 (`own`), O3 (Oct globals), and enclosing-array capture.
  - **E6 layer 2 (general dynamic dispatch) — spec [`lang-full-e6-dispatch.md`](lang-full-e6-dispatch.md).**
    ◑ STARTED. **E6d-1 ✅** — Twig `cons`/`car`/`cdr` (TW3-core) proven on the
    code-gen backends (matrix: `(car (cons 42 0))` → 42, run-verified on WASM +
    real dotnet CLR). Remaining: dynamic arithmetic (E6d-2), list ops (E6d-3),
    symbols (E6d-4), records/unions (E6d-5/6, TW6), closures-on-WASM (E6d-7, TW5),
    dynamic globals (E6d-8).
- **E7 — Subroutine / return-stack.** ✅ COMPLETE. `GOSUB`/`RETURN` and procedure
  call/return ([`lang-full-e7-subroutine-return-stack.md`](lang-full-e7-subroutine-return-stack.md)).
  Structured procedure call/return was already done (`call`/`ret` — ALGOL AL3,
  BASIC `DEF FN` BA5). BASIC `GOSUB`/`RETURN` is *unstructured* (the same `RETURN`
  resumes at the dynamically most-recent `GOSUB`) and `call`/`ret` cannot express it,
  so BA1 lowers it inside `main` as an E5 `array<i64>` return-PC stack + the AL5
  computed-goto chain (`cmp_eq`+`jmp_if_true`). `dartmouth-basic-iir-compiler` 0.10.0
  added the frontend lowering, and `iir-to-wasm` 0.18.0 fixed the dispatch-loop edge
  case it exposed; both BA1 proof programs now run on all 7 backends.
- **E8 — Numeric conversions (`integer` ↔ `real`).** ✅ **COMPLETE**
  ([`lang-full-e8-numeric-conversions.md`](lang-full-e8-numeric-conversions.md)).
  Three ops (`int_to_real`, `real_to_int_trunc`, `real_to_int_floor`); `real→int`
  traps out-of-range. Unblocks **AL8 `entier`** (floor), int→real **coercion**,
  BASIC **`INT()`** / **BA7**.
  - ✅ **PR-1 — interpreter-ir + vm-core + JIT** (interpreter-ir 0.8.0,
    vm-core 0.9.0). `is_conversion` classifier + value-producing registration;
    VM reference semantics (range-checked, fail-closed trap on NaN/∞/overflow);
    JIT inherits via cold-interpret. 5 vm-core unit tests + a jit-core
    integer→real→integer round-trip.
  - ✅ **PR-2 — LLVM** (iir-to-llvm 0.16.0). `int_to_real`→`sitofp`;
    `real_to_int_*`→`@llvm.trunc.f64`/`@llvm.floor.f64` + range-check
    (`@llvm.trap`, matching the VM — a bare `fptosi` of out-of-range is UB) +
    `fptosi`. RUN-verified on real clang (`floor(45.0−2.7)`⇒42).
  - ✅ **PR-3 — WASM** (iir-to-wasm 0.17.0). `int_to_real`→`f64.convert_i64_s`;
    `real_to_int_trunc`→`i64.trunc_f64_s`; `real_to_int_floor`→`f64.floor` then
    `i64.trunc_f64_s`. The **non-saturating** trunc traps out-of-range, matching
    the VM *for free* (the saturating `trunc_sat` would clamp — deliberately not
    used, so no explicit guard needed). RUN-verified on a real wasm runtime
    (`floor(45.0−2.7)`⇒42, trunc/floor sign cases).
  - ✅ **PR-4 — JVM** (iir-to-jvm-class-file 0.18.0). `int_to_real`→`i2d`/`l2d`
    (per the source's value model); `real_to_int_trunc`→`d2i`/`d2l` (truncate
    toward zero); `real_to_int_floor`→`invokestatic Math.floor(D)D` then
    `d2i`/`d2l`. **Documented trap divergence** (spec §7): `d2i`/`d2l`
    *saturate* (NaN→0, ±∞→MIN/MAX) where the VM traps — agrees bit-for-bit on
    every finite, in-range value, so the matrix cells match; a JVM range-check +
    `athrow` has no reusable precedent in this backend. RUN-verified on real
    `java` (`floor(int_to_real(45)−2.7)`⇒42).
  - ✅ **PR-5 — CLR** (iir-to-cil-bytecode 0.26.0). `int_to_real`→`conv.r8`;
    `real_to_int_trunc`→`conv.ovf.i4`; `real_to_int_floor`→`call
    System.Math::Floor(float64)` then `conv.ovf.i4`. The **overflow-checking**
    `conv.ovf.i4` truncates toward zero AND traps (`OverflowException`) on
    NaN/±∞/out-of-range — the VM's fail-closed contract *for free*, strictly
    better than the JVM's saturating divergence. Scalar ints are uniformly 32-bit
    here, so the narrow target is always `conv.ovf.i4`. RUN-verified on real
    `ilasm` + `dotnet` (`floor(int_to_real(45)−2.7)`⇒42).
  - ✅ **PR-6a — native aarch64** (aarch64-encoder 0.5.0 + aarch64-backend 0.13.0).
    `int_to_real`→`scvtf`; `real_to_int_trunc`→`fcvtzs`; `real_to_int_floor`→
    `frintm` then `fcvtzs`. True i64↔f64 (full Xn). `fcvtzs` saturates (documented
    divergence, like the JVM). **Executed on real Apple Silicon**:
    `floor(int_to_real(45)−2.7)`⇒42, plus the sign-sensitive `floor(−2.7)=−3` vs
    `trunc(−2.7)=−2`.
  - ✅ **PR-6b — native x86_64** (x86_64-encoder 0.5.0 + x86_64-backend 0.15.0 +
    x86-simulator 0.7.6). `int_to_real`→`cvtsi2sd`; `real_to_int_trunc`→
    `cvttsd2si`; `real_to_int_floor`→`roundsd …,1` then `cvttsd2si`. True i64↔f64.
    `cvttsd2si` yields the integer-indefinite on OOB (saturating divergence, like
    JVM/aarch64). **RUN-verified end-to-end through real x86_64 codegen executed
    in the x86-simulator** (`floor(int_to_real(45)−2.7)`⇒42, `trunc(42.3)`⇒42).
    With this, E8's conversion ops are implemented on **all seven backends**.
  - ✅ **PR-7 — ALGOL `entier`** (algol-iir-compiler 0.10.0 + lang-aot matrix). The
    standard function `entier(E)` (§3.2.5) — the largest integer ≤ the *real* `E`,
    floor toward −∞ — lowers to a single E8 `real_to_int_floor` op (the floor + the
    real→integer narrowing fused), so every backend emits its native floor-then-convert.
    A `real` argument is required. **RUN-verified on all 7 backends** via a new
    `lang_matrix.rs` cell: `45 + entier(0.0 − 2.7)` = `45 + (−3)` = 42 (the negative
    operand distinguishes floor from trunc — trunc would give 43). **E8 COMPLETE.**

---

## Per-language tracks (ordered; each item is one PR)

Ordered so the quick, frontend-local wins (lower to **existing** IIR ops, run on every
backend immediately) come before the enabler-dependent items.

### Nib  (closest to done — start here)
- ✅ **N1** — `*` and `/` (`STAR`→`mul`, `SLASH`→`div`). New `mul_expr` grammar level
  (binds tighter than additive); lowers to existing IIR `mul`/`div`; verified by RUNNING
  `6 * 7`→42 and `84 / 2`→42 across native/LLVM/WASM/JVM/CLR/VM/JIT.
- ✅ **N2** — `for` loop. `for NAME: type in lo .. hi block` (exclusive range) desugars to
  the canonical counter loop; verified by RUNNING a sum-loop (`1..6`→15) and nested loops
  (3×2→6) across native/LLVM/WASM/JVM/CLR/VM/JIT.
- ✅ **E-LLVM-1** — *reassigned parameters on LLVM* (LLVM first-class). The N2 build
  surfaced that the IIR-to-LLVM backend kept parameters in SSA, so reassigning a parameter
  across a loop back-edge (`fn run(acc) { for … { acc = acc + 6 } }`) silently dropped the
  update. Fixed in `iir-to-llvm` 0.10.0 — a reassigned parameter is promoted to an i64
  stack slot, initialised from the incoming argument. Verified by RUNNING `acc`-accumulator
  → 42 across every backend.
- ✅ **N3** — bitwise `&` `|` `^` `~`. `&`/`|`/`^` lower to IIR `and`/`or`/`xor` (verified by
  RUNNING `12 & 10`→8, `12 | 3`→15, `6 ^ 5`→3 across all 7 backends — also fixed a CLR
  textual-`.il` gap in `iir-to-cil-bytecode` 0.19.0). **Unary `~`** ✅ (nib-iir-compiler 0.16.0):
  `compile_unary` lowers `~` → IIR `not` carrying the narrow result width, so every backend masks
  it mod-2ⁿ — `~0u8 == 255`, `~15u4 == 0`, verified by RUNNING on native/LLVM/WASM/JVM/CLR/VM/JIT.
  Two fixes: `~` was being silently dropped (the single-child-wrapper passthrough counts only child
  *nodes*, so a `[TILDE, operand]` unary_expr looked like a transparent wrapper); and the textual
  CIL emitter had no unary-`not` arm (`iir-to-cil-bytecode` 0.21.0 adds it). Needed `iir-to-llvm`
  0.12.0's `not` op. (Logical `!` is covered by N9.)
- ✅ **N4** — `&&` / `||` short-circuit. `compile_short_circuit` lowers to a result slot
  guarded by `jmp_if_false`/`jmp`/`label` (portable subset — CLR textual path has no
  `jmp_if_true`); verified by RUNNING divide-by-zero short-circuit proofs (`1==2 && 84/0==0`
  →7, `1==1 || 84/0==0`→7, RHS never evaluated) + a true-path program, across all backends.
- ✅ **N5** — `const` declarations. Module-scoped integer-literal consts are collected and
  folded to their literal at each use (no runtime storage, runs everywhere); verified by
  RUNNING `const N: u8 = 42; … return N`→42 and `const A=30; const B=12; … A + B`→42 across
  all backends. Const-*expression* folding is covered by N10; mutable `static` is covered by N8.
- ✅ **N6** — u4/u8 wrap semantics (`nib-iir-compiler` 0.14.0 + E2). Plain narrow
  arithmetic now wraps mod-2^n before the value is observed: the matrix proves
  `200u8 + 100u8` by comparing the in-register result to `44`, and keeps `6 * 7`
  at `42` in a `u8` return context so the literal-typing guard cannot regress to
  accidental u4 masking. Both programs run on all 7 backends.
- ✅ **N7** — `+%` (wrap add) / `+?` (saturating add) (nib-iir-compiler 0.15.0). `+%` lowers to
  the narrow-typed `add` (E2 wraps it: `15u4 +% 1 = 0`, `200u8 +% 100 = 44`); `+?` lowers to a
  *wide* add + a clamp branch `min(sum, MAX)` (`15u4 +? 1 = 15`, `200u8 +? 100 = 255`,
  `3 +? 4 = 7`). Verified by RUNNING on all 7 backends (comparison-based matrix proofs) + a
  vm-core unit test. The grammar already had the `WRAP_ADD`/`SAT_ADD` tokens; no type-checker
  change (additive operators are type-inferred from operands).
- ✅ **N8** — mutable module `static` globals (nib-type-checker 0.4.0,
  nib-iir-compiler 0.17.0). Top-level `static NAME: type = integer-literal;`
  declarations are visible in every function, seed shared IIR globals at `main`
  entry (`const` + `global_store`), and unshadowed reads/writes lower to
  `global_load`/`global_store` (the E6 substrate). Verified by RUNNING
  `static counter: u8 = 40; bump(1); bump(1); return counter` → 42 across
  native/LLVM/WASM/JVM/CLR/VM/JIT. Const/static-expression folding is covered by N10;
  BCD storage semantics and Intel-4004 RAM mapping remain follow-ups.
- ✅ **N9** — logical `!` (nib-type-checker 0.5.0, nib-iir-compiler 0.18.0).
  `unary_expr` now types leading `!` as `bool` and lowers it through the existing
  truthiness branch contract to a 0/1 scalar. Verified by RUNNING
  `if !(1 == 2) { return 42; }` across native/LLVM/WASM/JVM/CLR/VM/JIT; the old
  passthrough behavior would have returned 0.
- ✅ **N10** — const/static expression folding (nib-type-checker 0.6.0,
  nib-iir-compiler 0.19.0). Top-level `const` and `static` initializers now type
  and fold deterministic integer/boolean expressions, including references to
  previously declared consts. Calls and non-const names remain rejected in
  initializer expressions. Verified by RUNNING
  `const BASE: u8 = 6 * 7; static counter: u8 = BASE + 0; return counter` → 42
  across native/LLVM/WASM/JVM/CLR/VM/JIT.

### Oct  (sister to Nib)
> **Oct had no observable output** (void `main` → always exits 0), which made its value-level
> features unverifiable-by-running — so the user chose to **give Oct an output op first**
> (decision 2026-06-13). With `out` → stdout (O-OUT below), Oct is now observable and its
> feature items are verifiable.
- ✅ **O-OUT** — the 8008 `out(port, value)` intrinsic prints `value` to stdout
  (`call_builtin "print_i64"`, all ports → stdout). Verified by RUNNING `out(1, 200)`→`200`
  and `out(1, 100 + 100)`→`200` across native/LLVM/WASM/JVM/CLR/VM/JIT. **Unblocks O1/O2/O3
  verification.** (`in` + the arithmetic/rotation intrinsics stay rejected — see O4.)
- ✅ **O1** — `&&` / `||` short-circuit (was eager bitwise). `compile_short_circuit` (result
  slot + jmp_if_false/jmp/label). **Proven by running** via a side-effecting function call in
  the RHS: `if 1 == 2 && side() == 1 { … } else { out(1, 9) }` where `side()` prints 5 →
  stdout `9` (old eager printed `5`,`9`); `||` analogue → `7`. Across **all 7 backends** —
  the JVM column was unblocked as a BA-JVM-1 follow-through (iir-to-jvm-class-file 0.13.3): a
  `mov` now bridges int↔long when the dest slot width differs, so Oct's bool comparison result
  mov'd into a `long` short-circuit accumulator (Oct keeps the i64 value model) widens with
  `i2l` instead of leaving the long slot's second half uninitialized. With this, **every
  `lang_matrix.rs` program runs on all 7 backends**. Also fixed Oct non-void function returns
  to materialise as `i64` (the `side() -> u8` helper exposed `define i8 @side()` mismatching its
  i64 body on LLVM).
- ✅ **O2** — bitwise `~` 8-bit mask + u8 wrap. Oct's only integer type is `u8` (the 8008
  byte) and the spec wraps mod-256, so `oct-iir-compiler` 0.7.0 emits the `u8` type_hint on
  arithmetic/bitwise/`~` (comparisons stay i64); every backend masks the result. Verified by
  RUNNING `out(1, ~0)`→`255` and `out(1, 200 + 100)`→`44` on native/LLVM/WASM/JVM/CLR/VM/JIT.
  Surfaced + fixed a JVM dual-model bug: Oct's *printing* programs keep the i64/long model, so
  a narrow op had `long` operands — `iir-to-jvm-class-file` 0.14.0 now masks those with
  `i2l; land` (the int `iand` was unverifiable over longs → empty output). (Logical `!` is
  covered by O-! below; only `~` is in O2.)
- ✅ **O3** — `static` module globals (LANG-FULL O3). Top-level `static` was silently
  dropped at IIR-gen; `oct-iir-compiler` 0.8.0 lowers it to the IIR module-global ops
  (`global_load`/`global_store`, the E6 substrate). A `static counter: u8 = 40` shared
  across functions — `bump()` (a separate fn) increments it twice, `main` prints it —
  runs on **all 7 backends** → `42` (a per-function register would print `40`). Surfaced
  + fixed two latent *void-function* gaps the proof's void `bump()` exposed: the Oct
  frontend now emits a dest-less IIR `call` for a void callee (a named void call is
  malformed LLVM), and `iir-to-cil-bytecode` 0.25.0's textual emitter lowers `ret_void`
  → bare `ret`, a `void` return signature, and a `call void …` with no trailing store.
- ✅ **O-!** — logical `!` (LANG-FULL O-!). `oct-iir-compiler` 0.9.0 now lowers unary
  logical NOT through `jmp_if_false` / `jmp` / `label`, assigning a clean 0/1 bool result
  instead of reusing bitwise `not` (`not 0` = -1, `not 1` = -2). Verified by RUNNING
  `if !(1 == 2) { out(1, 42) } else { out(1, 0) }` on native/LLVM/WASM/JVM/CLR/VM/JIT
  → stdout `42`.
- ☐ **O4** — ⚠ Intel-8008 intrinsics (`in`/`out`/`adc`/`sbb`/`rlc`/`rrc`/`ral`/`rar`/`carry`/`parity`).
  These are hardware-specific; on general backends they need a host/IIR-builtin model or a
  defined semantics. **Decision point — surface to the user before implementing.**

### Brainfuck  (semantics complete; gap is cross-backend *execution* of real programs)
- ✅ **B1** — **execute real programs cross-backend**, output-checked. A nested-loop
  multiply program → stdout `"HA"` and a two-sequential-loop program → `"OK"` run on
  native/LLVM/WASM/JVM/CLR/VM/JIT (`lang_matrix.rs`), proving nested loops + multi-cell
  pointer movement + multiple `putchar`s lower everywhere — not just the trivial 1-loop "A".
- ✅ **B1-stdin** — **read real input (`,`) cross-backend**, output-checked. Two programs:
  `,+.` (read a byte, `+`, print: input `"A"` → `"B"` — output depends on input *and* a
  computation on it) and `,.,.` (echo two bytes: `"Hi"` → `"Hi"` — repeated reads advance
  the stream) run on all 7 backends. Harness-only (every backend already compiled
  `,`→`getchar`): the four subprocess columns pipe real process stdin (`output_with_stdin`),
  WASM/VM/JIT drain a `program_stdin` byte buffer. lang-aot 0.90.0. (These two programs read
  exactly the supplied bytes, so they never hit EOF — the EOF convention is handled by B1-eof.)
- ✅ **B1-eof** — **EOF normalised to 0 → the canonical cat `,[.,]` runs cross-backend**,
  output-checked (input `"Hi"` → stdout `"Hi"`). The backends disagreed on `getchar`-EOF
  (JVM/VM/JIT → 0; libc `getchar`/`Console.Read`/wasm host → -1 → the u8 cell wrapped to 255),
  so cat looped forever on the -1 backends. `brainfuck-iir-compiler` 0.4.0 now normalises EOF
  to 0 **in the shared IIR**: `,` reads `getchar` at i64, tests the bits above the low byte
  (`v & ~0xFF` — non-zero ⟺ EOF, *sign- and width-agnostic*: a signed `<0` test is unportable
  because the native path zero-extends the i32 `-1`), and branches to store `0` (EOF) or the
  byte — each arm doing its OWN `store_mem` so nothing crosses the merge (Brainfuck keeps state
  in the tape; the IIR has no phi nodes). Surfaced + handled two backend pitfalls: native's
  non-sign-extended getchar, and the CIL int32 emitter (mask must be `-256`, not `0xFFFFFF00`).

### Dartmouth BASIC
- ✅ **BA0** — BASIC control flow on the code-gen backends. The real bug wasn't wasm
  (`iir-to-wasm` already lowers all `cmp_*`; the `#[ignore]`s were stale and are removed) —
  it was the BASIC compiler emitting `cmp_*` with a **`bool`** type hint, so LLVM compared
  at 1-bit `i1` width (`7 > 5` → `1 > 1` → false). Fixed to emit the `i64` operand width
  (like Nib/Oct/ALGOL). Verified by RUNNING `FOR I = 1 TO 5: S = S + I`→`15` and
  `IF A > 5 THEN 100`→`7` across native/LLVM/WASM/CLR/VM/JIT.
- ✅ **BA-JVM-1** — BASIC branch (`IF`/`FOR`) **+ `print_i64`** on the JVM (iir-to-jvm-class-file
  0.13.2). The diagnosis wasn't a StackMapTable issue (the backend emits class version 49 to
  skip StackMapTables) but a slot-typing bug: `build_type_map` typed a comparison's dest by its
  `type_hint` (the *operand* width), so a comparison over BASIC's i64 operands got a `Long` slot
  — yet a comparison always produces a 0/1 `int` stored with `istore`, and the later
  `jmp_if_false` read it with the long guard (`lload; lconst_0; lcmp`) → `VerifyError:
  uninitialized register pair`. (Nib's loops escape it because scalar Nib is concretized to i32;
  BASIC prints, so it keeps the i64 model.) Fix: comparison dests are typed `int`. **Verified on
  real `java`** — the BASIC `FOR` sum (`15`) and `IF` branch (`7`) now run on the JVM; both added
  to the matrix JVM column.
- ✅ **BA1** — `GOSUB` / `RETURN` (enabler **E7**). `dartmouth-basic-iir-compiler` 0.10.0
  lowers unstructured `GOSUB`/`RETURN` *inside* `main` per the E7 spec: a fixed-capacity
  `array<i64>` return-address stack + an AL5 computed-`goto` (`cmp_eq`+`jmp_if_true`)
  dispatch — **no new backend op**. The same `RETURN` resumes at the dynamically
  most-recent `GOSUB`. Proven by two executed matrix programs (`919` = one `RETURN`,
  two call sites; `876` = nested LIFO) on **all 7** backends
  (native/LLVM/WASM/JVM/CLR/VM/JIT). The final WASM gap closed in `iir-to-wasm` 0.18.0:
  when the last basic block contains conditional jumps, wasm lowering appends an
  unreachable sentinel block so the dispatch chain can restart `$dispatch` instead of
  falling out with a `StackUnderflow`.
- ✅ **BA2** — multi-item `PRINT`, `;`/`,` separators (`dartmouth-basic-iir-compiler`
  0.9.0). `PRINT` now prints several items on ONE line: each numeric item lowers to a
  `call __basic_print_int` — a synthetic *recursive* helper that renders digits one at a
  time through the universal `putchar` builtin (the same one Brainfuck uses) — instead of
  the old per-item `print_i64` (which appended a newline, forcing items onto separate
  lines). `;` joins tightly, `,` inserts a space, a trailing separator suppresses the
  newline, and bare `PRINT` emits a blank line. Because the helpers reuse only ops every
  backend already runs (`call`, integer `div`/`mul`/`sub`/`add`, `cmp_*`, `putchar`), BA2
  needed **zero** backend changes — verified by two executed matrix cells (`PRINT 0 - 12;
  34` ⇒ `-1234`, `PRINT 5, 6` ⇒ `5 6`) on all 7 backends. *More relops* were already done
  (grammar + `extract_relop_op` cover all six `= < > <= >= <>`). Deferred: string variables
  and string code-gen backends (→ BA4/E4); true 14-column `,` print zones (a single space
  approximates them).
- ✅ **BA3** — arrays / `DIM` (enabler **E5**). `DIM A(n)` lowers to `alloc_array`
  (BASIC arrays are 0-based + inclusive, so `n + 1` elements); `LET A(i) = e` →
  `array_set` and `A(i)` rvalues → `array_get`, with the subscript used directly as
  the 0-based index (no lower-bound subtraction, unlike ALGOL `[lo:hi]`). BA7-3
  promotes the element storage to `array<f64>` while keeping subscripts as the
  explicit `i64` boundary. These are the same shared array ops ALGOL's E5
  arrays use, so BASIC arrays RUN on all 7
  backends — verified by a straight-line array program (`DIM A(3); A(1)=40; A(2)=2;
  PRINT A(1)+A(2)` ⇒ `42`) in `lang-aot/tests/lang_matrix.rs`. (`dartmouth-basic-iir-compiler`
  0.7.0.) Undeclared subscript use is a clean `Unsupported` error.
  **BA-DIM-2D ✅**: multi-dimensional `DIM A(m,n)` (and 3-D+) now run on **all 7
  backends**. The grammar gains comma-separated subscripts (`dim_decl`/`variable`),
  and the frontend records per-array **row-major strides** at `DIM` (`DIM A(M,N)` →
  strides `[N+1, 1]`), folding `A(i,j)` to the flat 0-based index `i*(N+1) + j` via
  `const`+`mul`+`add`. Still the same E5 `alloc_array`/`array_set`/`array_get` ops —
  no backend change; 1-D `DIM` is unchanged (stride `[1]`, bare subscript). Verified
  by `DIM A(1,2); A(0,0)=40; A(1,2)=2; PRINT A(0,0)+A(1,2)` ⇒ `42`.
  (`dartmouth-basic-iir-compiler` 0.35.0 / `-parser` 0.3.0.) String arrays and array
  `INPUT` remain BA4 follow-ups.
- ◑ **BA4** — string literal `PRINT` runs on all 7 backends via **E4**.
  Literal-backed scalar string variables now run too: `LET A$ = "HI"; PRINT A$`
  produces `HI`, `LET A$ = "NO"; LET A$ = "OK"; PRINT A$` produces `OK`,
  `LET A$ = "O" + "K"; PRINT A$` proves literal `str_concat`,
  `LET A$ = "OK"; LET B$ = A$; PRINT B$` proves scalar string copy,
  `PRINT A$; B$` proves ordered repeated string output, and
  `LET A$ = "O"; PRINT A$ + "K"` proves `PRINT` can consume a temporary string
  expression result directly. `LET A$ = "O"; LET B$ = "K"; PRINT A$ + B$`
  proves both concat operands can be scalar string slots in that direct-print path.
  `LET A$ = "O"; IF A$ + "K" = "OK" THEN n`
  proves that same expression path before `str_eq` line-control branching.
  `LET A$ = "O"; LET B$ = "K"; IF A$ + B$ = "OK" THEN n` proves the
  variable-variable concat expression also feeds the standard equality branch
  path (`str_eq` plus `jmp_if_true`).
  `LET A$ = "O"; LET B$ = "K"; IF A$ + B$ <> "NO" THEN n` proves the
  variable-variable concat expression also feeds the standard inequality branch
  path (`str_eq` plus `jmp_if_false`).
  `LET A$ = "O"; LET B$ = A$ + "K"; PRINT B$` proves variable-backed concat
  assignment into a second scalar string slot, and
  `LET A$ = "A"; LET B$ = A$ + "B" + "C"; PRINT B$` proves left-associative
  chained concat through repeated E4 `str_concat`.
  `PRINT A$, B$` proves BA2's comma separator (`putchar(' ')`) composes with
  ordered E4 `print_str` calls and produces `O K` on all seven backends.
  `IF A$ = "Y" THEN n` / `IF A$ <> "Y" THEN n` lower to `str_eq` in the frontend
  and now drive real line-control branching on all seven backends (`OK`/`BAD`
  matrix proofs).
  `IF A$ < "B" THEN n` / `IF "B" > A$ THEN n` lower through `str_cmp` plus typed
  zero comparisons and also run on every backend.
  **String `INPUT` ✅** (E4d-BA-input, all 7 backends): `INPUT A$` reads a whole
  stdin line as a runtime string via `call_builtin "input_str"`; two matrix cells
  (`INPUT A$` → `OK`, runtime concat `INPUT A$ / INPUT B$ / PRINT A$ + B$` → `OK!`).
  **String arrays ✅** (E4d-BA-arr, **all 7 backends**): `DIM A$(n)` lowers to an
  `array<str>` (the E5 aggregate carrying E4-dyn string handles); `A$(i)=s`/`A$(i)`
  are `str`-typed `array_set`/`array_get` feeding PRINT / `+` concat. Static
  backends carry an 8-byte (LLVM/native) or 4-byte (WASM) handle element; JVM/CLR
  use native reference arrays (`String[]` / `System.String[]`). Matrix cell
  `DIM A$(2); A$(0)="O"; A$(1)="K"; PRINT A$(0)+A$(1)` → `OK` on all 7. Broader
  dynamic string expressions and string `READ`/`DATA` remain.
- ✅ **BA5** — `DEF FN` single-line user functions. `DEF FNx(P) = expr` lowers to a
  sibling `IIRFunction` (one numeric param, `FullyTyped`) and `FNx(arg)` lowers to the
  shared IIR `call` — the same convention ALGOL's value procedures (AL3) run on every
  backend. A pre-pass registers all `DEF` names first so a call may precede its `DEF`
  (BASIC forward use). **Verified by RUNNING** `DEF FNS(X) = X * X : PRINT FNS(7)` → `49`
  across native/LLVM/WASM/JVM/CLR/VM/JIT (`lang_matrix.rs`). Surfaced & fixed a JVM bug:
  `lang-aot`'s `concretize_scalar_any_for_jvm` decided the i32/i64 value model
  **per-function**, so the printing `main` (kept i64) called the non-printing helper `FNS`
  (narrowed to `(I)I`) with a `long` arg → `VerifyError`, empty output. Concretization is
  now a **whole-module** decision (any printing function ⇒ the whole scalar module stays
  i64), keeping cross-function call signatures consistent (lang-aot 0.94.0). **Limits:** one
  numeric parameter; body references its parameter only (globals need **E6**); built-in
  maths fns (`SIN`/`ABS`/…) need **E3**.
- ✅ **BA6** — `READ` / `DATA` / `RESTORE` (`dartmouth-basic-iir-compiler` 0.8.0,
  real-valued in 0.12.0).
  Lowers onto the **E5 array** substrate — no new IIR op, no enabler: a pre-pass
  gathers all finite `DATA` literals (line order) into a pool materialised once at
  the top of `main` as an `array<f64>` + an `i64` `__basic_data_ptr` register (a
  register, not a global, since the program is one `main` function). `READ` does `array_get
  pool, ptr` + `ptr := ptr + 1`; `RESTORE` resets `ptr := 0`; out-of-DATA traps via
  the bounds-checked `array_get`. **Runs on all 7 backends**: `DATA 21 / READ A /
  RESTORE / READ B / PRINT A+B` ⇒ 42 (proves sequential consumption + rewind).
  BA7-3 adds the fractional proof: `DATA 3.14, 0.25 / READ A(0) / READ B / PRINT
  A(0) / PRINT B` ⇒ `3.14` and `.25` on all 7 backends.
- ✅ **BA7** — floating-point (needs **E3**, ✅; **E8**, ✅). Design spec is
  **decision-complete** ([`lang-full-ba7-floating-point.md`](lang-full-ba7-floating-point.md))
  by historical Dartmouth BASIC fidelity (no sign-off gate). **BA7-1a/1b landed**
  (`dartmouth-basic-iir-compiler` 0.11.0): decimal/exponent and integer-spelled
  scalar literals now lower to `Operand::Float`; scalar arithmetic/variables,
  `DEF FN`, `IF`, `FOR`, and `PRINT` run as `f64`; array/index integer
  boundaries use `real_to_int_trunc`; and `PRINT` has a staged
  `__basic_print_real(x: f64)` helper for whole-valued, rounded fixed-decimal,
  and `E`-notation output. **Verified by RUNNING** `PRINT 42`, `PRINT 6.0 * 7.0`
  ⇒ `42`, `3.14`/`.25`/`-2.5` fractional output, and `1.23457E+08` /
  `1.23457E-04` formatter output on native/LLVM/WASM/JVM/CLR/VM/JIT. **BA7-3 landed** in
  `dartmouth-basic-iir-compiler` 0.12.0: `DIM` arrays and `DATA` pools now store
  `f64`, with index/read-pointer boundaries left as `i64`; fractional `DATA`
  through array and scalar `READ` runs on native/LLVM/WASM/JVM/CLR/VM/JIT. BA7
  numeric formatting completed in 0.13.0. **BA7 COMPLETE.**
- ✅ **BA-^** — integer-literal exponentiation (`dartmouth-basic-iir-compiler` 0.26.0).
  The parser already supported right-associative `^`, but the compiler rejected it pending
  a runtime math helper. The backend-neutral slice now recognizes nonnegative
  integer-valued literal exponents `0..=64` and lowers `base ^ n` to repeated `f64`
  `mul`, so no backend learns a new operation. Verified by RUNNING `PRINT 6 ^ 2 + 6`
  on native/LLVM/WASM/JVM/CLR/VM/JIT → stdout `42`. General variable, nested, negative,
  fractional, and large exponents still need a cross-backend runtime math helper.
- ✅ **BA-builtins** — `SQR`, `INT`, `ABS`, `SGN` built-in functions
  (`dartmouth-basic-iir-compiler` 0.32.0). All four reuse existing IIR ops — no new
  backend code needed. `SQR(X)` → `f64_sqrt` (the same hardware-sqrt op ALGOL uses).
  `INT(X)` → `real_to_int_floor` + `int_to_real` (E8 ops, floor toward −∞, result is
  real). `ABS(X)` and `SGN(X)` lower inline via `cmp_lt`/`cmp_gt` + store-per-branch
  conditionals (same pattern as ALGOL `abs`/`sign`). **Verified by RUNNING** on
  native/LLVM/WASM/JVM/CLR/VM/JIT: `PRINT SQR(49)` → `7`, `PRINT INT(3.7)` → `3`,
  `PRINT ABS(-42)` → `42`, `PRINT SGN(-5)` → `-1`. **`ATN(X)` ✅** and **`TAN(X)` ✅**
  (dartmouth-basic-iir-compiler 0.33.0, BA-arctan) lower to `f64_atan` / `f64_tan` IIR ops;
  `PRINT ATN(0)` → `0`, `PRINT TAN(0)` → `0` verified on all 7 backends.
  `SIN`, `COS`, `LOG`, `EXP`, and `RND` still need a cross-backend math helper — deferred.

### ALGOL 60
- ✅ **AL1** — real arithmetic + `/` (algol-iir-compiler 0.4.0): `real` → IIR `f64`, `REAL_LIT`
  → `Operand::Float`, `+`/`-`/`*`/unary-minus over reals emit the `f64` hint, `/` is real
  division, real comparisons compare at `f64` width; `div`/`mod` stay integer-only; integer
  values widen with `int_to_real` whenever a real is required (mixed numeric operations, `/`,
  real assignments/array elements/formals, and real standard functions). **Verified by RUNNING on ALL 7 backends**
  (`lang_matrix.rs` — real `*`+`=`→42, real `/`+`<`→1): VM/JIT (tagged value model), LLVM
  (`double` slots), WASM (typed locals), JVM (`CONSTANT_Double`+`dcmpl`), CLR (`float64`+`ldc.r8`),
  and native-AOT (aarch64 `fadd`/`fcmp` executed on Apple Silicon + x86_64 SSE2 on CI). **E3 done.**
- ✅ **AL2** — 1-D arrays with runtime bounds (E5). `integer`/`real array A[lo:hi]` →
  `alloc_array` (run-time span); `A[i]` reads/writes → bounds-checked `array_get`/`array_set`
  with the 0-based index `i - lower`. A sum-of-squares `Prog` runs on VM/JIT/JVM/CLR (exit 55)
  + 9 unit tests; a **straight-line** array `Prog` runs on **all 7 backends** (exit 42) — VM/JIT,
  JVM native `int[]` (PR-3a), CLR native `int32[]` on real `dotnet` (PR-3b), LLVM static `@calloc`+
  `llvm.trap` via `clang` (PR-4a), WASM linear-memory+`unreachable` via `wasm-runtime` (PR-4b),
  and **native** x86_64/aarch64 `__twig_alloc_bytes`+`ud2`/`udf` trap (PR-4c — aarch64 local,
  x86_64 CI). The for-loop sum-of-squares array Prog now runs on LLVM too (the ALGOL-for-loop
  guard-type fix landed in `algol-iir-compiler` 0.5.1).
  **AL-multidim ✅**: `integer array M[1:2, 1:2]` (2D) runs on **all 7 backends**
  via row-major flat index `(i-lo1)*stride + (j-lo2)` computed during declaration; strides
  accumulated right-to-left; `alloc_array`/`array_set`/`array_get` with flat 0-based index;
  aarch64 NativeAot required large-frame split prologue (frames > 504 bytes → `SUB SP`+`STR`×2,
  frames ≤ 504 bytes unchanged); `algol-iir-compiler` 0.23.0 / `aarch64-backend` 0.20.0.
  **AL-multidim-real ✅**: `real array M[1:2, 1:2]` (fractional f64 elements) runs on **all 7
  backends** — same flat-index machinery carrying `ScalarType::Real` elements on the E5 8-byte
  slots, summed and floored via `entier`; `algol-iir-compiler` 0.24.0 / `lang-aot` 0.166.0.
  **AL-multidim-3D ✅**: `integer array M[1:2, 1:2, 1:2]` (3D) runs on **all 7 backends** —
  proves the lowering is genuinely N-dimensional (stride product `stride[0]=size[1]*size[2]=4`,
  `stride[1]=2`, `stride[2]=1`); no compiler change (the right-to-left stride loop just walks one
  more iteration); `algol-iir-compiler` 0.25.0 / `lang-aot` 0.167.0.
  **AL-multidim-bounds ✅**: `integer array M[-1:1, 2:3]` (arbitrary, incl. negative, per-dimension
  lower bounds) runs on **all 7 backends** — proves the per-dim `sub−lower` subtraction composes
  with the row-major strides (`flat = Σ_d (sub[d]−lower[d])*stride[d]`); no compiler change (the
  `ArrayDim.lower_slot` subtraction already existed); `algol-iir-compiler` 0.26.0 / `lang-aot` 0.169.0.
  **AL-array-params ✅**: an array `value` formal infers its rank from its
  indexed uses in the procedure body, then receives the caller's typed handle,
  every lower bound, and each non-final row-major stride. The descriptor keeps
  `a[i,j,...]` in the actual's declared index space, with rank mismatches and
  inconsistent formal subscript counts rejected before lowering. A 2-D,
  nonzero-lower-bound captured actual runs through a forwarding procedure on
  all seven standard backends; the compiler unit suite also executes a 3-D
  formal on the VM. One-dimensional formals retain the original handle-plus-
  lower-bound ABI. Nested procedures can likewise capture an outer scalar
  `value` formal: the outer function publishes its typed incoming value before
  the nested sibling runs, and shadowing formals remain local.
  **AL-captured-arrays ✅**: procedures can now read/write arrays declared in an
  enclosing block. The frontend globalizes the handle and every lower-bound /
  row-major-stride metadata value, so `integer array values[4:5]; procedure
  seed; values[4] := 40; values[5] := 2; seed` returns 42 on all seven standard
  backends. Array `value` descriptors use the call ABI above, and a nested
  procedure can capture an outer formal by reloading its globalized handle and
  every bound/stride component in the fresh nested frame. A 2-D `string array`
  formal now exercises that captured descriptor on all seven backends, with
  dynamic string cells checked by lexical ordering, equality, and inequality.
- ✅ **AL3** — typed procedures with value parameters. `integer procedure sq(x);
  value x; integer x; sq := x*x; result := sq(7)` ⇒ exit 49, **verified by running**
  across native/LLVM/WASM/JVM/CLR/VM/JIT (`lang-aot` `lang_matrix.rs`). Lowered to a
  sibling `IIRFunction` + IIR `call`; supports forward references + recursion + multi-arg.
  Surfaced & fixed a real `jit-core` constant-propagation bug (reassigned result slot
  propagated its dead seed → only the JIT returned 0). **Limits (follow-ups):** `value`
  params only (by-name is AL7); zero-argument calls may use explicit `f()` in
  value or statement position; procedures may capture enclosing scalar, array,
  array-formal, and scalar-formal declarations through typed globals.
- ◑ **AL4** — literal string `print`/`output` I/O runs on all 7 backends via
  **E4**. Undeclared statement-position `print('HI')`/`output('HI')` calls lower
  to `str_const` + `print_str`, and literal-backed scalar string variables
  (`string s; s := 'HI'; print(s)`) now run the same way. The matrix also proves
  the `output(s)` spelling, multi-argument `output(s, t)`, plus literal-backed
  scalar copy snapshot semantics (`string s, t; s := 'OK'; t := s; s := 'NO';
  print(t)`), plus string equality/inequality and lexical ordering predicates, all on
  native/LLVM/WASM/JVM/CLR/VM/JIT. **String-typed value parameters in typed
  procedures are now proven** (`algol-iir-compiler` 0.18.0): `integer procedure
  echo(s); value s; string s; print(s)` passes a literal or named-variable
  string to the body's `print_str` on all 7 backends (`lang-aot` 0.154.0, AL4-str-params).
  Runtime string procedure results can now be copied into initialized scalar
  locals, compared for equality or lexical ordering, and printed across the
  standard seven backends; the same program is executed on BEAM using
  printable-ASCII character lists. `string array A[1:2]` now reuses the E5
  `array<str>` substrate on all seven standard backends: literal elements can
  be read for lexical ordering and output. Captured strings use typed globals,
  and `own string` initializes once to the empty string before retaining later
  assignments across calls. Unicode-aware BEAM strings remain.
- ✅ **AL5** — switches (computed goto) + conditional designational expressions.
  `switch s := a1,a2,a3; … goto s[3]` ⇒ exit 49, **verified by running** across
  native/LLVM/WASM/JVM/CLR/VM/JIT (`lang_matrix.rs`). `goto s[i]` lowers to a 1-based
  `index == k ? jmp Lk` chain; `goto if b then L1 else L2` lowers via the portable
  branch subset. **Surfaced & fixed a latent ALGOL cmp bug**: comparisons emitted a
  `bool` type_hint, so LLVM compared `i64` operands at 1-bit `i1` and emitted invalid IR
  (the cell *failed to run*) — fixed to emit the i64 operand width (the BA0 fix). This was
  the first ALGOL comparison ever exercised on a code-gen backend. Switch-list
  elements retain their full designator until the selected `goto`: a conditional
  branch and a nested switch subscript execute at that time, so both see current
  variables. A cyclic switch graph is rejected before it can recursively expand
  the IIR dispatch chain. **Limit:** switches aren't block-scope-shadowable.
- ✅ **AL6** — `own` variables (static lifetime). `coding-adventures-algol-parser`
  0.2.0 adds the `[ "own" ] type ident_list` rule; `algol-iir-compiler` 0.7.0 lowers
  an `own` scalar to a module **global** (the E6 substrate), keyed by its unique
  per-procedure slot, and crucially drops the per-declaration `const` zero-init for
  globals so the value is not re-zeroed each call. `bump(1) + bump(1) + bump(1)`
  accumulates `1 + 2 + 3 = 6` (a non-`own` local gives `3`) on **all 7 backends**.
  (Grammar was patched surgically, not full-regen — the checked-in `algol.grammar`
  has drifted ahead of the compiled grammar in other rules; resync is follow-up.)
- ☐ **AL7** — ⚠ call-by-name (Jensen-style expression thunks). **Hardest item in the
  campaign — design pass + user check before implementing.**
- ✅ **AL8** — standard functions (§3.2.4/§3.2.5). All pure-IIR and transcendental
  functions are done: **`abs` ✅** (algol-iir-compiler 0.8.0), **`sign` ✅** (0.9.0),
  **`entier` ✅** (0.10.0), **`sqrt` ✅** (0.17.0), **`sin`/`cos`/`ln`/`exp` ✅**
  (0.18.0, AL8-trig), and **`arctan` ✅** (0.19.0, AL8-arctan). `abs`/`sign` lower inline
  to compares + `jmp_if_false` + `mov`-into-one-slot (store-per-branch, no phi).
  `entier(E)` lowers to the E8 `real_to_int_floor` op. `sqrt(E)` lowers to the `f64_sqrt`
  IIR op — hardware sqrt everywhere (no libm). The four trig/log transcendentals lower to
  `f64_sin`, `f64_cos`, `f64_ln`, `f64_exp` IIR ops via the shared `emit_f64_unary` helper.
  `arctan(E)` lowers to the new `f64_atan` IIR op (inverse tangent, §3.2.4 — range
  −π/2..+π/2; `arctan(0.0)` = 0.0 exactly; LLVM uses direct libm `@atan` declaration since
  there is no `@llvm.atan.f64` intrinsic, unlike sin/cos/log/exp). Backend mappings:
  WASM `env.__sin/cos/ln/exp/__atan` host imports; LLVM `@llvm.sin/cos/log/exp.f64`
  intrinsics + `@atan` libm direct; JVM `Math.sin/cos/log/exp/atan`; CLR
  `System.Math.Sin/Cos/Log/Exp/Atan`; native aarch64/x86_64 `BL`/`call rel32` to libm
  `sin/cos/log/exp/atan`; VM/JIT dispatch handlers. Note: `ln` maps to `log` in all backends.
  Verified by RUNNING `abs(0-42)`⇒42, `43+sign(0-1)`⇒42, `45+entier(0.0-2.7)`⇒42,
  `entier(sqrt(49.0))`⇒7, `entier(cos(0.0))+41`⇒42, `entier(exp(0.0))+41`⇒42,
  `entier(sin(0.0)+42.0)`⇒42, `entier(ln(1.0)+42.0)`⇒42,
  `entier(arctan(0.0)+42.0)`⇒42 on all 7 backends.
- ✅ **AL-pow** — the `↑` exponentiation operator (§3.3.4; spelled `^`/`**`).
  A **nonnegative integer-literal exponent** unrolls to repeated multiply
  (`k−1` `mul`s; `x↑0=1`, `x↑1=x`), **keeping the base's type** — `2↑10` is the
  *integer* 1024, unlike BASIC's always-`real` BA-pow. A **`real↑real`** exponent
  lowers to the `f64_pow` op (libm `pow`) BA-pow already proved on every backend.
  No new IIR op, no backend change. An `integer` base with a `real`/runtime/negative
  exponent is a clean `Unsupported` (needs int→real coercion / reciprocals — a later
  slice). Verified by RUNNING `10 + 2 ^ 5` ⇒ 42 (integer unroll) on all 7 backends
  (`algol-iir-compiler` 0.27.0 / `lang-aot` 0.170.0).

### Twig
- ✅ **TW1** — variadic arithmetic typed lowering. An all-`i64` `(+ a b c …)` /
  `-` / `*` / `/` call folds to a left-associated chain of typed binary CIR ops, so
  n-ary arithmetic clears the code-gen backend validators (previously only binary
  `(+ a b)` was typed; 3+ args fell back to the rejected `call_builtin "any"` path).
  **Verified by running** `(+ 10 20 12)` ⇒ exit 42 across native/LLVM/WASM/JVM/CLR/VM/JIT
  (`lang_matrix.rs`). Chained comparisons (`(< a b c)`, a predicate not a fold) and
  unary/nullary forms stay on the dynamic path.
- ✅ **TW2** — top-level value `define` on the code-gen backends. A value `define`
  not captured by a lambda (read only from `main`) keeps its statically-typed value
  in a `main` register instead of the `call_builtin "global_set"`/`global_get` pair
  the backends reject; reads return the register directly. **Verified by running**
  `(define x 40) (define y 2) (+ x y)` ⇒ exit 42 across native/LLVM/WASM/JVM/CLR/VM/JIT
  (`lang_matrix.rs`). Added a reusable escape analysis (`free_vars::lambda_captured_globals`).
  **Limits:** a value captured by a closure, or a top-level forward reference, stays on
  the host global table (unchanged) — full mutable globals on code-gen backends need **E6**.
- ◑ **TW3** — list / cons ops on code-gen backends. **Cons core ✅** (E6d-1):
  `(car (cons 42 0))` and nested `(car (cdr (cons 1 (cons 42 0))))` run on the
  code-gen backends (WASM + real dotnet CLR verified; native/LLVM/JVM via CI) —
  the Twig frontend's `call_builtin "cons"/"car"/"cdr"` lowers through the shared
  `iir-builtin-lowering` heap passes to the same `ref<any>` substrate McCarthy
  uses. List builtins (`list`/`length`/`append`/…) remain (E6d-3, needs the
  allowlist/lowering extension). See [`lang-full-e6-dispatch.md`](lang-full-e6-dispatch.md).
- ✅ **TW4** — typed E4 strings on code-gen backends. Direct literals,
  immutable top-level string value defines, and lexical `let`/`let*` string
  locals lower to shared `str_const`/`str_len`/`str_index`/`str_slice`/`str_eq`/`str_cmp`/`str_concat`
  ops instead of the dynamic `call_builtin` path. **Verified by running** literal
  `string-length`/`string-ref`/`string=?`/`string-append`, named string
  concat/equality/index, the `str_index` out-of-bounds trap, local string index,
  `let*` string length, local string equality branches, and local string concat
  plus local concat, `substring`, computed `string-length` indexes feeding string
  indexing, lexical `string<?`/`string>?` ordering, function-call return typing,
  annotated parameter typing, and conservative direct-call-inferred unannotated
  parameter typing from literal, static-expression, named top-level, lexical,
  derived sequential `let*`, and multi-parameter actuals across
  native/LLVM/WASM/JVM/CLR/VM/JIT
  (`lang_matrix.rs`). **Limits:** captured or reassigned strings and the
  dynamic-`any` string path still need **E6**/dynamic representation work.
- ☐ **TW5** — closures / lambdas / general `call_builtin` on code-gen backends (needs **E6**).
- ☐ **TW6** — `match` / records / unions on code-gen backends (needs **E5**/**E6**).

### McCarthy Lisp  (reference — mostly complete)
- ☐ **MC1** — cons / symbols on the remaining backends (continues existing L3b work; see
  `project_mccarthy_l3_backend_map`).

---

## Suggested global ordering

1. **E4 dynamic string follow-ups** — continue beyond the typed immutable
   scalar/local foothold into captured/reassigned strings, arrays/input,
   unobserved/conflicting or closure-derived parameters,
   and fuller byte-string representations without per-frontend shortcuts.
2. **E6 dynamic/global value model** — unblock the remaining Twig list/closure/record
   work and any frontend code that still needs shared state across functions.
4. The hard tails: **AL7 call-by-name**, **O4 8008 intrinsics**,
   and **MC1 cons/symbol values on the code-gen backends** — explicit user decision points.

This roadmap is the contract; each ☐ becomes a `feat(lang-full): …` PR, checked off here as
it merges.
