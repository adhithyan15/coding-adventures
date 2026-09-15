- **#14758: two rows asserting a superlative, warranted by a sentence about the equator.**
  `geography/reference-lines.adj`'s `tropic_of_cancer → northernmost_sun_overhead` and
  `tropic_of_capricorn → southernmost_sun_overhead` both assert a BOUND — the furthest north
  and south the noon Sun can stand overhead. The table envelope states no bound at all; its
  `source` is about the equator. Every recall of either row shipped NOAA, an `authoritative`
  tier, and that irrelevant sentence.

  Fixed with **RS-5e per-row provenance** (`ADJ-TABLES.md:71`) — the rule already specified and
  already shipped in `environment/air-quality-index.adj`. No language change, no spec change.
  Each row now carries the `+23.5°/−23.5°` span as its `source`, the "Outside the tropic zones
  … the sun is never directly overhead" span as a `cites`, and **`trust inferred`**, because the
  superlative is REASONED from the two together rather than read off either.

  **The same sentence now carries two different tiers in this stdlib, and both are correct.** It
  is `authoritative` in `reference-line-degree.adj` and `reference-line-hemisphere-location.adj`,
  where it states its row outright (a latitude; a hemisphere). It is `inferred` here. The tier
  describes the claim, not the sentence.

  ### What was measured, and over what scope

  Across **all 1,330 `.adj` files under `code/specs/data`**, whitespace collapsed on both sides,
  comment text taken from lines starting `%` and machine values from `source`/`cites` fields.
  Counted **on the parent commit**, before the rows were annotated; the arrow is what this change
  did to each count. A single present-tense number would have been falsified by the commit
  carrying it:

  | span | in machine values | in comments |
  | --- | --- | --- |
  | the `+23.5°/−23.5°` sentence | **2 → 3 files** | 3 files |
  | "… the sun is never directly overhead" | **0 → 1 files** | 1 file |

  So only the second span had to come out of the comments; the first was already shipped, twice.
  #14758's own table reports the first as appearing in no machine value — true of this file, but
  the issue did not say that was its scope. Corrected there.

  A line-based `grep` for the second span returns **zero** corpus-wide, because the comment block
  wraps and the sentence spans two lines. The zero was the instrument, not the absence. Controls
  for that scan: a known machine value reported in both buckets, a comment-only measurement note
  reported in comments and zero in values, a nonsense needle reported zero in both.

  ### The `locator` is restated deliberately

  RS-5e inherits field by field. This table's envelope locator is the oceanservice `latitude.html`
  page, but the row's span is NESDIS. Omitting the row's `locator` ships a NESDIS sentence under a
  `latitude.html` address — **exit 0, empty stderr, no diagnostic**. Hit on the first attempt and
  caught only by reading the output. `air-quality-index.adj` cannot hit it because all six of its
  rows come from one page; this is the first table whose rows span two. Filed as #14987, together
  with the measurement that per-row `cites` ACCUMULATE onto the envelope's rather than replacing
  them (corroborations here go 3 → 4).

  **Stated plainly, because it is the limit of this fix:** because `cites` accumulate, the tropic
  answer still ships spans that have no bearing on it — corroboration `[0]` is the prime-meridian
  sentence, about longitude. The change moves the irrelevant-warrant problem off `source`, where
  it decided the answer's warrant and tier, into `corroborations`, where it is noise. It does not
  remove it. Closing that needs the envelope decision open on #14986, or the `cites` semantics
  question on #14987.

  ### Pins, and a first draft that proved nothing

  Two tests, one per row, each in its **own program** so the output holds exactly one answer and
  a needle necessarily belongs to that row. Each pins the whole `source`/`locator`/`trust` object
  key-anchored, asserts the second span reaches the answer, and carries a named negative that the
  equator sentence is not the warrant.

  **The first draft queried both rows in one program and was worthless.** Mutating the
  `tropic_of_cancer` row's `locator`, its `cites`, or its `source` each left the suite GREEN — the
  untouched `tropic_of_capricorn` row satisfied every needle. Only the `trust` mutant reddened,
  and only because a global negative caught it. A citation pin not bound to a binding proves just
  that a string exists somewhere.

  After splitting, each of these mutations of the `tropic_of_cancer` row reddens the cancer test:
  drop `trust inferred` (the row inherits `authoritative`); drop the row's `locator` (it inherits
  `latitude.html`); drop the `cites`; swap `source` back to the equator sentence; delete the whole
  annotation block. Perturbing `trust` on the **capricorn** row reddens the capricorn test alone
  and leaves the cancer test green — the per-row isolation the first draft lacked. Restoring the
  file returns the suite to green, so the harness is not simply reporting red.

  One gap, named rather than left implicit: moving a row's `cites` up to the envelope keeps both
  tests green, because it serialises identically as a corroboration.

