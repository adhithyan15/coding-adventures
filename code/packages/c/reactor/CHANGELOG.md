# Changelog

All notable changes to the `reactor` (C) package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to semantic versioning.

## [Unreleased]

### Changed

- **Scalable readiness backends: `poll` → `kqueue` (macOS) + `epoll` (Linux).**
  The Unix backend split from the single `reactor_posix.c` (`poll`, O(n) rescan
  per wait) into `reactor_mac.c` (`kqueue`) and `reactor_linux.c` (`epoll`), each
  registering interest once in the kernel and waking only for the ready
  descriptors (O(ready)). `run.sh` now selects the source + feature macros by OS
  (`_DARWIN_C_SOURCE` / `_GNU_SOURCE`). The public interface, semantics, and test
  are unchanged — one coalesced event per ready descriptor, `EOF`/error surfaced
  as readable. `kqueue`'s per-filter events are de-duplicated by fd to preserve
  the one-event-per-descriptor contract; `epoll` coalesces natively and carries
  the token in `epoll_event.data.ptr`. Both keep a small `{fd, interest, token}`
  record for interest-delta computation and the "nothing registered → return now"
  short-circuit. Windows stays on `WSAPoll` (readiness-based); **IOCP is
  deliberately not used** — it is a completion-model API that does not fit this
  readiness interface. Verified on macOS/arm64 (kqueue: 21 checks / 0 failed,
  ASan+UBSan, 0 leaks); the epoll path is exercised by the Linux CI matrix.

### Added

- **Initial package + readiness reactor** (CCPP02 Phase 3, PR 2). Watch many
  descriptors from one thread and wake for the ready ones, on top of the
  `os-platform` core (reuses `osp_status`). Built by `platform-harness`; per-OS
  source selection via `BUILD` / `BUILD_windows`.
  - `osp_reactor_create` / `osp_reactor_add` (interest mask + opaque token;
    re-add updates in place) / `osp_reactor_del` / `osp_reactor_wait` (timeout in
    ms; returns ready tokens + readiness bits) / `osp_reactor_destroy`.
  - `osp_fd` typedef reconciles the OS descriptor type (`int` fd on POSIX,
    `SOCKET` on Windows); `OSP_READABLE`/`OSP_WRITABLE` interest bits.
  - Backends: `reactor_posix.c` (`poll()`; EINTR-safe; libc only) and
    `reactor_windows.c` (`WSAPoll()`; links `ws2_32`). Both keep a growable
    `{fd, interest, token}` array rebuilt into a pollfd array per wait; a
    closed/broken peer surfaces as readable.
  - Scope note: `poll()`/`WSAPoll` is the portable, developer-verifiable
    readiness primitive; the scalable epoll/kqueue/IOCP backends are a follow-up
    behind the same interface.
  - Test (`tests/reactor_test.c`): a connected socket pair (`socketpair` on
    POSIX, raw Winsock loopback on Windows), proving not-ready → write → ready
    (correct token) → del → not-ready, plus NULL-argument validation. Verified
    under ASan+UBSan with 0 leaks.
