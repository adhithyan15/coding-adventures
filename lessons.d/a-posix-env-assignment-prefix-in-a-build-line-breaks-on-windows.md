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

**Correction (same week):** a plain prefix is NOT a problem in itself. On
Windows the executor rewrites a leading `VAR=value cmd` or `VAR="value" cmd`
into `set "VAR=value"&& cmd` (`rewriteInlineEnvPrefixForWindows`), so
`RUSTDOCFLAGS="-D warnings" cargo doc` works there. Two kinds of line break:
- a value the rewrite refuses: `$`, a backtick, `&|;()<>`, or chained
  assignments, such as `RUSTC="$(rustup which rustc)"`;
- any assignment that is not at the start of the line, e.g. after `&&` or
  inside `( ... )`.

**Do instead:** let the build tool tell you. `-validate-build-files` now runs
each shared BUILD line through that same rewrite and rejects any `VAR=` left in
command position when the package has no `BUILD_windows`. Its first run found 18
more latent packages beyond the five fixed by hand. For the Windows file, use
cmd's `set "VAR=value" && cmd`, `;` as the path separator, and `\` in paths.
