---
category: BUILD files & dependency management
---

# Multi-line `if`/`for`/`case` blocks inside BUILD itself break

because the build-tool dispatches each LINE through a fresh shell process. Shell state such as environment variables, current directory, and `setlocal`/`pushd` does not carry over to the next BUILD line. Symptom: `Syntax error: end of file unexpected (expecting "fi")`, or a later command cannot find a dependency installed in a prior line's temporary tree. Put stateful multi-step recipes in a sibling script (for example `tools/build.sh` and `tools/build.cmd`) and make BUILD/BUILD_windows each a one-line invocation of the matching script. Keep short independent commands on separate lines; single-line `&&` chaining is also suitable for short pipelines.
