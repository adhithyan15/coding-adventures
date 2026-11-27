opam install . --deps-only --with-test --with-dev-setup -y
opam exec -- dune clean
opam exec -- dune build @fmt @all --instrument-with bisect_ppx
BISECT_FILE="$PWD/_build/coverage" opam exec -- dune runtest --force --instrument-with bisect_ppx
opam exec -- bisect-ppx-report summary --per-file --expect src/rank_statistics.ml --coverage-path _build
