- `biology/heredity-term.adj` — all 7 rows converted to per-row provenance (RS-5e, #14986),
  and the cleanest case in this cascade so far: seven rows, seven NHGRI glossary pages, one
  term each. The GENE sentence was this table's `source`, the field that carries the tier, for
  every row, so `? heredity_term(phenotype, $D)` came back proved by *"The gene is considered
  the basic unit of inheritance."* **The header said so in its own words** — the envelope held
  *"the strongest single span (the `gene` row's text, which fixes the first row)"* — an
  accurate description of a defect, with the emphasis on **the first row** added here rather
  than quoted. Six of the seven rows were in that position.

  **The six `cites` are promoted, not dropped**: each was already in the file with its own
  glossary URL, in row order, and is now the `source` of the row it defines, at the envelope's
  tier instead of untiered. Nothing had to be found here; it had to be attached. The envelope
  becomes a FRAMING span (the glossary's definition of genetics), which mentions genes — said
  plainly rather than claimed otherwise — but states nothing that warrants any of the seven
  rows.

  All seven spans were verified against the fetched glossary pages before writing, read out of
  the shipped file rather than retyped, each with a negative arm (final word altered) that was
  absent from every page. 7/7 verbatim. The row-to-warrant pairing has two **necessary but not
  sufficient** checks inside `assert_term` — each row's key must appear in the span it is given
  and in that span's locator. Measured: four of the seven spans name another row's key, and
  every URL here is `genome.gov/genetics-glossary/...`, so the prefix `gene` matches all six
  other locators. What makes "the `cites` were in row order" a checked fact rather than a lucky
  one is a separate test that reads the shipped file and asserts seven distinct spans and seven
  distinct locators.

  `facts_heredityterm_e2e.rs`: 6 tests. The whole-chain `HEREDITY_TERM_PIN` is replaced — it
  ran from the bindings through five `corroborations` entries that are now row `source`s. Its
  own comment warned that *"a corroboration pin bound to the wrong entry is unique, anchored,
  and tests nothing"*; a chain pin that outlives its corroborations is that hazard one step
  later. What it existed to protect is kept as its own test: the phenotype span carries a CURLY
  apostrophe (U+2019) in `individual’s`, and normalising it to ASCII is one of the mutants.
  15 of 15 mutants killed, baseline green.

