---
category: Testing & coverage
---

# Use the repository-native entrypoint before assuming a generic test runner

During FM-B075 validation, the Forme spec-map contract was first invoked with
`pytest`, which was not installed, and lesson validation was aimed at a guessed
script name that does not exist. Both contracts already expose repository-native
entrypoints: the spec-map test runs directly with Python's standard-library
`unittest` harness, and `code/scripts/lessons.py validate` checks lesson shards.

Before invoking a repository test or validator, inspect its checked-in entrypoint
and nearby documentation. Do not infer a third-party runner or script name when
the repository already supplies a dependency-free command.
