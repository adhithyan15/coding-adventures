# ADJ93 — Opus-spider + Haiku-reason: does a better spider fix Haiku? (the CAS division-of-labor test)

Hypothesis: in open-book mode, if a *better spider* (Opus doing retrieval + source→IR decomposition)
lets *Haiku* reason its way to the answer, then the expensive model's job is one-time source
digestion into the CAS and the cheap model does the repeated reasoning — strengthening the CAS case.

Design: split the framework into **spider** (search the web + decompose sources into a clean grounded
IR) and **reasoner** (reason over that IR only, no new search). 4 arms per item, vary who spiders
while Haiku always reasons: `plain-Haiku` (floor); `Haiku→Haiku`; **`Opus→Haiku` (the test)**;
`Opus→Opus` (ceiling). 5 ADJ88 Opus-failure items, N=2. Open-book.

## Result
| arm | correct /10 |
|---|---|
| plain-Haiku (no retrieval) | 1 |
| Haiku→Haiku | 2 |
| **Opus→Haiku** | **2** |
| Opus→Opus | 2 |

Per-cell grade matrix (the binary count hides the real signal — quality differences):
| item | plain-H | H→H | O→H | O→O |
|---|---|---|---|---|
| PIE | inc | inc (*wrong root*) | **partial** | partial |
| BAR | 1/2 | correct | correct | correct |
| ferrite | inc | inc | inc | inc |
| Spin bordism | inc | inc | inc | inc |
| Al(OH)₃ | inc (1.2e-7) | inc (1.2e-7, *drops K_f*) | inc (5.8e-3, *uses K_f*) | inc (5.8e-3) |

## Three findings

**1. CAS division-of-labor SUPPORTED: `Opus→Haiku` ≈ `Opus→Opus` on every cell.** Given the same
Opus-built IR, the cheap reasoner (Haiku) landed *identically* to the frontier reasoner (Opus) —
PIE both partial, BAR both correct, Al(OH)₃ both `5.8e-3`, bordism/ferrite both wrong. **The reasoner
can be cheap with no loss; the value lives in the IR.** This is the core CAS thesis: digest sources
into IR once with the big model, reason cheaply many times with the small one.

**2. A better spider lifts Haiku's reasoning QUALITY, but not binary accuracy on this hard set.**
Where the spider mattered, `Opus→Haiku` beat `Haiku→Haiku` in quality:
- **PIE:** Haiku-spider used the *wrong PIE root* (→ `scheweth`); Opus-spider's IR carried the right
  sound-change chain → Haiku-reason produced a structured derivation graded **partial** (inc → part).
- **Al(OH)₃:** Haiku-spider's IR let Haiku *drop K_f* again (→ `1.2e-7`); Opus-spider's IR forced K_f
  in → Haiku got `5.8e-3` (right model, wrong final value).

Neither converted to *correct*, so binary accuracy tied at 2/10. The only item all framework arms
nail is **BAR**, which *either* spider retrieves — so Opus-spider adds nothing there.

**3. BOUNDARY — the spider/reasoner SPLIT underperformed the integrated framework on derivation.**
ADJ90's *integrated* open-book framework got **Spin bordism 2/2**; here the clean spider→IR→reasoner
handoff got **0/2 — including `Opus→Opus`**. Hypothesis: derivation-bound problems need retrieval and
reasoning to *interleave* (fetch a fact, reason, fetch more); a one-shot handoff loses that iterative
refinement. (Caveat: bordism is high-variance at N=2, so this is suggestive, not proven.)

## Synthesis
- **The CAS case is real for retrieval-bound problems:** Haiku reasoning over an Opus-built IR matches
  Opus itself (finding 1) — validating "big model populates the CAS, small model reasons cheaply."
- **A better spider helps** (finding 2), but lifts *quality* more than *binary accuracy* on genuinely
  hard items, and adds nothing where the cheap spider already retrieves the fact (BAR).
- **It's bounded** (finding 3): the clean split is weaker than an integrated retrieval↔reasoning loop
  for *derive*-bound problems. The CAS pays off where the work is fetch-and-read-off, not derive.

## Honest caveats
- **N=2 on a 5-item adversarial set (Opus failures)** — low absolute accuracy (≤2/10) caps the
  binary-accuracy signal; the *qualitative* differences (partial vs incorrect, `5.8e-3` vs `1.2e-7`)
  are the real evidence and are robust within this run.
- The reasoner was instructed to use only the provided IR (soft constraint); the spider used live web
  search (open-book).
- Bordism's split-vs-integrated regression is the most interesting datum but also the noisiest item.

## Next
- Re-run on **retrieval-bound items** (where the CAS division of labor is expected to shine) at larger
  N to quantify the `Opus→Haiku` vs all-Haiku quality lift cleanly.
- Test an **integrated** cheap-reasoner-over-cached-IR loop (Haiku re-queries the CAS mid-reasoning)
  to see if it recovers the derivation items the one-shot split lost.
