- **#13934 installment 4n: two sentences from two different paragraphs, welded.**
  **1** value line rewritten into **2** `cites`, alongside **26** comment lines added against
  **0** removed, in **1** `.adj` file (28 added / 1 removed lines in total) —
  `physics/energy-conversion-example` — plus one e2e test file. **Six mutations redden.**

  ### What line 160 held

  Two sentences from the U.S. EIA "Forms of energy" page under one locator. Counted under both
  extractors — text nodes breaking only on block elements, and the crude every-tag-is-a-break one —
  with HTML comments stripped in both:

  | | occurrences | where |
  |---|---|---|
  | S1 "For example, chemical energy is converted to thermal energy when people burn wood in a fireplace or burn gasoline in a car's engine." | 1 | **ends** its paragraph — the first tag after it is `</p>` |
  | S2 "When a person rides a bicycle down a steep hill and picks up speed, the gravitational energy is converting to motion energy." | 1 | sits **mid-paragraph** in a different `<p>`, **606 raw characters** later, **6** block-element tags in between. (Corrected by 4o: this row read "ends a different paragraph"; 143 characters of prose follow S2 inside its `<p>`. The S1 row above is correct.) |
  | **the shipped 257-character value** | **0** | — |

  In the collapsed rendered text they are **471 characters** apart, with the paragraphs on
  mechanical and nuclear energy between them. Each now has its own `cites` under the same locator,
  and **both are needed**: S1 states chemical → thermal for burning wood in a fireplace, S2 states
  gravitational → motion for the bicycle. Each says its row outright; neither is inferred — the
  distinction [#14758](https://github.com/adhithyan15/coding-adventures/issues/14758) was filed
  about, checked here rather than assumed.

  **The header already had it separate.** Its evidence block quotes S1 and S2 as two distinct
  quotations, at lines 39/41 and 115/117. Only the machine-readable field welded them.

  The `source` on the sibling "Laws of energy" page was measured too and needed no repair: 179
  characters, one occurrence **byte-exact in the raw HTML**, inside a single `<p>`, no tag anywhere
  within it.

  ### Why it survived: the pin was a host and a trust tier

  `contains("eia.gov") && contains(trust)` — the same shape as the oceans and reference-lines pins
  that 4l and 4m replaced, and satisfied by the welded string exactly as happily as by the two real
  spans. The needle is now the whole citation object as the serialiser emits it, closing on the
  corroborations `]`, so it bounds the source text, both locators, the trust tier and the
  corroboration set at once.

  Its string appears **eight** times, at eight distinct JSON paths — `citations/[0]` and `steps/[0]`
  for each of the four queries. That was counted before the needle was trusted, and each of the six
  mutants drove all eight to zero together: echoes of one field, not the independently-driftable
  copies of [#14745](https://github.com/adhithyan15/coding-adventures/issues/14745).

  ### One instrument note, as a count with its scope

  Of **20** body sentences drawn from `<p>` elements on the "Forms of energy" page, the block
  extractor found **20** and the crude one found **18**; both found **0** of the 20 negative
  controls (the same sentences with one word swapped). Neither sentence quoted here is among the
  two the crude extractor missed. No rule about *when* it misses is asserted —
  [#14752](https://github.com/adhithyan15/coding-adventures/issues/14752) says that population is
  to be measured, and this is one page's count.

  The controls themselves needed two corrections before they could say anything. The first version
  used each page's `<title>` as the positive needle — but both extractors drop `<head>`, so that
  control could never pass on any page and read 0/0 where the real sentences all read 1/1. The
  second used a single body sentence, which passed under `block` and failed under `crude` — and a
  single needle cannot separate "the instrument is broken" from "the instrument is wrong about this
  one string". Hence the population of twenty.

  ### The review found the same shape in the assertion message

  The message read "carries the U.S. EIA citation, its two corroborations, **and nothing else**".
  The needle bounds the citation *object* — source text, both locators, trust tier, and the
  corroboration set, closing on the `]` — but nothing asserts that `citations` holds exactly one
  element, or that no other citation object appears in the output. The message described a property
  the test does not check, which is what
  [#14758](https://github.com/adhithyan15/coding-adventures/issues/14758) was filed about one
  installment ago — here committed by an assertion message rather than a comment.

  The fix says what is checked rather than adding an assertion. Pinning the count instead would be
  a stronger test, but every input that could falsify it — a duplicate `source`, a second table with
  the same relation name — is rejected by the lowerer before it reaches the output, so no mutant
  could ever be observed to trip it, and an assertion never observed to fire is decoration. The
  comment now also records what the needle does *not* bound.

  ### Mutants

  `RESTORE` (the 257-character weld put back), `DROP-S2`, `SWAP`, `WORD` (`wood` → `coal`),
  `LOCATOR-TAIL`, `EXTRA-CITES` (under `https://evil.example/facts`). Each turns the real suite red;
  none is vacuous; none crashes the CLI. `RESTORE` is caught by the no-weld assertion and the other
  five by the citation object, so **both assertions have a live positive control** — the ordering
  4m established, applied here from the start. The shipped `.adj` was restored byte-identically
  afterwards (sha256 checked).

