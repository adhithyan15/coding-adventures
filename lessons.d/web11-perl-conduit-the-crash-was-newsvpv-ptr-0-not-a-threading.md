# WEB11 Perl Conduit — the crash was `newSVpv(ptr, 0)`, NOT a threading wall

**Date:** 2026-06-14

**What happened:** The Perl Conduit cdylib SIGSEGV'd on the first request dispatch. I initially (wrongly) blamed non-threaded Perl + web-core worker threads. The real cause was a one-line memory bug, and the threading model is actually fine.

**Two corrected findings:**

1. **`newSVpv(ptr, len)` treats `len == 0` as "call `strlen(ptr)`".** Perl's `newSVpv` uses the C string length when you pass 0. An empty Rust `&str` (`""`) has a non-NUL-terminated pointer, so `strlen` reads out of bounds → segfault. The very first empty field (an empty `QUERY_STRING`) crashed it. **Fix: use `newSVpvn` (explicit length, never strlens) for ALL Rust→Perl string conversions** where the value can be empty. The crash reproduced single-threaded, which is what unmasked the misdiagnosis.

2. **The embeddable-http-server serve path is single-threaded inline — it does NOT spawn.** `HttpServer::bind` builds a single `TcpRuntime` (not the `ShardedTcpRuntime`); `TcpRuntime::serve()` → `StreamReactor::serve()` runs the event loop on the *calling* thread and dispatches handlers inline. So foreground `serve()` runs handlers on the calling thread — perfect for a single-interpreter language. (The `ShardedTcpRuntime` that DOES `thread::spawn` per worker is a separate, unused-by-this-path API.) The only spawn was in my own cdylib's `serve_background`; that one path is unsafe for non-threaded Perl, so the Perl port serves in the foreground and tests fork a client process.

**Rule:** When a foreign-language port over web-core crashes on dispatch, reproduce it **single-threaded** (foreground serve in a standalone process + curl) before blaming threads. And from Rust always cross the boundary with the explicit-length string constructor (`newSVpvn`, not `newSVpv`); the strlen-on-zero footgun is silent until an empty value hits it. For a single-interpreter language, prefer foreground `serve()` (the inline reactor runs handlers on the calling thread) and fork a client for concurrent E2E tests.

---
