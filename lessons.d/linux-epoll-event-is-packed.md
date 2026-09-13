---
category: Rust
---

# Linux `epoll_event` is packed

A plain `#[repr(C)]` mirror works for single events but corrupts/drops readiness when `epoll_wait` returns multiples. Always use the kernel's packed layout.
