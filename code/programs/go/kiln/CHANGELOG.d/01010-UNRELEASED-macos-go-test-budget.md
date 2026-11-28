## [0.1.0] - UNRELEASED

### Fixed

- On macOS, the Kiln package's full Go test suite now has a bounded 20-minute
  package timeout and verbose test timings. The complete build, vet, and test
  commands still run; no tests or per-operation deadlines were removed or
  relaxed. This accommodates the observed full-suite runtime without asserting
  a cause for the slower macOS run.
