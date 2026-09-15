### Windows execution-corpus volume identity

- Reconciled Python 3.10 and 3.13 Windows `st_dev` projections by retaining the
  exact legacy and extended volume serials from the same open corpus-root
  handle. The snapshot still rejects truncation, aliases, reparse traversal,
  mutation, and every serial not supplied by that retained root.
- Added cross-version regressions for exact legacy fallback, full-width serial
  matching, and rejection of a different 64-bit serial sharing the same low
  DWORD. Execution policy remains disabled and gains no process authority.

