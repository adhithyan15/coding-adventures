---
category: Cross-platform & Windows BUILD_windows
---

# A Rust MSVC staticlib's native-static-libs note names a cargo-private windows.0.52.0.lib that must not be linked

`cpp/conduit` and `cpp/conduit-hello` link the Rust `conduit-capi` staticlib with `cl.exe`. The Unix scripts copy the library list out of `cargo rustc ... -- --print native-static-libs`; doing the same on MSVC would also copy `windows.0.52.0.lib`.

That name comes from the `windows-targets` 0.52 crate (`#[link(name = "windows.0.52.0")]`). The import library behind it lives in the `windows_x86_64_msvc` crate's own directory and is found through a `cargo:rustc-link-search` that only cargo-driven links see. A foreign consumer that puts `windows.0.52.0.lib` on its link line gets `LNK1181: cannot open input file` (the GNU target prints `-lwindows.0.52.0` and the Go recipe likewise leaves it out of its hand-written list).

Rule: for a non-cargo consumer of a Rust staticlib, start from a fixed list of SDK libraries (Rust 1.100 on MSVC: `kernel32 advapi32 ntdll userenv bcrypt ws2_32 synchronization`), merge in what rustc reports for diagnosis, and drop `windows.<version>.lib`. `build-conduit-capi-windows-msvc.ps1` does exactly that. Written without a Windows machine: the first Windows CI run is the proof, so read its log for `LNK` lines before the compile lines.
