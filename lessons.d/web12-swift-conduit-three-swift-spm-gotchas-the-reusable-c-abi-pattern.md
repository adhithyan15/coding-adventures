# WEB12 Swift Conduit — three Swift/SPM gotchas + the reusable C ABI pattern

**Date:** 2026-06-14

Built `conduit-capi` (a reusable Rust C ABI for the whole Conduit framework, to be
shared by WEB12–WEB18) and the Swift port on top of it. Three gotchas worth
remembering:

1. **`"\r\n"` is ONE extended grapheme cluster in Swift.** A CRLF check written as
   `location.contains("\r")` (Character-based) returns FALSE for a string
   containing `\r\n`, because Swift treats CRLF as a single `Character`. Scan
   **unicode scalars** instead: `location.unicodeScalars.contains { $0 == "\r" || $0 == "\n" }`.
   The response-splitting guard silently passed until a test caught it.

2. **A library's relative `-L` linker path breaks for downstream packages.** SPM
   runs the linker from the *root* package being built. So `Sources/CConduit` in
   the Conduit library's `linkerSettings` resolves correctly for Conduit's own
   tests but NOT when a demo/executable in another package links it — you get
   `library 'conduit_capi' not found`. Fix: the downstream package re-adds its own
   `-L <relative-path-to-the-staged-lib>` in its target's `linkerSettings`. (The
   library's wrong path then just emits a harmless `search path not found` warning.)

3. **Edition-2021 disjoint closure capture defeats a `Send + Sync` wrapper.** A
   closure that touches `cb.ctx` (a raw pointer field) captures *that field*, not
   the whole `Send + Sync` struct, so the closure isn't `Send`/`Sync`. Call a
   `&self` method (`cb.call(...)`) instead — a method borrows the whole struct, so
   the closure captures the wrapper and inherits its `Send + Sync`.

**Reusable C ABI pattern:** the seven C-ABI-capable ports (Swift/C++/Go/C#/F#/Dart/
Haskell) share ONE `conduit-capi` crate instead of re-wrapping the facade each
time. The trust boundary (header_safe, status clamp, UTF-8 validation, panic
isolation) is audited once. Handlers cross as a C function pointer + opaque `ctx`
+ a `ctx_free` destructor; the host boxes its closure. `crate-type = ["staticlib",
"cdylib", "lib"]` serves compile-time linkers, FFI loaders, and `cargo test`.
---
