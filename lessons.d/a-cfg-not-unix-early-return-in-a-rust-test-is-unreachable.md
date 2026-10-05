---
category: CI & GitHub Actions
---

# A cfg(not(unix)) early return in a Rust test is unreachable code on Windows, where CI builds with -D warnings

**What went wrong.** PR #16738 added a builder test whose symlink case only
works on Unix:

```rust
#[cfg(unix)]
std::os::unix::fs::symlink(..).unwrap();
#[cfg(not(unix))]
return;
let out = TempDir::new().unwrap(); // ...
```

On Linux the `return` is compiled out and everything passed, clippy
included. On Windows the `return` stays, so the rest of the test is
unreachable and `pkg` is unused. The `build (windows-2025)` job checks
packages with `-D warnings`, so `unreachable-code` and `unused-variables`
became errors and the package failed to compile there. Note that the
package's BUILD file (`cargo test`) does not show this flag, so reading BUILD
was not enough.

**The fix.** Scope the platform-only part to a `#[cfg(unix)] { ... }` block
and let the portable part of the test run everywhere.

**Next time.** Never end a platform-gated branch with an early `return` in
Rust. Gate a block, or the whole test (`#[cfg(unix)]` on the `#[test]`
function). When a change adds `cfg(...)` code, run clippy for the other
target too:
`rustup target add x86_64-pc-windows-gnu`, then
`cargo clippy -p <crate> --all-targets --target x86_64-pc-windows-gnu -- -D warnings`.
That needs only metadata, so no Windows linker is required.
