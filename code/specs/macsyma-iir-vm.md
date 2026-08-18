# Macsyma → IIR → VM interpreter

**Status:** Draft — 2026-08-14 (spec-first; sign-off = merge)
**Depends on:** none (additive — no change to `interpreter-ir`, `dynval-runtime`,
or `macsyma-parser`/`macsyma-runtime`).
**Unblocks:** a future native-codegen wave for Macsyma (LLVM/NativeAOT/WASM/
JVM/CLR) and the same IIR-bridge pattern for the rest of the SIR23 CAS family
(Wolfram, Maxima, Derive, Reduce, Maple, Axiom).

## 0. One-paragraph summary

This repo has two compiler pipelines that have never been bridged. The first,
**Semantic IR (SIR)**, is source-to-source translation: `macsyma-to-semantic-ir`
already lowers Macsyma to [SIR23](SIR23-symbolic-pattern-semantic-ir.md) for
JS/TS/Rust/Go/Python/Ruby/C codegen — no native execution, everything
ultimately runs as transpiled JS via `node`. The second, **interpreter_ir
(IIR)**, is the [AOT00](AOT00-native-aot-robustness-roadmap.md) chain: a
shared IR lowered to 7 real backends (NativeAOT arm64/x86_64, LLVM, WASM,
JVM, CLR, VM interpreter, JIT). Today the only frontends onto IIR are
`twig-ir-compiler` and `mccarthy-lisp-iir-compiler` — no math language has
ever touched it. This spec adds a **third, independent** lowering off the
same Macsyma CST (alongside the existing `macsyma-runtime` REPL interpreter
and the `macsyma-to-semantic-ir` SIR frontend): `macsyma-iir-compiler`, which
lowers a scoped v0 subset (literal arithmetic, assignment, unevaluated
symbolic expressions) to `IIRModule`, executed by a new `macsyma-vm` — a tiny
interpreter built directly on `dynval-runtime`, the same foundation
`mccarthy-lisp-vm` already proves works for exactly this shape of problem
(a symbolic, S-expression-as-data language).

```text
                                 ┌─→ macsyma-runtime (+ repl)                [existing, unchanged]
macsyma source → GrammarASTNode ┤─→ macsyma-to-semantic-ir → any SIR backend [existing, unchanged]
                                 └─→ macsyma-iir-compiler → IIRModule → macsyma-vm   [NEW]
```

## 1. Why this is additive, not a rewrite

`macsyma-to-semantic-ir/src/lower.rs`'s own module doc describes itself as
"retargeting `macsyma-compiler`, not starting from scratch": `macsyma-compiler`
already walks Macsyma's `GrammarASTNode` CST and compiles it to
`symbolic_ir::IRNode` (`Symbol`/`Integer`/`Rational`/`Float`/`Str`/`Apply`);
`macsyma-to-semantic-ir` mechanically retargets the *same* rule-name dispatch
(`match node.rule_name.as_str() { "assign" => ..., "additive"|"multiplicative"
=> ..., "postfix" => ..., ... }`) to build `semantic_ir::Expr::{SymSymbol,
SymApply}` instead. `macsyma-iir-compiler` is a **third retarget of the same
dispatch table**, this time emitting `interpreter_ir::IIRModule` instructions.
It shares no code with `macsyma-runtime` or `macsyma-to-semantic-ir` beyond
the parser; all three are independent consumers of one CST, exactly as
`python-to-semantic-ir` and CPython's own interpreter both consume Python
source without depending on each other (the same reuse argument HML01 §1
makes for the SIR frontends).

## 2. Precedent: `mccarthy-lisp-iir-compiler` + `mccarthy-lisp-vm`

