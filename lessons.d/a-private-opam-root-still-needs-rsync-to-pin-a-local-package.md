---
category: Supply chain & CI pinning
---

# A private opam root still needs rsync to pin a local package path

`opam pin add --working-dir ../rank-statistics` failed on the bare executor
because `rsync` was absent. The repository's OCaml setup action provides it,
but a private `OPAMROOT` only isolates opam state; it does not supply host
utilities that opam invokes while snapshotting a directory pin.

Check the exact OCaml workflow prerequisites before local dependency setup. If
the host lacks `rsync`, install a verified OS package in a private tools prefix
or use the repository's existing setup convention; do not bypass package
checksums or change the opam repository to work around the missing command.
