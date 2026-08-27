# Changelog

## 1.0.2 - 2026-08-26

- Add dependency-free generated Scytale consumers for every established lane.
- Bind all 18 native test adapters to the exact corpus digest, complete expected
  objects, and the current implementation-language roster with a fail-closed
  regeneration check.

## 1.0.1 - 2026-08-26

- Add Scytale encrypt/decrypt vectors for a decomposed combining sequence so
  scalar-based grids cannot silently regress to grapheme-cluster counting.

## 1.0.0 - 2026-08-26

- Define the closed CR01-CR03 language-neutral corpus.
- Pin complete Atbash ASCII mappings and scalar-preserving passthrough.
- Pin Scytale Unicode-scalar grids, literal-space padding loss, key ordering,
  ascending brute force, and its 4,096-scalar preflight limit.
- Pin Vigenère ASCII progression, the 90% shortest-near-maximum IC rule,
  smallest-shift chi-squared ties, analysis limits, and full requested keys.
- Add a fixed long-English `SECRET` recovery vector, stable error IDs, bounded
  strict JSON and raw-structure preflight, iterative fixture traversal,
  fragment-local schema references, hostile schema mutations, and a
  semantic-oracle drift gate.
