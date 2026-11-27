---
category: BUILD files & dependency management
---

# Run switch-dependent build helpers inside `opam exec`

Each line in a package `BUILD` runs in its own shell process. An earlier
`opam install` does not leave the OCaml switch's `bin` directory on `PATH` for
the next line. If a helper launches tools such as `dune` or `ocamlc`, invoke
the helper with `opam exec -- python ...` so the child process inherits the
selected switch environment. This applies to Unix and Windows recipes.
