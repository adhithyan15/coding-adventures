- Parser-approved initial tokenizer contexts now include seeded RCDATA, RAWTEXT,
  script data, and escaped script end-tag continuation substates, carrying the
  current end tag and temporary buffer required by those lexer states.
