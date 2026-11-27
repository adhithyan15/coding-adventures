## Unreleased — historical Unix Ruby package identity

- Bind the shared HOS-002 fixture relation and its cache input to the actual
  discovered package name `ruby/unix_history_simulator`.
- Reject the obsolete hyphenated identity and test selection using package
  discovery rather than deriving expected names from the selector itself.
