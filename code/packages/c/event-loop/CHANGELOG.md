# Changelog

All notable changes to the `event-loop` (C) package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to semantic versioning.

## [Unreleased]

### Added

- **Initial package — a pluggable, generic event loop** (CCPP02 port campaign,
  bucket B / thread slice, port #1). The C port of the Rust `event-loop` crate,
  and the first proof that a deferred *thread*-using crate runs on os-platform's
  concurrency primitives without any per-OS code of its own.
  - `event_loop_create`, `event_loop_add_source` (a pull-based `poll` + its
    state), `event_loop_on_event` (a handler + its captured `user`),
    `event_loop_run` (collect → dispatch → idle-nap until `EVL_EXIT` or a stop),
    `event_loop_stop`, `event_loop_stop_handle` / `evl_stop_signal` (stop from
    another thread), `event_loop_destroy`. Reuses `osp_status`.
  - **Generics → C.** Rust's `EventLoop<E>` becomes an opaque `const void *`
    event with caller-defined meaning; sources and handlers are function pointers
    plus state — the same idiom as `tcp_runtime`'s handler.
  - **Thread story.** `run()` is single-threaded; the concurrency is the stop
    signal — a handle wrapping an `osp_mutex`-guarded flag, so a worker thread's
    `evl_stop_signal` and the loop's per-iteration read never race. The idle nap
    uses `osp_sleep_ns`. Composes os-platform's `thread` + `clock` backends;
    no changes to os-platform itself.
  - **Build.** OS-agnostic single source; `run.sh` compiles it with
    `os-platform/src/thread_posix.c` + `clock_posix.c` and links `-pthread`
    (`_POSIX_C_SOURCE=200809L` exposes both pthreads and `clock_gettime`/`nanosleep`
    on glibc and Darwin); `run.ps1` uses `thread_windows.c` + `clock_windows.c`
    (CRT + kernel32, linked by default).
  - **Test (`tests/event_loop_test.c`).** Mirrors the Rust crate's tests (all
    events delivered, `EVL_EXIT` stops dispatch immediately, multiple handlers all
    see an event, multiple sources merged, stop-from-within a handler) and adds
    the payoff: a real worker thread (`osp_thread_spawn`) stops the loop from
    outside. 66 checks, verified under ASan+UBSan with 0 leaks.
