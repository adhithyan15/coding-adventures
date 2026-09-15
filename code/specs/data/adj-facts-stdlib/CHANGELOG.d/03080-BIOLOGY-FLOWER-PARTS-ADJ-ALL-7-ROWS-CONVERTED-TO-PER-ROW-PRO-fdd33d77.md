- `biology/flower-parts.adj` — all 7 rows converted to per-row provenance (RS-5e, #14986).
  The PETAL sentence was this table's `source`, the field that carries the tier, for every row,
  so `? flower_part_function(ovary, $F)` came back proved by *"Petals attract pollinators and
  are usually the reason why we buy and enjoy flowers."* **The header said so in its own
  words** — the envelope held *"the single cleanest span: the Illinois Extension statement that
  fixes the FIRST ROW"*.

  **Every span here is on the same page**, so the locator was never wrong and a hostname
  assertion could never have caught this — only the span was. That is why the shipped test
  passed unchanged through the conversion, and it is recorded in the test as such.

  **Block structure was checked before any span was cut.** This page's whole content sits in
  one `<td>` of a LAYOUT table and the prose inside is ordinary sentences, so sentence-level
  spans are legitimate: the table markup is layout, not semantics. That check exists because
  flattened text hides the difference — see the correction on #15185, where a span that read as
  a cut sentence turned out to be a complete paragraph followed by an ordered list.

  All eight spans (seven warrants + the framing envelope) confirmed verbatim, each occurring
  **exactly once**, each with a negative arm absent. Two rows share one sentence (`stamen` and
  `pistil` are fixed in the same clause), so each carries its own copy and each is pinned
  separately. The envelope becomes a FRAMING span that names no part — checked against all
  seven row keys, not assumed.

  `facts_flowerparts_e2e.rs`: 5 tests. **A claim here had to be corrected against its own
  code.** The pairing check was documented as deliberately not a loose substring test "because
  'pistil' occurs inside the stigma sentence" — and it *is* that substring test, which admits
  exactly that hazard. It is now labelled what it is: necessary, not sufficient. The property
  it was reaching for is supplied instead by a new test that reads the shipped `.adj`, parses
  its row blocks, and asserts that exactly one span is shared and that it is shared by exactly
  `stamen` and `pistil` — a check against the file rather than between two test literals.

  15 of 16 mutants killed; the single survivor is equivalent — every row here cites the
  envelope's own URL and `row_provenance` assigns `locator` only when a row supplies one, so
  dropping one yields byte-identical output. That declaration now sits **in the harness**,
  beside the assertion it is about, rather than only in this entry.

