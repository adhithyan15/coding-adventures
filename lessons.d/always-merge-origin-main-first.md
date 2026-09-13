---
category: CI & GitHub Actions
---

# Always merge `origin/main` first

before reasoning about CI failures — the CI already merges your branch into main before building, so local reasoning about "what crates exist" is wrong if main moved.
