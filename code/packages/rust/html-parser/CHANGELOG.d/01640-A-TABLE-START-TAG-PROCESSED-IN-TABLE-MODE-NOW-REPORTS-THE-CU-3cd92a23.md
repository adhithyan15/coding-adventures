- A `table` start tag processed in table mode now reports the current-Standard
  parse error before closing the open table and reprocessing the token. Nested
  tables inside cells remain valid and diagnostic-free.
