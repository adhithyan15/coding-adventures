# Changelog — jvm-jar-writer

## 0.1.0 — 2026-04-29

### Added — JVM02 Phase 1: pure JAR writer

- ``JarManifest`` dataclass: ``main_class`` + ``extra_attributes``.
- ``JarWriterError`` for invalid input (reserved paths,
  duplicate entries, bad attribute keys/values).
- ``write_jar(classes, manifest) -> bytes`` — produces a fully-
  formed JAR (ZIP archive with ``META-INF/MANIFEST.MF`` first).
- Deterministic ZIP timestamps (1980-01-01) so byte-equal inputs
  produce byte-equal JARs.
- 72-byte manifest line-wrapping per the JAR spec, with
  continuation lines.
- Validation: rejects entries under reserved ``META-INF/``
  prefixes, rejects duplicate paths, rejects malformed manifest
  attribute names.
- Tests:
  - Pure unit tests for the manifest line-wrapping helper.
  - Round-trip tests via Python's ``zipfile`` reader (the same
    parser ``java -jar`` ultimately uses) — every emitted JAR
    decodes cleanly.
  - Optional real-``java`` smoke test (skipped when ``java`` not
    on PATH) executing a hand-built ``Hello`` JAR.
