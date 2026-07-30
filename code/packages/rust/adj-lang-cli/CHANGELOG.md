# Changelog

## [0.30.0] — 2026-07-30 — ADR-3: argument grounding gate — an un-sourced argument won't compile

Picks up adj-lang 0.67's structural grounding gate. New e2e (`argument_surface_e2e.rs`, now 6):
an un-sourced premise and an un-warranted inference are each named `ArgMissingProvenance` compile
errors, while the sourced axle argument still derives its cited thesis. Byte-neutral to
`--json`/golden (the gate only rejects; a valid argument is unchanged).

## [0.29.0] — 2026-07-30 — ADR-2: `argument` derives + cites its thesis end-to-end

Picks up the new `argument` surface (adj-lang 0.66): a multi-step argument now compiles, and the
engine **derives** its thesis by chaining the inference rules through the premise facts, with the
proof carrying every premise's byte citation — the argument is auditable to its sources.

- New e2e (`argument_surface_e2e.rs`, 4): the ADJ-ARGUMENT-IR §8 axle-fatigue argument derives
  `mechanism(axle, fatigue)` cited to its premises; a dangling `from` reference, an unknown premise
  kind, and a duplicated element name are each named compile errors with no answer produced.
- No CLI code change: because `argument` desugars to facts + rules, recall/`--json` render it
  through the existing path; `--json`/golden output is byte-for-byte unchanged (golden 38 hold).
- Note: rendering the argument chain in `--explain` (SLD proof chains, distinct from the
  differential/compute trace it renders today) lands with the `adj-verify` argument pass in ADR-4.

## [0.28.0] — 2026-07-30 — NUM-6v: `adj-verify` re-checks the precision/format narrowings

`adj-verify` now re-executes the *arithmetic* of a trail, not only its logic
(`ADJ-NUMERIC-SUBSTRATE.md` §4.3, §6, §7). For every `let`-bound derived value it walks the
derivation tree and, for each `round_to`/`round_sig`/`to_scientific`/`to_percent`/`to_currency`
narrowing, re-rounds the recorded **exact** source under the recorded mode and confirms the
recorded result and rendered string reproduce (`logic_engine::recheck_narrowings`).

- New report fields: `totals.narrowings_rechecked` / `narrowings_unverifiable` /
  `narrowings_mismatched`, and a `narrowings` array (one entry per derived value carrying a
  narrowing, each check tagged `rechecked` / `unverifiable` / `mismatch`).
- A `mismatch` is a hard failure: it flips `verified` to `false`, exits non-zero, and is named
  in `first_failure` (`pass: "narrowing"`, with the binding name, depth, and the disagreeing
  recorded/recomputed forms) — the same standard the negation and logit re-checks already meet.
- The four derivation-tree JSON emitters gained the new node field via `..` patterns, so the
  CLI's `--json` / `derivation` output is byte-for-byte unchanged.
- 3 e2e tests (built binary): all five narrowing kinds re-check green; a plain formula reports
  zero narrowings and still verifies; a nested `round_to(to_percent(...))` re-checks both levels.

## [0.27.0] — 2026-07-30 — RS-3c: `statemachine` driver + typed outcomes + `--explain` of the run

Runs the provenance-stamped `statemachine`s RS-3b lowered (ADJ-STATEMACHINE §3–§4),
surfacing each run's typed outcome in a new `state_machines` JSON section and in the
`--explain` narrative. The driver introduces **no new evaluator**: a comparison guard
reuses the engine's exact-first predicate comparison (`observed_numeric` + `compute` +
`CmpOp::eval_values`), a presence guard reuses the SLD resolver (`enumerate_all`,
"has any proof?"), and an `assert` action adds a `Fact::certain` to a **working clone**
of the KB (asserts never leak into the rest of the program).

- **Driver** (`adj-lang::statemachine::run_state_machine`): the §3 loop — exit-check →
  budget-check → cycle-check → first-guard-wins transition → apply asserts. Total by
  construction; it returns exactly one typed [`StateMachineOutcome`] and **cannot hang**
  (the declared `budget` caps the loop at `budget + 1` iterations even if cycle detection
  never fires).
- **Four typed outcomes** (§4): `Halted { state, result }` (numeric yield with its
  derivation tree, or a bare symbol like `at_target`), `StepBudgetExceeded { steps,
  budget, state }`, `NonTerminating { state }` (a `(state, asserted-set)` livelock, §3.1),
  `Stuck { state }` (dead end — no transition and no exit).
- **CLI JSON**: a `"state_machines":[…]` section — each machine's name, typed outcome,
  ordered provenanced steps (guard tested, target, asserted facts, cited source), and the
  machine's own citation. **Omitted when the program declares no `statemachine`**, so all
  existing output is byte-for-byte unchanged (the `cli_golden` shape is preserved).
- **`--explain`**: a `Run of <name>:` block per machine — the ordered transition lines
  (`state s: transition on <guard> to s' [<cited provenance>]  (asserted …)`) ending in
  the typed outcome line (`=> Halted at …, yields …` / `=> StepBudgetExceeded after N
  steps (budget M)` / `=> NonTerminating (cycle at …)` / `=> Stuck in …`). Projection-only
  and deterministic (P1/P4). `explain()` gained a `state_machine_runs` parameter.
