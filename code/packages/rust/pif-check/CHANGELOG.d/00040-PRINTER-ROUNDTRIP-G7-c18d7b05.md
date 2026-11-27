## [0.4.0] - 2026-11-24

### Added

- PIF-003d (VER32 3.6 G7, N4, N5, R3, 3.8): the printer `pif-pp` (`printer`), its independent parser (`ppparse`, a hand-written stack machine; grammar-tools was not used, see the README), the file model decoded from PIF text with the file's own binder names (`filemodel`), the printed file format and its parser (`filetext`), and the checks (`printcheck`).
- `parse(print(G)) = G` for every goal on the goals list, and `parse(print(F)) = F` for every prelude, ADL, model and axioms file in a chain, plus a declaration-by-declaration agreement between the decoded file and the kernel's accepted environment (a reader or decoder disagreement is fatal). The printer refuses shadowed binders (when either binder is referred to), unprintable names, keywords as names, literals and projections.
- `names`: rule G7 (ASCII class for constants, binders and universe parameters; binders single-component, pairwise distinct with `_` exempt, and not constants of the environment), enforced on the goals list, and in constants-only form on the stored goal term when there is no list.
- `pif-pp file|check|goal` binary.
- Tests: the worked example's goal prints as `forall (a : Bool), forall (b : Bool), Eq.{1} Bool (Pif.and #1 #0) (Pif.and #0 #1)` and round trips; the prelude round trips and agrees with the kernel; the worked example's whole file is refused for shadowing; every term form round trips; G7 corpus (Cyrillic `a` beside Latin `a`, a binder named `Eq`, a binder named `Bool.false`, a non-ASCII universe parameter, zero-width characters, duplicate binders, numeric components); strict-parser and decoder corpora; truncation of every line; a kitchen-sink model file using every declaration kind.

### Changed

- The decoder checks the `end k` trailer count.
