### Fixed

- `generate:gentle-snapshots` now validates a canonical prior direct-owner tree
  from its stored metrics before replacing it, so legitimate lesson additions
  and removals no longer make write mode reject the old tree against the new
  corpus identities. Interrupted installs can likewise restore the valid prior
  tree before completing the new atomic replacement.
