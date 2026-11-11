---
category: CI & GitHub Actions
---

# A BUILD file must find rustup on PATH

The macOS CI job for PR #19125 successfully ran `rustup` during toolchain
setup, then failed the MACSYMA WebAssembly package because its `BUILD` file
invoked the missing executable `$HOME/.cargo/bin/rustup`.

Invoke `rustup` through `PATH` so the build uses the executable provided by
the runner or developer environment. When the build uses `rustup run stable`,
pass `--toolchain stable` to `rustup target add` so both commands use the
same toolchain even if the directory overrides the default.
