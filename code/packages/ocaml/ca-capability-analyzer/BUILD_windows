opam install . --deps-only --with-test --with-dev-setup --yes --require-checksums
opam exec -- dune build @fmt
BISECT_FILE="$PWD/bisect" opam exec -- dune runtest --force --instrument-with bisect_ppx
opam exec -- bisect-ppx-report summary --per-file --expect src/coding_adventures_capability_analyzer.ml bisect*.coverage
opam exec -- dune build @install
