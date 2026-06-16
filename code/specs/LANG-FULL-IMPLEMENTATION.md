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
| Dartmouth BASIC | `PRINT 42` | integer-only: no `GOSUB`, strings, arrays, `DEF FN`, `READ`/`DATA`, `^`; loops/IF/GOTO execute only on the VM/JIT |
| Oct | `let`/`if` | rejects **all 10 Intel-8008 intrinsics** (its raison d'être), u8 not modeled, `&&`/`||` not short-circuit, `~` knowingly wrong |
| ALGOL 60 | `result := 17 mod 5` → 2 | scalar `integer`/`boolean` only: no arrays, procedures, call-by-name, reals, strings, switches, `own` |

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
  - ◑ **Integration** — wire Nib (then Oct) to emit narrow `type_hint`s for narrow-declared
    values + an executed matrix proof (`200u8+100u8=44`, Nib unary `~`) across all backends;
    flip N3-`~`, Nib N6/N7, Oct. The two code-gen backends that type the *op* (rather than
    masking the *value*) needed real wrap work first — surfaced when the integration was
    started (the roadmap's earlier "LLVM already wraps natively (u8→i8)" assumption was
    never executed and is false: every IIR value rides an i64 slot, so an `add i8 %a,%b`
    over i64 operands is invalid IR):
    - ✅ **iir-to-llvm** (v0.11.0) — a narrow unsigned op computes at i64 then `and i64 …,
      <mask>` (u4/u8/u16/u32). Adds `u4` to the supported types. **Executed proof** on real
      `clang`: `200u8+100u8` → exit `44`. Matches the value-mask of the 5 register backends.
    - ✅ **lang-aot native codegen** (NativeAot, twig-aot v0.15.0) — the IIR prep pass
      `mask_narrow_width_arith` appends `const <mask>; and dest,tmp,mask` after each narrow
      op (before u64 normalisation), so the aarch64/x86_64 backends wrap with **no
      machine-backend change**. **Executed proof**: `200u8+100u8` compiled to real ARM64
      and run in-process → `44`. No-op for i64/u64 programs.
    - ☐ **Nib frontend + matrix proof** — emit narrow `type_hint`s; executed cross-backend
      proof; flip N3-`~`, Nib N6/N7. Then Oct (O2).
- **E3 — Real / floating-point (`f64`).** End-to-end f64 arithmetic, comparison, and
  literals on every backend. Unlocks ALGOL reals and BASIC floats. *(Audit which backends
  already emit f64 ops; extend the rest.)*
- **E4 — Strings.** ⚠ An IIR string value model + core ops (length, concat, index,
  compare, print) with backend support (heap/host). Unlocks BASIC strings, Twig strings,
  ALGOL strings/I-O. **Architectural fork — needs a design pass before implementation.**
- **E5 — Arrays / linear aggregates.** ⚠ An IIR array/aggregate model (alloc, bounds-checked
  index, multidim) on every backend. Unlocks ALGOL arrays, BASIC `DIM`, Twig lists/cons.
  **Architectural fork — design pass first** (relates to Brainfuck's flat-memory byte-tape
  and the existing `alloc_bytes`/`load_byte`/`store_byte`).
- **E6 — General `call_builtin` / closures / dynamic dispatch on code-gen backends.** ⚠
  Today the IIR-to-{wasm,jvm,clr,llvm} validators reject `call_builtin`/`type_hint="any"`,
  which is why most of Twig only runs on the VM. Closing this is the biggest single unlock
  for Twig (and McCarthy cons/symbols). **Architectural fork — design pass first.**
- **E7 — Subroutine / return-stack.** `GOSUB`/`RETURN` and procedure call/return —
  likely expressible with existing `call`/`ret`; confirm and add if needed.

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
- ◑ **N3** — bitwise `&` `|` `^` ✅ (lower to existing IIR `and`/`or`/`xor`; verified by RUNNING
  `12 & 10`→8, `12 | 3`→15, `6 ^ 5`→3 across native/LLVM/WASM/JVM/CLR/VM/JIT — also fixed a CLR
  textual-`.il` gap in `iir-to-cil-bytecode` 0.19.0). Unary `~` ☐ — still deferred; a correct
  width-mask needs enabler **E2** (`~x` on a u8 flips 8 bits, not 64).
- ✅ **N4** — `&&` / `||` short-circuit. `compile_short_circuit` lowers to a result slot
  guarded by `jmp_if_false`/`jmp`/`label` (portable subset — CLR textual path has no
  `jmp_if_true`); verified by RUNNING divide-by-zero short-circuit proofs (`1==2 && 84/0==0`
  →7, `1==1 || 84/0==0`→7, RHS never evaluated) + a true-path program, across all backends.
- ✅ **N5** — `const` declarations. Module-scoped integer-literal consts are collected and
  folded to their literal at each use (no runtime storage, runs everywhere); verified by
  RUNNING `const N: u8 = 42; … return N`→42 and `const A=30; const B=12; … A + B`→42 across
  all backends. (Const-*expression* folding and mutable `static` deferred.)
- ☐ **N6** — u4/u8 wrap semantics (needs **E2**).
- ☐ **N7** — `+%` (wrap add) / `+?` (saturating add) (needs **E2**).

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
  stdout `9` (old eager printed `5`,`9`); `||` analogue → `7`. Across native/LLVM/WASM/CLR/VM/JIT
  (JVM = BA-JVM-1). Also fixed Oct non-void function returns to materialise as `i64` (the
  `side() -> u8` helper exposed `define i8 @side()` mismatching its i64 body on LLVM).
- ☐ **O2** — proper `~` 8-bit mask + u8 wrap (needs **E2**).
- ☐ **O3** — `static` globals (currently silently dropped) — now verifiable via `out`.
- ☐ **O4** — ⚠ Intel-8008 intrinsics (`in`/`out`/`adc`/`sbb`/`rlc`/`rrc`/`ral`/`rar`/`carry`/`parity`).
  These are hardware-specific; on general backends they need a host/IIR-builtin model or a
  defined semantics. **Decision point — surface to the user before implementing.**

### Brainfuck  (semantics complete; gap is cross-backend *execution* of real programs)
- ◑ **B1** — **execute real (non-input) programs cross-backend**, output-checked. ✅ A
  nested-loop multiply program → stdout `"HA"` and a two-sequential-loop program → `"OK"`
  now run on native/LLVM/WASM/JVM/CLR/VM/JIT (`lang_matrix.rs`), proving nested loops +
  multi-cell pointer movement + multiple `putchar`s lower everywhere — not just the trivial
  1-loop "A". ☐ Remaining: `,`/stdin programs (cat `,[.,]`) need per-backend stdin wiring —
  a separate follow-up (B1-stdin); the no-input gap is the higher-signal one and is closed.

### Dartmouth BASIC
- ✅ **BA0** — BASIC control flow on the code-gen backends. The real bug wasn't wasm
  (`iir-to-wasm` already lowers all `cmp_*`; the `#[ignore]`s were stale and are removed) —
  it was the BASIC compiler emitting `cmp_*` with a **`bool`** type hint, so LLVM compared
  at 1-bit `i1` width (`7 > 5` → `1 > 1` → false). Fixed to emit the `i64` operand width
  (like Nib/Oct/ALGOL). Verified by RUNNING `FOR I = 1 TO 5: S = S + I`→`15` and
  `IF A > 5 THEN 100`→`7` across native/LLVM/WASM/CLR/VM/JIT.
- ☐ **BA-JVM-1** — BASIC branch (`IF`/`FOR`) **+ `print_i64`** on the JVM. The two BA0
  control-flow programs are excluded from the JVM cell: `iir-to-jvm-class-file`'s
  StackMapTable generation trips on the frame at a branch target when several `long`
  locals are live across a host-method invoke. (A print with no branch and a loop with no
  print both work on JVM; only the combination fails.) A self-contained `iir-to-jvm` fix.
- ☐ **BA1** — `GOSUB` / `RETURN` (needs **E7**).
- ☐ **BA2** — multi-item `PRINT`, `;`/`,` separators, more relops.
- ☐ **BA3** — arrays / `DIM` (needs **E5**).
- ☐ **BA4** — strings + string PRINT (needs **E4**).
- ☐ **BA5** — `DEF FN`.
- ☐ **BA6** — `READ` / `DATA` / `RESTORE`.
- ☐ **BA7** — floating-point (needs **E3**).

### ALGOL 60
- ☐ **AL1** — real arithmetic + `/` (needs **E3**).
- ☐ **AL2** — arrays with runtime bounds (needs **E5**).
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
- ☐ **AL6** — `own` variables (static lifetime).
- ☐ **AL7** — ⚠ call-by-name (Jensen-style expression thunks). **Hardest item in the
  campaign — design pass + user check before implementing.**
- ☐ **AL8** — standard functions (`abs`/`sign`/`entier`/`sqrt`/`sin`/`cos`/… — needs **E3**).

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
