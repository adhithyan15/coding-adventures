- `biology/amino-acid-three-letter-code.adj` (new) — a sibling to the already-shipped
  `amino-acids.adj` (`amino_acid_code(amino_acid, code)`, the ONE-letter code): a new
  `amino_acid_three_letter_code(amino_acid, code)` table names the OTHER column the SAME
  already-cited DDBJ page tables, which the existing table's schema had no room for.
  `amino-acids.adj`'s own header already explains the source page has three columns
  (3-letter/1-letter/name) and quotes the verbatim triple for glycine, but only ever paired the
  name with the one-letter code. WebFetch-verified TWICE this cycle for consistency, both
  byte-identical: all twenty standard amino acids' three-letter codes read directly off the same
  page. New e2e test file `facts_aminoacidthreeletter_e2e.rs` (3 tests: both-direction recall,
  the two acidic residues, honest abstention on a non-standard amino acid). No manifest
  objective, matching `amino-acids.adj`'s own precedent of not having one.
