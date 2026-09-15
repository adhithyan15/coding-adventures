- `physics/wave-types.adj` — all 10 rows converted to per-row provenance (RS-5e, #14986). The
  MECHANICAL membership sentence was this table's `source`, the field that carries the tier,
  for every row, so `? wave_family(gamma, $F)` came back proved by *"Water waves, sound waves,
  and waves on a rope are all examples of mechanical waves."* — **a sentence that names no
  electromagnetic band at all.** Seven of the ten rows were in that position; they now carry
  the page's electromagnetic caption, which names all seven.

  **Both sentences are on the same page**, so the locator was never wrong and a hostname
  assertion could never have caught it — only the span was. That is why the shipped test passed
  unchanged through the conversion, and it is recorded in the test as such.

  The envelope becomes the mechanical-wave DEFINITION, and **that choice is disclosed rather
  than dressed up as neutral framing**: this page frames each family separately and has no
  sentence framing the table as a whole. What the framing slot requires is that the envelope
  not mis-warrant a row, and this sentence names no specific wave — checked against all ten row
  keys **in the page's own wording**, where `xray` is "X-rays" and `rope` is "waves on a rope".

  All three spans confirmed verbatim, each with a negative arm absent. The two
  membership sentences occur exactly once in the document; the definition sentence occurs once
  in the page's prose and four more times in `<head>` metadata (meta/og/twitter descriptions
  and JSON-LD), all copies of the same caption — "exactly once" was the first wording and is
  true of the prose, not of the document. Block structure checked first: the membership sentences are the tails of two `<p>`
  figure captions, ordinary prose, no list items.

  `facts_wavetypes_e2e.rs`: adds 3 tests (5 total), including the #15193 structural check (three mechanical
  rows, seven electromagnetic, no row with a third sentence). 18 of 19 mutants killed; **no survivor, and nothing to declare**: the ten rows
  no longer repeat the envelope's URL, so the unkillable "drop a row's locator" mutant does not
  exist. Review raised that redundancy and I deferred it to #15197 as an open convention
  question — it was not open. `geography/reference-lines.adj` already ships the rule (restate
  when the row's page DIFFERS from the envelope's, inherit when it is the same), and every row
  here is on the one NASA page.
  rows, seven electromagnetic, no row with a third sentence). 18 of 19 mutants killed; the
  single survivor is equivalent — every row repeats the envelope's own URL, so a dropped row
  locator is byte-identical while a changed one still dies. **That declaration now ships in the
  harness**, beside the assertion it concerns; review caught the same omission one table
  earlier (#15191), where it had lived only in the changelog.
  single survivor is declared in the harness in advance as equivalent — every row cites the
  envelope's own URL, so a dropped row locator is byte-identical while a changed one still
  dies.
