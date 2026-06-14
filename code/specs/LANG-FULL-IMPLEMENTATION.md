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
- **E2 — Integer width & wrap semantics.** Model `u4`/`u8`/`u16`/`u32` wraparound
  (mod-2ⁿ) consistently across all backends, so Nib/Oct arithmetic and bitwise-NOT are
  *correct*, not "collapse to i64." (Brainfuck already proves the u8-tape pattern; this
  generalises it to register values.)
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
  (3×2→6) across native/LLVM/WASM/JVM/CLR/VM/JIT. (Note: reassigning a *parameter* in a
  loop is invalid on the IIR-to-LLVM backend — a separate backend limitation; the for-loop
  idiom uses a `let` local.)
- ☐ **N3** — bitwise `&` `|` `^` (existing IIR `and`/`or`/`xor`) + proper unary `~` (needs E2 for the width mask).
- ☐ **N4** — `&&` / `||` short-circuit (desugar to branches).
- ☐ **N5** — `const` declarations (constant folding / global).
- ☐ **N6** — u4/u8 wrap semantics (needs **E2**).
- ☐ **N7** — `+%` (wrap add) / `+?` (saturating add) (needs **E2**).

### Oct  (sister to Nib)
- ☐ **O1** — `&&` / `||` short-circuit (currently eager bitwise — wrong for side effects).
- ☐ **O2** — proper `~` 8-bit mask + u8 wrap (needs **E2**).
- ☐ **O3** — `static` globals (currently silently dropped).
- ☐ **O4** — ⚠ Intel-8008 intrinsics (`in`/`out`/`adc`/`sbb`/`rlc`/`rrc`/`ral`/`rar`/`carry`/`parity`).
  These are hardware-specific; on general backends they need a host/IIR-builtin model or a
  defined semantics. **Decision point — surface to the user before implementing.**

### Brainfuck  (semantics complete; gap is cross-backend *execution* of real programs)
- ☐ **B1** — Wire `,`/stdin input on each code-gen backend and **execute real programs
  cross-backend**: cat (`,[.,]`), nested-loop multiply, and "Hello World!" — output checked
  on native/LLVM/WASM/JVM/CLR, not just the VM. (Highest-signal: converts the biggest
  smoke-test gap into real coverage with no new language features.)

### Dartmouth BASIC
- ☐ **BA0** — fix the `#[ignore]`d wasm `cmp_le`/`cmp_gt` lowering so FOR/IF actually
  *encode+run* on WASM (currently a known bug). Add executed FOR/IF/GOTO matrix programs.
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
- ☐ **AL3** — procedures with value parameters (call — partly present; verify cross-backend).
- ☐ **AL4** — strings + `print`/`output` I/O (needs **E4**).
- ☐ **AL5** — switches + conditional designational expressions in `goto`.
- ☐ **AL6** — `own` variables (static lifetime).
- ☐ **AL7** — ⚠ call-by-name (Jensen-style expression thunks). **Hardest item in the
  campaign — design pass + user check before implementing.**
- ☐ **AL8** — standard functions (`abs`/`sign`/`entier`/`sqrt`/`sin`/`cos`/… — needs **E3**).

### Twig
- ☐ **TW1** — variadic arithmetic typed lowering (so `(+ a b c)` clears backend validators).
- ☐ **TW2** — top-level value `define` on the code-gen backends.
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
