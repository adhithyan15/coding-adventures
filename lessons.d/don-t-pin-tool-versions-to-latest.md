---
category: CI & GitHub Actions
---

# Don't pin tool versions to `latest`

`astral-sh/setup-uv@v4` with `version: latest` resolved to a release missing `aarch64-apple-darwin`. Use a known-good version range like `"0.6.x"`.
