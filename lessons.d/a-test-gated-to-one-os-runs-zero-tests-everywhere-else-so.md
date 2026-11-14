---
category: Testing & coverage
---

# A test gated to one OS runs zero tests everywhere else, so write behaviour tests against the public API and run them on every OS

`tcp-reactor`'s only tests were `#[cfg(target_os = "macos" | "freebsd" | ...)]` and
poked the engine's private methods. On Linux `cargo test -p tcp-reactor` therefore ran
**zero** tests and reported success, and on Windows the crate was
`compile_error!`-gated off. For months the package looked covered in every CI leg while
being exercised on one OS, and nothing flagged it.

What to do instead:

- Write behaviour tests against the public API (a `bind_native` constructor plus plain
  `std::net` clients) so the same assertions run on every OS and every engine.
- Check the test count, not just "ok". `running 0 tests` is a gap, not a pass.
- When a platform gate is unavoidable, put only the construction behind it (a
  `NativeTcpReactor` alias), never the assertions.

Moving those tests to the public API immediately gave the Linux epoll engine eight real
tests, and they passed unchanged, which is the baseline the Windows engine was then held
to.
