### Removed — `interaction.multiline-commit-unbound` is retired (UI25 §4)

- Every backend now commits a multiline `HostInput` or `Input` on Ctrl+Enter
  or Cmd+Enter, so `onCommit` there has a trigger and is no longer reported.
  The test that pinned the report now pins its absence: `onCommit` is clean on
  all nine backends, for both primitives and both shapes.
