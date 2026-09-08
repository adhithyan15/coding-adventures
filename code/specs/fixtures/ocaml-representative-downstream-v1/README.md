# OCaml representative downstream fixture

This fixture links and executes all four representative OCaml libraries against
closed JSON receipts. A second executable declares only the state-machine leaf
and invokes its directed-graph-backed reachability path with Dune implicit
transitive dependencies disabled.

Run it after installing the four version 0.1.0 packages:

    opam exec -- dune build @fmt
    opam exec -- dune runtest --profile release
