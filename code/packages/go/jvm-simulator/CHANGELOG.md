# Changelog

## [0.1.0] - Unreleased

### Added
- Created `JVMSimulator` structurally validating Two-s complement variables evaluating against 32-bit bounds explicitly.
- Defined extensive `JVMTrace` exposing historically accurate snapshots defining Stack limits exclusively per operation execution.
- Implemented Typed subset of JVM commands isolating integers natively (`iadd`, `iconst_X`, `iload`) providing pedagogical emphasis onto JVM validation constraints directly inside code comments.
- Mapped explicit error checking conditions ensuring Division-By-Zero and limit overflows are documented structurally across runtime invocations mirroring modern `javap` compiler conditions.
