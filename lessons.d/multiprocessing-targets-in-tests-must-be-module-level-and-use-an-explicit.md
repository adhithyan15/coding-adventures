---
category: Python
---

# multiprocessing targets in tests must be module-level and use an explicit spawn context; macOS and Windows default to spawn

A test ran a nested function `attempt` in a `multiprocessing.Process`. On Linux the
default start method is `fork`, which never pickles the target, so the test passed
locally. On macOS and Windows the default is `spawn`, which pickles the target by
qualified name, and a nested function has no importable name. The child then fails with
`AttributeError: Can't get local object 'test_...<locals>.attempt'` (CI job 134361217950,
PR #19581, three parametrized FIFO tests in `warrant-oracle`).

The fix: put the target at module level and take every value it needs as an argument, then
create the process from an explicit `multiprocessing.get_context("spawn")`. Using the same
context on every platform means Linux exercises the code path macOS and Windows use.
Module import must stay free of side effects, because the spawned child re-imports the
test module to find the target.

Check every `multiprocessing` or `Pool` use in a test directory for nested targets, not only
the one that failed. Do the same for queues and other arguments: they must pickle.
