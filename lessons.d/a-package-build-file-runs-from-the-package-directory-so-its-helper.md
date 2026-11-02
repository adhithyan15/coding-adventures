---
category: CI & GitHub Actions
---

# A package BUILD file runs from the package directory, so its helper-script paths must not repeat the package's own folder name

**What went wrong.** #18401 (`zpu-simulator`) failed `build (ubuntu-latest)`
and `build (macos-latest)` with `python3: can't open file
'.../code/packages/rust/zpu-simulator/zpu-simulator/tools/verify_oracle_vectors.py'`.
The build tool runs each BUILD command with the package directory as its
working directory, so `zpu-simulator/tools/...` doubled the folder name.
`cargo test` passed earlier in the same BUILD, so local runs from the
repository root hid the mistake.

**The fix.** Use paths relative to the package directory, such as `python3
tools/verify_oracle_vectors.py`.

**What to do differently.** Check a new BUILD file by running each command
from inside the package directory (`cd code/packages/<lang>/<pkg> && sh -c
'<command>'`), not from the repository root.