Verified in code, not assumed. `mccarthy-lisp-iir-compiler` already solves
this exact shape of problem: a symbolic language (McCarthy 1960 Lisp) whose
values are S-expressions lowers to `IIRModule` on top of `dynval-runtime`'s
tagged `LispyValue` substrate, and runs on `mccarthy-lisp-vm`, "a tiny
interpreter for McCarthy 1960 Lisp... built directly on `dynval-runtime`...
deliberately independent of `twig-vm`... both languages share only the
`dynval-runtime` foundation" (`mccarthy-lisp-vm/Cargo.toml` description).
`macsyma-iir-compiler`/`macsyma-vm` follow the identical shape: their own
crate, sharing only `dynval-runtime`, not `twig-vm` or `mccarthy-lisp-vm`.

The specific mechanism this spec reuses is `lower_quote`
(`mccarthy-lisp-iir-compiler/src/lib.rs:888`), which materialises an inert
S-expression as a `cons`-chain of runtime values (`'(A B C)` →
`(CONS 'A (CONS 'B (CONS 'C ())))`). An unevaluated Macsyma `Apply(head,
args)` node is exactly this shape — `(head arg0 arg1 …)` — so representing
one needs no new IIR opcode: it is `call_builtin "cons"` applied the same
way `lower_quote` already does.

## 3. Value representation (the load-bearing design decision)

Two facts, verified directly against `interpreter-ir`/`dynval-runtime`
source and the [DVAL01](DVAL01-generic-dynamic-value-substrate.md) spec,
that shape everything below:

- **`dynval-runtime`'s tagged value model has no float or rational tag
  today.** `DVAL01` §1.2 lists "New tag kinds / float boxing... NaN-boxed
  doubles are a later layer" as an explicit non-goal, and `mccarthy-lisp-vm`
  itself traps a float operand as unsupported. This means v0 cannot
  represent `Rational`/`Float` at all — not a scope choice, a substrate gap.
  v0 is **integer-only**, matching `mccarthy-lisp-iir-compiler`'s own
  precedent scope exactly.
- **`LispyBinding::resolve_builtin`** (`dynval-runtime/src/binding.rs:314`)
  already resolves `"+"`, `"-"`, `"*"`, `"/"`, `"="`, `"<"`, `">"`, `"cons"`,
  `"car"`, `"cdr"` to real `builtins::*` functions, using bare, unprefixed
  names — this is how the VM-interpreter path works; the DVAL01 `dyn_*`
  rename and `iir-builtin-lowering`'s type-directed boxing only matter for
  the native/structural codegen backends (Wave 4, §6), which v0 does not
  touch. **Zero new runtime Rust code is needed for v0's arithmetic.**

| Macsyma construct | IIR / VM representation |
|---|---|
| `Integer(n)` | `const %v, Int(n) : "i64"` — read directly as `LispyValue::int(n)` |
| `Symbol`, bound (assigned earlier in program order) | substitute the register already holding its value |
| `Symbol`, free/unbound | `const %v, Var(name) : "symbol"` — interned tagged symbol |
| `x: expr` (assignment) | lower `expr`; bind `name → register` in the lowering environment; the statement's own value is `expr`'s value (matches the REPL echo) |
| `Add`/`Sub`/`Mul`, every operand already concrete | `call_builtin "+"/"-"/"*",  [a, b]` — genuinely executed by the VM at run time, proving real opcode execution, not frontend-folded |
| `Add`/`Sub`/`Mul`, any operand symbolic | inert cons-chain via `call_builtin "cons"`, mirroring `lower_quote` (§2) — matches `macsyma-runtime` leaving e.g. `x+y` unevaluated |
| `Div`, both operands **direct literal integers** dividing evenly | `call_builtin "/", [a, b]` — the narrower exactness rule below |
| `Div`, any operand symbolic | inert cons-chain (matches `macsyma-runtime` leaving `x/y` unevaluated) |
| `Div`, any other shape (non-exact literal division, or a concrete-but-non-literal operand on either side) | **rejected — explicit `MacsymaIirError`** (neither evaluating nor building inert data is verifiably correct — see below) |
| `Neg`, operand concrete | `call_builtin "-", [a]` (unary) |
| `Neg`, operand symbolic | inert cons-chain |
| `Pow`, comparisons (`=`/`#`/`<`/`>`/`<=`/`>=`), `and`/`or`/`not`, a user call `f(x)` | **rejected outright — explicit `MacsymaIirError`**, no inert-data fallback (see below) |
| `Rational`, `Float`, `Str` | **rejected — explicit `MacsymaIirError`** (substrate gap, not scope choice) |
| `if`/`elseif`/`else`, `while`, `for`, `block(...)`, `return(...)`, `[...]` (list), `:=` (function def) | **rejected — explicit `MacsymaIirError`, "construct not supported in v0"** |

