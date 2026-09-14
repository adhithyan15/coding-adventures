---
category: CI & GitHub Actions
---

# bash -n does not catch a mis-indented heredoc terminator

2026-09-13.

A `run: |` block in a GitHub workflow is dedented by YAML before bash sees it,
so whether `<<'PY' ... PY` works depends on an indentation that nothing in the
file makes visible. Get it wrong by one space and bash swallows the rest of the
job as heredoc body.

`bash -n` does not help. Measured: a script with the terminator at column 0 and
the same script with it indented one space both return 0 with empty stderr.

Check it by asserting `"\nPY\n"` appears in the post-YAML-parse string, and
better, by extracting the block from `ci.yml` and RUNNING it. A lint that cannot
fail on the defect is not evidence about the defect.
