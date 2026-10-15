# Do not chain a dependency-presence probe to a test command that ignores its result

A diagnostic command printed that a fresh worktree had no `node_modules`, but
the shell command used `;` and launched the package test anyway. The predictable
`vitest: command not found` added noise without testing the intended red state.
Probe dependencies in a separate command or use a conditional that installs
them before invoking package scripts. Treat a known prerequisite as control
flow, not merely as log output.
