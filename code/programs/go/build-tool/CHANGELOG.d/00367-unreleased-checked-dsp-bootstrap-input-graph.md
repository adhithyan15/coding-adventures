## Unreleased — DSP filter tests follow test-bootstrap edits (AF-02b)

- Added a real-repository regression for Python and TypeScript
  `test-bootstrap` test inputs and their DSP consumers. It validates the
  relevant BUILD prerequisites, checks language-scoped affected-package
  membership and exclusions, rejects package lookalikes, and fails if a
  required consumer disappears from discovery.
