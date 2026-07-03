# ADJ92 — closed-book lift: is the knowledge latent, and does the framework help without the web?

Direct test of two questions: (1) does Opus's training data most likely already contain the facts
needed for these HLE items (knowledge latent in the weights)? (2) with the web **stripped**
(closed-book), does the framework still lift Opus? Motivation: the ADJ90 batch (plain 2/10 →
framework 6/10) was technically **open-book** — the `general-purpose` agent has WebSearch, and BAR
was solved by searching. So that lift could not be cleanly attributed to reasoning discipline.

Setup: same 5 ADJ88 Opus-failure items, every prompt prefixed CLOSED-BOOK (no tools/web, internal
knowledge + reasoning only), two arms (plain-Opus vs framework-Opus = convergence-controlled +
grounded adversarial-read support gate), N=2 each. **Closed-book verified: URL-leaks = 0** (no
answer in either arm cited a web source).

## Result
| item | OPEN-book plain | OPEN-book framework | CLOSED-book plain | CLOSED-book framework |
|---|---|---|---|---|
| Spin bordism | 0/2 | **2/2** | **1/2** | 0/2 |
| Al(OH)₃ | 0/2 | 1/2 | 0/2 | 0/2 |
| ferrite | 0/2 | 1/2 | 0/2 | 0/2 |
| BAR (recall) | 2/2 | 2/2 | 0/2 | 0/2 |
| PIE | 0/2 | 0/2 | 0/2 (1 partial) | 0/2 (1 partial) |
| **TOTAL** | **2/10** | **6/10** | **1/10** | **0/10** |

## Q1 — knowledge latent in the weights? YES (confirmed)
Closed-book, with no web (URL-leaks = 0), **plain-Opus derived Spin bordism = Z⁵ correctly** — a
graduate algebraic-topology computation that is *not* retrievable, produced from memory with a clean
characteristic-number rank count (2+1+2 from H¹²/H⁸/H⁴ paired with Ω₀/Ω₄/Ω₈^Spin). The constituent
facts (Ω₈^Spin = Z², the cohomology of BG₂, the pairing structure) are in the weights. PIE came in
*partial* closed-book. The knowledge is present; it is surfaced *unreliably* (1/10).

## Q2 — framework lift closed-book? NO (and it corrects the ADJ90 story)
**The entire ADJ90 2→6 lift was open-book. Closed-book it vanishes and slightly inverts: plain 1/10,
framework 0/10.**

- **The smoking gun is Spin bordism.** Open-book the framework *won* it 2/2; closed-book **plain got
  it right (1/2) and the framework got it wrong both times**, its decompose→audit→re-reason loop
  *derailing a correct one-shot derivation* down to `Z` / `Z²⊕Z/2`. So the framework's bordism win
  was **retrieval-assisted**: open-book it could ground the AHSS component facts in looked-up
  sources; closed-book it can't, and the support-auditor — unable to confirm a correct-but-
  ungroundable fact like `Ω₈^Spin = Z²` — pushes the solver toward a "safer", wrong standard value.
- **BAR collapses 2/2 → 0/2** in both arms (pure recall; no web → no answer). Confirms BAR's
  open-book correctness was pure retrieval, gate-neutral.

## Interpretation
The framework's *measured accuracy lift is an open-book / retrieval-grounding phenomenon*, not a
closed-book reasoning-discipline one. This is consistent with its design — it is a grounding-and-
defensibility instrument, and closed-book final-answer accuracy is the axis it is built to lose. But
the bordism reversal is **sharper than "built to lose"**: closed-book, the open-book gate can
*actively degrade* a model that would otherwise reason correctly one-shot, because the support-
auditor (lacking retrieval) cannot distinguish a correct-but-ungroundable intermediate from a wrong
one, and "corrects" the right one away.

So: knowledge latent — yes. Framework surfaces it *without retrieval* — no; retrieval is doing the
work in the open-book wins. The framework's value is **open-book auditable/defensible reasoning**,
not closed-book accuracy.

## Honest caveats
- **N=2/item is very noisy.** `1/10` vs `0/10` is within noise — this does NOT establish that the
  framework *reliably* hurts closed-book. The robust claims are: (a) latent knowledge **confirmed
  present** (plain closed-book derived bordism); (b) the accuracy lift is **open-book only** (does
  not survive web removal); (c) the bordism reversal is a real, concerning signal.
- Al(OH)₃ is high-variance (framework right in ADJ90/91, wrong both times here) — closed-book N=2
  can't separate signal from noise on it.
- Closed-book was enforced by prompt + verified by URL-leak=0; it is a soft constraint, but no web
  source appeared in any answer.

## Next
- Re-score this run for **defensibility** (the axis the framework targets), not just final-answer
  correctness — the closed-book framework answers are more structured/auditable even when wrong.
- A clean **open-book A/B** that logs *which intermediate facts the framework retrieves* on bordism,
  to quantify how much of the open-book lift is retrieval-grounding vs reasoning structure.
