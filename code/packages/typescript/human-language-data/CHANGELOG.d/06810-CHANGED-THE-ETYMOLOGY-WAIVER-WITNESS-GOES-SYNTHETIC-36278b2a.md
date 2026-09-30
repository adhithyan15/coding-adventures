### Changed — the etymology-waiver witness goes synthetic

- `tests/level-gate.test.ts`: "waives etymology atoms from the reinforcement
  criterion" used to read the waiver off a real track that still had a
  reinforcement blocker. The A2 reinforcement tranches for Spanish, French,
  German, Italian and Portuguese closed the last of them. The test now builds
  its witness instead:
  - it takes the first track that has waived etymology atoms at or below the
    level it is working on;
  - it adds one non-etymology atom there that nothing revisits;
  - it asserts the reinforcement blocker names that level and the number of
    hooks waived.

  The counterfactual half, which renames the etymons and checks that the
  shortfall rises, is unchanged.
