---
category: BUILD files & dependency management
---

# Multi-line `if`/`for`/`case` blocks inside BUILD itself break

because the build-tool dispatches each LINE through a fresh `sh -c`. Symptom: `Syntax error: end of file unexpected (expecting "fi")`. If you need a real shell script, put it in a sibling file (e.g. `tools/run-tests.sh`) and make BUILD a one-liner that invokes it: `sh tools/run-tests.sh`. Single-line `&&` chaining also works for short pipelines.
