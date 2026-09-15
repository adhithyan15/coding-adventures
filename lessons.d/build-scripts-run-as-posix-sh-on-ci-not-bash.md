---
category: BUILD files & dependency management
---

# BUILD scripts run as POSIX sh on CI, not bash

`set -euo pipefail` errors with `Illegal option -o pipefail` on Ubuntu's `/bin/sh` (dash). Use `set -e` only; replace `[[ ]]` with `[ ]`; no arrays; no `local`. Shebangs are ignored when the script is sourced/dispatched by name. Verify locally with `sh ./BUILD`, not `bash ./BUILD`.
