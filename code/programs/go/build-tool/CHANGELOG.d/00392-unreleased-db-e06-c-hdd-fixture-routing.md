## Unreleased — DB-E06 C HDD fixture routing

- Added `c/hdd-simulator` as an exact reader of the shared DB02 HDD vector for
  Go change selection, fallback package hashing, and Kiln `repo_inputs`. The
  DB01 NAND vector remains isolated to its three readers.
- Added the C HDD package root to the existing exact HDD source-input rule and
  regenerated the Swift and C# projections and digest-pinned corpus metadata.
  The ledger still has 29 rules, 44 input rows and 35 unique paths; its corpus
  summary now reports 520 scopes and 541 authorizations.
