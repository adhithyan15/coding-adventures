# Changelog — conduit-hello (C++)

## [0.1.1] - 2026-11-13

### Added

- **Real Windows build.** `BUILD_windows` was an echo-skip ("uses POSIX sockets"); it
  now runs `tools\run.ps1`, the MSVC counterpart of `tools/run.sh`: it builds the MSVC
  `conduit_capi.lib` through the shared `code\scripts\build-conduit-capi-windows-msvc.ps1`
  (PATH fix so `link` is MSVC's, system libraries `kernel32 advapi32 ntdll userenv
  bcrypt ws2_32 synchronization`), compiles the demo and `tests/smoke.cpp` with `cl.exe`
  (`/std:c++17 /EHsc /W4 /MD`) and runs the smoke test against a real server.
- `tests/net_compat.h`: BSD-socket/Winsock shim for the smoke test's HTTP client (same
  design as the one in `cpp/conduit`; `SO_RCVTIMEO` is a DWORD of milliseconds on
  Windows). Unix behaviour is unchanged.

### Changed

- `tests/smoke.cpp` now checks `send()` and returns `false` on failure instead of
  ignoring the result.

## [0.1.0] - 2026-06-13

### Added

- Initial C++ `conduit-hello` demo exercising the full Conduit DSL: `html`,
  `json`, `text`, `respond`, `redirect`, route params, query params, a body echo,
  a `before` filter that short-circuits with `halt`, an `after` header-stamping
  hook, a throwing handler routed to `onError`, and a custom `notFound`.
- `tests/smoke.cpp`: launches the demo on an OS-assigned port and asserts on real
  HTTP responses (POSIX-socket client, watchdog thread).
