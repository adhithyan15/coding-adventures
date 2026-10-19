---
category: Repo policy / workflow reminders
---

# On macOS a mktemp template must end in X characters; do not append a filename suffix

BSD `mktemp` requires the replacement `X` characters to end the template. A
template such as `/tmp/build-plan.XXXXXX.json` is treated as a literal path on
macOS; a prior file at that path then produces `mkstemp failed: File exists`
instead of a fresh filename. Use a suffix-free template such as
`/tmp/build-plan.XXXXXX`, or create a temporary directory first and place a
fixed-suffix filename inside it. Keep `set -euo pipefail` enabled so this setup
failure stops the validation command before later steps can obscure it.
