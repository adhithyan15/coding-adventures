### Scytale language-neutral fixture consumers

- Added a stdlib-only generator and drift gate that turns all 18 normative
  Scytale objects into native tests for every established implementation lane.
  The generated tests compare complete text, error, and ordered-candidate
  results without adding production dependencies or runtime capabilities.
- Hardened the generator's source boundary with safe identifiers, bounded
  fixture and output reads, interpolation-safe native string construction, and
  fixture-derived resource-limit inputs and normalized error assertions.

