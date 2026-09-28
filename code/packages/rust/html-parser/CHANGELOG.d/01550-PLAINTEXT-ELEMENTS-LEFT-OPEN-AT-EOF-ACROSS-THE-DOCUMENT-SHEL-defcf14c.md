- Plaintext elements left open at EOF across the document shell and templates
  now report their required parse error. Text-mode lexer handoff now occurs
  only after tree construction accepts the start tag, so plaintext rejected in
  or after a frameset remains tokenized as markup and reports the rejected
  start and end tags. This covers 12 previously silent malformed corpus cases.