- **Tests**: `tests/rs3c_statemachine_run_e2e.rs` — the §6.1 terminating titrate (Halted +
  yield), the §6.2 spin (NonTerminating/budget, proven to RETURN — no hang), a Stuck dead
  end, `--explain` determinism, the omit-when-empty invariant, and provenanced steps.

## [0.26.0] — 2026-07-30 — RS-4 PR-E2: `--explain` inference + adjudication surfaces

Extends the `--explain` renderer (ADJ-REASON-MATH §E.8) from the derivations
surface to the **inference** and **adjudication** surfaces, so a *differential*
query — not just an arithmetic one — explains itself. Still a projection only: it
reads the decided differential (`Differential`) the engine already produced and
re-runs nothing.

```
Inference for bacterial:
  prior on bacterial = logit -0.847 [source "x" trust empirical]
  bacterial contributes logit 2.708 via [source "Straus 2006" trust authoritative] [evidence: [unattributed]]

Decision:
  bacterial — posterior 0.865 (logit 1.861)
  => bacterial (determinate; posterior 0.865, margin 0.865; trust empirical)
```

- **Inference** — the ordered proof steps per hypothesis: prior, likelihood-ratio
  contributions (each showing the CLAUSE that licensed it *and* the observed
  evidence it consumed), joint interactions, predicate-gated contributions (the
  CPU comparison `observed <op> threshold`), rule-derived premises, and
  negation-as-failure. The walk is **total** over `DerivationOrigin` (a new kind
  ⇒ compile error), mirroring the JSON `trace_steps_json`.
- **Adjudication** — the ranked hypotheses with their posteriors, and the
  comparative decision: `determinate` with its margin, or a `Kickback` rendered as
  the honest "cannot commit" verdict with its reason (the differential's own
  abstention). P3: the leader shows the trust tier propagated as the `min`
  (weakest link) over the graded knowledge — prior, rules, and contribution
  clauses — it relied on.
- A pure computation (a `let` with no differential evidence) still renders
  derivations-only; the JSON trail is byte-for-byte unchanged when `--explain` is
  absent.

New: `tests/rs4e2_explain_inference_e2e.rs` (4 tests). Extends `src/explain.rs`;
touches `src/main.rs` (threads the decided `diff` into `explain`).

## [0.25.0] — 2026-07-30 — RS-4 PR-E1: `--explain` renderer (derivations surface)

Adds the human-readable *"explain its reasoning"* view — `adj-lang-cli --explain <prog.adj>` —
specified by ADJ-REASON-MATH §E.8. Where the default output is the byte-cited JSON trail (the
machine artifact `adj-verify` re-checks), `--explain` renders the *same* reasoning as text a
person reads. It is a **projection only**: it reads the derivation trees the engine already
built and re-runs nothing, so the explanation can never say more than the proof.

This first slice renders the **derivations** surface — the arithmetic behind each `let`/formula
value, shown operand-by-operand down to its cited leaves (the §E.8.4 shape):

```
total = 5 [scalar]   <= source "…" locator "…" trust authoritative
  5 = a + b
    a = 2   [unattributed]
    b = 3   [unattributed]
```

- **P1 projection-only** — walks `derived_bindings`; no engine re-run, no new value computed.
- **P2 provenance on every line** — a computed value carries its applied `formula`'s citation;
  an observed leaf with no attribution renders an explicit `[unattributed]`, never silently blank.
  A literal constant asserts nothing new and is shown inline in its parent expression.
- **P4 determinism** — first-seen binding order, stable numeric `Display`, no time/locale/map-order;
  the same program renders byte-identical text every run (pinned by a test).
- **P6 addressed structure** — each operand renders on its own line, indented one level deeper.

The default (no-flag) output is byte-for-byte unchanged; the JSON trail is untouched. Later PR-E
slices extend `--explain` to the premises / inference / adjudication / abstention surfaces of the
§E.8.1 linearization (this slice discharges the derivations portion of FL-7's explanation renderer).

New: `src/explain.rs`, `tests/rs4e_explain_e2e.rs`. Touches: `src/main.rs` (`--explain` flag +
branch).

## [0.24.0] — 2026-07-24 — RS-5f: nearest / nearest-neighbour table lookup tactic

Adds the fourth member of the `table` lookup family. A `? lookup … mode nearest give <val>`
snaps the query key to the single row whose key is CLOSEST to it — where `range` floors and
`interpolated` blends, `nearest` snaps:

```
? lookup trial_lenses power = 0.6 mode nearest give stocked      % 0.5 (the nearest stocked lens)
```

- **Exact distance.** `|k − q|` is computed as a `BigRational` (`sub` then `abs`) and candidates
  are compared on that exact distance — never an `f64` hop.
- **Deterministic ties.** An exact halfway query breaks to the SMALLER key, so the answer is
  reproducible and independent of row order.
- **Verbatim value.** Like `range` (unlike `interpolated`), the value cell is returned as-is, so
  a category-label value column is allowed; only the key column must be numeric.
- **Snaps out of domain.** A query beyond the last key snaps to the nearest endpoint — it never
  abstains for a non-empty table. It abstains only when there is genuinely no nearest key: an
  empty table (`no_grounded_support`) or a truncated search (`search_limit_exceeded`).

New `nearest_lookup_json` in `main.rs`, dispatched from the lookup map on `mode == "nearest"`.
Every answer carries the snapped row's citation (the same `via_facts → provenance` flow as the
other tactics). 0 answer-time model calls — pure exact comparison over the CAS-grounded rows.
New e2e suite `rs5f_nearest_lookup_e2e.rs` (snap, exact-tie → smaller key, non-numeric value cell,
out-of-domain snap). Requires adj-lang 0.63.0.

