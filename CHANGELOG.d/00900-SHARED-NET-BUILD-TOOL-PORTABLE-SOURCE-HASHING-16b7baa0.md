### Shared .NET build-tool portable source hashing

- Embedded the complete checked package-local and repository-boundary source-
  input registries in the shared C# engine, exposed independent no-inline F#
  facades, and consumed all 13 neutral selection cases plus all three portable
  package-digest cases through both .NET front doors.
- Added exact generated-component and repository-boundary selection, direct
  Starlark declared-source capture, reverse boundary-diff mapping, canonical
  registry digests, and repository-relative Hashing-v1 raw-byte frames.
- Hardened live reads with incremental resource ceilings, native no-follow and
  single-link regular-file checks, constant-descriptor file/directory identity
  revalidation, and a scrubbed bounded Git-index query whose complete
  mode/OID/stage/path evidence must remain stable across the package batch.
  Removed stale discovery helpers that bypassed the secure path and made .NET
  changes select the Windows CI leg. Dependency and combined-digest conformance
  remain in the separate dependency-hashing tranche.
- Made the shared portable-glob class grammar and exact 50,000,000-unit match
  budget normative, then aligned Swift—the only already-complete registry
  adopter—with Python-compatible class semantics and bounded dynamic
  programming for source and diff selection.

