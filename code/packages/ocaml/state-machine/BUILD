opam pin add --no-action --working-dir --no-checksums -y coding-adventures-graph ../graph
opam pin add --no-action --working-dir --no-checksums -y coding-adventures-directed-graph ../directed-graph
opam install . --deps-only --with-test --with-dev-setup -y
opam exec -- dune build @fmt
BISECT_FILE="$PWD/bisect" opam exec -- dune runtest --force --instrument-with bisect_ppx
opam exec -- bisect-ppx-report summary --per-file --expect src/coding_adventures_state_machine.ml bisect*.coverage
