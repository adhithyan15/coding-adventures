---
category: CI & GitHub Actions
---

# Green CI without a selected Windows C build is not MSVC package evidence

NN47 head e20ded7a22e passed required CI and actually built c/bounded-json and
c/iso-harness on Linux and macOS. The ordinary Windows matrix predicate did not
include C-only inputs, so neither that green roll-up nor local MSVC validation
proved a Windows CI execution. Hold the merge and repair the narrow package
selection, using the existing Windows runner, MSVC bootstrap and shared ISO
harness. Test actual planner output and required-build ownership. Always inspect
selected job names and package logs before making a platform claim; no failed
source test or cancelled runner is implied by a missing lane.
