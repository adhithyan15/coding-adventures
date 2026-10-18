# coding-adventures-closure-pass-constant-fold

## CLOC41 unary Math result lineage

The existing abs/floor/ceil/round/trunc/sign numeric handler now joins
available original-call, folded-member, receiver, property and folded-argument
identities in that order, with first-seen alias deduplication. Five borrowed
stack slots bound frontier storage and comparison count; this is not a bound
on arbitrary ID byte lengths, recursive folding or whole-process memory.
The returned numeric node owns its checked fold contribution; nested child
history and real pass/sweep order survive. Missing IDs are not manufactured.

Admission, numeric bits/raw formatting, labels, negative-zero/result-finite
guards, eager child folding and first-error behavior remain. Native
sign-of-Infinity inputs still yield positive/negative one. Textual Math
matching does not certify a global binding. Actual ingested source intervals,
Unicode/CRLF and repeated spellings, traced/plain JS/map/manifest equality,
checked graph/resource failures and all-artifact refusal sentinels are tested.

Actual Rust execution `fa8b81c2a8c7408ec31114617a7c176f7a5cbffd` completed 3277 native tests/doctests,
zero failures or filtered tests and 41 existing ignored cases,
across 248 metadata-declared executable harness targets and
14 separate doctest targets. Both affected all-target strict checks
passed. The selected reverse cone plus parser/CV covers fourteen shared
packages and the CLI, with thirty-six forward input packages; all 1,293 Rust
manifests and complete dependency declarations were re-audited. Focused CLOC41
native/CLI tests are included in those totals, not additional counts.
Source review `6fa6977512742b052856543517bcca112d157c5009e4a3d28701466194d0d0b6` and terminal review
`c1dd1b03d23a33f239537877374f04695097f082c901244c81d2f7198a85f86d` bind this bounded local execution. Core receipt
`31126ef9572a1297bfba5e9d8ebf32eb799e54c161dc5de58494e3561d12e382`. This later Markdown update claims no Rust re-execution.

This is bounded local CLOC41 lineage, not complete source/output mapping,
all call/composite families, full transform-role lineage or Closure parity.
Current-main reconciliation, conflict-free exact-head required CI and actual
merge/delivery remain OPEN. Base ESTree stays the canonical serialized
boundary, with own types and no new crates.io dependencies. Miri
DEFERRED/PENDING, never passed; frozen run/checkout/inputs/target/receipts remain
untouched and unpolled. Genuine final-source Miri follows implementation
delivery. Full objective OPEN; heartbeat ACTIVE.


The **first concrete optimization pass** for the Closure Compiler
clone. Folds compile-time-evaluable expressions per
[CLOC06](../../../specs/CLOC06-pass-interface-contract.md)'s canonical
pass set.

## Implemented behavior

The pass performs a bottom-up walk. It folds arithmetic, bitwise operations, primitive comparisons, string
members/methods and other guarded constant operations. See the changelog and
ported upstream tests for the implemented families.

[CLOC31](../../../specs/CLOC31-primitive-fold-lineage.md) repairs traced
string-literal `.length`, primitive binary and primitive unary folds: each
replacement retains available operand CVs and owns its `constant-fold/folded`
contribution. Nested folds retain child history. Other families still require
the CCR-065 lineage audit; this is not a claim of complete output-byte tracing.

Equal conditional branches use borrowed AST comparison excluding CV metadata,
including values nested inside composite branches. Signed zero and every other
represented field remain distinct. Primitive branch collapse retains both
histories; composite branch lineage remains in CCR-065.

- `ConstantFoldPass` implementing the `Pass` trait from
  [`coding-adventures-closure-pass-pipeline`](../closure-pass-pipeline).
- `name = "constant-fold"`, `iteration_policy = FixedPoint`, `cost = 2`.
- The bottom-up walk handles nested folds in one pass; later pipeline passes
  may expose new foldable expressions for the next sweep.
- A large-stack worker protects the recursive walk on deeply nested inputs.

Examples of implemented folds:

- Number folding: `2 + 3 → 5`, `10 * 4 → 40`, `7 - 9 → -2`.
- Bitwise / shift folding (CLOC15.D): `0xFF & 0x3C → 60`, `1 << 4 | 2 → 18`,
  `8 >>> 1 → 4` — ES `ToInt32`/`ToUint32` 32-bit semantics (`>>>` is unsigned).
- String concatenation: `"foo" + "bar" → "foobar"`.
- Boolean short-circuit: `true && x → x`, `false || y → y`.
- `typeof` of literals: `typeof "s" → "string"`, `typeof 0 → "number"`.
- Negation folding: `!true → false`.
- Comparison: `1 < 2 → true`.
- Conditional folding when condition is constant: `true ? a : b → a`.

Each family has guards for supported input shapes, coercion and observable
effects. Division by zero is deliberately retained rather than replaced with
the shadowable `Infinity` or `NaN` names. The typechecker remains a separate
passthrough scaffold; this pass does not claim inference-backed safety.

## Dependency whitelist

- `coding-adventures-closure-pass-pipeline` — the `Pass` trait + types.
- `coding-adventures-javascript-ast` — `Program` input/output.
- `coding-adventures-type-sidecar` — type-aware fold safety.
- `coding_adventures_correlation_vector` — `Contribution` plumbing
  per CLOC03.
