# LANG-FULL-IMPLEMENTATION — every matrix language, fully implemented and *run* on every backend

## Why this campaign exists

The LANG-PLATFORM matrix (`LANG-PLATFORM-MATRIX.md`) proved the **plumbing**: six
frontends → shared `interpreter_ir::IIRModule` → seven backends → a runnable artifact.
But an honest audit (2026-06-13) found the green checkmarks rest on **one executed
program per language**, and each frontend is a **deliberate subset**:

| Language | What the matrix actually runs end-to-end on the code-gen backends | The subset gap |
|---|---|---|
| Twig | `42` | rich Lisp frontend, but only typed int-arith/`if` clears the backend validators; lists/lambdas/strings/`print`/symbols need the VM only |
| Nib | `double(21)` → 42 | no `*` `/`, no `for`, no bitwise, no `&&`/`||`, no `const`/`static`; u4/u8 collapse to i64 (no wrap) |
| Brainfuck | one 1-loop "print A" | all 8 ops are correct **but cat/Hello-World/nested-multiply run only on the VM/JIT**, never on the code-gen backends |
| Dartmouth BASIC | `PRINT 42` | integer-only: no `GOSUB`, strings, `^`; has `FOR`/`NEXT`, `IF`/`GOTO`, `DEF FN` (BA5), `DIM` arrays (BA3), `READ`/`DATA`/`RESTORE` (BA6) — all run on every backend |
| Oct | `let`/`if` | rejects **all 10 Intel-8008 intrinsics** (its raison d'être); `&&`/`||` short-circuit ✅ (O1), u8 wrap + `~` ✅ (O2), `static` module globals ✅ (O3); intrinsics remain |
| ALGOL 60 | `result := 17 mod 5` → 2 | `integer`/`real`/`boolean` scalars, typed procedures, switches, 1-D arrays, `own` static-lifetime variables ✅ (AL6, all 7 backends), `abs`/`sign`/`entier` standard functions ✅ (AL8 + E8, all 7 backends); arrays + reals run on VM/JIT only so far; no call-by-name, strings, multidim arrays |

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
- **E2 — Integer width & wrap semantics.** ◑ *In progress (approach B — each backend
  masks narrow-typed arithmetic by `type_hint`, mirroring the byte-tape precedent).* Model
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
    - ◑ **Stack-backend rework (compute-wide + mask).** Wiring Nib to emit narrow `type_hint`s
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
- **E4 — Strings.** ◑ *Design pass complete — see
  **[`lang-full-e4-strings.md`](lang-full-e4-strings.md)**; implementation gated on
  sign-off.* An IIR string value model (six ops: `str_const`, `str_len`, `str_index`,
  `str_concat`, `str_eq`, `print_str`) + per-backend support, lowered to all 7 backends and
  verified by RUNNING (observable via **stdout**). A v1 string is an **immutable,
  length-counted byte buffer** — it reuses the E5 array substrate (length-prefixed flat
  buffer on the static backends; native `String` / managed `(array i8)` on the managed
  backends), so E4 is the *byte-aggregate sibling of E5*, not a new allocator. The one new
  host primitive is `__print_str`/`printStr` (the string sibling of `print_i64`). Unlocks
  BASIC strings + string `PRINT` (BA4), ALGOL strings/I-O (AL4), Twig strings (TW4).
- **E5 — Arrays / linear aggregates.** ✅ **COMPLETE** *(PR-1..4c — runs on all 7 backends:
  VM, JIT, JVM, CLR, LLVM, WASM, native x86_64+aarch64).* An IIR
  array model (`alloc_array`/`array_len`/`array_get`/`array_set`, `array<T>` type hint,
  bounds-checked) that is **representation-agnostic** so it lowers to BOTH static-allocation
  (length-prefixed flat memory + explicit guard/trap on the native + LLVM backends, reusing the
  byte-tape allocator) AND garbage-collected targets (native managed arrays with native bounds
  checks on JVM/CLR/WasmGC). Bounds-checked from the start (OOB → trap). Full design + PR
  breakdown in **[`lang-full-e5-arrays.md`](lang-full-e5-arrays.md)**. Unlocks ALGOL arrays (AL2),
  BASIC `DIM` (BA3), Twig lists (TW3).
- **E6 — General `call_builtin` / closures / dynamic dispatch on code-gen backends.** ⚠
  Today the IIR-to-{wasm,jvm,clr,llvm} validators reject `call_builtin`/`type_hint="any"`,
  which is why most of Twig only runs on the VM. Closing this is the biggest single unlock
  for Twig (and McCarthy cons/symbols). **Architectural fork — design pass first.**
  - **E6 layer 1 (typed module globals) — spec [`lang-full-e6-globals.md`](lang-full-e6-globals.md).**
    The tractable, run-verifiable first slice: a typed `i64` module global a *function*
    can read/write, on all 7 backends. `global_load`/`global_store` already work on
    BEAM/WASM/native; the work is LLVM/JVM/CLR (the `LANG32b` rejections) + an ALGOL
    enclosing-scope-variable frontend + a matrix proof. Unblocks AL6 (`own`), O3 (Oct
    globals); foundation for closures. The general `any`-dispatch / closure layers
    stack on top.
- **E7 — Subroutine / return-stack.** `GOSUB`/`RETURN` and procedure call/return.
  ◑ *Design spec written, pending user sign-off*
  ([`lang-full-e7-subroutine-return-stack.md`](lang-full-e7-subroutine-return-stack.md)).
  **Confirmed:** structured procedure call/return is already done (`call`/`ret` —
  ALGOL AL3, BASIC `DEF FN` BA5). BASIC `GOSUB`/`RETURN` is *unstructured* (the same
  `RETURN` resumes at the dynamically most-recent `GOSUB`) and `call`/`ret` cannot
  express it — but it needs **no new backend op**: lower it inside `main` as an E5
  `array<i64>` return-PC stack + the AL5 computed-goto chain (`cmp`+`jmp_if_false`+`jmp`),
  both already proven on all 7 backends. Pure frontend lowering (BA6-sized), one PR;
  unblocks **BA1**.
- **E8 — Numeric conversions (`integer` ↔ `real`).** ◑ *Spec signed off
  ([`lang-full-e8-numeric-conversions.md`](lang-full-e8-numeric-conversions.md)); implementation in progress.*
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
  0.12.0's `not` op. (Logical `!` still passthrough — boolean lowering is a separate item.)
- ✅ **N4** — `&&` / `||` short-circuit. `compile_short_circuit` lowers to a result slot
  guarded by `jmp_if_false`/`jmp`/`label` (portable subset — CLR textual path has no
  `jmp_if_true`); verified by RUNNING divide-by-zero short-circuit proofs (`1==2 && 84/0==0`
  →7, `1==1 || 84/0==0`→7, RHS never evaluated) + a true-path program, across all backends.
- ✅ **N5** — `const` declarations. Module-scoped integer-literal consts are collected and
  folded to their literal at each use (no runtime storage, runs everywhere); verified by
  RUNNING `const N: u8 = 42; … return N`→42 and `const A=30; const B=12; … A + B`→42 across
  all backends. (Const-*expression* folding and mutable `static` deferred.)
- ☐ **N6** — u4/u8 wrap semantics (needs **E2**).
- ✅ **N7** — `+%` (wrap add) / `+?` (saturating add) (nib-iir-compiler 0.15.0). `+%` lowers to
  the narrow-typed `add` (E2 wraps it: `15u4 +% 1 = 0`, `200u8 +% 100 = 44`); `+?` lowers to a
  *wide* add + a clamp branch `min(sum, MAX)` (`15u4 +? 1 = 15`, `200u8 +? 100 = 255`,
  `3 +? 4 = 7`). Verified by RUNNING on all 7 backends (comparison-based matrix proofs) + a
  vm-core unit test. The grammar already had the `WRAP_ADD`/`SAT_ADD` tokens; no type-checker
  change (additive operators are type-inferred from operands).

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
  `i2l; land` (the int `iand` was unverifiable over longs → empty output). (Logical `!` still
  deferred — a separate item; only `~` is in O2.)
- ✅ **O3** — `static` module globals (LANG-FULL O3). Top-level `static` was silently
  dropped at IIR-gen; `oct-iir-compiler` 0.8.0 lowers it to the IIR module-global ops
  (`global_load`/`global_store`, the E6 substrate). A `static counter: u8 = 40` shared
  across functions — `bump()` (a separate fn) increments it twice, `main` prints it —
  runs on **all 7 backends** → `42` (a per-function register would print `40`). Surfaced
  + fixed two latent *void-function* gaps the proof's void `bump()` exposed: the Oct
  frontend now emits a dest-less IIR `call` for a void callee (a named void call is
  malformed LLVM), and `iir-to-cil-bytecode` 0.25.0's textual emitter lowers `ret_void`
  → bare `ret`, a `void` return signature, and a `call void …` with no trailing store.
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
- ◑ **BA1** — `GOSUB` / `RETURN` (enabler **E7**). `dartmouth-basic-iir-compiler` 0.10.0
  lowers unstructured `GOSUB`/`RETURN` *inside* `main` per the E7 spec: a fixed-capacity
  `array<i64>` return-address stack + an AL5 computed-`goto` (`cmp_eq`+`jmp_if_true`)
  dispatch — **no new backend op**. The same `RETURN` resumes at the dynamically
  most-recent `GOSUB`. Proven by two executed matrix programs (`919` = one `RETURN`,
  two call sites; `876` = nested LIFO) on **six** backends (native/LLVM/JVM/CLR/VM/JIT).
  **BA1-WASM gap:** the computed-`goto` is an irreducible CFG that trips iir-to-wasm's
  dispatch-loop with a runtime `StackUnderflow` (compiles, traps) — a focused
  iir-to-wasm follow-up; the 7th backend lands with that fix.
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
  (grammar + `extract_relop_op` cover all six `= < > <= >= <>`). Deferred: string `PRINT`
  items (→ BA4/E4); true 14-column `,` print zones (a single space approximates them).
- ✅ **BA3** — arrays / `DIM` (enabler **E5**). `DIM A(n)` lowers to `alloc_array`
  (BASIC arrays are 0-based + inclusive, so `n + 1` elements); `LET A(i) = e` →
  `array_set` and `A(i)` rvalues → `array_get`, with the subscript used directly as
  the 0-based index (no lower-bound subtraction, unlike ALGOL `[lo:hi]`). These are
  the same shared array ops ALGOL's E5 arrays use, so BASIC arrays RUN on all 7
  backends — verified by a straight-line array program (`DIM A(3); A(1)=40; A(2)=2;
  PRINT A(1)+A(2)` ⇒ `42`) in `lang-aot/tests/lang_matrix.rs`. (`dartmouth-basic-iir-compiler`
  0.7.0.) Undeclared subscript use is a clean `Unsupported` error.
- ☐ **BA4** — strings + string PRINT (needs **E4**).
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
- ✅ **BA6** — `READ` / `DATA` / `RESTORE` (`dartmouth-basic-iir-compiler` 0.8.0).
  Lowers onto the **E5 array** substrate — no new IIR op, no enabler: a pre-pass
  gathers all `DATA` integer literals (line order) into a pool materialised once at
  the top of `main` as an `array<i64>` + a `__basic_data_ptr` register (a register,
  not a global, since the program is one `main` function). `READ` does `array_get
  pool, ptr` + `ptr := ptr + 1`; `RESTORE` resets `ptr := 0`; out-of-DATA traps via
  the bounds-checked `array_get`. **Runs on all 7 backends**: `DATA 21 / READ A /
  RESTORE / READ B / PRINT A+B` ⇒ 42 (proves sequential consumption + rewind).
  Integer DATA only (real DATA = follow-up).
- ☐ **BA7** — floating-point (needs **E3**, ✅). ◑ *Design spec **decision-complete***
  ([`lang-full-ba7-floating-point.md`](lang-full-ba7-floating-point.md)) — §7 resolved
  by historical Dartmouth BASIC fidelity (no sign-off gate). Cutover of BASIC's value
  model from the V1 i64-truncation to **`f64` end-to-end** (real Dartmouth semantics —
  every number is floating-point). Builds on E3 (f64 on all backends), E8
  (`real_to_int_trunc` for array subscripts), and BA2 (the `putchar` digit substrate —
  adds `__basic_print_real`). **No new backend op.** Slices: BA7-1 (value model +
  arithmetic + whole-valued PRINT) → BA7-2 (fractional formatting + `E` notation,
  6 significant digits, no leading zero) → BA7-3 (comparisons/FOR/arrays/DATA reals).

### ALGOL 60
- ✅ **AL1** — real arithmetic + `/` (algol-iir-compiler 0.4.0): `real` → IIR `f64`, `REAL_LIT`
  → `Operand::Float`, `+`/`-`/`*`/unary-minus over reals emit the `f64` hint, `/` is real
  division, real comparisons compare at `f64` width; `div`/`mod` stay integer-only; no implicit
  int→real coercion (mixing is a clean error). **Verified by RUNNING on ALL 7 backends**
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
  guard-type fix landed in `algol-iir-compiler` 0.5.1). Multidim + array params + `f64` native
  elements are follow-up.
