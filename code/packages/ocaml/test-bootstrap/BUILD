opam install . --deps-only --with-test --with-dev-setup -y
{ printf 'let json =\n  {pbt|'; cat ../../../specs/fixtures/pbt-v1/cases.json; printf '|pbt}\n'; } | cmp - test/corpus_data.ml
opam exec -- dune build @fmt
BISECT_FILE="$PWD/bisect" opam exec -- dune runtest --force --instrument-with bisect_ppx
opam exec -- bisect-ppx-report summary --per-file --expect src/ --do-not-expect src/val.ml bisect*.coverage