## [0.23.0] — 2026-07-23 — RS-5d: interpolated table lookup tactic

Closes the table-lookup trio (exact / range / **interpolated**). A `? lookup … mode interpolated
give <val>` reads the `table` as a piecewise-linear function: it finds the two breakpoint rows
that bracket the query key and returns the exact linear blend

```
v = v0 + (v1 - v0) * (q - k0) / (k1 - k0)
```

computed entirely on `ExactRational` (`add`/`sub`/`mul`/`div`) — a terminating blend renders every
digit, a repeating one renders as the reduced fraction (`10/3`), never a rounded `f64`. **Both**
bracketing rows' citations ride along, so the answer is traceable to the two measured points it
sits between (nomograms, calibration curves, growth charts). Honest edges:

- **exact hit** (`q` equals a breakpoint): the `0/0` blend is short-circuited to that row's value
  with its single citation;
- **out of domain**: below the lowest / above the highest breakpoint abstains with the typed
  `below_table_domain` / new `above_table_domain` reason — interpolation never extrapolates;
- **truncated search**: abstains `search_limit_exceeded` rather than blend against a partial scan.

Adds the `AboveTableDomain` abstention reason and an end-to-end suite (`rs5d_interpolated_lookup_e2e`):
linear blend with both citations, exact-fraction rendering, exact-breakpoint hit, below/above-domain
abstention, and correct-segment selection. The RS-5c suite's reserved-mode test becomes a
non-numeric-value-column guard (interpolating `category` is a compile error).

## [0.22.0] — 2026-07-23 — NUM-6c: render `to_currency` in the audit trail

The derivation-tree renderer gains a `DerivationNode::ToCurrency` arm (NUM-6c): a
`{"node":"to_currency","code":"USD","places":n,"mode":"half_even","rendered":"USD 33.33","value":…,"operand":{…}}`
object exposing the currency code, the decimal-place count, the stated mode, the rendered
`CODE d.dd` string, the narrowed numeric value (the rounded amount), and the operand subtree —
everything `adj-verify` needs to re-render from the exact source. Adds an end-to-end test driving
`to_currency(x, code [, places])` through the built CLI: exact rendering, trailing-zero padding,
`0`-places (JPY), the optional-`places` default, and rejection of a negative place count and a
non-identifier currency code.

## [0.21.0] — 2026-07-23 — NUM-6c: render `to_percent` in the audit trail

The derivation-tree renderer gains a `DerivationNode::ToPercent` arm (NUM-6c): a
`{"node":"to_percent","places":n,"mode":"half_even","rendered":"33.33%","value":…,"operand":{…}}`
object exposing the decimal-place count, the stated mode, the rendered `d.dd%` string, the
narrowed numeric value (the fraction the percentage denotes), and the operand subtree —
everything `adj-verify` needs to re-render from the exact source. Adds an end-to-end test
driving `to_percent(x [, places])` through the built CLI: exact rendering, trailing-zero
padding, `0`-places (`"50%"`), the optional-`places` default, and rejection of a negative /
non-integer place count.

## [0.20.0] — 2026-07-22 — NUM-6c: render `to_scientific` in the audit trail

The derivation-tree renderer gains a `DerivationNode::ToScientific` arm (NUM-6c): a
`{"node":"to_scientific","figures":n,"mode":"half_even","rendered":"6.022e23","value":…,"operand":{…}}`
object exposing the significant-figure count, the stated mode, the rendered boundary
string, the narrowed numeric value, and the operand subtree it narrowed — everything
`adj-verify` needs to re-render from the exact source. Adds an end-to-end test driving
`to_scientific(x [, figures])` through the built CLI: exact rendering across scales
(large integer, repeating rational), the optional-`figures` default, and rejection of
a zero / non-integer figure count.

## [0.19.0] — 2026-07-22 — NUM-6b: render `round_sig` in the audit trail

The derivation-tree renderer now names the KIND of precision narrowing: `"places"`
for `round_to` (NUM-6a) and `"sig_figures"` for `round_sig` (NUM-6b), e.g.
`{"node":"round","sig_figures":3,"mode":"half_even","value":31500,"operand":{…}}`.
Adds an end-to-end test driving `round_sig(x, n)` through the built CLI across
scales (large integer → power of ten, fraction, sub-1 value) with `n = 0` rejected.

## [0.18.0] — 2026-07-22 — NUM-6a: render the `round_to` narrowing in the audit trail

