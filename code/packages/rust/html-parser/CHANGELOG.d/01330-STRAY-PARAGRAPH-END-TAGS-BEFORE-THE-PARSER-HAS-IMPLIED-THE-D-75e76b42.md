- Stray paragraph end tags before the parser has implied the document body now
  reuse the existing pre-body diagnostic, closing 4 previously silent malformed
  corpus cases without changing DOM recovery.
