---
category: Testing & coverage
---

# Type JsonElement property-name helpers as strings in F# tests

An F# Arc2D neutral-fixture test defined `readPoint entry key` and passed
`key` directly to `JsonElement.GetProperty`. The .NET API has overloads for
`string`, `ReadOnlySpan<char>`, and `ReadOnlySpan<byte>`, so F# could not infer
which overload an unconstrained `key` intended. The test failed to compile
before it could demonstrate the expected behavior regression.

Annotate the helper argument as `(key: string)` (and its JSON result as a
`JsonElement` when indexing). Run the focused test once in the red phase to
confirm that the test builds and fails on the actual behavior, not on its own
fixture plumbing.
