## [0.1.0] - UNRELEASED

### Added

- `internal/source`: the `Source` interface (KILN00 section 13) with a Git
  implementation (`ls-files -z`, `rev-parse HEAD`, `diff --name-only -z`;
  revisions starting with `-` or containing `..` are rejected, unresolvable
  ones give `known=false`) and a plain-folder implementation (`.kilnignore`,
  symlinks listed but never followed, content-derived `Revision`, `Changed`
  always unknown). One shared glob matcher (`*`, `?`, `**`).
- `internal/host`: the `Host` interface with `Local`, `GitHub` (trust exactly
  per KILN00 section 10.2, fail closed; workflow-command escaping), `Detect`,
  and `ScrubSecrets`.
- `internal/key.ValidatePath`, exporting the KILN02 section 3.5 path check.
- `required_capabilities.json`: process exec (git), env read, temp dir write.
