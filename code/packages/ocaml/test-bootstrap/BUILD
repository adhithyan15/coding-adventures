opam install . --deps-only --with-test --with-dev-setup -y
{ printf 'let json =\n  {pbt|'; cat ../../../specs/fixtures/pbt-v1/cases.json; printf '|pbt}\n'; } | cmp - test/corpus_data.ml
opam exec -- dune build @fmt
BISECT_FILE="$PWD/bisect" opam exec -- dune runtest --force --instrument-with bisect_ppx
opam exec -- bisect-ppx-report summary --per-file --expect src/bigint.ml --expect src/budget.ml --expect src/canon.ml --expect src/corpus.ml --expect src/error.ml --expect src/escape.ml --expect src/gen.ml --expect src/prng.ml --expect src/runner.ml --expect src/safe.ml --expect src/shrink.ml --expect src/strict_json.ml --expect src/tfv.ml --expect src/types.ml --expect src/values.ml bisect*.coverage
