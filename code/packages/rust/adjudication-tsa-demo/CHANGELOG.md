# Changelog

All notable changes to this project will be documented in this file.

## [0.13.0] - 2026-05-13 — VERDICT: ESCALATE (with-rulebook only)

### Motivation

The ADJ18 bench surfaced a regression: rulebook injection
*decreased* mean verdict accuracy across the 8-declaration set
(57.5% → 50.0% → 40.0%). The pocket-knife case was the clearest
single failure — every model got it correct without a rulebook
and incorrect WITH the fixture rulebook, because the model was
forced to commit to a binary verdict in cases where it could not
reliably evaluate a rule's threshold.

The right answer to "rule says blades over 2.36 in are
prohibited; declaration says 4 in" when the model can't do the
arithmetic isn't COMPLIANT or NON-COMPLIANT — it's *"ask a
supervisor."* That's what a real TSA officer does when they're
unsure.

### Added

A third verdict option `VERDICT: ESCALATE` in the with-rulebook
Arm A prompts (both single-turn and priming dispatch).

The conservative semantic: **silence in the rulebook is not
permission.** The model must have an explicit rule that either
permits or prohibits to return a confident verdict. Otherwise
ESCALATE.

ESCALATE is the correct verdict when:
- No rule in the rulebook either explicitly prohibits or
  explicitly permits the items declared.
- A rule applies but the model cannot evaluate its condition
  from the declaration (ambiguous, missing measurement, or
  unable to reliably perform the comparison).
- The declaration is ambiguous and the answer depends on details
  not provided.

This is consistent with how a real TSA officer behaves: if the
manual doesn't cover the case, ask a supervisor — don't wave the
passenger through and don't fabricate a rule.

### Where ESCALATE applies

- `build_raw_system_prompt(Some(rulebook))` — yes, three-verdict.
- `build_priming_system_prompt()` — yes, three-verdict.
- `build_priming_turn2_user_prompt()` — reminded.
- `build_raw_system_prompt(None)` — NO, binary verdict preserved.
  In the no-rulebook case the model is allowed to use
  training-data knowledge; ESCALATE doesn't apply there.

### Changed

Removed wording about "do not invent additional rules". Replaced
with "Use only the rules above. Do not invent or infer
additional rules." This is structurally cleaner and pairs with
the explicit ESCALATE option.

### Harness changes

`scripts/adj18_bench.py`'s `VERDICT_RE` extended to recognise
`ESCALATE` as a third matched verdict (was binary
COMPLIANT/NON-COMPLIANT). Existing bench data continues to parse
correctly; future bench runs against v0.13 prompts will record
ESCALATE verdicts as a third value rather than parse-fails.

### Tests

4 new tests added (42 lib total, all passing):
- `raw_system_prompt_no_rulebook_keeps_binary_verdict` — verifies
  the no-rulebook prompt is unchanged.
- `raw_system_prompt_with_rulebook_offers_escalate_verdict` —
  verifies the three-verdict set + the "silence is not permission"
  semantic.
- `priming_system_prompt_offers_escalate_verdict` — same for
  priming dispatch.
- `priming_turn2_user_prompt_lists_escalate_as_an_option` —
  verifies the turn 2 reminder.

### What's NOT in this PR

- **Bench re-run.** A follow-up PR will re-bench ADJ18 against
  v0.13 prompts and document whether ESCALATE rates correlate
  with the cases ADJ18 identified as regressions.
- **Source decomposition.** Independent gap: Arm A doesn't
  decompose the source text (e.g., "4 inch pocket knife" doesn't
  become `blade_length(quantity(4, inches))`). The current bench
  doesn't exercise the structured extraction path; that's a
  separate spec.
- **clinical-demo and contract-demo prompts.** Same change
  should apply for consistency but is staged separately to keep
  the review surface focused.
- **Expected-verdict refinement.** Some bench declarations are
  arguably ambiguous (e.g., "matches" — strike-anywhere or
  safety?). The expected-verdict column currently assumes
  worst-case interpretation; an ESCALATE on these is defensible
  but currently scored as wrong. Refinement queued for the
  bench re-run PR.

### Compatibility

- `build_raw_system_prompt(None)` unchanged (binary verdict).
- Existing tests still pass (the prior assertions don't depend
  on the new wording specifics).
