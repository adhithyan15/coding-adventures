---
category: Compiler / VM / language pipeline
---

# Discover specification filenames before composing numbered names

The Perl release audit tried to open a guessed
`LANG82-perl-release-grammar-inventory.md` filename. The actual contract is
`LANG82-perl-release-grammars.md`; the read failed before any edits.
Discover the checked-in filename with `rg --files code/specs` or `git ls-tree`
before constructing a numbered spec path. Reuse the discovered path for reads
and edits so a plausible invented name cannot create a parallel contract.

Repeat during PREP01 octal selection: a guessed `PREP01-token-preprocessor.md`
read failed. `git ls-tree` resolved the actual
`PREP01-generic-source-preprocessor.md` before any implementation edits.
