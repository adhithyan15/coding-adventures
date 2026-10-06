## Added

- Treat exact conditional-statement selectors as control dependencies of their
  branch writes, allowing bounded recurrence analysis to track selector cycles
  whose selected boolean assignments contain exact conditional expressions.