- ✅ **AL3** — typed procedures with value parameters. `integer procedure sq(x);
  value x; integer x; sq := x*x; result := sq(7)` ⇒ exit 49, **verified by running**
  across native/LLVM/WASM/JVM/CLR/VM/JIT (`lang-aot` `lang_matrix.rs`). Lowered to a
  sibling `IIRFunction` + IIR `call`; supports forward references + recursion + multi-arg.
  Surfaced & fixed a real `jit-core` constant-propagation bug (reassigned result slot
  propagated its dead seed → only the JIT returned 0). **Limits (follow-ups):** typed
  procedures only — proper (void) procedures rejected (inert on this slice); bodies are
  lexically flat (no enclosing-scope access yet); `value` params only (by-name is AL7).
- ☐ **AL4** — strings + `print`/`output` I/O (needs **E4**).
- ✅ **AL5** — switches (computed goto) + conditional designational expressions.
  `switch s := a1,a2,a3; … goto s[3]` ⇒ exit 49, **verified by running** across
  native/LLVM/WASM/JVM/CLR/VM/JIT (`lang_matrix.rs`). `goto s[i]` lowers to a 1-based
  `index == k ? jmp Lk` chain; `goto if b then L1 else L2` lowers via the portable
  branch subset. **Surfaced & fixed a latent ALGOL cmp bug**: comparisons emitted a
  `bool` type_hint, so LLVM compared `i64` operands at 1-bit `i1` and emitted invalid IR
  (the cell *failed to run*) — fixed to emit the i64 operand width (the BA0 fix). This was
  the first ALGOL comparison ever exercised on a code-gen backend. **Limits:** switch-list
  elements must be plain labels (no conditional/nested elements); switches aren't
  block-scope-shadowable.
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
- ◑ **AL8** — standard functions (§3.2.4). **`abs` ✅** (algol-iir-compiler 0.8.0)
  and **`sign` ✅** (0.9.0): both built-in, resolved by name (overridable by a user
  `procedure`), lowered inline to compares + `jmp_if_false` + `mov`-into-one-slot
  (store-per-branch, no phi). `abs(E)` = `if E<0 then -E else E` (preserves
  `integer`/`real`); `sign(E)` = `if E>0 then 1 else if E<0 then -1 else 0` (always
  `integer`). Verified by RUNNING `abs(0-42)`⇒42 and `43+sign(0-1)`⇒42 on
  native/LLVM/WASM/JVM/CLR/VM/JIT. **Remaining:** `entier` (floor of a real → integer:
  needs a float-floor+convert, not a portable IIR op — closer to the transcendentals
  than to abs/sign), then `sqrt`/`sin`/`cos`/`ln`/`exp` (need a cross-backend runtime
  math library).

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
- ☐ **TW3** — list / cons ops on code-gen backends (needs **E5**/**E6**).
- ☐ **TW4** — strings on code-gen backends (needs **E4**).
- ☐ **TW5** — closures / lambdas / general `call_builtin` on code-gen backends (needs **E6**).
- ☐ **TW6** — `match` / records / unions on code-gen backends (needs **E5**/**E6**).

### McCarthy Lisp  (reference — mostly complete)
- ☐ **MC1** — cons / symbols on the remaining backends (continues existing L3b work; see
  `project_mccarthy_l3_backend_map`).

---

## Suggested global ordering

1. **Nib N1–N5** and **Oct O1, O3** — pure frontend wins, lower to existing IIR, immediate
   cross-backend execution. Builds the executed-battery habit cheaply.
2. **Brainfuck B1** and **BASIC BA0** — convert the biggest existing smoke-test gaps
   (real programs that today run only on the VM) into real cross-backend execution.
3. **Enabler E2** (int wrap) → unblocks Nib N6/N7, Oct O2.
4. **Enabler E3** (reals) → unblocks ALGOL AL1/AL8, BASIC BA7.
5. **Enablers E5 (arrays), E4 (strings), E6 (dynamic dispatch)** — the deep, fork-bearing
   work; each gets a design pass and a user check, then unblocks the array/string/Twig items.
6. The hard tails: **AL7 call-by-name**, **O4 8008 intrinsics** — explicit user decision points.

This roadmap is the contract; each ☐ becomes a `feat(lang-full): …` PR, checked off here as
it merges.
