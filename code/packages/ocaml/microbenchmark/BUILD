opam pin add --no-action --working-dir --no-checksums -y coding-adventures-rank-statistics ../rank-statistics
opam install . --deps-only --with-test --with-dev-setup -y
opam exec -- dune clean
opam exec -- dune build @fmt @all --instrument-with bisect_ppx
BISECT_FILE="$PWD/_build/coverage" opam exec -- dune runtest --force --instrument-with bisect_ppx
BISECT_FILE="$PWD/_build/coverage" opam exec -- dune exec --instrument-with bisect_ppx coding-adventures-array-microbenchmark -- --pairs 40 --revision coverage-validation > _build/native-40.json
opam exec -- dune exec coding-adventures-array-microbenchmark -- --replay _build/native-40.json
opam exec -- bisect-ppx-report summary --per-file --expect src/microbenchmark.ml --expect src/array_workload.ml --expect bin/array_microbenchmark.ml --do-not-expect src/native_clock.ml --coverage-path _build
sh tools/installed-consumer-smoke.sh