The derivation-tree JSON now renders the new `DerivationNode::Round` (NUM-6a): a
`{"node":"round","places":n,"mode":"half_even","value":…,"operand":{…}}` object
exposing the precision, the stated rounding mode, and the operand subtree it
narrowed — so a checker can re-round the operand's exact value and confirm the
rendering (ADJ-NUMERIC-SUBSTRATE §4.3). Adds an end-to-end test driving
`round_to(x, n)` through the built CLI (exact value, audit fields, and a
non-integer/negative precision rejected as a compile error).

## [0.17.0] — 2026-07-21 — the `adj-verify` binary (RS-4 PR-D2)

Implements `ADJ-REASON-MATH.md` §E.5/§E.6: a standalone re-checker that reads an
`.adj` program and **re-executes its reasoning**, reporting per step whether it
still holds. It is not `adj-replay` (ADJ08) — it never invokes a model and never
leaves the process; it is the deep re-execution *inside* one engine artifact,
which ADJ08's linter should call rather than reimplement.

### Added

- **`adj-verify <PROGRAM> [--snapshots DIR]`** — prints a JSON report and exits
  **1 when anything failed**, so it composes as a CI gate rather than as prose a
  human has to read. Both reasoning paths are examined and labelled: `sld`
  (recall, rules, tables, negation) and `lr` (likelihood-ratio aggregation).
  Verifying only one while printing "verified" would leave half the trail
  unexamined behind a clean headline.
- `--snapshots DIR` reads pinned source documents from a content-addressed
  directory whose filenames are the lowercase SHA-256 hex of their contents. The
  bytes are **re-hashed after reading** — a store that trusted the filename would
  let anyone who can write into that directory make an arbitrary document answer
  to a pinned hash, which is the exact substitution pinning exists to prevent.
- **`verified` and `fully_verified` are different claims.** The first means every
  step re-executed; the second additionally requires that every proof had its
  quotes confirmed against a snapshot, and that there was something to check.
  Today's stdlib is `unmigrated`, so it is honestly `verified: true`,
  `fully_verified: false` — the report refuses to let a wholly unchecked corpus
  read as a clean bill of health.
- **`src/lib.rs`** — `esc`, `payload`, `query_echo`, `sensitive_input` and
  `FsProvider` moved out of `main.rs` so both binaries link the *same* copy. A
  security check that exists twice exists zero times: a second implementation of
  the sensitive-channel test or the import-sandbox containment check would
  eventually disagree with the first, silently.

### Security

- Quoted spans and goal terms leave through `payload()` — redacted on a sensitive
  channel, length-capped, and JSON-escaped. Both are untrusted text: a quote is
  lifted verbatim from a spidered page, and an unescaped newline in a
  line-oriented trail lets a span forge its own `"logic":"rechecked"` line.
- No network access. See the `logic-engine` 0.44.0 entry.

## [0.16.0] — 2026-07-21 — typed abstention reasons (RS-4 PR-C)

Implements `ADJ-REASON-MATH.md` §E.4. Every abstention used to be one bit —
`"abstained": true` — for situations that are not merely different but opposite.

### Added

- **`abstention` object on every abstaining answer**, carrying a typed `reason`,
  the specifics, and a plain-language `explanation`:
  - `below_table_domain { table, key, min_key }` — the question was well-formed
    and the source does not reach that far. Reports the table's actual floor, so
    the caller learns WHICH domain they fell outside.
  - `non_numeric_key { table, column, key }` — the question was malformed.
    Nothing is wrong with the table.
  - `no_grounded_support { goal }` — the search completed and found nothing.
  - `search_limit_exceeded { goal }` — the search STOPPED. It established no
    absence at all.

### Fixed

- **Below-domain and malformed-key emitted BYTE-IDENTICAL JSON.** Those are
  opposite failures — the source being honest versus the caller being wrong —
  and no consumer could tell them apart, which made the abstention unactionable:
  you could not tell whether to widen the source or fix the query.
- **A truncated search could be laundered into a claim of absence.** PR-B's
  recursion guard made a divergent search abstain; without a reason it was
  indistinguishable from "the knowledge base has no support for this". It now
  reports the limit, and says explicitly that this is *not* evidence that no
  proof exists.

### Notes

- Additive: `"abstained"` is unchanged and still emitted, and the `abstention`
  object appears only when abstaining — an answered query's bytes are identical
  to before.

## [0.15.0] — 2026-07-20 — the unified reasoning trace (RS-4 PR-B)

Implements the ordered/addressed/self-contained step contract of
`ADJ-REASON-MATH.md` §E.1–E.2. You could already ask ADJ *what* it concluded and
*what it cited*; now you can ask **how it got there**.

### Added

- **`trace_steps_json` — a TOTAL walker over `DerivationOrigin`.** Renders a
  proof as an ordered list of steps, each carrying `step` (index), `depth`
  (nesting), the goal, and its **inline resolved provenance** — not a `FactId`
  pointer. The trace is therefore self-contained: it can travel to a reviewer or
  another machine and still be readable without the KB that produced it.
  The match has **no wildcard arm**. The previous renderer ended in `_ => {}`,
  which would silently discard four of six step kinds. That arm was latent in
  shipped paths (the likelihood-ratio kinds are rendered by `proof_json`), but a
  wildcard that drops reasoning is a trap for whoever adds the next step kind —
  as `FromNegation` immediately proved. Adding a variant now breaks the build,
  which is the point.
