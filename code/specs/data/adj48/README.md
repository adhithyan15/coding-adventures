# ADJ48 — MYCIN-2026 in Adj-Lang

End-to-end demonstration of the framework on a real clinical
domain: adult ED chest pain, with the Acute Coronary Syndrome (ACS)
risk question as the target.

The MYCIN-2026 framing: take an actual rulebook compiled from
authoritative sources (AHA / ACC / ESC guidelines + Panju 1998 +
Diamond/Forrester 1979 + HEART Score), run real-ish clinical
vignettes through the IR → ProbLog → run pipeline, and show that
the output is defensible — every claim cites a source the reader
can click and verify.

## Files

```
adj48/
├── rulebook.adj          The canonical ACS rulebook in Adj-Lang.
│                         29 clauses across prior / contributes /
│                         interacts. Every clause cites a source.
├── vignettes/
│   ├── 01-jane-doe.adj         The ADJ36 Jane Doe case, ported.
│   ├── 02-classic-stemi.adj    Unambiguous STEMI.
│   ├── 03-pleuritic-mss.adj    Low-probability pleuritic MSS pain.
│   ├── 04-nstemi-equivocal.adj Three concurrent uncertainties.
│   └── 05-troponin-rules-in.adj Same Jane Doe but with dynamic
│                                 troponin rise.
├── src/main.rs           The runner binary. Concatenates each
│                         vignette with the rulebook, compiles
│                         through adj-lang, runs LR aggregation,
│                         emits a per-case audit document.
├── Cargo.toml
├── output.txt            Captured cargo-run output across all five
│                         vignettes.
└── README.md             (this file)
```

## What the demo shows

| Vignette | Posterior | Kickback at 30%? | What the framework demonstrated |
|---|---|---|---|
| 01 Jane Doe | 0.369 | **Yes** | precipitator uncertainty straddles the decision threshold — band [0.260, 0.594]. Resolving the precipitator could move the answer to either side; the system rightly defers. |
| 02 Classic STEMI | 0.997 | No | overwhelming evidence; no kickback machinery fires. |
| 03 Pleuritic MSS | 0.002 | No | joint interaction term (pleuritic + normal vitals) crushes the posterior. |
| 04 NSTEMI equivocal | 0.734 | (multiple uncertainties; see output) | three concurrent uncertainties; the proof DAG names each citation. |
| 05 Troponin rises | 0.824 | **No** | same precipitator uncertainty as Jane Doe, but no kickback — the troponin rise has already resolved the decision regardless of precipitator. |

The contrast between vignettes 1 and 5 is the framework's actual
value proposition for clinical care: **the system distinguishes
"this uncertainty changes the decision" from "this uncertainty
exists but is no longer load-bearing."** A status-quo LLM cannot
make that distinction because it cannot represent the uncertainty
explicitly in the first place.

## Running

```sh
cd code/specs/data/adj48
cargo run
```

Wallclock: < 100 ms per vignette including rulebook parse.

## How this differs from ADJ36 and ADJ44

- **ADJ36** demonstrated the math (Python LR multiplication) on
  Jane Doe with a custom non-executable rulebook syntax.
- **ADJ44** demonstrated the recursive rulebook derivation pipeline
  with provenance grading.
- **ADJ46** demonstrated the toolchain shakedown on the production
  `logic-engine` crate and catalogued 10 awkwardnesses.
- **ADJ47** dissolved all 10 awkwardnesses across `logic-engine`
  0.3.0 → 0.6.0 and a new `adj-lang` crate.
- **ADJ48** is the first end-to-end use of the full stack on
  multiple cases. Every artifact in this directory is the form an
  actual deployed system would have: a versioned rulebook .adj
  file, a per-case .adj vignette, a runner that emits a clinical
  audit document.

## Sources used in the rulebook

Each clause cites its source in the .adj file. The set of
authorities the rulebook draws from:

- **AHA / ACC / ESC guidelines** (consensus tier): STEMI 2013
  update, NSTE-ACS 2014 / 2015 guidelines.
- **Single authoritative sources** (authoritative tier): Pope JH
  et al., NEJM 1995; Panju AA et al., JAMA 1998; Diamond GA &
  Forrester JS, NEJM 1979.
- **Empirical / cohort studies** (empirical tier): HEART Score
  (Six AJ et al., Neth Heart J 2008); Sandoval Y et al., 2019
  hs-cTn rule-out pathways.
- **Empirical interaction terms** (empirical tier, three of them):
  pressure + diaphoresis synergy; ECG + troponin synergy;
  pleuritic + normal-vitals suppression.

## What's deliberately not here

- **Physician comparison.** The 1979 Yu et al. evaluation of MYCIN
  v. five infectious-disease faculty is the right next-step
  experiment. Wallclock-cheap to do; physician-recruitment-expensive.
  Out of scope for this PR.
- **Recursive rulebook derivation.** ADJ44's pipeline can compile
  this rulebook from the cited PDFs; here it's hand-written for
  reviewability. Both approaches converge on the same .adj file.
- **VOI-driven test ordering.** The kickback report names
  uncertainties but doesn't yet rank them by *test cost* (a 4-hour
  serial troponin vs. a 5-minute history). That's the natural
  follow-on once an EMR adapter exists.

## See also

- [ADJ46 — awkwardness catalogue](../../ADJ46-acs-rulebook-on-logic-engine-toolchain-shakedown.md)
- [logic-engine 0.6.0 changelog](../../../packages/rust/logic-engine/CHANGELOG.md)
- [adj-lang 0.2.0 changelog](../../../packages/rust/adj-lang/CHANGELOG.md)
- [LP19e spec](../../LP19e-likelihood-ratio-aggregation.md)
- [ADJ44 — original MYCIN-2026 reproduction](../../ADJ44-mycin-2026-meningitis.md)
