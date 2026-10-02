---
category: Cross-platform & Windows BUILD_windows
---

# A POSIX env-assignment prefix in a BUILD line breaks on Windows, where lines run under cmd /C

`journal-mosaic-app` and `spice-mosaic-app` had only a BUILD file, with the
line `RUSTC="$(rustup which rustc)" rustup run stable cargo build ...
--target wasm32-unknown-unknown`. On Windows the build tool runs each line
under `cmd /C` (`internal/executor/executor.go`). cmd treats `=` as a
delimiter, so it runs a command named `RUSTC`. That resolves
case-insensitively to rustup's `rustc.exe` proxy, which fails with
`error: unknown proxy name: 'RUSTC'` after every test has already passed.
The failure showed up on #16391, which changed a dependency (the mosaic app
crates) and so rebuilt both apps on Windows.

**Fix:** add a `BUILD_windows` that runs the same steps without the POSIX
prefix (`rustup run stable cargo build -p <crate> --target
wasm32-unknown-unknown`). On Windows CI, rustup's stable is the only rustc.

**Do instead:** any BUILD line that uses shell-only syntax (`VAR=value cmd`,
`$(...)`, `&&` chains that rely on POSIX semantics, quoting) needs a
`BUILD_windows` alongside it. Before pushing, grep for
`^[A-Z_]+=.* ` in BUILD files that have no `BUILD_windows`.
