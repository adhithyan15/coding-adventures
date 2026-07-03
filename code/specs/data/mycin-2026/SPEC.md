# MYCIN-2026 — full rebuild on the byte-provenance + constraint substrate

A from-scratch rebuild of MYCIN: a clinical-differential expert system where the
**model only decomposes** messy input into typed terms, and a **deterministic
CPU engine** (adj-lang + the constraint solver + the probabilistic differential)
does all the reasoning, produces the diagnosis, names what new data would move
it, and emits a fully cited, machine-checkable proof. The rulebook is *derived
once* under an adversarial gate, *checked in as code*, and reused at **zero
answer-time model calls**.

This supersedes the `mycin-prototype/` proof-of-concept. It keeps the proven
parts (the rulebook clause style, the adj52 grounded corpus, the adversarial
machinery) and adds three things the user asked for:

1. **Rulebooks and dictionaries are a first-class language concept** — written
   once, checked in, `import`ed and reused. No more JSON dictionary + a separate
   linter; the controlled vocabulary lives *in the language* and is enforced at
   compile time.
2. **The constraint solver is in the reasoning loop** — `check` + the minimal
   IIS (track E1) catches *rulebook contradictions* and localizes them to the
   exact conflicting clauses; `constrain` gates dimensional thresholds.
3. **Value-of-information** — the engine reports *what new finding or test would
   most shift the diagnosis*, so MYCIN-2026 doesn't just diagnose, it tells you
   what to order next.

## 1. The five claims (what the rebuild proves)

1. **Golden rulebook** — derive the rulebook once (cold, with the model), reuse
   it across every case at **0 answer-time model calls**.
2. **Cost-to-correct is small** — a wrong/over-counting rule localizes to one
   clause (now also via the **IIS** when rules contradict), and a single CAS edit
   fixes it and propagates to every citing case.
3. **The proof is easy to follow and machine-checked** — every diagnosis cites
   its clauses → source bytes, and the constraint certificate (IIS / solved
   value) renders *under* the verdict step (track E3).
4. **Errors are localizable** — a wrong verdict points at one clause; a
   contradictory rulebook points at the irreducible conflicting set.
5. **Inference is CPU-bound** — the model decomposes; the engine reasons.

## 2. The new language constructs — `rulebook`, `dictionary`, `define`, `use`, `import`

A **rulebook** is the unit of adjudicatable knowledge, so it is a *first-class,
named grammar construct* — not a file convention or a tooling concept. A
**dictionary** (the controlled vocabulary) is likewise a named construct. Both
are grammar-driven (`.tokens`/`.grammar` → `GrammarLexer/Parser`, per the repo's
no-handwritten-lexers rule). Five additions:

```adj
% clinical.adj — a named dictionary, written once, checked in as code
dictionary meningitis_vocab {
    define bacterial_meningitis : hypothesis
        surface "bacterial meningitis", "pyogenic meningitis"
    define csf_gram_stain : finding values [positive, negative]
        surface "Gram stain positive", "organisms on Gram stain", "no organisms seen"
    define csf_neutrophilic_pleocytosis : finding values [high, normal]
        surface "neutrophil-predominant pleocytosis", "PMN predominance"
}
```

```adj
% rulebooks/meningitis.adj — a named rulebook, the grounded knowledge, checked in
import "../clinical.adj"
rulebook meningitis {
    use meningitis_vocab
    prior 0.037 for bacterial_meningitis
        source "Nigrovic 2007 JAMA (PMID 17200475)" trust authoritative
    contributes 85 from csf_gram_stain(positive) to bacterial_meningitis
        source "WHO 2025 NBK614844; sens 85% spec 99%" trust consensus
}
```

```adj
% a case — import the rulebook, observe, ask. The model writes only the observes.
import "rulebooks/meningitis.adj"
use meningitis
observe csf_gram_stain(positive)
observe csf_neutrophilic_pleocytosis(high)
? bacterial_meningitis
? viral_meningitis
```

### Semantics
- **`rulebook <name> { … }`** — a named unit grouping `prior`/`contributes`/
  `interacts`/`uncertain` clauses (and a `use` of its dictionary). The unit of
  knowledge: derived once, gated, checked in, cited as a whole.
- **`dictionary <name> { define … }`** — a named controlled vocabulary.
- **`define <term> : hypothesis [surface …]`** registers a hypothesis term;
  **`define <term> : finding values [v1, …] [surface …]`** registers a finding
  functor with a *closed* value domain. `surface "...", "..."` are the
  decomposer's surface forms (constrain the model; not engine-semantic).
