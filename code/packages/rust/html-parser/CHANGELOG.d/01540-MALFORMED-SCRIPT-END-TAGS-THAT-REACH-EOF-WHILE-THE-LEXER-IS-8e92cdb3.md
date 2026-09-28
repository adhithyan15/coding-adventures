- Malformed script end tags that reach EOF while the lexer is assembling the
  tag now report `eof-in-tag`, covering 14 previously silent malformed corpus
  cases without changing DOM recovery or undeclared-diagnostic coverage.
