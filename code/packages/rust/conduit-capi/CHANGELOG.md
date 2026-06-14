# Changelog — conduit-capi

## [0.1.0] - 2026-06-13

### Added — reusable Conduit C ABI (WEB12 foundation)

- First release: the whole Conduit framework exposed over `extern "C"` behind
  `include/conduit_capi.h`, to be shared by the Swift/C++/Go/C#/F#/Dart/Haskell
  ports rather than re-wrapped per language.
- App/server/request/response surface; opaque handles; function-pointer + opaque
  `ctx` + `ctx_free` callback model; transforming after-hook; thread-local error
  channels (`conduit_last_error`, `conduit_capi_report_error`).
- Trust boundary enforced once: `header_safe` (drops CR/LF/control/`:` headers),
  status clamping (100–599), UTF-8 validation on every inbound string, panic-free
  helpers.
- `crate-type = ["staticlib", "cdylib", "lib"]`; 5 pure-Rust unit tests.