**Why `Pow`/comparisons/`and`/`or`/`not`/calls are rejected outright, unlike
`Add`/`Sub`/`Mul`/`Div`:** `symbolic-vm`'s handler table — the real evaluator
`macsyma-runtime` runs — has genuine handlers for `Pow`/comparison/logical
heads (`pow_handler`, the `LESS`/`GREATER`/… handlers, `and_handler`,
`or_handler`) that *numerically evaluate* a concrete pair of operands
(`2^3` → `8`, `3<5` → `true`), unlike leaving them symbolic. Building inert
data for a *concrete* instance of one of these would therefore silently
disagree with `macsyma-runtime`'s own ground truth — the same mis-lowering
risk control-flow forms have (below), not a mere scope gap. `Add`/`Sub`/`Mul`
never have this hazard (integer arithmetic is always exact, so their real
handlers and "stay symbolic when not fully concrete" behavior coincide
exactly with the inert-data fallback). `Div` has a narrower version of the
same hazard — Macsyma's exact `Rational` result for a non-exact division
isn't representable in v0 at all — resolved by the literal-only exactness
rule above rather than blanket rejection, since the common case (`20/4`)
would otherwise be needlessly unavailable.

**Why control-flow forms are rejected outright, not built as inert data like
Apply nodes:** `macsyma-to-semantic-ir/src/lower.rs`'s own module doc
explains `if`/`while`/`for`/`block` lower to plain `Apply` data *because*
`macsyma-runtime` genuinely interprets that data as live, side-effecting
control flow at evaluation time — a tree-walking interpreter over `IRNode`,
the same way a Lisp interpreter treats `(if ...)` specially even though it
is itself a list. If v0's IIR lowering built the identical inert
`Apply(If, ...)` s-expression without an interpreter loop that actually
branches on it, running the program would silently do nothing resembling
`macsyma-runtime`'s behavior — a real mis-lowering, not a disclosed cut.
Rejecting these constructs outright (mirroring `matlab-to-semantic-ir`
v0.1.0's "each excluded construct returns an explicit error rather than
being silently mis-lowered") is the honest choice; a real control-flow
evaluator loop in `macsyma-vm` is Wave 3 (§6).

**Division landmine, disclosed up front:** Macsyma's `/` on two integers
that don't divide evenly returns an exact `Rational` (`7/2` stays `7/2`),
never a float or truncated int. Because v0 has no `Rational`, the oracle
corpus (§5) restricts evaluated-division cases to exact results (`20/4`);
`7/2` is a dedicated unit test asserting `compile_source` returns
`Err(MacsymaIirError::Unsupported(Rational))`, not a diffed corpus entry.

## 4. Scope (v0) — accepted grammar subset

Of Macsyma's 24 grammar rules (`code/grammars/macsyma/macsyma.grammar`, the
same set `macsyma-to-semantic-ir` covers in full):

**Accepted:** literal integers; `+ - * /` (binary chains and unary
`-`/`+`); assignment (`x: expr`); free-symbol references; `+`/`-`/`*`
(and `/`, within its exactness rule) applied to any symbolic operand —
represented as inert data, never evaluated (e.g. `x+y`, `2*x` where `x`,
`y` are free).

