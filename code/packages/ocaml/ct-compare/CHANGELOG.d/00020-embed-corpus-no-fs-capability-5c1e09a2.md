## 0.1.0 (unreleased amendment)

- Tests now read the CT01 corpus from `test/corpus_data.ml`, an OCaml string that
  embeds `code/specs/fixtures/ct-compare-v1/cases.json` verbatim, instead of opening
  `test/fixtures/.../cases.json` at run time. The OCaml capability gate scans test code
  too, and the file read and `Sys.file_exists` walk were undeclared `fs:read`/`fs:list`
  uses in a package whose manifest is (correctly) empty.
- The drift test moved out of the suite into `BUILD`: it rebuilds `corpus_data.ml` from
  the canonical corpus and `cmp`s it, so drift still fails the build, now with no
  filesystem access inside the tests. It is content-only, so `BUILD_windows` skips it.
