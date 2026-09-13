---
category: Repo policy / workflow reminders
---

# Generated BUILD scripts must be POSIX-compatible

The repo's build-tool runs `BUILD` files via `sh`, which on Ubuntu CI is dash. Dash rejects `set -o pipefail` ("Illegal option -o pipefail"). Don't emit `#!/usr/bin/env bash` shebangs or `set -euo pipefail` from any code-generator's BUILD template. Plain commands match the convention of `cli-builder`, `state-machine`, etc.
