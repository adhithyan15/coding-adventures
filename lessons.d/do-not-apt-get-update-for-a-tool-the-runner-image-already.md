---
category: CI & GitHub Actions
---

# Do not apt-get update for a tool the runner image already ships

On 2026-11-19 a stalled Ubuntu mirror made all three `apt-get update` attempts in
`code/scripts/ci/apt-install.sh` time out, failing the required `kiln-sandbox` job
on PR #19689 with nothing Kiln-related wrong. strace was already on both runner
images (6.8 on 24.04, 6.19 on 26.04), so the update was pure risk.

Fix: guard the install with `command -v <tool>` and only call apt-install.sh when
the tool is missing; always print the version actually used. Before adding an apt
step, check the runner image's package list; an unconditional `apt-get update`
turns every mirror outage into a red required check.
