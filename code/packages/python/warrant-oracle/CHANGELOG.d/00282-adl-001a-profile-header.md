## Unreleased — `profile` header clause (VER33 ADL-001a)

- The reader recognises `warrant NAME v1 profile algorithm`. A malformed clause (no `v1`, a
  name other than `algorithm`, a missing name, a second clause) is a `parse` error, as the Rust
  reader's W0801; a well-formed algorithm document is a model file, not a corpus subject, so it
  is refused by name with `OracleError("level", level="profile.algorithm")`, never read as a
  pure document. Pure documents are unchanged. Tests in `tests/test_profile_header.py`.
