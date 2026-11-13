## Unreleased — CT01 corpus lanes as repository-input consumers

- `code/specs/fixtures/ct-compare-v1/cases.json` now also selects the Dart,
  Haskell, Lua, OCaml, Perl and Swift `ct-compare` packages, whose tests read
  the corpus outside their own directories (OCaml compares a vendored copy with
  it). Only the exact corpus file selects them: the TFV projection and Python
  scripts remain Rust-adapter inputs. Missing consumers fail closed and language
  filters are retained.
