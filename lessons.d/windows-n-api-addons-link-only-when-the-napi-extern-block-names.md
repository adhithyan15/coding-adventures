---
category: Cross-platform & Windows BUILD_windows
---

# Windows N-API addons link only when the napi extern block names node.exe via raw-dylib

**What went wrong.** `typescript/conduit` failed on `build (windows-2025)`
with `rust-lld: error: undefined symbol: napi_wrap` (and 41 more `napi_*`).
node-bridge's `build.rs` only handled macOS; Linux tolerates undefined
symbols in a shared object, but a Windows DLL must name the module behind
every import.  Other native packages hid the gap behind BUILD_windows stubs
that skip the build, so it surfaced only for conduit, which has no stub.

**Fix.** Mark node-bridge's `extern "C"` block
`#[cfg_attr(windows, link(name = "node.exe", kind = "raw-dylib", modifiers = "+verbatim"))]`.
rustc then writes the `node.exe` import table itself: no `node.lib`
download, no Node.js install lookup.  `+verbatim` keeps the name `node.exe`
instead of `node.exe.dll`.

**Do differently.** Verify a Windows link from Linux before pushing:
`rustup target add x86_64-pc-windows-msvc`, make empty system libs with
`rust-lld -flavor link /lib /llvmlibempty /out:kernel32.lib /machine:x64`
(likewise ntdll, userenv, ws2_32, dbghelp, msvcrt), then
`CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER=rust-lld RUSTFLAGS="-L native=<dir> -C link-arg=/errorlimit:0" cargo build --target x86_64-pc-windows-msvc`
and count `undefined symbol: napi_` lines.  The CRT symbols the empty libs
leave unresolved are expected noise; `/errorlimit:0` matters because lld
stops at 20 errors and can hide the ones you care about.  Prefer a real
link fix over a BUILD_windows skip stub.
