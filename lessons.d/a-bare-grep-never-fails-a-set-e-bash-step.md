---
category: CI & GitHub Actions
---

# A bare '! grep' never fails a set -e bash step

Bash's `set -e` does **not** exit when a command's status is inverted with `!`.
So in a `set -euo pipefail` CI step, `! grep -F "error" "$log"` that *matches*
returns 1, and the script carries on. The "no error in the log" gate never fires.

This was copied into Journal's Qt launch smoke (J5a, #14416) from the TaskApp and
Counter blocks above it in `ci.yml`, which have the same hole. The pre-push
security review caught it.

**Do instead:** make the failure explicit:

```bash
if grep -F "Mosaic Rust runtime unavailable" "$log"; then exit 1; fi
```

When reviewing or copying a CI step, treat any line starting with `! ` under
`set -e` as a gate that does not gate. Pin the explicit form in the workflow's
unit test, not the `!` form.
