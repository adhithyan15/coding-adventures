opam install . --deps-only --with-test --with-dev-setup --yes --require-checksums
opam exec -- dune build @fmt
BISECT_FILE="$PWD/bisect" opam exec -- dune runtest --force --instrument-with bisect_ppx
opam exec -- bisect-ppx-report summary --per-file --expect src/coding_adventures_capability_analyzer.ml bisect*.coverage | tee _build/coverage-summary.txt
python test/check_coverage.py --summary _build/coverage-summary.txt --source src/coding_adventures_capability_analyzer.ml --minimum 95
opam exec -- dune build @install
