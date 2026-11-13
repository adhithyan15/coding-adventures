---
category: BUILD files & dependency management
---

# A shared cargo target dir bakes the first builder's repository root into path-based tools, so rebuild them from your own worktree

**What went wrong.** Two worktrees of the same repository were pointed at one `CARGO_TARGET_DIR` to save disk. `gen_evidence` (and `warrant-grammar`'s repository-scanning tests) find the repository root through `env!("CARGO_MANIFEST_DIR")`, which is fixed when the binary is compiled. Cargo treated the second worktree's identical sources as fresh and ran the first worktree's binary, which read and wrote the *first worktree's* files: `fixture "code/packages/rust/lzw/evidence.bindings" does not exist`. Worse, had the lookup succeeded it would have regenerated another worktree's harness.

**Fix.** Before running anything that locates the repository from `CARGO_MANIFEST_DIR`, `touch` its source so cargo rebuilds it against this worktree, copy the binary out (`strings <binary> | grep worktrees` shows whose paths are in it), run the copy, then `touch` the source again so the other worktree's next build is not served your binary. The packages' own test binaries are unaffected: they `include_str!` relative to their own source.

**Do differently.** A shared target dir is safe only for artifacts that embed nothing path-specific. Tools that scan or write the checkout should take the root as an argument or from the current directory, not from `CARGO_MANIFEST_DIR`; until they do, check which worktree a cached tool belongs to before trusting its output, and prefer a private target dir for them when the disk allows.
