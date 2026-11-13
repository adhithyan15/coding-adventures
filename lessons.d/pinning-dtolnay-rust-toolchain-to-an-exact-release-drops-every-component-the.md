---
category: Supply chain & CI pinning
---

# Pinning dtolnay/rust-toolchain to an exact release drops every component the runner's preinstalled stable carried

PR #19215 changed every `dtolnay/rust-toolchain` step from the floating
`stable` to `toolchain: "1.99.1"`. CI on that PR stayed green, but after it
merged every BUILD running `cargo fmt -- --check` failed with
`'cargo-fmt' is not installed for the toolchain '1.99.1-x86_64-unknown-linux-gnu'`.

GitHub's runner images preinstall rustup's `stable` with rustfmt and clippy.
With `toolchain: stable` the action updated that toolchain in place, so the
components were already there and nobody had to list them. An exact release
is a *new* toolchain: it gets the action's minimal profile plus exactly the
`components:` input, which for the build job was `clippy,llvm-tools`. The
PR's own CI did not catch it because no package with a `cargo fmt` BUILD line
happened to rebuild there.

Fix: `components: clippy,llvm-tools,rustfmt` on the build job's Rust step.

Next time: when you replace a floating toolchain with an exact one, list
every component the old setup got implicitly. Check by running the step's
toolchain locally (`rustup component list --toolchain <ver> --installed`)
and comparing with what the BUILDs invoke (`cargo fmt`, `cargo clippy`,
`llvm-cov`, ...). A green PR proves only the packages it rebuilt.