- **`use <name>`** brings a named dictionary (or rulebook) into the current
  scope — a rulebook `use`s its dictionary; a case `use`s its rulebook. Clauses
  may still appear bare at top level (backward-compatible); `use`/blocks add the
  named, composable layer.
- **`import "<relative path>"`** parses the target `.adj` and makes its named
  `rulebook`/`dictionary` blocks (and any bare clauses) available to `use`.
  Resolved relative to the importing file; **idempotent** (a file imported twice
  is included once, by canonical path); **acyclic** (a cycle is a compile
  error); depth/fan-out bounded.
- **Compile-time vocabulary enforcement (replaces the prototype's
  `dict_lint.py`)**: every finding / value / hypothesis used in a
  `prior`/`contributes`/`interacts`/`observe`/`?` must be `define`d in a
  dictionary that is in scope (`use`d, after imports resolve), and a finding
  value must be in its declared domain. Violations are
  `LowerError::UndefinedTerm` / `ValueNotInDomain`. The IR the model emits and
  the rulebook it compiles against share one closed vocabulary by construction.
  "Finding absent" (a value observed) vs "finding not yet observed" (term legal,
  no `observe`) stays distinguishable because the domain is closed.

### Why first-class in the grammar (not tooling)
A rulebook is what knowledge work adjudicates *over* — the thing you author,
version, gate, cite, and reuse. Making it a named language construct means: one
parser, one provenance/CV vector, the dictionary enforced by the *compiler*
(not a side-car linter that drifts), composition (`use`/`import`) with cycle +
scope checks, and `adj-lang-cli` operating on a single composed program — the
same path the differential + constraint engine + proof DAG already take. A
rulebook can then be `check`ed for self-consistency (the IIS localizes
contradictions) like any other constraint system.

## 3. Architecture

```
COLD (once, model + adversarial gate) ───────────────────────────────────────
  grounded LRs (adj52 corpus byte_quotes)
      │  cas_write_gate: per clause, N adversarial readers ask "does this
      │  byte_quote ENTAIL this LR magnitude + direction?" × byte-stability ×
      │  blind judge, + a completeness/discard read (no grounded finding wrongly
      │  omitted). decision-sensitivity gated (ADJ65). → ACCEPT(trust) | KICKBACK
      ▼  rulebooks/meningitis.adj + dictionary.adj  (checked in; CAS-addressed)

WARM (per case, CPU-bound, model only decomposes) ───────────────────────────
  messy clinical vignette (prose)
      │  decompose.workflow.js (LLM, DICTIONARY-CONSTRAINED, decompose-ONLY):
      │  prose → observe <functor>(<value>) lines + a DISCARD list + inference
      │  justifications (ENTAILED|LEAP). The model writes NO diagnosis.
      │  adversarial_read: inference read + DISCARD read × N-reader vote ×
      │  decision-sensitivity.
      ▼  case.adj  (import rulebook + observe… + ? hyp…)
      │  adj-lang-cli: differential diagnosis + proof DAG (each step cited);
      │  check / IIS over the rulebook catches contradictions; `uncertain {…}`
      │  emits the value-of-information report (what to order next).
      ▼  diagnosis + proof DAG + VOI + (if any) the IIS of conflicting rules
         answer-time model calls = 0
```

## 4. Constraint solver in the loop (the user's ask)

- **Rulebook consistency**: a rulebook can carry `constrain`/`check` over its own
  structure (e.g. mutually exclusive hypotheses, value-domain bounds, LR sanity).
  `check` returns SAT or, on contradiction, the **minimal IIS** (E1) — the exact
  conflicting clauses. This is the machine-checked "these two rules contradict"
  that localizes a golden-rulebook bug.
- **Threshold gating**: dimensional findings (`csf_wbc(quantity(1200, per_uL))`)
  drive predicate thresholds via `constrain`, reusing the dimensional + predicate
  machinery already shipped.
- **Feed-a-verdict** (E2): a constraint outcome can itself drive a hypothesis
  (`contributes from infeasible to rulebook_conflict`), and E3 renders the
  certificate under that verdict.

## 5. Value-of-information — "what new data would shift the probabilities"

adj-lang already has `uncertain { e1, e2, … } for <hyp>`: it marks a set of
*unobserved* candidate findings and the LR aggregator emits a VOI report —
for each unobserved finding value, what it *would* contribute to each
hypothesis and how much it would move the differential. MYCIN-2026 surfaces this
as the **"order next"** output: the unobserved finding whose observation would
most change or most confirm the leading diagnosis, ranked, each cited to the
rulebook clause that would fire. No new engine logic — it's the existing VOI
mechanism applied to the closed dictionary's unobserved terms.

## 6. Adversarial reads (the user's ask: "catch issues")

