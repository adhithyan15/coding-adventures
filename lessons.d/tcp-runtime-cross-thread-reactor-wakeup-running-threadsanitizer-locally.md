# tcp-runtime — cross-thread reactor wakeup + running ThreadSanitizer locally

**Date:** 2026-06-14

Multi-core PR2: a `Send + Sync` `WakeHandle` lets an off-reactor mailbox interrupt
the reactor's `poll` immediately instead of waiting the 10 ms poll timeout. Two
durable lessons:

1. **Decouple the cross-thread trigger from the `&mut self` platform.** The
   platform's `wake(&mut self)` is owned by the reactor thread; producers are on
   other threads. The fix is a handle that owns a *duplicated* OS primitive
   (`Kqueue::try_clone` of the kqueue fd; `OwnedFd::try_clone` of the `eventfd`)
   and re-issues the trigger via a `&self` syscall (`kevent`/`write`, both
   thread-safe). Give the trait a **default `wake_handle` returning
   `Unsupported`** so a backend that can't share its primitive (IOCP, today) keeps
   working — callers just fall back to the poll timeout. Never make the whole
   reactor depend on a capability one platform lacks.

2. **Running ThreadSanitizer on this repo needs `-Zbuild-std`.** Plain
   `RUSTFLAGS="-Zsanitizer=thread" cargo +nightly test` fails with "mixing
   `-Zsanitizer` will cause an ABI mismatch" because the prebuilt std/deps weren't
   compiled with the sanitizer. Rebuild everything from source:
   ```
   rustup +nightly component add rust-src
   RUSTFLAGS="-Zsanitizer=thread" cargo +nightly test -Zbuild-std \
     -p stream-reactor --lib --target aarch64-apple-darwin <test-filter>
   ```
   The `--target` is required (build-std needs an explicit target). A clean TSan
   run on the wake stress test (8 threads firing `wake()` while the reactor
   drains) is the canonical check for the new cross-thread `unsafe` fd sharing.
