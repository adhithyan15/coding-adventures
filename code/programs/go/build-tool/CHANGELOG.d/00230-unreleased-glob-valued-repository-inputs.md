## Unreleased — glob-valued repository inputs

- **`repo_input_globs` and `repo_inputs_unknown`.** Each exported package gains
  a sorted `repo_input_globs` list; `rust/mosaic-package-resolver` exports the
  globs equivalent to its predicate (`code/**/?*.mil|mll|msl`,
  `code/**/mosaic-package.toml`; `?*` because the predicate rejects an empty
  stem). `repo_inputs_unknown` is omitted unless a consumer declares neither
  paths nor globs. Tests pin globs to the predicate over a candidate tree and
  require every table row to declare something.
