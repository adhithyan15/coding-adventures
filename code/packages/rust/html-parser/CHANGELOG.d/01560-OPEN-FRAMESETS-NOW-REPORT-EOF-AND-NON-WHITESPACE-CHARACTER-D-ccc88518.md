- Open framesets now report EOF, and non-whitespace character data discarded
  in or after a frameset reports its insertion-mode parse error. This covers 7
  previously silent malformed corpus cases without changing DOM recovery or
  undeclared-diagnostic coverage.
