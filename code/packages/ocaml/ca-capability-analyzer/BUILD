opam install . --deps-only --with-test --with-dev-setup --yes --require-checksums
opam exec -- dune build @fmt
BISECT_FILE="$PWD/bisect" opam exec -- dune runtest --force --instrument-with bisect_ppx
python test/check_coverage.py --coverage-glob "bisect*.coverage" --summary _build/coverage-summary.txt --source src/coding_adventures_capability_analyzer.ml --source bin/main.ml --minimum 95
opam exec -- dune build @install
