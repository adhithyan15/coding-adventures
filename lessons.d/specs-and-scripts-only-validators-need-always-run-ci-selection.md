---
category: CI & GitHub Actions
---

# Specs-and-scripts-only validators need always-run CI selection

**What went wrong.** A language-neutral fixture added a strict validator and a
thorough falsification suite, but changed no native package. The build planner
correctly selected nothing, and the always-run metadata job did not call either
new command, so hosted CI could pass without validating the fixture at all.

**Fix.** Add both the validator and its dedicated tests to the always-run
repo-wide metadata-contract step, and add a regression that inspects that exact
step for both commands.

**Do differently.**

- For every specs-and-scripts-only foundation, identify its hosted execution
  path before claiming that repository checks use it.
- Treat “no package files changed” as a signal to verify an always-run gate,
  not as evidence that no CI integration is needed.
- Pin the production validator and its adversarial tests separately; running
  only one leaves either implementation or selection drift undetected.
