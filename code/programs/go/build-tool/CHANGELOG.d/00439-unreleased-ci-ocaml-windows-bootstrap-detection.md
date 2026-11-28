`ci.yml` changes to the OCaml action's Windows compiler or environment inputs
now select the OCaml toolchain instead of leaving a Windows bootstrap change
without an affected language plan. This keeps generic Windows bootstrap
validation attached to the shared CI configuration. Generic Windows CI also
checks the configured MinGW compiler and archiver from inside the opam
environment before building its pinned OCaml tool dependencies.
