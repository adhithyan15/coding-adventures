---
category: Testing & coverage
---

# Coverage launchers must preserve the script import search path

The Warrant corpus script passed when run directly and in its unit suite, but
the local coverage launcher failed before execution with ModuleNotFoundError
for its sibling tfv module. That invocation did not cover the CLI entrypoint
and was not a product-validator failure.

For that instrumentation command, set PYTHONPATH to the repository's
code/scripts directory in the command's child environment. The real CLI then
ran under coverage and its transport check passed. Do not change parser or
validator semantics to work around an instrumentation import context, or
claim entrypoint coverage from unit imports alone.
