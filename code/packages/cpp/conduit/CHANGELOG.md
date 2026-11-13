# Changelog — Conduit (C++)

## [0.1.2] - 2026-11-13

### Added

- **Real Windows build.** `BUILD_windows` used to be an echo-skip ("tests use POSIX
  sockets"); it now runs `tools\run-tests.ps1`, which mirrors `tools/run-tests.sh`
  with MSVC: it builds the MSVC-ABI `conduit_capi.lib` (default Rust toolchain, so no
  GNU target), compiles `tests/test_*.cpp` with `cl.exe` (`/std:c++17 /EHsc /W4 /MD`),
  links the system libraries a Rust staticlib needs (`kernel32 advapi32 ntdll userenv
  bcrypt ws2_32 synchronization`; `synchronization.lib` holds `WaitOnAddress` /
  `WakeByAddress*`) and runs every suite, including the end-to-end server test. A
  failing suite fails the recipe with its exit code; nothing is skipped.
- `code/scripts/build-conduit-capi-windows-msvc.ps1`, shared with `cpp/conduit-hello`:
  drops Git for Windows' `usr\bin` from `PATH` so a bare `link` is MSVC's linker (and
  asserts it), asserts `cl`/`cargo`/`rustc` and an `x86_64-pc-windows-msvc` host,
  builds only the staticlib (`cargo rustc --release --crate-type staticlib -- --print
  native-static-libs`) and merges the libraries rustc reports with the fixed list above.
- `tests/net_compat.h`: a small shim so the E2E HTTP client builds on BSD sockets and
  Winsock (`WSAStartup`/`WSACleanup` once, `SOCKET`/`INVALID_SOCKET`, `closesocket`,
  int-sized `send`/`recv`, and `SO_RCVTIMEO` as a DWORD of milliseconds rather than a
  `timeval`, which Winsock would silently read as 5 ms). On Unix each wrapper is the
  same system call as before.

### Changed

- `CMakeLists.txt` picks the library per platform: `target/release/libconduit_capi.a`
  (unchanged on Unix), `target/release/conduit_capi.lib` on MSVC, and the
  `x86_64-pc-windows-gnu` archive on MinGW, adding the Windows system libraries on
  `WIN32`.
- `tests/test_server.cpp`: no direct POSIX includes; the header-name lowercasing now
  casts through `unsigned char` before `std::tolower`.

### Verification

Unix `sh tools/run-tests.sh` passes (3/3 suites). The Windows-shimmed tests were also
compiled with `-Wall -Wextra -Wpedantic -Werror` and linked with mingw-w64 against the
`x86_64-pc-windows-gnu` staticlib, and ran green under Wine (Winsock loopback E2E
included). The MSVC/PowerShell path itself runs only on the Windows CI leg.

## [0.1.1] - 2026-10-23

### Fixed

- **Use-after-free after `bind()`.** `bind()` marked the `Application`
  consumed but registration methods ignored the flag and kept writing through
  the stale `app_` pointer, which `conduit_server_bind` had already freed (it
  takes the app on success *and* on failure). `bind()` now nulls `app_` before
  the native call, and every method goes through a private `live()` check that
  throws `std::logic_error` on a bound or moved-from `Application`.
- `live()` is called in its own statement before each `new Handler(...)`:
  C++ leaves function-argument evaluation order unspecified, so folding it into
  the argument list could allocate the closure first and leak it on the throw.
- New tests: `application_is_unusable_after_bind`,
  `application_is_unusable_after_failed_bind`,
  `moved_from_application_is_unusable`.

## [0.1.0] - 2026-06-13

### Added — WEB13 C++ Conduit port

- A header-only Sinatra/Express-style web framework for C++ over the Rust
  web-core engine, via the reusable `conduit-capi` C ABI (the second consumer
  after Swift). No third-party deps.
- **DSL**: `Application` with `get/post/put/del/patch`, `route`, `before`,
  `after` (transforming), `notFound`, `onError`, `set`/`getSetting`, `bind` —
  all chainable. Handlers are `std::function<Response(const Request&)>`.
- **Response** helpers `html/json/text/respond/redirect`; `redirect` throws on
  CR/LF. **halt(...)** for Sinatra-style non-local exits (throws `Halt`).
- **Request**: `method/path/queryString/body/contentType/remoteAddr/error`,
  `param/query/header` (returning `std::optional`).
- **Server**: `serve` (foreground), `serveBackground`, `stop`, `localPort`,
  `running` — RAII-owned.
- Closures are heap-boxed and freed via a `ctx_free` trampoline; trampolines have
  C linkage (so function-pointer types match the ABI exactly) and catch all C++
  exceptions so none unwind across the C boundary.

### Tests

16 tests (zero-dependency harness): response helpers incl. native round-trip and
CR/LF guard; Application DSL/settings/bind; a full end-to-end server driven by a
POSIX-socket HTTP/1.0 client with a watchdog thread. `tools/run-tests.sh` builds
the C ABI, queries the platform's `native-static-libs`, and links each test.

### Security

Header-injection defense, status clamping, and UTF-8 validation are inherited
from `conduit-capi` (audited once for all C-ABI ports).