- Bench harness change is backward compatible — the regex still
  matches the two existing verdicts.

## [0.12.0] - 2026-05-12 — Arm A truncation hardening

### Motivation

ADJ17 documented a real workflow problem: gemma4 in adversarial
mode burns its 512-token output cap on chain-of-thought narration
of the injected rulebook and never reaches the `VERDICT:` line.
The framework surfaces this as a typed `OutputTruncated` error —
the *failure shape* is correct, but the *failure rate* is too
high to be ignored. v0.12 addresses the root cause with three
coordinated changes.

### Added

- **`ADJ_DEMO_MAX_ANSWER_TOKENS`** env var. Controls Arm A's
  output-token cap. Default is **2048** (was 512 hard-coded).
  Configurable per-run; lower it to test the truncation path,
  raise it for very verbose models. Plumbed through to the
  `CompletionRequest.max_tokens` field.
- **`ADJ_DEMO_ARM_A_MODE`** env var, accepting:
  - `single-turn` (default): unchanged v0.11 behaviour.
  - `priming`: two-turn dispatch. Turn 1 hands the model the
    rulebook with an explicit instruction to respond only with
    `ACK` (no rule narration, no analysis). Turn 2 sends the
    declaration and asks for a verdict-first response. The model
    digests the rulebook silently in turn 1, then produces a
    tight verdict in turn 2 — drastically reducing the
    chance of output truncation on verbose 8B-class models.
- `ArmAMode` enum: `SingleTurn` / `Priming`. Added to
  `DemoConfig` along with `max_answer_tokens: usize`. Both have
  sensible defaults.
- New public prompt builders: `build_priming_system_prompt`,
  `build_priming_turn1_user_prompt`,
  `build_priming_turn2_user_prompt`. Pulled out so tests can
  assert on prompt structure without invoking the model.
- `build_raw_system_prompt` is now `pub` (was crate-private) so
  external callers and tests can inspect it.

### Changed

