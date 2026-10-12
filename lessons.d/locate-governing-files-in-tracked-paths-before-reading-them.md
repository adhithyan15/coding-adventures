---
category: Repo policy / workflow reminders
---

# Locate governing files in tracked paths before reading them

The post-merge parity refresh guessed `.claude/package-parity-roadmap.md` and
`code/specs/fixtures/language-source-input-registry.json`; both reads failed.
The actual files are `code/specs/package-parity-roadmap.md` and the registry
under `code/specs/fixtures/build-tool-v1/`. No repository content was changed
by those failed reads. Resolve uncertain paths with `rg --files` or the
checked fixture directory listing before batching governing-reference reads.
Commands in one batch can also need different working directories: a later
lessons validation was accidentally launched from the Haskell package and
failed to locate `code/scripts/lessons.py`. Run repository validators from the
repository root and Cabal checks from the package root, with separate exit codes.