- **`steps` on `recall` and `lookups` answers.** Both previously rendered only
  from `via_facts`, which is `sort()`ed and deduplicated — a citation SET that
  says which sources were involved but never in what order or through which
  rules. `citations` is kept unchanged for existing consumers; `steps` is the
  derivation beside it.
- **`derivation` on every `derived` value.** The engine builds a
  `DerivationNode` tree for every `let` and every formula application, and the
  CLI **dropped it at the JSON boundary** — `derived_json` never read `.tree`.
  It is now emitted: `op` nodes show the arithmetic, and `leaf` nodes resolve
  through their real `FactId` to the source fact's provenance. That
  compute-to-bytes bridge already existed inside the engine; this is what makes
  it reachable from outside the process.

### Notes

- Additive only: no existing field changed or was removed, and a program with no
  `let`, recall, or lookup produces byte-identical output.

## [0.14.0] — 2026-07-18 — table answers now cite the row that produced them (ADJ-TABLES RS-5e)

### Changed

- **A table-backed answer's `citations` now quote the span defending *that row*** — not the table's
  single envelope. Ask the shipped `environment/air-quality-index` table for AQI 120 and the answer
  cites *"Orange Unhealthy for Sensitive Groups 101 to 150 …"*; previously it cited the `0 → good`
  sentence regardless of which band was selected. This holds for **exact** recall and **range**
  lookup alike, and flows through the proof DAG's `via_facts` the same way.
- **No CLI source changed.** Each row already lowered to its own `Fact`, and the citation path
  already cited *the fact that produced the answer*; adj-lang 0.55.0 simply gives that fact the
  row's provenance. The version bump records the observable change in emitted citations.
- Tables whose rows carry no provenance block are byte-for-byte unchanged.

## [0.13.0] — 2026-07-18 — RS-5c: range / bracket lookup answers (ADJ-TABLES)

### Added

- **`"lookups"` output section** for range/bracket lookups. Each `? lookup <table> <key_col> = <n>
  mode range give <value_col>` resolves the table (its rows are its facts) by enumerating the
  relation, selects the breakpoint row whose key is the greatest key `<= n`, and emits
  `{query, mode, answers:[{bindings, citations}], abstained}` — the value column bound, the
  **matched breakpoint key** named in the audit, and the selected row's citation carried through
  (the same `via_facts → provenance` flow as exact recall). A query **below the smallest key**
  honestly `"abstained": true` — "below the table's domain", never a fabricated classification.
