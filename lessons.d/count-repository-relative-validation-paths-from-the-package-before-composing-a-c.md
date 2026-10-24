# Count repository-relative validation paths from the package before composing a chained command

A chained package-validation command guessed that the repository lesson tool
was four parent directories above the package. The fourth parent is the
workspace root, while the tool lives under `code/scripts`, so the correct path
from `code/packages/typescript/human-language-data` is
`../../../scripts/lessons.py`. Resolve or print the target path before adding a
root-owned validator to a package-local command chain. Prefer running
repository-wide validators with the worktree root as the explicit working
directory.

This recurred from a Rust crate at the same `code/packages/rust/<package>`
depth: `../../../../scripts/lessons.py` incorrectly reached root-level
`scripts`, while `../../../scripts/lessons.py` reaches `code/scripts`. Keep the
repository validator in a separate root-cwd command to avoid this arithmetic.
