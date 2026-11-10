---
category: CI & GitHub Actions
---

# Capability gates scan test code, so tests of a pure package must not read fixtures at run time

The OCaml ct-compare port (PR #19082) has an empty `required_capabilities.json`.
Its tests passed locally, but the "OCaml capability gate" job failed:

```
test/test_ct_compare.ml:113:11: [CAP001] undeclared capability: fs:read:* (open_in_bin call)
test/test_ct_compare.ml:230:9: [CAP001] undeclared capability: fs:list:* (Sys.file_exists call)
```

The OCaml analyzer scans every dune stanza, test stanzas included. It also fails
closed on `(rule ...)` generated-source stanzas, so generating the embedded
corpus with dune is rejected too ("unsupported generated-source stanza").

Fix: commit the fixture as an OCaml quoted string (`let json =\n  {ct|...|ct}`,
in ocamlformat's layout) and move the drift check against the canonical file
into a `BUILD` line that rebuilds the string and `cmp`s it.

Do differently: before pushing a new package in a language that has a capability
gate, run that language's analyzer over the package (`--dir <pkg>`), not just the
tests. Do not widen a pure library's manifest to cover test-only I/O.
