---
category: CI & GitHub Actions
---

# A pinned CI Rust toolchain is not the stable alias, so a BUILD must never name stable explicitly

CI's Rust step tracked `stable`, so when the runners' stable moved from 1.97 to 1.100 the
`-clippy -D warnings` gate went red in ~30 crates nobody had touched, and Cargo 1.100's new
build-dir layout broke five `tests/optimized_ir.rs` gates at once. Pinning an exact version
fixes the drift, but an earlier pin attempt had been reverted: a pinned default toolchain is
a differently named toolchain from the `stable` alias (rustup refuses to link anything named
`stable`), so BUILDs that said `rustup run stable cargo build --target X` or
`rustup which --toolchain stable rustc` ran a *second*, unpinned toolchain that never got the
targets installed by the plain `rustup target add X` (which targets the default).

What to do: keep exactly one Rust on the runner. The `Set up Rust` step in `ci.yml` pins
`toolchain: '1.100.0'`; BUILDs call plain `cargo` / `rustup which rustc` /
`rustup target add`, never `stable`. To bump: run
`cargo clippy --workspace --all-targets -- -D warnings` on the new version first, fix what it
reports, then change the number in the same PR. Other workflows (release-*, neural-framework,
lang-runtime-safety) still say `stable` and are not covered by this pin.
