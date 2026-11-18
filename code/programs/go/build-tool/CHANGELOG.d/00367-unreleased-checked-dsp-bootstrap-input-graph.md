## Unreleased — DSP filter tests follow test-bootstrap edits (AF-02b)

- Added a real-repository Go build-plan regression for Python and TypeScript
  `dsp-filters` test consumers of their local `test-bootstrap` packages.
- The test pins Python's explicit BUILD dependency and TypeScript's local
  development dependency, checks language-scoped selection, rejects package
  lookalikes, and fails if either required consumer disappears from discovery.
