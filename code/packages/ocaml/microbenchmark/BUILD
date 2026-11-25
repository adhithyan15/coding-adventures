opam pin add --no-action --working-dir --no-checksums -y coding-adventures-rank-statistics ../rank-statistics
opam exec -- dune build @fmt
opam exec -- dune runtest --force --instrument-with bisect_ppx
opam exec -- dune exec coding-adventures-array-microbenchmark -- --pairs 40
