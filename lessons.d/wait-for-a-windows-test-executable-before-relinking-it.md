---
category: Testing & coverage
---

# Wait for a Windows test executable before relinking it

Starting a second Cargo test invocation after editing the same integration test
while its earlier executable was still running failed at link time with
`permission denied` writing the executable. Windows locks a running executable;
this was not a Rust compilation or product test failure. Wait for the earlier
session to finish before rebuilding the same test target, then rerun validation
on the final source.