- **Verdict-first prompt format.** Both `build_raw_system_prompt`
  (single-turn) and `build_priming_system_prompt` (two-turn) now
  instruct the model to put `VERDICT: COMPLIANT` or
  `VERDICT: NON-COMPLIANT` as the FIRST line of the response,
  followed by 2-3 sentences of reasoning. Rationale: even when
  truncation hits during the reasoning portion, the verdict line
  survives. The system prompt explicitly cites this as the reason
  for the format ("ensures the verdict is captured even if your
  reasoning is truncated").
- Arm A's startup banner now displays the active mode and token
  cap so the run is self-documenting:
  `Arm A mode:    SingleTurn (max 2048 output tokens)`.

### How the three changes interact

- **Verdict-first prompt** is a free defense: zero overhead, no
  extra calls, and the verdict survives truncation when it happens.
- **Raised output cap** is the backstop: the default doubles to
  2048, so most truncations the v0.11 demo would hit just don't.
- **Priming mode** is the architectural fix for the
  large-rulebook case: the model's chain-of-thought happens
  silently in turn 1's hidden state, so turn 2's output budget
  is spent on the verdict, not on rulebook narration.

### Tests

7 new tests added (38 lib total, all passing):
- default config uses SingleTurn + 2048 token cap
- raw system prompt demands verdict-first AND mentions truncation
  (both no-rulebook and with-rulebook variants)
- priming system prompt describes the two-turn protocol (Turn 1,
  Turn 2, ACK, no analysis until turn 2)
- priming turn 1 user prompt embeds the rulebook and demands ACK
- priming turn 2 user prompt embeds the declaration and re-states
  the verdict-first format
- ArmAMode enum round-trips through Debug/Clone/Eq
- DemoConfig.arm_a_mode is field-addressable

The existing test asserting on the v0.11 prompt's exact wording
was updated to match the v0.12 phrasing
("citing specific rule numbers", removed the article).

### Compatibility

- `DemoConfig` gained two new public fields
  (`max_answer_tokens`, `arm_a_mode`). Soft break for callers
  that pattern-destructure `DemoConfig`; no in-tree caller does.
  Existing tests and downstream demos (clinical / contract)
  build unchanged.
- Existing `run_raw_arm(cfg)` signature unchanged. Inside, it
  now dispatches via `cfg.arm_a_mode`.
- Default behaviour preserved: a no-knobs invocation runs
  `SingleTurn` with the new 2048-token cap and the verdict-first
  prompt. The verdict-first change is observable in the output
  (verdict line moves to the top) but not behaviourally
  load-bearing for downstream tooling.

### Why this isn't ADJ16 step 6

ADJ16's spec didn't enumerate Arm A hardening as a step — the
ADJ16 sequence is about replacing answer-time LLM with the
engine (Arm C), not about making Arm A more robust. This change
is a tactical fix to keep the empirical-comparison value of Arm
A meaningful: if half of gemma4's adversarial runs truncate to
"Arm A failed", the comparison against Arms B and C loses
signal. Hardened Arm A → cleaner Engine-Arm-vs-LLM-Arm bench
data, which is what the next ADJ-numbered spec (probably
"ADJ18") will document.

## [0.11.1] - 2026-05-12 — Engine Arm in the CLI (ADJ16 step 5.5)

### Added

The engine arm (introduced as a library API in v0.11) is now
wired into the CLI binary behind the new `ADJ_DEMO_ENGINE_ARM`
env var. After Arms A and B print, the demo loads one or both
fixture rulebooks and runs the engine arm.

Accepted values:

- `ADJ_DEMO_ENGINE_ARM=strict` (or `=1`): use only
  `tsa_rulebook_strict_ir()`. Demonstrates the canonical
  "matches → non-compliant" categorical leap as an external,
  auditable rule.
- `ADJ_DEMO_ENGINE_ARM=lenient`: use only
  `tsa_rulebook_lenient_ir()`. Demonstrates a deliberately wrong
  rule (declaring carry-on baggage doesn't make you compliant).
- `ADJ_DEMO_ENGINE_ARM=both` (or `=adversarial`): load both
  fixture rulebooks. Exercises the ADJ16 step 3 dispute detection
  path; the printed dispute count reflects any cross-rulebook
  conflicts the proof DAG surfaces.

Unrecognised values fall back to `strict` with a warning on
stderr.

### Output

The new Arm C block prints below the existing side-by-side:

```text
[arm C] running the deterministic engine arm with 1 rulebook(s) (no LLM)...
--- ARM C: deterministic engine ---
rulebooks:       1 attached
verdict:         RESOLVED: 1 answer(s)
dispute count:   0
KB attribution:  2 fact(s), 1 rule(s) from 2 source(s)
  answer 1: query=Compound { functor: "compliant", args: [Atom("passenger_a")] }
```

The structured outcome is still available via `ADJ_DEMO_AUDIT=1`
which dumps the full audit trail (including the engine arm's
clause-provenance attribution) as JSON.

### Rationale

ADJ16 step 5 landed the library API; v0.11.1 plugs it into the
CLI so users see all three arms together. The CLI flag is opt-in
to keep the no-knobs invocation unchanged for callers who only
want Arms A and B.

### Compatibility

- Arms A and B are unchanged.
- Default invocation (no `ADJ_DEMO_ENGINE_ARM` set) prints the
  same two-arm output as v0.11.
- The `ADJ_DEMO_AUDIT` flag, when combined with
  `ADJ_DEMO_ENGINE_ARM`, dumps the full audit trail from Arm B as
  before. (Future iteration could fold Arm C's audit into the
  same dump.)

## [0.11.0] - 2026-05-12 — Engine Arm (ADJ16 step 5)

### Added

A third arm — the **Engine Arm** — that runs the canonical TSA
source through `adjudication_pipeline::run_with_rulebooks` to
produce a verdict deterministically. **No LLM is called at answer
time.** Arm A and Arm B both involve LLM calls (Arm A directly,
Arm B for ADJ04/05 checkers). The engine arm uses only the logic
engine over a merged KB built from the source IR + caller-supplied
rulebooks.

- `EngineArmReport { verdict_summary, pipeline_output, dispute_count }` —
  the engine arm's output type. Preserves the full
  `PipelineOutput` (audit trail + clause provenance + disputed
  answers) so a reviewer can reconstruct the derivation step by
  step.
- `run_engine_arm(cfg, rulebooks)` — the entry point. Takes a
  `DemoConfig` and a slice of `(IRDocument, RulebookProvenance)`
  pairs; returns an `EngineArmReport`. Always populates the
  audit-trail clause-provenance table (via
  `run_with_rulebooks`).
- `tsa_rulebook_strict_ir()` — fixture rulebook IR with a single
  definitional rule: `non_compliant(passenger_a) :- prohibited(matches).`
  Demonstrates the "matches → non-compliant" categorical leap as
  an *external, auditable* step rather than a hidden LLM inference.
- `tsa_rulebook_lenient_ir()` — fixture rulebook IR with a single
  definitional rule: `compliant(passenger_a) :- carry_on(1).`
  Deliberately wrong for the canonical TSA case (declaring carry-on
  doesn't make you compliant), used to demonstrate the dispute
  detection path from ADJ16 step 3 when merged with the strict
  rulebook.

### Rationale (ADJ16 step 5)

[ADJ16](../../../specs/ADJ16-engine-programmatic-adjudication.md)'s
"Implementation sequence §5" calls for a TSA demo arm that runs
the engine over the (eventually adversarially-elicited) rulebook.
Step 5 lands the lib-level wiring: any caller can now construct
the engine arm from the source IR + rulebook IRs, and the verdict
is byte-for-byte reproducible across hardware and time. The next
sub-step (step 5.5, follow-up) wires this into the demo binary as
an opt-in arm so users can run all three arms side-by-side via the
CLI.

### Tests

7 new tests added (31 lib total, all passing):
- both fixture rulebooks decode to one definitional rule each
- engine arm with strict rulebook only — verdict, dispute count,
  provenance attribution
- engine arm with lenient rulebook only — verdict + attribution
- engine arm with both rulebooks — both attributions land in the
  provenance table; documents the wiring soundness even when the
  source query happens to single out one rulebook's contribution
- engine arm with no rulebooks — engine runs without error,
  source-fact provenance attributed to document id
- verdict summary is human-readable (starts with "RESOLVED")

### Compatibility

- Arms A and B are unchanged.
- New API is additive: `EngineArmReport`, `run_engine_arm`,
  `tsa_rulebook_strict_ir`, `tsa_rulebook_lenient_ir`.
- `main.rs` is unchanged — the engine arm is library-only in v0.11.
  Wiring it into the CLI binary will be a follow-up PR with its
  own env-var flag.

### Note on the demo binary

The engine arm is currently exposed only through the library API.
A follow-up PR will wire it into `main.rs` so users can run all
three arms side-by-side via `ADJ_DEMO_ENGINE_ARM=1` or similar.
The library-first split keeps this PR's review surface small and
lets the lib API stabilise before the CLI commits to a specific
env-var shape.

## [0.10.1] - 2026-05-12 — Wire `ADJ_DEMO_CACHE_DIR` through Stage 0

### Fixed

The `ADJ_DEMO_RULEBOOK_MODE=elicit` and
`ADJ_DEMO_RULEBOOK_MODE=adversarial:...` paths now respect
`ADJ_DEMO_CACHE_DIR`. Previously both paths constructed raw
`OllamaClient`s without wrapping them in
`llm_cache::CachingClient`, so the cache directory was honoured for
Arm B's full pipeline but bypassed for the rulebook-elicitation
Stage 0. A repeated adversarial bench paid the full ~250 s × N
elicitation cost on every answerer iteration instead of replaying
from disk.

Surfaced by [ADJ17 §Caveats(5)](../../../specs/ADJ17-adversarial-rulebook-empirical-results.md).
The cost mattered most for the 5-answerer adversarial bench, which
was paying 4× the elicitation cost it should have.

### Implementation

A new `cached_client(inner, cache_dir)` helper in `main.rs` mirrors
the private `wrap_with_cache` in `lib.rs`, wrapping each
`OllamaClient` in a `CachingClient::with_disk_persistence` (when a
cache_dir is set) or a memory-only `CachingClient::new` (when it
isn't). Both Stage 0 paths now route their per-model clients
through this helper before composing them into a `GatewayConfig`.

The fix is observable as a shorter Stage 0 log timing on the second
run of any adversarial bench against the same cache directory: the
cache hit replays elicitation responses without an HTTP round-trip.

## [0.10.0] - 2026-05-12 — Adversarial multi-model elicit-mode wiring

### Added

`ADJ_DEMO_RULEBOOK_MODE=adversarial:model1,model2,...` — elicit
rulebooks from N independent models and inject the
provenance-tagged merged text into Arm A's system prompt. Builds
on `adjudication_rulebook::acquire_rulebook_adversarial` (added in
that crate's v0.2) and the existing elicit-mode plumbing (v0.9).

The demo binary parses the comma-separated model list, builds one
OllamaClient + GatewayConfig per model, and dispatches the
adversarial elicit. Each model's outcome is logged with a checkmark
or cross plus byte count and validation status:

```text
[stage 0] adversarial elicitation across 2 models: gemma4:latest, llama3.1:8b
[stage 0] adversarial elicit: 2/2 models succeeded (0 failed)
[stage 0]   ✓ `gemma4:latest`: 1938 bytes (validation=FAILED (...))
[stage 0]   ✓ `llama3.1:8b`: 1709 bytes (validation=FAILED (...))
```

A reviewer reading Arm A's answer can grep the cited rule back to
the `=== RULEBOOK FROM <model> ===` section header in the audit
trail and see which model produced it. A rule appearing in only
one model's section is lower-trust than one in both.

`ADJ_DEMO_DUMP_RULEBOOK=1` works in adversarial mode too — dumps
the full provenance-tagged merged rulebook between markers.

### Why this matters

ADJ15's empirical results showed single-model recursive elicitation
flips verdicts at 3B+ but every elicited rulebook had at least one
fabrication (the matches-as-flammable-materials leap, the
fabricated doctor's-note exception, the wrong knife-length limit).
Multi-model elicitation reduces this risk: a fabricated rule that
appears in only one model's training shows up as a single-source
rule in the merged text, where reviewers can spot it. Rules
multiple models produced independently are higher trust.

24 unit tests still pass. Demo binary smoke-tested with
`adversarial:gemma4:latest,llama3.1:8b`.

## [0.9.0] - 2026-05-12 — elicit-mode wiring + visibility fixes

### Added

- **`ADJ_DEMO_RULEBOOK_MODE=elicit`** — the binary's main loop now
  calls `adjudication_rulebook::acquire_rulebook` against the
  configured model BEFORE Arm A runs, then injects the elicited
  rulebook text into Arm A's system prompt. This is the recursive
  use of the framework: the model is judged against rules it just
  produced from its own weights, audited by the same checker
  discipline that protects extracted facts.

  Flow when `ADJ_DEMO_RULEBOOK_MODE=elicit`:
  ```text
  [stage 0] elicit_rules + decompose_text + validate
            → Tentative Rulebook + audit_trail
  [arm A]   run_raw_arm(cfg with rulebook_text=Some(rb.source_text))
  [arm B]   run_pipeline_arm — unchanged
  ```

- **`ADJ_DEMO_DUMP_RULEBOOK=1`** — companion to elicit-mode. When
  set, prints the elicited rulebook text to stdout between
  `----- BEGIN RULEBOOK -----` and `----- END RULEBOOK -----`
  markers. Useful for reviewers who want to see what each model
  produced.

### Fixed

- **Stage 0 failures now print to stdout too**. Previously the
  elicitation failure message went only to stderr, which benchmark
  scripts that capture only stdout treated as silent success. Now
  the failure log line shows up in both streams; benchmark scripts
  can grep stdout for `"[stage 0] rulebook elicitation FAILED:"`.
- The success-path log line now reports the **validation
  diagnostic** when `validation_passed = false`, not just the
  boolean — so reviewers see *why* the rulebook IR failed
  `adjudication_ir::validate` (most commonly a coverage gap or a
  schema-invalid response from the model).

### Note on the qwen2.5:1.5b "silent" failure observed in ADJ15

The first ADJ15 benchmark run flagged a "silent failure" on
qwen2.5:1.5b — the demo binary's stage-0 success log line was
missing but no error was visible. After re-running with the
visibility fix, the failure mode is now clear: the model **collapses
into degenerate repetition** during the rulebook decompose phase,
producing 90+ identical IR nodes (`{"functor": "authority", "args":
[{"atom": "tsoa"}]}`) until it exhausts the output-token budget
mid-string. `complete_json_with_truncation_retry` correctly
surfaces `Gateway(SchemaInvalid { ... EOF while parsing ... })`;
the demo prints the typed error and falls back to no-rulebook
mode. The framework's behaviour is correct (graceful degradation,
clear diagnostic); the failure itself is an upstream small-model
coherence issue, not a framework bug. Recorded for follow-up:
investigating prompt-level anti-repetition mitigations and / or
explicit `max_nodes` caps in `decompose_text`.

### Unused-mut warning fixed

`json_to_ir_flat`'s `node` binding doesn't need `mut` after the
v3 cascade dropped the `part_of` re-write logic. Cleanup.

## [0.7.0] - 2026-05-12

### Added

ADJ06 clarification loop now also covers ADJ03 polarity/modality
failures, not just ADJ02 coverage failures.

- Each iteration of the loop picks the FIRST failing gating check
  and dispatches to the matching retry primitive:
  - ADJ02 coverage   → `retry_decompose_on_coverage_failure`
  - ADJ03 polarity   → `retry_decompose_on_polarity_failure`
- The framework auto-synthesizes a polarity hint when the source
  text contains a common negation token (e.g., "no", "not",
  "never", "without", "denies", "denied", "absent", "ruled out").
  Helps the model recognise that nodes covering negated text should
  be `Denied`, not `Affirmed`.
- The summary line in the side-by-side report now reports WHICH
  checker the loop ended on:
  - `"N rounds — resolved (ADJ02 + ADJ03 both pass)"`
  - `"N rounds — exhausted (ADJ02 fixed, ADJ03 still failing)"`
  - `"N rounds — exhausted (ADJ02 still failing)"`
- New `format_first_adj03_violation` + `build_polarity_hint`
  helpers next to the existing ADJ02 ones.

### Why

The ADJ12 small-model benchmark showed qwen2.5:1.5b recovering
from ADJ02 via clarification, then getting stuck on ADJ03 (likely
on the polarity of "matches"). With v0.7 of the demo + v0.2 of
the clarification crate, the same 1.5B model can now self-correct
on both gating checks within the same `max_clarification_attempts`
budget.

## [0.6.0] - 2026-05-12

### Added

`PipelineArmReport::cache_stats` — aggregate `CacheStats` across
every role's `CachingClient`. Side-by-side report prints a
`cache: N hits / M misses (X% hit rate), K entries` line whenever
any cache activity happened, making the cache's economic value
visible.

Measured against the canonical TSA fixture (gemma4 + llama3.1):

- Cold run: `2 hits / 9 misses (18%)` — the two hits come from
  `render_node` being called by both ADJ04 and ADJ05 with the same
  prompts, so the second checker hits the in-memory cache.
- Warm run with persisted cache: `11 hits / 0 misses (100%)`.

## [0.5.0] - 2026-05-12

### Added

Optional disk-persisted prompt cache. When `ADJ_DEMO_CACHE_DIR=<path>`
is set, every LLM client used by Arm B is wrapped in
`CachingClient::with_disk_persistence`. A first run populates the
cache directory; a second run replays every prompt from disk with
zero round-trips to Ollama.

Verified on the local TSA demo:

- Cold run (gemma4:latest extractor + renderer + nli, llama3.1:8b
  adversary, 9 LLM calls total): ~60 s wall-clock.
- Warm run with the same cache dir: ~0.5 s of actual demo work
  (rest is Rust binary launch).

- `DemoConfig::cache_dir: Option<String>` — opt-in. Defaults to
  `None` (in-memory cache only, which is essentially a no-op for a
  one-shot binary).
- `ADJ_DEMO_CACHE_DIR=<path>` env var to enable disk persistence
  from the binary.
- Side-by-side report header surfaces the cache configuration.

### Notes

ADJ06 retry rounds also benefit: when the model self-corrects an
ADJ02 failure, the corrected prompt is cached so a re-run replays
both the original failed attempt AND the correction at disk speed.

## [0.4.0] - 2026-05-12

### Added

ADJ06 clarification dialogue wired into the LlmExtracted flow. When
the model's first IR fails ADJ02 coverage, the demo re-prompts the
model with the structured violation and the model's previous output,
then re-runs the entire pipeline. This is the **self-correction
loop** that turns small local models into reliable extractors.

- `DemoConfig::max_clarification_attempts: usize` (default 2). Set
  via `ADJ_DEMO_MAX_CLARIFY_ATTEMPTS=N`. `0` disables clarification
  entirely (the v0.3 behaviour).
- `IrSourceTelemetry::LlmExtracted` gains `clarification_summary:
  Option<String>` and `clarification_turns: Vec<DialogueTurn>`. The
  side-by-side report shows the summary (`"1 round (resolved)"` /
  `"2 rounds (exhausted)"`); the audit-trail dump includes the full
  dialogue.
- The pipeline output's `audit_trail.dialogue` is populated with the
  retry turns, so ADJ07 captures the conversation end-to-end.

### What the loop does

1. Initial decompose_text → IR → pipeline.
2. If ADJ02 passed: done.
3. Otherwise: extract the first ADJ02 violation, call
   `retry_decompose_on_coverage_failure` with the prior IR JSON +
   violation description, get a corrected IR back, re-run the full
   pipeline.
4. Repeat up to `max_clarification_attempts` times.

The loop is dormant in the v2-prompt happy path — gemma4 produces a
clean IR on the canonical fixture and ADJ02 passes on the first try.
ADJ06 stands by for the harder cases (longer documents, more
ambiguous spans, smaller models).

## [0.3.0] - 2026-05-12

### Added

ADJ05 adversarial check now actually runs when a second model is
registered. v0.2 had ADJ05 hard-Skipped; v0.3 plumbs a real Adversary
client through the pipeline. With `gemma4:latest` as Extractor/
Renderer/Nli and `llama3.1:8b` as Adversary, the pipeline runs the
full chain ADJ02 + ADJ03 + ADJ04 + ADJ05.

- `DemoConfig::adversary_model: Option<String>` — opt-in. Defaults to
  `None`, which preserves the v0.2 "ADJ05 records Skipped" behaviour.
- `ADJ_DEMO_ADVERSARY_MODEL=llama3.1:8b` — env var to flip it on at
  the binary level.
- The pipeline now registers four roles: Extractor, Renderer, Nli,
  Plausibility (all served by the primary model) plus Adversary
  (served by the second model).
- ADJ05 findings (`Adj05AdversarialFinding`) are surfaced in the
  side-by-side report alongside ADJ04 drift findings. Each entry
  prints the IR's rendering, the adversary's contradicting reading,
  the adversary's explanation of how they differ, and the judge's
  reason for ruling the alternative reading plausible.
- ADJ05 skipped/errored telemetry (e.g., independence violation,
  missing role) surfaced in the report instead of disappearing.

## [0.2.0] - 2026-05-11

### Added

The **full LLM-driven flow** end-to-end. Where v0.1 hand-built the
TSA IR programmatically, v0.2 adds an `IrMode::LlmExtracted` mode
that calls `llm_primitives::decompose_text` to ask the model to
produce the IR, then converts the JSON output into a typed
`IRDocument` via a tolerant parser. The pipeline then runs ADJ02 +
ADJ03 + ADJ04 over the model-generated IR.

- `IrMode { HandBuilt, LlmExtracted }` — selected by
  `ADJ_DEMO_IR_MODE=hand|llm`. Defaults to `HandBuilt` for the
  clean-baseline experience; `llm` drives the full flow.
- `IrSourceTelemetry` recorded on `PipelineArmReport` so the printed
  report shows the IR's provenance (hand-built, LLM-extracted with
  the raw JSON + converter warnings, or LLM-extraction-failed with
  the fallback path).
- `json_to_ir_document` — the tolerant converter. Accepts both
  `kind` and `node_type`, both `term` and `text` field names, walks
  nested `children` arrays (Gemma 4 emits a tree, not the flat list
  the prompt asks for), defaults missing kind/polarity/modality to
  safe neutrals, clamps out-of-bound spans to source length, skips
  degenerate spans, synthesizes a `compliant(passenger_a)?` Query
  node if the LLM omits queries. Every fallback is logged in a
  `warnings` vector and surfaced in the report.
- Side-by-side report now surfaces:
  - The IR's source (hand-built vs LLM-extracted vs LLM-failed) +
    converter warning count.
  - **ADJ02 coverage violations** when the pipeline blocks (e.g.,
    `RootsDoNotTileDocument { missing_ranges: [(2, 3)] }` when the
    model's IR has a 1-byte gap at the space between "1" and
    "carry-on").
  - **ADJ04 round-trip drift findings** with quantified NLI scores
    in both directions (e.g., `source→rendering = 0.95`,
    `rendering→source = 0.10` when the model adds claims not in the
    source).
- `ADJ_DEMO_IR_MODE` env var documented in `main.rs`.
- `ADJ_DEMO_AUDIT=1` now also dumps the LLM-extracted IR (raw JSON
  from `decompose_text`) and the converter warnings before the
  audit-trail JSON.

13 new unit tests cover the converter (well-formed shape, clamping,
default kind, atom term, root-not-object rejection, nodes-array
required, degenerate-span skip, missing-spans fallback,
empty-spans-OK-for-Query, nested-children flattening, node_type
alias, text-as-term fallback, query synthesis).

### Verified end-to-end against `gemma4:latest`

- **Hand-built mode**: ADJ02 + ADJ03 pass; ADJ04 catches the model's
  renderer drifting from source on both Facts (`"1 carry-on bag, "`
  → `"The carry-on bag is available."`, NLI scores 0.10/0.10). Engine
  runs.
- **LLM-extracted mode**: decompose_text produces a nested-tree IR
  with spans 0..2 and 3..24. The converter flattens it. ADJ02 catches
  the 1-byte gap at byte 2 (the space between "1" and "carry-on")
  as `RootsDoNotTileDocument { missing_ranges: [(2, 3)] }`. Pipeline
  Blocks before the engine runs — token-burn avoided.

## [0.1.0] - 2026-05-11

### Added

A/B comparison demo: raw Ollama model vs the full adjudication
pipeline, both fed the same TSA carry-on declaration. Ships as a
library + binary; the binary is the headline
`cargo run -p adjudication-tsa-demo` entry point.

- `DemoConfig { endpoint, model, timeout, source_text }` — the
  configuration knob. Reads from `ADJ_DEMO_ENDPOINT`,
  `ADJ_DEMO_MODEL`, `ADJ_DEMO_SOURCE`, `ADJ_DEMO_TIMEOUT_SECS`.
- `run_raw_arm(&cfg)` — single `OllamaClient::complete` call with a
  TSA-officer system prompt. Returns `RawArmReport` with the answer
  + token counts + latency.
- `run_pipeline_arm(&cfg)` — wraps the Ollama client in a
  `GatewayConfig` against `Role::Renderer` + `Role::Nli` and runs
  `adjudication_pipeline::run_with_gateway` over a hand-built TSA
  IR. Returns `PipelineArmReport` with the four checker outcomes,
  the verdict summary, and the full `PipelineOutput`.
- `tsa_ir_document(&source_text)` — builds the canonical
  `1 carry-on bag, matches.` IR (two facts tiling the document plus
  one query); falls back to a single-fact IR for arbitrary text.
- `format_side_by_side(&raw, &pipeline)` — renders both arms into a
  multi-line human-readable report.
- 6 offline unit tests cover: default text yields the canonical
  three-node IR, non-default text yields fallback IR, empty source
  yields the query-only IR, raw prompt embeds source text + verdict
  cue, default config targets the local Ollama port, the outcome
  formatter handles each variant.

### Notes

- `decompose_text` is intentionally not wired into Arm B yet —
  `adjudication-ir` does not derive `serde::Deserialize`, so the
  primitive's `serde_json::Value` output can't be converted to a
  typed `IRDocument`. The demo uses a hand-built IR until those
  derives ship.
- ADJ05 stays Skipped because the demo uses one model family for
  every role. Pulling a second model and registering it as
  `Role::Adversary` flips ADJ05 from Skipped to Passed/Failed.
- On macOS, default to `ADJ_DEMO_ENDPOINT=http://127.0.0.1:11434`
  rather than `localhost`. The hostname `localhost` often resolves
  to `::1` (IPv6) on macOS and Ollama binds only IPv4, which
  produces `Connection refused` from the bespoke HTTP client.
