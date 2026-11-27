---
category: BUILD files & dependency management
---

# A single-package Dune project must give its package the same name as the project

A package declaration such as `(package (name unix-history-simulator))` must
match the top-level `(name unix-history-simulator)` when a Dune project declares
only one package. Dune stops before compiling sources if the project name uses
an underscore while the package name uses hyphens. The OCaml Unix simulator's
first local BUILD exposed this mismatch; using the package's distribution name
for both declarations let Dune proceed to the actual build.

Before running a new package, compare the `dune-project` name, package stanza,
`.opam` filename, library `public_name`, and each consumer's `libraries` entry.
Those package references use the published name consistently. The library's
OCaml module and archive names may still use underscores; those are separate
identifiers from the Dune package name.
