---
category: CI & GitHub Actions
---

# A test that imports ignored generated artifacts must make every workflow that runs it generate those artifacts first

TaskApp's Web Component acceptance test passed locally after the parity build,
but the independent release workflow only regenerated the React host before it
ran the full web test suite. A clean runner therefore could not import the
gitignored emitted Custom Element. Every workflow that runs a test over ignored
generated output must invoke that output's generator first; keep the generation
adjacent to the test command so later test-suite expansion cannot silently rely
on residue from another build.
