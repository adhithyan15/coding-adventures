---
category: CI & GitHub Actions
---

# Don't pin tool versions to `latest`

`astral-sh/setup-uv@v4` with `version: latest` resolved to a release missing `aarch64-apple-darwin`. A range such as `"0.6.x"` avoids that one failure but still floats.

**Current policy (2026-11):** every toolchain version is exact. `mise.toml` holds
the version of each tool a developer installs locally, and
`code/specs/data/toolchain-pins.json` holds CI-only tools and reasoned
exceptions. `code/scripts/toolchain_pins.py check` (run by the contracts job)
rejects `latest`, `stable`, `lts`, bare `x.y` ranges, an undated `nightly`,
`rustup run stable` / `+stable`, and any workflow version input that disagrees
with those two files. To bump a tool, edit `mise.toml` and let the checker list
every workflow line that has to move with it.
