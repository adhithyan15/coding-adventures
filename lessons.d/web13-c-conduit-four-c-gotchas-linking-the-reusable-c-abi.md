# WEB13 C++ Conduit — four C++ gotchas linking the reusable C ABI

**Date:** 2026-06-14

Second consumer of the `conduit-capi` C ABI (after Swift). Header-only C++ wrapper.
Four things worth remembering:

1. **A reused HTTP-client result struct must be RESET each request.** The E2E
   client appended parsed headers with `emplace_back` into a caller-provided
   `out` struct reused across requests, so `headerValue("content-type")` returned
   the FIRST match (from an earlier route) — the echo content-type assertion
   failed even though the server was correct. Always `out = Result{};` at the top
   of the request function. (The server/port was never wrong; the test client was.)

2. **A joinable `std::thread` destroyed during stack unwinding calls
   `std::terminate`.** A failing assertion in the E2E threw, which destroyed the
   still-joinable watchdog `std::thread` before the harness could catch it →
   `libc++abi: terminating`, masking the real failure. Wrap the body in
   `try { ... } catch (...) { join_watchdog(); throw; }` so the thread is always
   joined first and the real error surfaces.

3. **`extern "C" inline` trampolines, not C++ ones.** Passing a C++ function
   pointer where the C ABI typedef expects a C-linkage pointer warns under
   `-Wpedantic -Werror` (and is technically a language-linkage mismatch). Declare
   the trampolines `extern "C"` (and `inline` so the single header stays
   include-safe across TUs).

4. **Delete the copy ctor → you must add a move ctor for return-by-value.** A
   `make_app()` factory returning `Application` by value fails to compile if the
   copy ctor is deleted and no move ctor exists — NRVO is not guaranteed for a
   named local. Add `Application(Application&&) noexcept` that transfers the handle
   and marks the source consumed.

**Link flags:** a Rust staticlib needs its platform `native-static-libs` (e.g.
`-liconv -lSystem -lc -lm` on macOS, `-lpthread -ldl -lrt -lm` on Linux). Don't
hardcode them — query `cargo rustc --release --crate-type staticlib -- --print
native-static-libs` and pass the result to the C++ link line.

**Build-tool note:** C++ packages are discovered as "unknown" language (the
build-tool's language list has no "cpp"), so the undeclared-local-ref validator
skips them — but declare `# build-tool: deps=rust/conduit-capi` anyway so the
build graph pulls the Rust crate in and the runner gets `cargo`.
