---
category: BUILD files & dependency management
---

# A local OCaml BUILD must install declared test and development dependencies before Dune runs

because `opam exec -- dune ...` only activates an existing switch; it does not
install the package's test or development tools. A previously populated switch
can hide this mistake, so a clean CI host fails when `alcotest`, `bisect_ppx`,
or `ocamlformat` is absent. Pin local path dependencies first, then run
`opam install . --deps-only --with-test --with-dev-setup -y` before formatting,
tests, or executable smoke commands. Keep that install step in both Unix and
Windows BUILD entrypoints and let opam resolve the dependencies declared in
the package's `.opam` manifest.