**Rejected, with an explicit error, in every shape (concrete or
symbolic):** `Rational`/`Float`/`Str` literals; `:=` (function
definition); `if`/`elseif`/`else`; `while`; `for ... in ... do` / `for
... thru/while/unless ... do`; `block(...)`; `return(...)`; `[...]`
(list literals); `^`/`**` (power); comparisons (`= # < > <= >=`);
`and`/`or`/`not`; any postfix function call `f(x)`. Power, comparisons,
and logical operators are rejected even when every operand is a free
symbol (e.g. `x^y`, `x and y`) — unlike `+`/`-`/`*`/`/`, they get no
inert-data fallback at all, since `macsyma-runtime`'s real handlers for
these heads evaluate a *concrete* instance rather than leaving it
symbolic, and a per-operand-shape carve-out would be more complexity than
this v0 slice warrants (§3 explains the underlying hazard). A function
call is rejected outright too: applying an unknown head requires either
evaluation semantics v0 doesn't have, or a bound-function lookup v0
doesn't have either.

This mirrors the disclosed-subset convention every `<lang>-to-semantic-ir`
v0.1.0 already follows (HML01 §3's `matlab-to-semantic-ir` precedent:
"each excluded construct returns an explicit error rather than being
silently mis-lowered").

## 5. Crate structure

### `code/packages/rust/macsyma-iir-compiler`

```rust
pub fn compile(tree: &GrammarASTNode, module_name: &str) -> Result<IIRModule, MacsymaIirError>;
pub fn compile_source(source: &str, module_name: &str) -> Result<IIRModule, MacsymaIirError>;
```

`lower.rs`'s rule-name dispatch is structurally copied from
`macsyma-to-semantic-ir/src/lower.rs` — same `unwrap_single`/`child_nodes`/
`as_token` CST-walking helpers and the same `MAX_EXPR_DEPTH = 256`
recursion guard plus `check_chain_length` (the flat additive/multiplicative
chain guard). v0's much narrower accepted grammar doesn't reach
`macsyma-to-semantic-ir`'s other three guards — `check_postfix_chain_length`
guards chained calls, `check_if_chain_length` guards an `elseif` chain, and
`check_apply_arg_count` guards an arglist, and all three constructs are
rejected outright in v0 (§4), so there is no path that builds the deep
structure those guards defend against. The same is true of the iterative
`measure_depth_iterative`/`drop_iterative` pair: that machinery exists in
`macsyma-to-semantic-ir` because it builds a separate `Expr` tree that a
*later* pass walks again, recursively, across nested `(...)` boundaries the
per-node guards don't compose across; `macsyma-iir-compiler` instead emits
flat `IIRInstr`s directly during the one CST-walking recursion `MAX_EXPR_DEPTH`
already bounds, so there is no second recursive walk to guard — the same
reasoning `mccarthy-lisp-iir-compiler` (which also has no iterative
measure/drop pass) already establishes for this crate shape. Every non-v0
rule arm returns `Err` instead of lowering.

Dependencies: `coding-adventures-macsyma-parser`, `interpreter-ir`, `parser`,
`lexer`, `symbolic-ir` (for the canonical `Add`/`Sub`/`Mul`/`Div`/`Neg` head
name constants, keeping this frontend in lockstep with
`macsyma-to-semantic-ir`). Dev-dependencies: `macsyma-vm`, `dynval-runtime`
— enough for this crate's own unit tests (every accepted construct
round-tripped through `macsyma-vm`, every rejected construct's explicit-error
path). `coding-adventures-macsyma-runtime` and `cas-pretty-printer` (oracle
ground truth + text diff) are added later, in the oracle-test PR (§8 PR 4)
— pulling them in here would be dead weight until that harness exists.

### `code/packages/rust/macsyma-vm`

```rust
pub fn run(module: &IIRModule) -> Result<LispyValue, VmError>;
```

