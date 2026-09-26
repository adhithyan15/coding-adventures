- Parser-approved initial tokenizer contexts now include seeded HTML comment
  continuation substates, carrying current comment data through comment body,
  pending dash/bang, nested-comment, abrupt-close, and bogus-comment recovery
  paths exposed by the lexer.
