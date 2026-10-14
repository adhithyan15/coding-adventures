---
category: Cross-platform & Windows BUILD_windows
---

# Put Node on the child shell PATH before npm lifecycle scripts

The fresh imported documentation checker install reached postinstall, but its
cmd.exe child could not find node even though PowerShell resolved node.exe.
The npm install is failed validation, not a fresh runtime build. Preserve its
npm debug log. Prepending Node still failed because the inherited PATH exceeded
cmd.exe's environment-command limit, so even where.exe was unavailable by name.
Use a bounded child-only PATH containing the selected Node/npm, System32,
Windows and required task tool bins; keep the original environment outside that
child unchanged. Rerun the real install and source build/checks. Do not change
system PATH or another run's environment to fix a local validation process.
