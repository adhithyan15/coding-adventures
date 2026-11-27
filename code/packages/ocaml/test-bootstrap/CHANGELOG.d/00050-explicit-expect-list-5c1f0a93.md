## 0.1.0 (Windows coverage)

- BUILD and BUILD_windows pass one `--expect src/<file>.ml` per instrumented source file instead of
  `--expect src/ --do-not-expect src/val.ml`; the directory form did not match Windows `src\` paths
  and failed the coverage step. Same semantics: every file but the type-only `val.ml` must appear.
