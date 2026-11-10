opam install . --deps-only --with-test --with-dev-setup -y
{ printf 'let json =\n  {ct|'; cat ../../../specs/fixtures/ct-compare-v1/cases.json; printf '|ct}\n'; } | cmp - test/corpus_data.ml
opam exec -- dune build @fmt
BISECT_FILE="$PWD/bisect" opam exec -- dune runtest --force --instrument-with bisect_ppx
opam exec -- bisect-ppx-report summary --per-file --expect src/coding_adventures_ct_compare.ml bisect*.coverage
