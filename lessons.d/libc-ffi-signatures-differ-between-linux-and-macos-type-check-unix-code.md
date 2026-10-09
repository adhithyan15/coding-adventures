---
category: Cross-platform & Windows BUILD_windows
---

# libc FFI signatures differ between Linux and macOS; type-check unix code for an Apple target before pushing

`chief-of-staff-spawn-isolation`'s pty test called
`libc::openpty(&mut m, &mut s, null_mut(), null(), null())`. On Linux the
`libc` crate declares the termios and winsize arguments `*const`, so it
compiled. On macOS it declares them `*mut`, so the `build (macos-latest)` job
failed with E0308 after a 50-minute run. The code was `#[cfg(unix)]` and had
only ever been compiled on Linux.

What to do instead:
- Pass `null_mut()` for a nullable pointer argument. A `*mut` coerces to
  `*const`, so it fits both declarations.
- Before pushing `cfg(unix)` or macOS code, type-check it for an Apple target
  from the Linux container. Checking needs no linker or SDK:
  `rustup target add aarch64-apple-darwin`, then
  `cargo check --target aarch64-apple-darwin --all-targets -p <crate>`.
