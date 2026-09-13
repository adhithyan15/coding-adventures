---
category: Native extensions & FFI
---

# Do not drive a Lua state from the test runner while native worker threads invoke callbacks on that same state

A Rust mutex can serialize the worker
callbacks with each other, but it cannot guard ordinary Lua execution in the
parent test thread. The result is nondeterministic stack corruption and
SIGSEGVs. Exercise a foreground native server in a dedicated Lua child process
and drive it over TCP from the parent.
