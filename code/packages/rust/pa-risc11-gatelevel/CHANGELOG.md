# Changelog

## 0.1.0

- Added the bounded educational PA-RISC 1.1 gate-level core from Spec 07au2.
- Added exact 525,436-DFF state authority, independent decode/datapath/control,
  transactional lifecycle behavior, structural anti-bypass checks, and full
  state differential tests against the checked functional oracle.
- Verified 25 release tests and 97.65% line coverage (1,040/1,065 lines),
  including post-progress allocation rollback, exact functional lifecycle
  parity, exhaustive subtractor relations, and the complete hostile matrix.
