---
category: Cross-platform & Windows BUILD_windows
---

# A Rust test that spawns echo or false has no Windows executable to run, and Wine finds that without a Windows runner

`echo` is a cmd.exe builtin and `false` does not exist on Windows, so a unit test
that does `Command::new("echo")` or `Command::new("false")` fails there even
though the code under test is fine. In `programs/rust/unix-tools` ten such tests
were the *only* thing standing between the suite and a Windows run (the rest of
the Unix-only tests were already behind `#[cfg(unix)]`); the old `BUILD_windows`
was an `echo` skip justified as "targets Unix/POSIX behavior".

You do not need a Windows runner to find these. Cross-build the test binaries for
`x86_64-pc-windows-gnu` and run them under Wine:

```bash
export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc
export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_RUNNER=/usr/lib/wine/wine64   # Debian/Ubuntu wine64
export WINEPREFIX=$SCRATCH/wineprefix WINEDEBUG=-all
cargo test --target x86_64-pc-windows-gnu --all-targets --no-fail-fast
```

Wine runs the same Rust std Windows code paths, so it reproduces missing
executables, `\\?\` canonicalised paths and drive-letter assumptions. It does not
reproduce symlink privileges, `/tmp` existing (Wine maps it to `Z:\tmp`, so a test
that passes on `/tmp/...` under Wine can still fail on a real runner), or file
sharing semantics; read the absolute-path tests by hand.

For a test that must spawn something portable, the test executable itself works:
libtest's `--list` exits 0 and an unknown flag exits 101, on every OS.
