# Changelog

## 0.1.0

- Enforce the semantic JSON value budget before Aeson allocates the replay tree.
- Validate present replay workload IDs using the exact ASCII blank set and reject NULs and malformed surrogate escapes.

- Add paired collection, exact-rank analysis, strict report replay, and an indexed-array CLI.
- Select the Command Line Tools SDK explicitly for Darwin Cabal build and smoke commands.
- Bound public benchmark analysis after inspecting at most 10,001 list elements, and read CLI replay paths through a 4 MiB plus one byte limit.
- Declare the monotonic-time, named revision, output, and replay-file runtime effects in the capability profile.
