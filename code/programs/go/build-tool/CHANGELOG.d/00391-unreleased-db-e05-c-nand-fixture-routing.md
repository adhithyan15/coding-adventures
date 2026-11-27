## Unreleased — DB-E05 C NAND fixture routing

- Added `c/nand-flash` as an exact reader of the shared DB01 NAND vector for Go
  change selection, fallback package hashing, and Kiln `repo_inputs`.
- Added the C package root to the existing NAND source-input authorization and
  regenerated the Swift and C# projections and digest-pinned fixture metadata.
  The boundary contains 29 entries, 44 input rows, 521 scopes and 542
  authorizations. The C NAND root adds one scope and one authorization.
