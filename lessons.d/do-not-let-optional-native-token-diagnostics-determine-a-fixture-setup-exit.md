---
category: Rust
---

# Do not let optional native token diagnostics determine a fixture setup exit status

Native Windows CI passed the new structural-policy rejection and supported
denial tests, but the inherited fixture stopped during setup. Its added
`whoami.exe /all` diagnostic resolved to Git's Unix binary and failed on `/all`.
The optional diagnostic's exit status then caused the strict setup assertion
to fail before publication was attempted. Prepending Git's `usr/bin` locally
reproduced the exact error and failing test line.

Resolve the native utility from `[Environment]::SystemDirectory`, never PATH.
Keep Get-Acl/Set-Acl failures fatal before the optional diagnostic block, record
diagnostic failures separately, and return setup success only after the actual
ACL operations complete. Test a child PATH containing a shadow executable,
verify native utility diagnostics and intended ACL, and keep an invalid setup
path as a failure control. Do not skip the inherited policy rejection test.
