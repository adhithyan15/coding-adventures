- Start and end tags rejected against seeded fragment-context elements now
  report their required parse errors, covering 37 previously silent table,
  select, frameset, document-shell, and foreign fragment cases without changing
  DOM recovery or undeclared-diagnostic coverage.
