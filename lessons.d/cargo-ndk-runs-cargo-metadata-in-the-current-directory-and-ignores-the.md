---
category: Rust
---

# cargo-ndk runs cargo metadata in the current directory and ignores the build's --manifest-path

PR #16255's CI step ran `code/scripts/build-mosaic-android-libs.sh` from the
repo root. The script called `cargo ndk ... build --manifest-path
code/packages/rust/Cargo.toml -p task-mosaic-app`, and cargo-ndk 4.1.2 failed
before building anything: "Failed to load Cargo.toml in current directory ...
could not find `Cargo.toml`". cargo-ndk runs its own `cargo metadata` in the
current directory first; the `--manifest-path` after `build` is only passed on
to cargo's build, too late.

Fix: run cargo-ndk from inside the workspace (`(cd -- "$workspace" && cargo ndk
... build -p "$package")`) and pass no manifest path.

Do differently: run a wrapper script locally from the directory CI runs it from
(the repo root), even when the real toolchain is missing -- a stand-in
ANDROID_NDK_HOME with a `source.properties` gets cargo-ndk as far as the
linker, which would have shown this at once.
