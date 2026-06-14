# ADJ49 — M&A Deal Completion in Adj-Lang: The Investment-Banker Objection Answered

> **The objection** (verbatim, from a real investment banker):
> "If I have to have an analyst cross-check the contract and math
> anyway, what's the point of using AI?"
>
> **The answer** (this PR): a 22-clause Delaware-law M&A rulebook
> with citations on every clause + three real-ish deal vignettes +
> per-case audit documents showing P(deal completes by drop-dead)
> with the proof DAG, active uncertainties, kickback recommendation,
> and counterfactual analysis. The analyst no longer re-reads a
> prose memo. They spot-check facts, verify the rulebook once, and
> trust the engine on every deal afterward.

## Why this milestone exists

ADJ48 demonstrated the framework on a clinical domain (ACS chest
pain). ADJ49 demonstrates it on a financial domain (M&A deal
completion). The point is to show that **the same engine, the same
language, and the same audit-trail discipline handle radically
different domains with no code changes** — only a different
rulebook. The amortized-cost-of-rulebook / per-query-cost-cheap
shape is what makes the framework shippable.

## The rulebook

22 clauses across:
- Antitrust regulatory risk (HSR Second Request, HHI concentration,
  CFIUS jurisdiction, regulated industry, no antitrust concern)
- Deal structure (all-cash vs stock component)
- Financing (committed debt with no market MAC outs vs financing
  condition in the merger agreement)
- MAC clauses (broad industry carveouts vs narrow no-industry vs
  disproportionate-impact qualifier — Akorn v. Fresenius is the
  anchor citation)
- Reverse termination fee structure (>3% vs <2%)
- Shareholder dynamics (activist position, voting opposition,
  major holders locked up)
- Three joint interaction terms (regulatory delay + financing
  condition, all-cash + no-market-out, narrow MAC + concentration)

Every clause cites: SRS Acquiom 2023 Deal Terms Study, FTC/DOJ HSR
annual reports, ABA Strategic Buyer Public Target Deal Points,
Akorn v. Fresenius (Del. Ch. 2018), Kwoka 2015 (Mergers, Merger
Control, and Remedies), CFIUS annual reports to Congress, Wachtell
Lipton M&A Practical Guide, Lazard Shareholder Activism Review, or
SharkRepellent.

## Vignettes and posteriors

| # | Deal | P(closes) |
|---|---|---|
| 1 | Friendly tech acquisition — $4.2B all-cash, low HHI, broad MAC, RTF 4% | 0.975 |
| 2 | Healthcare megamerger — $48B, high HHI, narrow MAC, stock-heavy | 0.100 |
| 3 | Cross-border CFIUS — $1.8B semiconductor, foreign strategic | 0.955 |

## The investment-banker answer in one paragraph

A status-quo LLM produces a prose memo on each deal. The junior
analyst's verification cost is "read the memo, redo the math,
verify every contract reference." Net: AI saves drafting time but
not verification time.

The framework produces a structured posterior + proof DAG. The
analyst's verification cost decomposes:
- **Facts → sources**: linear in observed facts (5–10 per deal).
  Click the citation; confirm against the actual merger agreement.
- **Rule → authority**: verified once when the rulebook was
  adopted. Reusable across every deal.
- **Posterior → math**: re-run the binary; deterministic.
- **Counterfactuals**: `counterfactual(query, kb, &[X])` returns in
  milliseconds.
- **Cross-claim consistency**: type system catches contradictions.

The 22 rulebook clauses are the **one-time amortized cost** that
recoups across every deal the bank evaluates. The analyst's role
shifts from "redo the work" to "verify inputs and trust the
engine."

That is the productivity delta the framework delivers — and it is
not a hand-wave. The captured `output.txt` in
[`code/specs/data/adj49/output.txt`](data/adj49/output.txt) is the
artifact a banker can show their analyst. Every audit document is
8–15 lines of cited facts + posterior + kickback recommendation,
not a 20-page memo.

## Cross-domain validation

The same `logic-engine` 0.6.0 + `adj-lang` 0.2.0 stack that
produced the ACS clinical demo (ADJ48) produces the M&A demo here
without a single line of engine or language changes. **Different
rulebook, different domain, same machinery.** This is the
cross-domain pluggability ADJ38 sketched and ADJ48+49 demonstrate.

## What's deliberately not here

- **Per-test cost in kickback ranking.** Resolving "is there an
  activist position?" is cheaper than resolving "will the FTC
  issue a Second Request?". The current kickback report ranks by
  VOI alone.
- **Multi-target queries.** P(closes) is one question; E[indemnity
  exposure] and E[regulatory delay in days] are separate. Each
  needs its own rulebook query but the architecture supports it
  trivially.
- **Live deal feed integration.** A production deal-room would
  pull merger-agreement-extracted facts via the ADJ29-35 IR
  pipeline. Mechanical wiring; out of scope here.

## Status

- 2026-06-02: ADJ49 runs three vignettes end-to-end against the
  M&A rulebook. Output captured.
- The ADJ47/48/49 sequence demonstrates the full framework — engine,
  language, two domain rulebooks, working from spec to running
  binary. The investment-banker objection has a concrete answer
  shipped to the repo.
- Next: ship to a friendly banker for review, collect feedback,
  iterate on rulebook clauses with their input.

## See also

- [ADJ48 — clinical sibling](ADJ48-mycin-2026-in-adj-lang.md)
- [ADJ47-A through E — the language work](https://github.com/adhithyan15/coding-adventures/pulls?q=adj47)
- [ADJ46 — awkwardness catalogue](ADJ46-acs-rulebook-on-logic-engine-toolchain-shakedown.md)
- [logic-engine](../packages/rust/logic-engine/) — the inference layer
- [adj-lang](../packages/rust/adj-lang/) — the surface frontend
