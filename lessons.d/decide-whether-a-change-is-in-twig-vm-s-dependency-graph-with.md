---
category: Repo policy / workflow reminders
---

# Decide whether a change is in twig-vm's dependency graph with cargo tree, not from a spec's list of crates

**What went wrong.** CLAUDE.md asks for the local Miri script on any change
to `twig-vm` "or its deps". The LANG83 spec said PR7b–PR7e "touch `iir-to-*`,
`jit-core` and `vm-core`, which `twig-vm` depends on", and four PRs repeated
that claim in their hand-offs. When PR7e checked it with
`cargo tree -p twig-vm -i <crate>`, only `interpreter-ir` was in the graph:
`vm-core`, `jit-core` and every `iir-to-*` backend printed "did not match any
packages". Miri still applied to PR7e (it changes `interpreter-ir`), but for
the wrong reason. A PR that touched only `vm-core` would have spent 30–90
minutes on a Miri run that could not see its change, and a PR touching a
crate the spec forgot would have skipped one it needed.

**Fix.** The spec now says only `interpreter-ir` puts these PRs in
`twig-vm`'s graph.

**Do differently.** For each crate the PR touches, run
`cargo tree -p twig-vm -i <crate> --offline` from `code/packages/rust`. A
dependency path printed means the crate is in the graph, so run Miri. "Did
not match any packages" means it is not. Quote the command's answer in the
hand-off, not a document's list. A dependency graph changes with every
`Cargo.toml` edit, and prose about it does not.