Reused/extended from `run100b` + the prototype, at **both** gates:
- **Cold (rulebook)**: per clause, N independent adversarial readers prompted to
  *refute* ("the byte_quote does NOT entail LR 85 / supports the wrong
  direction"); kill on majority-refute; byte-stability (does the quote survive
  re-extraction?); blind judge on normalized text (format-leak guard, per the
  ADJ99 lesson); completeness/discard read (a grounded finding must not be
  silently dropped). Gated by decision-sensitivity (only adjudicate where it
  flips a decision).
- **Warm (case)**: the decomposition's inferred findings get an inference read
  (ENTAILED vs LEAP) and the DISCARD list gets a discard read (was a mapped
  finding wrongly thrown away?), N-reader majority + decision-sensitivity.

The adversary model ≠ the reasoner; the gate writes to the content-addressed
store only on N-adversary agreement × byte-stability × blind-judge pass.

## 7. Phased PR roadmap (specs-first, each green + security-reviewed + babysat)

**Language foundation (M0–M3) — the `rulebook` concept in the grammar:**
- **M0 — this SPEC** + `dictionary.adj` authored in the new `dictionary { define … }`
  syntax (design artifact; grammar lands M1–M3).
- **M1 — `dictionary` + `define`** in adj-lang. tokens/grammar (regen via
  `regen_grammars`, never `cargo fmt` the generated files), AST, adapter, lower →
  register finding/hypothesis terms with closed value domains; **compile-time
  vocabulary enforcement** (undefined term / value-not-in-domain). Single-file.
- **M2 — `rulebook` + `use`** in adj-lang. Named `rulebook <name> { … }` block;
  `use <name>` brings a named dictionary/rulebook into scope; scope checks
  (`use` of an undefined name → error). Bare-clause programs stay valid.
- **M3 — `import`** in adj-lang. Path resolution relative to the importing file,
  idempotent + acyclic (cycle → error), depth/fan-out bounds; `adj-lang-cli`
  resolves imports before compile. Test: a 3-file dictionary→rulebook→case chain.

**MYCIN content (M4–M8) — after the language foundation; PAUSE for user before M4
(model/workflow + research claims):**
- **M4 — rulebook (cold) + dictionary** for bacterial-vs-viral meningitis,
  derived from the adj52 corpus byte_quotes, authored in the new constructs; the
  naïve correlated-CSF over-count left in deliberately (cost-to-correct proof).
- **M5 — adversarial CAS-write gate**: cold per-clause adversarial entailment ×
  byte-stability × blind judge × completeness; accept→CAS, kickback→report.
- **M6 — warm pipeline**: decompose.workflow.js (dictionary-constrained,
  decompose-only) → case.adj; adversarial inference+discard read; decide via
  `adj-lang-cli` at 0 answer-time calls.
- **M7 — constraint consistency + VOI**: rulebook `check`/IIS contradiction
  detection; `uncertain{…}` VOI "order-next" output wired into the case run.
- **M8 — the five proofs + FINDINGS**: golden-rulebook (derive once → all cases,
  0 calls), cost-to-correct (naïve over-saturation → IIS/DAG localizes → one CAS
  `interacts` fix → recalibrates → propagates), error-localization, audit-trail
  render, CPU-bound. Honest limits documented.

## 8. Reuse (grounded) vs build

- **Reuse**: adj-lang (`prior`/`contributes`/`interacts`/`observe`/`?`/
  `uncertain`), the constraint solver (`check`/IIS/`constrain`), the
  differential + proof DAG + feed-a-verdict + FromSolve (tracks A–E, all
  shipped); adj52 grounded corpus + byte_quotes; run100b adversarial workflows +
  decision-sensitivity gate; the content-addressed store pattern.
- **Build**: `define`/`import` in adj-lang (M1/M2) — the only new language
  surface; the cold gate + warm pipeline wiring for this domain; the proofs.

## 9. Non-goals (this round)

Treatment-recommendation arm; population stratification (peds /
immunocompromised); automated full-text PDF retrieval; LR credible-interval
propagation; a package/namespace system beyond file-path `import` (named
registries are a clean follow-up). Each is noted for later.

## 10. Operational constraints

LLM touchpoints are exactly two (cold rulebook gate; warm decompose+read), run
BATCH=10 (rate-limit safe), against a local model (Ollama; `llama3.1:8b` capable
arm, smaller arms swappable) — everything else is deterministic/CPU. Per PR:
specs→tests→impl→changelog→README; `/security-review` before push; babysit to
green. adj-lang grammar changes: regen via `regen_grammars`, never `cargo fmt`
the generated files (repo lesson).
