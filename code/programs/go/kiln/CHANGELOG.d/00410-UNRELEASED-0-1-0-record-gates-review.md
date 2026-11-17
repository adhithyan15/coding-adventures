## [0.1.0] - UNRELEASED

### Review fixes (stage 4 PR 2b-2c-3)

- Security (MEDIUM): the toolchain probes no longer share the steps' writable
  TMPDIR. They get their own `<dir>/probe/tmp` (0700, created fresh), and
  `prepare` re-runs `probesLeftNothing` on the REAL configuration after the
  toolchain probes: tmpdir and every state root must still be empty, else
  `unscrubbed:*`.
- An absent probe binary gates: `probe-absent:<tool>` at probe priority, in
  `gateFacts`, so `kiln legacy-plan` and the recorder agree. (It is not added to
  a plan's reasons, like `probe-refused:`.) The test fixtures that probed an
  always-included `cc` on a nonexistent root now give it a binary.
- A probe sandbox that is systematically broken fails `prepare`: probes ran and
  every one ended in sandbox exit 125. A single 125 stays a mismatch.
- `TestSandboxRecordGates` can now tell a working probe sandbox from a broken
  one: sed's mismatch detail must be real tool output, and a positive versioned
  case (grep pinned to the host's own version) must be `ok` with no gate.
- Spec P7: the above, and that `kiln legacy-plan --probe` still probes on the
  host.
