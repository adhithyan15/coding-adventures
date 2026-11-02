---
package: */ct-compare
lane: ocaml
rung: infra
status: open
---

# CT01's list of missing ct-compare ports omits OCaml

CT01's corpus section lists the missing `ct-compare` lanes as Dart, Haskell,
Lua, Perl and Swift. OCaml is missing too, and the package-parity loop has no
work item for it, so the gap is invisible to the tooling that tracks parity.

**Open:** add OCaml to CT01's list and to the parity loop's inventory when an
OCaml `ct-compare` lane is planned.