"A tiny interpreter for the Macsyma v0 arithmetic/assignment IIR subset,
built directly on `dynval-runtime`" — mirroring `mccarthy-lisp-vm`'s own
description, "deliberately independent" of `twig-vm`/`mccarthy-lisp-vm`
(sharing only the `dynval-runtime` foundation). The dispatch loop only needs
`const`/`call_builtin`/`ret` (v0 has no branches or closures) — trimmed
further than `mccarthy-lisp-vm`'s own loop. Every `call_builtin` dispatches
through a small local `resolve_builtin` match table calling
`dynval_runtime::builtins::{add,sub,mul,div,cons,car,cdr}` directly —
mirroring `mccarthy-lisp-vm`'s own `resolve_builtin`, which likewise hand-
rolls a per-language table over the shared `builtins::*` functions rather
than going through the generic `LangBinding`/`LispyBinding` trait (that
trait's `resolve_builtin` exists for the native-codegen backends'
type-directed dispatch, per `dynval-runtime/src/binding.rs`, and no
existing VM interpreter — Twig's or McCarthy Lisp's — uses it). No new
runtime logic either way; only an `Operand → LispyValue` reader copied from
`mccarthy-lisp-vm`'s `read_operand`/`konst_value`.

Dependencies: `interpreter-ir`, `dynval-runtime`, `lang-runtime-core` (for
`RuntimeError`) — identical to `mccarthy-lisp-vm/Cargo.toml`.

