- **#14986: twelve hormones, seven pages, and one envelope about beta cells.**
  `biology/hormone-glands.adj` warranted all twelve rows with the INSULIN row's own sentence, so
  recalling progesterone came back proved by *"Beta cells in the pancreatic islets secrete the hormone
  insulin…"*. Each row now carries its own span **and its own locator**.

  ### Seven pages, not one

  Twelve rows are grounded on **seven different pages** — SEER pancreas, thyroid, adrenal, pituitary and
  gonads, plus MedlinePlus cortisol-test and growth-hormone-tests. Per-row locators are not new here
  (`geography/reference-lines.adj` restated one for the tropic rows in #15056); what is different is
  that every row needs one. A locator is therefore part of each
  row's warrant, not a table-level detail, which makes one mistake possible that same-page conversions
  cannot make: **a row that loses its own locator silently inherits the envelope's**, and the envelope
  now points at the endocrine *index* page, which carries none of these sentences. Moving the envelope
  off `pancreas.html` orphaned the two pancreas rows exactly that way mid-conversion; both were given
  explicit locators, and a test now asserts no answer ever cites the index.

  ### Three spans widened, and one of them caught a claim I had already written

  Three rows are stated by a sentence that does not name its own subject, so each is widened per
  [`README.md`](README.md) ("A citation must name its own subject") to the nearest sentence that
  supplies the missing name, and no further:

  | row | what its own sentence says |
  | --- | --- |
  | `thyroxine → thyroid` | *"Internally, **the gland** consists of follicles…"* — names no gland |
  | `melatonin → pineal_gland` | *"**The pinealocytes** synthesize the hormone melatonin…"* — names no gland |
  | `growth_hormone → pituitary` | *"**GH** is made in the pituitary gland…"* — never spells out the hormone |

  The third was nearly shipped un-widened, under a row comment I wrote claiming *"the span defines GH
  as growth hormone in its own first line"*. It does not — the page defines GH two sentences earlier.
  Checking my own comment against the page is what caught it, and the same check then found the
  melatonin row had the identical defect I had just written a comment excusing.

  ### Where the page's wording differs from the atoms

  A draft of this entry called this "two spellings kept deliberately". There are more than two, and
  one of the two was not a spelling at all. The span is always quoted as the page has it — silently
  correcting a quote is how a citation stops being checkable — so atom and span differ in six rows:
  `glucagon` / *"glucagons"*, `estrogen` / *"estrogens"*, `testis` / *"testes"*, `ovary` /
  *"ovaries"*, and `pancreas` / *"pancreatic islets"*, `adrenal_gland` / *"adrenal medulla"*. A mutant
  that "fixes" the glucagons spelling reddens.

  The melatonin row is not one of these: its **locator** looks wrong and is not, citing
  `pituitary.html` because that SEER page covers the pituitary *and* pineal glands.

  ### Measured

  Read out of the shipped `.adj` after the last edit, each span against the page **its own locator
  names**: all twelve occur **exactly once**. Each names its hormone, and names its gland either
  directly or in the page's adjectival or plural form — `pancreatic islets` for `pancreas`, `adrenal
  medulla` for `adrenal_gland`, `testes` for `testis`, `ovaries` for `ovary`. Six of the twelve are of
  that second kind, so "each span contains its gland" would be false as a literal claim about the
  atoms; the checking script matches the page's form, and the relation is asserted in every case. The
  envelope's framing span occurs once on the index page and is no row's sentence. Controls: a
  near-miss of each span (final word replaced, asserted to differ from the span itself) scores zero.

  ### Pins

  **8 of 8 mutants killed. Two controls, counted separately.** Un-widening each of the three anaphoric
  spans, "correcting" the page's `glucagons`, deleting a row's locator so it inherits the index,
  repointing a row at a different page **on the same host**, rebinding a gland, and breaking only the
  progesterone copy of the sentence it shares with estrogen (each row carries its own copy precisely so
  that works). Controls: unmutated is green, and so is a fabricated envelope — disclosed, because once
  every row overrides `source` and `locator` the envelope's wording is unreachable from any answer.

  **The `trust` gap the entry below disclosed still applies here**, and a draft of this entry dropped
  it. Adjudicated in both directions again: changing the envelope to `trust consensus` reddens the
  suite (inheritance is real, and all twelve rows do inherit), but *deleting* the envelope's
  `trust authoritative` line leaves it green, because `lower.rs:2622` defaults an envelope carrying a
  `source` to `Authoritative`. So the `"trust":"authoritative"` in every needle here cannot
  distinguish inheritance from that default.

  The existing citation assertion pinned only the HOST. That cannot tell one SEER page from another,
  which now matters across seven pages, and it is the shape #15139 found rotting undetected in eight
  files. It now pins the whole `"locator":"…","trust":"…"` pair.

  **One harness bug, caught as a bug rather than reported as a gap.** The shared-sentence mutant
  anchored on the row plus a closing brace and missed, because the envelope block sits between the last
  row and the table's brace. Same shape as the two anchor misses earlier in this cascade that produced
  false "surviving mutant" reports; re-anchored on the row's own comment.

