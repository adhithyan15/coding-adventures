---
category: Cross-platform & Windows BUILD_windows
---

# SO_RCVTIMEO takes a DWORD of milliseconds on Winsock, so a ported timeval silently becomes a 5 ms timeout

Porting a POSIX loopback test client to Winsock, the line `timeval tv{5, 0}; setsockopt(fd, SOL_SOCKET, SO_RCVTIMEO, &tv, sizeof(tv));` still compiles after the headers are swapped (only the cast to `const char*` is needed), but Winsock reads the option as a single `DWORD` of milliseconds. The first four bytes of a `timeval` are `tv_sec`, so the socket gets a 5 ms receive timeout and a perfectly healthy server looks flaky.

Fix: set a `DWORD` of `seconds * 1000` on Windows and keep the `timeval` on Unix. `cpp/conduit/tests/net_compat.h` exposes one `set_recv_timeout_seconds()` for both. Other Winsock differences that compile but change behaviour: `send`/`recv` take and return `int`, `close()` is `closesocket()`, and nothing works before `WSAStartup()`. Wine runs Winsock programs, so a mingw-w64 build of the test is a cheap check on Linux that the shim is at least self-consistent.
