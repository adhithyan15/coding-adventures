---
category: CI & GitHub Actions
---

# choco install exits 0 when the feed fails, so judge the install by the binary

The required `build (windows-2025)` job on #17811, a Mosaic slot rename that
does not touch CI, failed in the qpdf install step:

```
Failed to fetch results from V2 feed at 'https://community.chocolatey.org/api/v2/Packages(Id='qpdf',Version='12.4.2')' ... 504 (Gateway Timeout).
Unable to find package 'qpdf'.
Chocolatey installed 0/0 packages.
qpdf: command not found
##[error]Process completed with exit code 127.
```

Two things are worth keeping:

- **choco's exit status does not mean "installed".** With the feed down it
  resolved nothing, installed "0/0 packages" and exited 0. Under `set -e` the
  step carried on, so the failure surfaced one line later as a confusing
  `command not found` instead of at the install. An `|| retry` keyed on the
  exit code would never have fired.
- **The community feed is a shared dependency of a required check.** One
  transient 504 failed the PR, and a re-run had to wait for the whole run.

The fix judges the install by its result. The step retries up to three times,
pausing 30 s and then 60 s, until `command -v qpdf` succeeds. It does not
suppress the failure: if the feed stays down, `qpdf --version` still fails the
step and names qpdf. Simulated locally with a stub `choco` in both directions:
it recovers on attempt three, and it still exits 127 when nothing installs.

Do differently: when a package manager installs a tool a later step needs,
check for the tool, not the installer's exit code, and retry around a public
feed. Compare `apt-install.sh`, which handles the same kind of failure for
apt.
