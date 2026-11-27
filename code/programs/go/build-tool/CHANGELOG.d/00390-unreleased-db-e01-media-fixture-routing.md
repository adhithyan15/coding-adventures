## Unreleased — DB-E01 exact media fixture routing

- Registered the exact DB01 NAND and DB02 HDD shared vectors as repository input
  consumers for only their Python and TypeScript readers. The Go diff selector,
  fallback cache hashing, and Kiln `repo_inputs` export now share these exact
  file paths without selecting the sibling device's packages.
- Added exact package-root authorizations to the repository source-input boundary
  and regenerated its Swift/C# projections and digest-pinned conformance cases.