Both crates get `BUILD`/`README.md`/`CHANGELOG.md` and workspace
registration in `code/packages/rust/Cargo.toml`'s `members` list, per repo
convention (CLAUDE.md rules #7/#12/#13).

## 6. Rollout waves (named here, designed when they start)

- **Wave 1 (this spec).** VM interpreter only, integer-only value model,
  arithmetic/assignment/unevaluated-Apply subset.
- **Wave 2.** Extend value representation to `Rational`/`Float` — requires a
  `dynval-runtime` substrate change (a new heap-boxed float/rational kind),
  explicitly flagged as needing its own sign-off since it is DVAL01-governed
  shared infrastructure other languages (Twig, McCarthy Lisp, future
  Python/Ruby/JS frontends) also ride, not something this crate pair owns.
- **Wave 3.** Control-flow constructs (`if`/`while`/`for`/`block`) — needs a
  real evaluator loop in `macsyma-vm`, not just inert-data construction.
- **Wave 4.** The other 6 IIR backends (NativeAOT arm64/x86_64, LLVM, WASM,
  JVM, CLR) — this is where `iir-builtin-lowering`'s DVAL01 rename/boxing
  passes finally become relevant, since those backends need the
  type-directed treatment this v0 slice never touches.
- **Wave 5.** The other SIR23 CAS languages (Wolfram, Maple, Reduce, Derive,
  Axiom), each getting their own `<lang>-iir-compiler` (per repo convention
  and the user's explicit choice of a dedicated-per-language frontend over a
  generic `semantic-ir-to-iir` backend). Whether they share one generic
  symbolic-VM crate or each get their own `<lang>-vm` is an open question to
  resolve when Wave 5 starts, not decided here.

## 7. Test / oracle plan

Same methodology as every existing `<lang>-to-semantic-ir` oracle file
(`macsyma-to-semantic-ir/tests/oracle.rs`, HML01 §5/§7), adapted for the VM
target instead of `node`:

- **Ground truth:** `MacsymaSession::eval_source(source)` →
  `Vec<EvalResult>` → each `.output_text` (`cas_pretty_printer::pretty(&node,
  &MacsymaDialect)`, no `%oN` decoration in the default path).
- **Compiled side:** `macsyma_iir_compiler::compile_source(source, "oracle")`
  → `IIRModule` → `macsyma_vm::run(&module)` → `LispyValue` → a test-local
  "un-quote" reader (`read_back`, the mirror image of `lower_quote`, walked
  **iteratively**, not recursively, for the same DoS-hardening reason
  `lower_quote`'s own iterative walk exists) → `symbolic_ir::IRNode` →
  the same `cas_pretty_printer::pretty()` call.
- **Corpus (~25-30 cases):** literal arithmetic (chains, precedence, unary
  neg, e.g. `2+3`, `2+3*4`, `-5+3`); assignment + reference including
  reassignment threading (`x: 3$ x: x+1$ x`); free-symbol symbolic results
  (`x+y`, `2*x`, `x-y+z`); exact-division cases only (`20/4`).
- **Explicit-error unit tests** (separate from the diffed corpus, each
  asserting a specific `Err` variant): `1.5`, `7/2`, `x: 6$ x/2` (concrete
  but non-literal division), `x := f(x)`, `if x>0 then 1 else 0`, `while
  x<5 do x`, `[1,2,3]`, `sin(x)` (a function call — always rejected, not
  just when applied, per §4's correction), `x=y`, `x and y`, `x^2`. These
  already exist as `macsyma-iir-compiler`'s own crate-local unit tests
  (shipped in the crate-skeleton/lowering PR, §8 PR 2); the oracle-test PR
  (§8 PR 4) reuses the same case list against the cross-checked harness
  rather than re-deriving it.
- **`known_bug`:** expected empty for v0's corpus — excluded constructs are
  cleanly rejected and unit-tested separately, not silently-mismatching
  diffed entries.

## 8. PR sequencing

1. **Spec only** (this document). No code.
2. **Crate skeletons + working lowering + VM dispatch** — both crates
   created with `BUILD`/README/CHANGELOG, workspace-registered; the full v0
   rule-dispatch table (§3/§4) and `macsyma-vm`'s dispatch loop, with every
   accepted and rejected construct crate-locally unit-tested. Originally
   planned as two PRs (an inert `const`/`ret`-only skeleton, then the real
   dispatch table); consolidated into one once building it showed v0's own
   accepted grammar is already narrow enough that a meaningfully smaller
   "just prove the plumbing" cut didn't exist — the two PRs would have
   differed mostly in the presence of the arithmetic `combine`/`/`-exactness
   logic, not in scale.
3. **Oracle test** — `macsyma-iir-compiler/tests/oracle.rs` per §7, CI green.
4. **Spec-sync follow-up** (CLAUDE.md rule #9) — this document was corrected
   in step 2 itself (not deferred) once implementation surfaced a real
   inconsistency in the original draft: §3's value-representation table and
   §4's scope list disagreed about whether `Pow`/comparisons/`and`/`or`/`not`
   ever get an inert-data fallback. They don't, in any shape — resolved in
   favor of correctness (§3's "why rejected outright" explanation) since
   `macsyma-runtime`'s real handlers evaluate concrete instances of those
   heads, so inert data would have silently disagreed with ground truth.

`/security-review` before pushing PR 2 and PR 3. Before PR 2: confirm
whether `code/scripts/miri-twig-vm.sh`'s scope already covers
`mccarthy-lisp-vm`-shaped crates (`grep -n mccarthy
code/scripts/miri-twig-vm.sh`) — this plan makes zero changes to
`dynval-runtime`/`lang-runtime-core`/`interpreter-ir` themselves (pure new
leaf-crate consumers, exactly like `mccarthy-lisp-vm` before it), so the
existing per-PR blocking Miri CI (scoped to `-p lang-runtime-core -p
dynval-runtime`) should already cover the real UB surface.

## 9. References

Internal: [`AOT00`](AOT00-native-aot-robustness-roadmap.md),
[`DVAL01`](DVAL01-generic-dynamic-value-substrate.md),
[`HML00`](HML00-historical-math-languages-roadmap.md),
[`HML01`](HML01-math-to-semantic-ir.md),
[`SIR23`](SIR23-symbolic-pattern-semantic-ir.md),
[`MCCARTHY-LISP-PLAN`](MCCARTHY-LISP-PLAN.md). Precedent crates:
`mccarthy-lisp-iir-compiler`, `mccarthy-lisp-vm`. Sibling frontends being
retargeted a third time: `macsyma-compiler`, `macsyma-to-semantic-ir`.
