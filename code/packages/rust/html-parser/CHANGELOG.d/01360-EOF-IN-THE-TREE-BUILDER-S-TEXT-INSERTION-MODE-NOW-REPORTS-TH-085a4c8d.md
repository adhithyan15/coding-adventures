- EOF in the tree builder's text insertion mode now reports the Standard's
  parse error, pops the authored text element, and reprocesses EOF in the
  original mode, closing 92 previously silent malformed corpus cases without
  changing DOM recovery or diagnostics for seeded fragment contexts.
