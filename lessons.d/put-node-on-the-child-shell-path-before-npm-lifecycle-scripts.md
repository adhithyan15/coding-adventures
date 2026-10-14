---
category: Cross-platform & Windows BUILD_windows
---

# Put Node on the child shell PATH before npm lifecycle scripts

The fresh imported documentation checker install reached postinstall, but its
cmd.exe child could not find node even though PowerShell resolved node.exe.
The npm install is failed validation, not a fresh runtime build. Preserve its
npm debug log. Prepend the selected Node runtime bin and npm bin to the task's
child PATH explicitly, retaining the original PATH, then rerun installation
and the actual source build/checks. Do not change system PATH or another run's
environment to fix a local validation process.
