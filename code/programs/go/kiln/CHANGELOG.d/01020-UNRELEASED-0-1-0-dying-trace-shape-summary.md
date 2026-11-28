## [0.1.0] - UNRELEASED

### Tests

- Dying-thread failure summaries now distinguish trimmed whitespace from
  unknown resumed-call tails using only byte counts and closed labels. They
  continue to omit syscall arguments and do not change the refusal oracle.
