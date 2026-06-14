# ADJ49 — M&A Deal-Completion Demo: The Investment-Banker Objection

> "If I have to have an analyst cross-check the contract and math
> anyway, what's the point of using AI?"

This directory is the answer.

The framework runs three real-ish M&A deal vignettes against a
22-clause Delaware-law rulebook compiled from authoritative sources
(SRS Acquiom, FTC/DOJ HSR reports, ABA Deal Points studies, Akorn
v. Fresenius, Kwoka 2015, CFIUS annual reports, Wachtell guides).
Every fired contribution in every audit document names its citation.

## Files

```
adj49/
├── rulebook.adj       The Delaware M&A rulebook in Adj-Lang. 22
│                      clauses covering antitrust, deal structure,
│                      financing, MAC clauses, termination fees,
│                      and shareholder dynamics + 3 joint
│                      interactions.
├── vignettes/
│   ├── 01-friendly-tech-acquisition.adj   $4.2B all-cash, no
│   │                                       antitrust concern.
│   ├── 02-healthcare-megamerger.adj       $48B, high HHI overlap,
│   │                                       narrow MAC. The
│   │                                       Akorn-style risk case.
│   └── 03-cross-border-cfius.adj          $1.8B foreign-buyer
│                                           semiconductor deal.
├── src/main.rs        The runner.
├── Cargo.toml
├── output.txt         Captured cargo-run output.
└── README.md          (this file)
```

## What the demo shows

| Deal | P(closes by drop-dead) | At 50% IC threshold |
|---|---|---|
| 01 Friendly tech acquisition | 0.975 | No kickback. Pursue. |
| 02 Healthcare megamerger | 0.100 | No kickback (uncertainties don't change the side of the decision). Pass. |
| 03 Cross-border CFIUS | 0.955 | No kickback. Pursue. |

## The answer to the investment-banker objection

The banker's argument was: AI produces a prose memo; the analyst
has to re-read every claim and re-run every model anyway, so the AI
saves drafting time but not verification time. Net win: marginal.

The argument assumes prose-form output. The framework's output is a
program. The verification cost decomposes differently:

| Verification task | Prose form | Program form (this demo) |
|---|---|---|
| Each fact ↔ source | Search the memo, then go to source | Click the cited URL/source next to the fact, confirm once |
| Each rule | Re-derive from first principles, every time | Verify once when adopting the rulebook (rulebook.adj), reuse across every deal |
| Each derived conclusion | Redo the math in Excel | Run this binary; trust the engine |
| Counterfactuals ("what if X were true?") | Re-read or rebuild | `counterfactual(query, kb, &[X])` — seconds |
| Cross-claim consistency | Manual, brittle | Type system catches contradictions |

The 22 rulebook clauses are the **one-time amortized cost**. Every
new deal contributes only its observed facts (typically 5–10 lines).
The analyst's verification job is now: verify the facts match the
deal documents (linear in facts), trust the engine, and spend their
saved time on the cases where kickback flagged a decision-relevant
uncertainty.

## Specific demonstrations across the three vignettes

**Vignette 1 (friendly tech)**: the framework correctly identifies
this as a high-confidence deal (0.975) without firing kickback.
The 22-clause rulebook is doing real work — the prior is 0.78, and
the deal-specific contributors push it to 0.975 with each
contribution carrying its citation. The IC could approve pursuing
this deal having spot-checked maybe 4 of the 8 fired contributions
against the underlying merger agreement.

**Vignette 2 (healthcare megamerger)**: 0.100. Even with the
financing and shareholder uncertainties unresolved, the kickback
machinery correctly identifies that the answer doesn't change at
the 50% threshold — both worst-case and best-case posteriors lie
below 50%. The IC's decision (pass) is robust. The counterfactual
shows that even assuming the most favorable financing structure,
the posterior only rises to 0.151 — not enough to flip the
decision. **This is the framework saving analyst time on a
clear-no.**

**Vignette 3 (cross-border CFIUS)**: 0.955. The CFIUS contribution
(LR 0.70) does fire and pulls the posterior down, but the
all-cash + no-market-out financing + broad MAC + RTF + no
antitrust profile pushes it back up. No kickback at 50%.

## Per-deal wallclock

Under 50 ms per vignette including rulebook parse. The marginal
compute cost of evaluating a new deal is trivial; the amortized
cost is the rulebook.

## What's deliberately not here

- **Loss-given-failure model.** P(closes) is one question. The
  banker also wants E[indemnity exposure] and E[regulatory delay
  in days]. Both can be expressed as separate adjudication queries
  with their own rulebooks; cross-cutting work for ADJ50+.
- **EMR-equivalent integration.** A production deal-room would
  ingest the merger agreement, antitrust filings, and proxy
  statements automatically. ADJ29-35's IR pipeline can do this; not
  wired up here.
- **Per-test cost in the kickback ranking.** Resolving "is there
  an activist position?" is cheaper than resolving "will the FTC
  issue a Second Request?". The current kickback report ranks by
  VOI alone; ranking by VOI/cost is straightforward to layer on.

## See also

- [ADJ48 — clinical sibling](../adj48/README.md): the same
  architecture on a chest-pain rulebook. The fact that the same
  engine handles both domains identically is part of the answer.
- [logic-engine 0.6.0](../../../packages/rust/logic-engine/CHANGELOG.md)
- [adj-lang 0.2.0](../../../packages/rust/adj-lang/CHANGELOG.md)
- [ADJ46 awkwardness catalogue](../../ADJ46-acs-rulebook-on-logic-engine-toolchain-shakedown.md)
