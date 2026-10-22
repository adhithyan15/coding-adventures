---
category: Python
---

# Check the pinned capability-test interpreter before running the gate

The capability-taxonomy test was launched with system Python even though this
repository's lessons already document that `jsonschema` is absent there. The
test failed before validation. Before running this gate in a fresh worktree,
select or create an isolated interpreter with CI's exact
`jsonschema==4.26.0` pin instead of retrying the known-incomplete interpreter.
