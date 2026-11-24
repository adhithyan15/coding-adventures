---
category: CI & GitHub Actions
---

# Pinned opam-repository needs archive-mirrors: upstream GitLab archives get regenerated and checksums break

Pinning opam-repository to a commit pins checksums, not bytes. Upstream hosts
such as gitlab.inria.fr regenerate their `archive.tar.gz` downloads (menhir
20260209, fix 20250919), so every OCaml job failed with opam exit 40 "Bad
checksum" without any package change (PR #19949, run 45451888718).

Fix: append `archive-mirrors: "https://opam.ocaml.org/cache"` to the opam
global config right after setup-ocaml in every workflow that uses the pinned
repository (ci.yml, build-ocaml.yml, build-ocaml-representative.yml). The cache
is content-addressed, so `--require-checksums` stays on and still verifies the
original archives. The contract validators (ocaml_toolchain_lock.py,
ocaml_representative_ci.py) now require the step. When adding a new workflow
that runs `opam install` against the pinned repository, copy the mirror step.
