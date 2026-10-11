---
category: Cross-platform & Windows BUILD_windows
---

# A Windows npm script can miss node with an inherited toolchain PATH

During a clean Forme landing-page bootstrap, PowerShell resolved `node` and
`npm` under `C:\Program Files\nodejs`, but the npm script's child shell failed
with `'node' is not recognized as an internal or external command`. The inherited
PATH contained many accumulated worktree tool directories.

Repeating the same bootstrap with a short, process-local PATH containing
Node.js, Windows, and Git resolved the command and installed the complete local
dependency graph. Keep this adjustment local to the verification process;
do not change the user's persistent PATH or patch the product bootstrap to
work around a session environment. This observation does not establish which
inherited PATH entry caused the child-shell failure.
