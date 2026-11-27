## 0.1.0 (unreleased amendment)

- Tests read the pbt-v1 corpus from `test/corpus_data.ml`, an OCaml string that embeds
  `code/specs/fixtures/pbt-v1/cases.json` verbatim, exactly like `ocaml/ct-compare`: the package
  keeps an empty capability manifest and the tests touch no filesystem. `BUILD` rebuilds the
  module from the canonical corpus and `cmp`s it, so drift fails the build; it is content-only, so
  `BUILD_windows` skips it.