- `serde_json` — `Contribution.meta` JSON values.

Plus `coding-adventures-javascript-tokens` as a dev-dependency for
`EsVersion` in tests.

## Upstream conformance tests

`tests/upstream/` ports Google Closure Compiler tests (Apache-2.0; see
`ATTRIBUTION.md` and `UPSTREAM_SHA`), per the CLOC12 test-port convention:

- `peephole_fold_constants_test.rs` — `PeepholeFoldConstantsTest` (binary/unary
  constant folding).
- `peephole_replace_known_methods_test.rs` — `PeepholeReplaceKnownMethodsTest`.
  Pins the String-method folds this pass performs (indexOf, lastIndexOf, case
  conversion, slice, substring, substr, charAt, charCodeAt, repeat, trim,
  includes/startsWith/endsWith), the numeric `Math.abs`/`floor`/`ceil`/`round`
  and `Math.max`/`Math.min` folds, the `Array.prototype.join` fold on an
  array literal of constants (gap-142, `["a","b"].join("-")` → `"a-b"`), and
  the `String#concat` fold with `ToString`-coerced primitive arguments (gap-143,
  `"x".concat(1, 2)` → `"x12"`). **This port is now fully active** — gaps
  141/142/143 are all closed, so there are no remaining `#[ignore]`
  placeholders. Run with
  `cargo test --test upstream_peephole_replace_known_methods` (every case is
  active — nothing is ignored).

## Checked provenance failures (CV02)

The recursive visitor retains its first derivation, merge or contribution
error. `Pass::run` returns it before accepting the transformed program; later
forks stop recording. Checked resource exhaustion is an error, not a panic or
an accepted replacement with missing evidence.

On a provenance error the worker disposes of the candidate AST on its own
large stack before returning, so rejection does not transfer a deep tree to
a small caller stack for destruction.

Array-literal `.length` and `["length"]` retain available member/array/property
identities and already folded immediate primitive elements in first-seen order.
A borrowed collector meters per-fold work and requested temporary storage
before graph mutation, including collision/equality work. These operand links
do not supply missing composite occurrences, opaque child lineage or complete
source/output mappings. Final-source Miri remains DEFERRED/PENDING.

### CLOC36 selected array replacement roots

In-bounds array selection stamps the actual returned root with its own checked
fold identity. Immediate parents retain member, array, actual numeric/string
key and selected-root histories in first-seen order. Every Expression variant
has an explicit private root slot; semantic spread/effect/index guards and all
non-CV/nested-CV fields are preserved. Exact typed-root, trace-context and
publication-refusal controls pass locally; see CLOC36 for execution receipts.
Opaque root-to-descendant graph closure and full parity/output mappings remain
OPEN. Miri DEFERRED/PENDING; genuine final-source acceptance follows delivery.

### CLOC37 selected-subtree lineage

Present in-bounds array selection joins member/array/actual-key identities and
the complete selected native subtree in deterministic preorder. Its returned
root reaches retained child rewrites, including opaque calls and function
bodies. A bounded borrowed count/replay walk charges cumulative work and all
simultaneously live requested buffers before checked graph mutation. Source
intervals, child CVs, payload and selection guards are preserved; see CLOC37
for exact local suite/runtime/security evidence and limits. Conditional/other
transformations, output-byte/source mappings, parity and delivery remain OPEN.
Miri DEFERRED/PENDING; genuine final-source acceptance follows delivery.

### CLOC38 literal conditional root lineage

Literal conditional selection now stamps its actual returned expression root
with the checked join of available conditional, folded literal test and complete
selected subtree identities in preorder. No IDs yields no identity/contribution.
Existing truthiness, evaluation order, nested CVs,
payload and equal-branch semantics remain. Exact lineage/clock/physical-source
and all-output refusal controls cover both choices and unchosen-child errors;
see CLOC38 for local suite/runtime/security evidence and limits. Equal-branch
and broader transformation/output-byte/source/parity/delivery work remain OPEN.
Miri DEFERRED/PENDING; genuine final-source acceptance follows delivery.

### CLOC39 equal-conditional evidence frontiers

Equal-branch collapse now joins available conditional and complete folded test,
consequent and alternate identities under one cumulative count/replay budget.
The pure returned root or impure sequence owns the checked fold; nested data,
evaluation order and existing equality/side-effect guards remain. Both arms
provide source/rewrite evidence while one arm executes. Exact graph/source/
pass clocks and child/outer refusal controls supplement prior scanner limits.
See CLOC39 for local acceptance evidence; typed roles/deletion, full output-byte/
source joins/parity/delivery remain OPEN. Miri DEFERRED/PENDING; genuine
final-source acceptance follows implementation delivery.

### CLOC40 logical literal-choice ownership

Admitted literal &&/||/?? choices now stamp the actual returned root with
checked ordered logical/left/selected-subtree ancestry, deduplicating chosen-
left aliases. Nested identities, payload, eager child folding, first-error
behavior, labels, existing guards and collector limits remain. Exact source/
pass clocks and child/outer publication-refusal controls supplement prior
resource tests. See CLOC40 for bounded local evidence. Reassociation/typed
roles/deletion/full output-byte/source joins/parity/delivery remain OPEN.
Miri DEFERRED/PENDING; final-source acceptance follows implementation delivery.
