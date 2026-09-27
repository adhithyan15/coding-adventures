- Fragment parsing now reports the in-body EOF parse error for authored
  disallowed open elements while excluding synthetic context-shell nodes,
  closing 54 previously silent malformed corpus cases without changing DOM
  recovery or diagnostics for empty contexts.
