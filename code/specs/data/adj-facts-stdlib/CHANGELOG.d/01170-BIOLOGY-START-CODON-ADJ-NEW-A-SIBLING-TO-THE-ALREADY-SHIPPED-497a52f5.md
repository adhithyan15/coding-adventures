- `biology/start-codon.adj` (new) — a sibling to the already-shipped `genetic-code.adj`
  (`codon_amino_acid(codon, amino_acid)` over all 64 codons): a new `start_codon(codon, role)`
  table names WHICH codons NCBI's own `Starts` annotation line flags, decoded from the SAME
  verbatim five-line block already quoted inside `genetic-code.adj`'s `source` field — no new
  WebFetch. The `Starts` line carries exactly three `M`s (columns 4, 20, 36), decoding to `ttg`,
  `ctg`, `atg`. Deliberately does NOT pair these with an amino acid (translation always initiates
  with the same methionine regardless of which start codon recruited it, so a `(codon,
  amino_acid)` pairing for `ttg`/`ctg` would misstate biology); `role` names only what the source
  itself asserts. New e2e test file `facts_startcodon_e2e.rs` (3 tests: recall the primary start
  codon, recall both alternative start codons, honest abstention on an unflagged codon). No
  manifest objective, matching `genetic-code.adj`'s own precedent of not having one.
