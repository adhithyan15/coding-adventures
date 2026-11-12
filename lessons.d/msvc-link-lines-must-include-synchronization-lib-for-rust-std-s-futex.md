---
category: Cross-platform & Windows BUILD_windows
---

# MSVC link lines must include synchronization.lib for Rust std's futex mutex

twig-aot hands `link.exe` a hand-written list of Win32 import libraries for the embedded gc-core-capi staticlib (which contains Rust std). When CI's unpinned `stable` moved to Rust 1.100.0 (2026-11-12), std started referencing `WaitOnAddress`/`WakeByAddressSingle`/`WakeByAddressAll`, which live only in `synchronization.lib`; unrelated PRs that trigger the "LANG Windows native execution" step failed with `LNK2019 __imp_WakeByAddressSingle` / `LNK1120`. Fix: add `synchronization.lib` (twig-aot `MSVC_WINDOWS_API_LIBRARIES`, and lang-aot's test `gc_link_args`).

Do differently: any hand-listed native-library set for a Rust staticlib is a toolchain-coupled contract. Keep it in one const with a reason per library, pin it with a Linux-runnable unit test, and when a Windows link fails with `__imp_` unresolved symbols after a Rust release, compare against `rustc --print native-static-libs`. A step that passed hours earlier on an unchanged tree points at the toolchain, not the PR.
