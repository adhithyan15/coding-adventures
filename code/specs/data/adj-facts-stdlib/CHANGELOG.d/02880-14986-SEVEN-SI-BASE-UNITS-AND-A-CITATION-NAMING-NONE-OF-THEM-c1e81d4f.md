- **#14986: seven SI base units, and a citation naming none of them.** `metrology/si-base-units.adj`
  had one envelope for seven rows, and that envelope says the SI *has* seven base units without
  naming one. A recall of `si_base_unit(mass, $U, $S)` shipped NIST, `authoritative`, and a
  sentence that does not contain the word "kilogram".

  Each row now carries the NIST page's own line for its own unit, via **RS-5e per-row provenance** —
  `{ source "Mass - kilogram (kg)" }` and the six siblings. No row restates `locator` or `trust`:
  all seven come from the one page at one tier, and a row inherits every field it does not write.
  The inherited `authoritative` is correct here because each span **states** its row outright —
  these are read, not reasoned. (Contrast the tropic rows in `geography/reference-lines.adj`,
  reasoned from two spans and carrying `trust inferred`.)

  **This was already scoped and deferred, with the text recorded.** The previous e2e test pinned the
  envelope on a row query and explained why in its own comment — *"a row-binding pin today would
  freeze the #14124 defect into a test"* — then listed all seven per-row spans as present on the
  page and closed with *"Deferred to #14124 with the text recorded, NOT unavailable."* This delivers
  exactly that, and the sound row-binding pin that comment was waiting for is now the test.

  ### Measured on the page's bytes

  Fetched 2026-09-13 **UTC** with **raw urllib, not a summarizing fetcher** — a summarizer has
  misquoted a cited page three different ways before, and a `source` must be verbatim. Each of the
  seven spans occurs **exactly once** in the raw HTML, once under an extractor that breaks only on
  block elements, and once under a crude one that treats every tag as a break — 21 counts, all 1.
  Each is the entire text of one `<li><a>`, which is *why* it survives all three modes: no tag
  falls inside it.

  The separator is SPACE, **U+002D HYPHEN-MINUS**, SPACE — read out of the extracted text rather
  than assumed. Controls, each stated with the mode it holds in: the nonsense span
  `"Zorblatt - quux (zz)"` scores zero in all three; an en-dash variant and a no-spaces variant
  each score zero in all three, so the count is sensitive to the exact characters rather than
  matching loosely.

  **A control of mine did not hold, and the failure is the interesting part.** I offered the
  envelope sentence as the positive control — known present, so the harness must find it. It scores
  **zero in raw HTML**: `<a href=…>NIST SP 1247</a>` splits it, with a trailing `&nbsp;`. Which
  means the thing worth reporting is not the control but its corollary: **the envelope `source` is
  not a verbatim raw-byte span of the page.** It is a de-tagged rendering — true of the text as
  displayed, not of the bytes. That predates this change and is not repaired here; it is named
  because the heading says "measured on the page's bytes", and a reader should know which values
  clear that bar. The seven per-row spans do. The envelope does not.

  ### One consequence, named rather than left silent

  Every row now overrides `source`, so **no answer can cite the envelope sentence** — it defends the
  table's shape and reaches no recall. Its only verbatim pin was removed along with the row-binding
  pin it sat in, and what replaced it asserts the sentence is *absent* from a row answer. A silent
  edit to the envelope text would redden nothing. `locator` and `trust` are still pinned, via the
  inheritance assertions.

  ### Pins

  Two tests, forward and reverse, each in its own program so the output holds exactly one answer
  and a needle necessarily belongs to that row. Each asserts the row's own span key-anchored to
  `source`, asserts the `locator` was actually **inherited** rather than restated, and carries the
  named negative that the framing sentence no longer warrants a row.

  **The `trust` assertion is weaker than it looks, and the test now says so.** `lower.rs:2622`
  defaults a tier to `Authoritative` whenever a `source` is present, so the pin cannot tell
  "inherited from the declared envelope tier" from "silently defaulted"; deleting the envelope's
  `trust authoritative` leaves it green. It does discriminate the other four tiers — setting the
  envelope to `trust inferred` propagates and reddens it.

  **6 mutants, plus a restore-to-green** (the restore is a control, not a mutant — the previous
  entry would have called this "7/7"). Swapping the mass row's span for the envelope sentence,
  dropping its block, changing its hyphen to an en dash, dropping the candela row's block, and
  dropping the shared locator each redden. **Dropping the block on the `time` row — which neither
  test queries — stays green**, which is the cross-row control the previous installment's first
  draft failed.

