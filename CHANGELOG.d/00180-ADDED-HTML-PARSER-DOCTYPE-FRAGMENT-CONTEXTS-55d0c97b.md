### Added — HTML Parser DOCTYPE Fragment Contexts
- Parser-approved initial tokenizer contexts now include seeded DOCTYPE
  continuation states for keyword, name, public/system identifier, bogus, and
  force-quirks recovery paths.
- DOM parser coverage now exercises parser/lexer handoff for partial DOCTYPE
  fragments while preserving lexer diagnostics and following body content.

