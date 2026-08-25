### Added — block-boundary knowledge closure

- Parse canonical `hl-knowledge` directives beside every schema-v2 body block
  while excluding the metadata from learner-facing Markdown.
- Validate introductions and assessments in rendered order, reject undeclared or
  unavailable prompt knowledge, and require production and recall blocks to name
  what they assess.
- Migrate all 51 Spanish Chapters 1–6 lessons to the fail-closed block contract
  and refresh their shared app/book source hashes.

