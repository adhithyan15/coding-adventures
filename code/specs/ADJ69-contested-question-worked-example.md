# ADJ69 — Defensible Output on a Contested Question (worked example)

> **Status (2026-06-04):** A worked example of the ADJ68 open-book defensibility pipeline
> on a *user-chosen* question whose honest answer is **"there isn't a single one."**
> *"In Greek mythology, who was Jason's maternal great-grandfather?"* — a question where
> bare recall confidently hallucinates and the framework produces a cited, branch-aware,
> auditable map of the genuine answer space. Implementation:
> [`pipeline/jason.workflow.js`](data/adj57/pipeline/jason.workflow.js); result:
> [`pipeline/jason-results.json`](data/adj57/pipeline/jason-results.json). Demonstrates
> both paper pillars at once on a fresh case.

## 1. Why this question is the cleanest test

Two independent ambiguities make a single confident name a *hallucination*:
1. **Definitional:** "maternal great-grandfather" can mean the father of the maternal
   *grandfather* **or** of the maternal *grandmother* — two different people.
2. **Source disagreement:** ancient authors disagree on Jason's *mother* (Apollonius:
   Alcimede, daughter of Phylacus; Apollodorus: Polymede, daughter of Autolycus).

So the correct answer is a *set with branch points*, not a name. A correctness-only metric
is impoverished here; **defensibility** is the right axis.

## 2. The run (open-book; grounded trace vs bare recall; adversarial audit)

### Bare recall (closed-book) — confidently hallucinated
Headline answer: **"Cretheus"** — who is Jason's **paternal grandfather** (his father Aeson's
father): wrong on *both* axes (paternal, not maternal; grandfather, not great-grandfather).
It then contradicted its own headline in the reasoning, waffling toward Hermes/Deion. **Zero
citations** on any of its ~6 genealogical links. Auditor: **NOT_DEFENSIBLE** (6/6 links
unsupported; led with a wrong-side ancestor; never surfaced the grandmother-line ambiguity).

### Framework (open-book, grounded) — defensible
A cited map of the answer space, refusing to collapse:

| tradition / line | maternal great-grandfather | source |
|---|---|---|
| Apollonius (mother Alcimede → Phylacus → …) | **Deion** | theoi.com, *Argonautica* I / Apollodorus I |
| Apollodorus (mother Polymede → Autolycus → …) | **Hermes** | theoi.com, *Bibliotheca* I |
| maternal **grandmother** line (Alcimede born of Clymene → …) | **Minyas** | theoi.com, *Argonautica* I |

Answer: *"No single answer. A: Deion. B: Hermes. Grandmother reading: Minyas. Do not collapse
Deion and Hermes."* All **6/6 links cited**; the contested mother flagged `contested=true`
and split per source; both ambiguities surfaced. Auditor: **DEFENSIBLE**.

## 3. Both paper pillars, on one fresh case

- **Accounting (no unaccounted bytes):** bare recall committed *commission* (asserted
  "Cretheus," a relationship not on the maternal line) and *omission* (dropped the
  grandmother branch). The grounded run did neither — no unsourced link, no swallowed
  ambiguity.
- **Correctability (auditable, editable):** every branch is a citable, editable node. A
  classicist who disagrees traces to the exact source and corrects *that link*. And there is
  a real one to correct: the grounded link *"Aeson had wedded his sister Alcimede"* is
  suspect ("his sister" is likely an imprecision — Alcimede is his wife, not sister). The
  framework made that error **findable**; bare recall buried six errors silently. This is the
  edit-and-correct loop in miniature.

## 4. Honest limitation

The auditor flagged the one real weakness: citations are at **document granularity** (the
theoi.com page), not line-level. Every link is checkable, but pinning each claim to the exact
passage is the **citation-precision verifier** (the named next build) — the same 6/7 → 7/7
upgrade ADJ68 needs, here too.

## 5. Takeaway

On a contested question with no single correct answer, the framework converted a
confidently-wrong single name into a **cited, branch-aware, auditable, correctable** answer —
which is what an expert actually wants, and what a correctness-only score cannot reward. The
value is not "got the answer"; it is *"here is the answer space, mapped and cited, with every
disagreement surfaced and every link a human can check and fix."*