- The comparison rides the exact `BigRational` order (`ExactRational::as_ratio()` — the identical
  total order the engine's `CmpOp` exact path uses), so there is **no `f64` hop** in the decision.
  0 answer-time model calls. The section is omitted entirely when the program declares no
  `? lookup … mode range …`, so existing output is byte-for-byte unchanged.

## [0.12.0] — 2026-07-14 — exact-number arc COMPLETE: computed results render every digit (ADJ-EXACT-NUMBERS NX-4)

### Changed

- **`derived.value` now renders exact-first.** When a `let`/`formula` computation stayed inside
  exact rational arithmetic (NX-3) and its result has a finite base-10 expansion, the CLI prints
  **all** of its digits (via the new `ExactRational::to_exact_decimal_string`), mirroring NX-2's
  exact recall-binding rendering. So a stored 39-digit π fed through the shipped `product` formula
  and doubled now renders `6.283185307179586476925286766559005768394`, not the f64-truncated
  `6.283185307179586`. The `f64` (`jnum`) remains the labeled-lossy fallback, used only when there
  is no exact sidecar or when the value **repeats** (e.g. `1/3`), which no finite decimal can hold.
  The field stays a JSON number literal — its type is unchanged; only its precision grows for values
  that were previously truncated. Non-terminating results (e.g. the ideal-gas pressure, a division
  whose denominator carries a prime other than 2/5) are byte-for-byte unchanged.

### Added

- E2E proofs closing the arc: the shipped `mathematics/constants.adj` binds π, e, and
  golden_ratio to their **full** published digit strings through the CLI (updated from the previous
  ~16-digit leading-substring assertion, which hedged that "the runtime binds a double-precision
  float" — no longer true); and a computed-result test doubles a stored 39-digit π through the
  shipped `product` formula and asserts the rendered `value` shows all 39 exact digits.

## [0.11.1] — 2026-07-14 — exact-number bindings render every digit (ADJ-EXACT-NUMBERS NX-2)

### Added

- E2E test: a native `table` whose cell is π to 39 decimal places binds through `? t(key, $V)`
  and renders **all 39 digits**, proving the exact-numbers path survives parse → store → query →
  render end-to-end (previously it came back as the f64-truncated `3.141592653589793`).

### Changed

- Behavior: a numeric literal whose magnitude overflows `f64` (`1e400`) now compiles and is stored
  exactly, instead of being reported as a malformed-literal error. The `malformed_numeric_clauses`
  golden was updated to probe a scale-amplification payload (`1e-2000000000`), which `BigDecimal`'s
  `MAX_SCALE` budget still rejects cleanly — so the "un-representable literal → `{"error":…}`, never
  a panic" invariant is unchanged.

## [Unreleased] — render applied-formula provenance in the `derived` section

### Added

- A `derived` value produced by APPLYING a provenanced `formula`
  (ADJ-FORMULA-LIBRARIES rung-0) now renders the formula's cited
  `source`/`locator`/`trust`/`corroborations` alongside its `name`/`value`/`dim` —
  the audit channel proving WHY the formula is trusted, beside the derivation tree
  proving HOW the number was computed. A plain `let` (no library claim) omits the
  field, so existing output is byte-for-byte unchanged.

## [0.11.0] - 2026-06-29 — surface `let`-derived dimensioned values

### Added

- New optional `"derived"` JSON section: one object per `let`-bound value with
  its `name`, computed `value`, exact rational (when integer/rational), and the
  `dim` tag the engine **inferred** (`"km/h"`, `"mol/l"`, `"scalar"`, …). The
  tag is formed by the engine's `Dimension::combine` at each operation, never
  written by the model — so a grader can reject a numerically-right-but-unit-wrong
  answer. The section is **omitted entirely** when a program binds no `let`, so
  existing rulebook/recall output is byte-for-byte unchanged. Backs the
  ADJ-LADDER `rung4_dimensional` rung's `compute_dimensioned` extractor.

## [0.10.0] - 2026-06-28 — render derived evidence proofs

### Added

- LR contribution proof JSON now includes an `"evidence_proof"` array when the
  contribution fired from a rule-derived evidence atom. The nested proof lists the
  SLD fact/rule steps and their provenance, so a `.adj` program that emits
  observations plus `rule { ... }` can show how the derived premise licensed the
  probabilistic verdict.

## [0.9.0] - 2026-06-21 — render corroborating citations (ADJ-A9)

### Added

- Each clause's provenance JSON now carries a **`"corroborations"`** array —
  `[{ "source": …, "locator": … }]` — listing the co-equal citations attached to
  the clause via `cites … locator …`. Empty in the common single-citation case.
  Existing `"source"/"locator"/"trust"` fields are unchanged, so existing
  recall/proof consumers keep working.

## [0.8.0] - 2026-06-17 — governing answers carry their grounded `context` (ADJ73 PR-B-3)

### Added

- Each `governing` answer now carries a **`"context"`** field — the grounded context its
  highest-standing deriving rule is in (`ninth_circuit`, `district_court`, `federal`, `state`, …),
  omitted for a context-free derivation. The audit reader sees *which context governed*, not just
  which term beat another — the lex-superior story made legible in the output.

### Worked example (committed, runnable)

- `code/specs/data/context-precedence/` — a grounded context-precedence rulebook
  (`context-precedence.adj`: `outranks_context` edges, each byte-quoting its charter — the
  Supremacy Clause for `federal > state`, vertical stare decisis for `ninth_circuit >
  district_court`) + a worked legal example (`worked-legal-example.adj`) that `import`s it and
  proves *lex superior* end-to-end: the Ninth Circuit's broad reading **governs** a district
  court's narrow reading **despite the latter's higher `mandatory` tier**. `SOURCES.md` is the
  provenance ledger. New golden test `tests/context_precedence_e2e.rs` runs the committed
  artifacts through the built CLI and asserts the override + that the edge is recallable WITH its
  charter. 0 answer-time model calls.

## [0.7.0] - 2026-06-16 — `governing` section: defeasible precedence output (ADJ73 PR-3)

### Added

- **`"governing"`** output section, emitted for each `$variable` binding query alongside
  `"recall"`. Runs `logic_engine::enumerate_governing` and renders every distinct answer with
  its precedence verdict: `status` ∈ `governing` | `defeated` (+ `defeated_by`) | `conflict_peer`,
  plus its `standing` (`asserted` for a fact, else the rule's tier) and `bindings` + ground
  `term`. A top-level `has_conflict` flags an unresolved tie. 0 answer-time model calls.
- For a predicate **not** declared `functional`, every answer is `governing` (mirrors `recall`)
  — the precedence-resolved view is back-compatible. For a functional predicate with
  `priority:` tiers it shows the override chain (the Python runtime reads this to get the
  governing decision — the enabler for the MYCIN `decide_timing` → ADJ refactor).

### Tests

- `tests/governing_e2e.rs`: the section tags every binding answer (non-functional baseline);
  it is absent for a ground hypothesis query. The functional-override path is covered at the
  engine (`logic-engine::govern`) + adj-lang lowering levels (the `functional`/`priority:`
  surface lands in adj-lang PR-C).

## [0.6.0] - 2026-06-14 — relational recall: binding-query `"recall"` output (MYCIN-2026 REL-3)

### Added

- **`"recall"` JSON section** for relational recall binding queries. A query goal
  containing a `$variable` (`? deficient_in(tay_sachs, $Enzyme)`) is routed out of
  the differential and resolved by SLD enumeration over the grounded knowledge
  graph; each answer reports its variable **bindings** and the **citations** of
  the edge(s) that prove it (source/locator/trust). An empty answer set is
  explicit **abstention** (`"abstained": true`) — no grounded edge supports an
  answer, so none is fabricated. 0 answer-time model calls.
- Ground hypothesis queries are unaffected (still flow to the differential
  `ranked`/`decision`); the `"queries"` echo lists every query (ground + binding).

## [0.5.0] - 2026-06-12 — `import`-aware compile + the filesystem trust boundary (MYCIN-2026 M3)

### Added

- **The CLI now resolves `import`s before compiling.** A program may `import`
  sibling `.adj` files (dictionary ← rulebook ← case); the CLI walks the graph
  via `adj_lang::compile_with_imports` and emits the decision over the composed
  program. Import errors (cycle, bound, missing/unparseable/escaping file)
  surface as a single `{"error": …}` line.
- **`FsProvider` — the import trust boundary.** The `adj-lang` library does no
  I/O; this filesystem-backed `ImportProvider` is the only thing that reads
  disk, so all path safety lives here: canonical ids are absolute,
  symlink-resolved real paths (so spellings dedupe and a symlink can't forge a
  second identity); import literals must be **relative** (absolute refused); the
  resolved real path must stay within the **sandbox root** (the program file's
  directory) — `../…` escapes and symlinks pointing outside the root are refused,
  so `import` cannot read arbitrary host files.
- 5 e2e tests (3-file decide; diamond no-duplicate; traversal / absolute / cycle
  all refused without hang).

## [0.4.1] - 2026-06-12 — `FromSolve` proof: the solver certificate renders under the verdict step (ADJ constraints E3)

### Added

- **The verdict's proof now descends into the solver certificate.** When a
  contribution fired from a constraint STATUS atom (E2 — `feasible` /
  `infeasible` / `solved` / `optimal` / `unbounded`), its proof step now carries
  a `"solver": …` field with that constraint's full result — the **IIS `core`**
  for `infeasible`, the assignment for `solved`, the value + binding constraints
  for `optimal`, etc. So `schedule_broken`'s proof step reads
  `…,"solver":{"outcome":"unsat","core":[0,1,2]}` — the verdict, *and* the exact
  conflicting constraints that forced it, in one auditable tree.
- Implemented entirely in the CLI renderer (no `logic-engine` change): a single
  `status_certificates` helper produces the `(status atom, certificate JSON)`
  pairs that both feed the differential (E2) and annotate the proof (E3); the
  certificate JSON reuses the existing `check_json`/`solve_json`/`optimize_json`
  renderers. 4 golden tests (IIS core / solved assignment / optimum under the
  step; no `solver` field on an ordinary contribution).

## [0.4.0] - 2026-06-12 — feed-a-verdict: constraint outcome drives the differential (ADJ constraints E2)

### Added

- **The constraint engine now feeds the differential — one engine, not two.**
  The CLI runs `solve`/`check`/`optimize` *first*, maps each outcome to a STATUS
  atom, and injects it as an observed fact into the KB *before* `decide` runs:
  - `check` Sat/SatReal → `feasible`; Unsat → `infeasible`
  - `solve` Solved/SolvedRoots → `solved`
  - `optimize` Optimal → `optimal`; Infeasible → `infeasible`; Unbounded → `unbounded`
  - Unknown / Unsupported / NoUniqueSolution → **nothing** (the engine never
    launders an undecided constraint into a verdict).
- An existing `contributes <lr> from <status> to <verdict>` clause then fires in
  the differential — composing solver result → verdict through the ordinary
  contribution + proof machinery, with **no new engine logic and no grammar
  change**. E.g. an infeasible schedule drives `schedule_broken`; loosening one
  deadline makes it feasible and the same program drives `schedule_ok` instead.
- 4 golden tests (infeasible `check` → verdict, feasible → the other verdict,
  infeasible LP → verdict, inert when no status clause references it).

## [0.3.7] - 2026-06-11 — decompose→solve golden tests (ADJ constraints track D2)

### Added

- `tests/decompose_run.rs` — deterministic golden tests over the **committed**
  decompositions from the live decompose→solve demonstration
  (`code/specs/data/adj-constraints-decompose-run/`). A local model
  (`llama3.1:8b`) turned 4 messy-prose word problems into adj-lang; the engine
  solved 3/4 to gold (44, 980, unsat) at **0 answer-time model calls**, and
  reported the 4th (a model mis-transcription) as `unsupported` rather than
  fabricating a value. The tests re-solve every committed `.adj` with no model in
  the loop — proving the engine is a pure function of the decomposition. Test +
  spec-data only; no CLI behavior change.

## [0.3.6] - 2026-06-11 — grant-allocation LP worked example (ADJ constraints track D)

### Added

- `grant_allocation.adj` worked example + golden test: an allocation LP
  (`maximize 3·outreach + 2·training` under a budget + capacity) solved
  end-to-end through the CLI at **0 model calls** → optimal 26 at (6, 4),
  binding constraints [0, 1]. Demonstrates the C2 `optimize` path in the
  worked-examples suite (now 5 examples). Test-only; no CLI behavior change.

## [0.3.5] - 2026-06-11 — emit LP optima (ADJ constraints track C2)

### Added

- When a `.adj` program declares a `minimize`/`maximize` objective, the CLI calls
  `adj_constraint_solver::optimize` and emits an `optimize` section:
  `{"outcome":"optimal","value":…,"assignments":[…],"binding":[…]}`, or
  `unbounded` / `infeasible` / `unknown`. A program with no objective emits no
  `optimize` key. 4 new golden tests. Tracks `adj-constraint-solver` 0.6.0.

## [0.3.4] - 2026-06-11 — emit real-feasibility verdicts (ADJ constraints track C1)

### Added

- Render the new `FeasibilityOutcome::SatReal` as
  `{"outcome":"sat_real","assignments":[{"name","value"}]}`, where `value` is a
  rational witness rendered as a JSON number. A `check` over a fractional or
  integer-infeasible-but-real system (`2 * x = 1` → `x = 0.5`) now emits
  `sat_real` instead of `unknown`. 2 new golden tests. Tracks
  `adj-constraint-solver` 0.5.0.

## [0.3.3] - 2026-06-11 — emit feasibility verdicts (ADJ constraints track B2c)

### Added

- When a `.adj` program ends with `check`, the CLI calls
  `adj_constraint_solver::check` and emits a `check` section:
  `{"outcome":"sat","assignments":[…]}` (with a witness integer per symbol),
  `{"outcome":"unsat","core":[…]}` (the conflicting constraint indices), or
  `{"outcome":"unknown","reason":…}`. A solve-only program emits no `check`
  key. 3 new golden tests (sat witness, unsat conflict, no-check). Tracks
  `adj-constraint-solver` 0.4.0.

## [0.3.2] - 2026-06-11 — emit nonlinear roots (ADJ constraints track C3)

### Added

- Render the new `SolveOutcome::SolvedRoots` as
  `{"outcome":"solved_roots","var":…,"roots":[…],"from_constraints":[…]}`, so a
  nonlinear single-unknown equation (`constrain x * x = 4`) emits its real roots
  (`[-2, 2]`). 1 new golden test.

## [0.3.1] - 2026-06-11 — solver substitutes observed facts (ADJ constraints track B3)

### Changed

- The `solve` call now passes the KB (`solve(&lowered.constraints, &lowered.kb)`),
  so a constraint that references an observed fact is solved with that fact's
  value substituted (`adj-constraint-solver` 0.2.0). 1 new golden test.

## [0.3.0] - 2026-06-11 — constraint solving in the CLI (ADJ constraints track B2b)

### Added

- When a `.adj` program declares a constraint system (`symbol` / `constrain` /
  `solve for`), the CLI now calls `adj_constraint_solver::solve` and emits a
  **`solve`** section in the JSON output:
  - `{"outcome":"solved","assignments":[{"name","value"}],"from_constraints":[…]}`
    — solved values, each cited to the constraints that determined them.
  - `{"outcome":"no_unique_solution"}` (singular / non-square), or
    `{"outcome":"unsupported","reason":…}` (inequality, non-linear term,
    aggregation) — **never a fabricated answer**.
  - The `solve` key is omitted entirely for a pure prior/contributes rulebook.
- New dependency on `adj-constraint-solver`. Linear-equality systems only this
  slice; feasibility (`check` → SAT/UNSAT via `constraint-engine`) and
  optimization follow.

## [0.2.0] - 2026-06-10 — predicate proof steps

### Added

- Render the new `predicate` proof-step kind in the JSON proof DAG:
  `{"kind":"predicate","slot","op","threshold","observed","logit",
  "source","locator","trust"}`. A predicate-gated contribution that fires
  shows the **literal comparison the engine evaluated on the CPU**
  (`gross_income >= 14600`, observed `18000`) alongside its citation — the
  byte-cited audit trail for a deterministic rule, reconstructable without
  the model.

## [0.1.0] - 2026-06-10 — initial CLI driver

### Added

- `adj-lang-cli PROGRAM.adj` — the CPU-bound reasoner entry point for the adj-lang
  DSL. Reads a `.adj` program (rulebook clauses + `observe`/`?` lines), compiles it
  via `adj_lang::compile`, runs `adj_lang::decide` (the `logic_engine` differential),
  and emits JSON: the ranked hypotheses with per-step proof DAGs, and the decision
  (`determinate` / `kickback` / `empty`). **Zero model calls.**
- Each proof step is joined back to its firing clause and emits the cited
  `source` / `locator` / `trust` tier — the byte-cited audit trail, reconstructable
  without the model. Non-finite numbers serialize as JSON `null`.
- Declarative argument parsing via `cli-builder` (embedded JSON spec). Exit codes:
  0 ok, 1 compile error (`{"error": ...}`), 2 bad args / unreadable file.
- 3 golden tests (`tests/cli_golden.rs`): single-hypothesis cited proof DAG,
  two-hypothesis differential ranking + decision, compile-error-as-JSON.
