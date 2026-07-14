# Changelog

All notable changes to the `net` (C) package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to semantic versioning.

## [Unreleased]

### Added

- **Initial package + `tcp` sockets** (CCPP02 Phase 3, PR 1). Real blocking TCP
  over IPv4, on top of the `os-platform` core (reuses `osp_status`). Built by
  `platform-harness`; per-OS source selection via `BUILD` / `BUILD_windows`.
  - `osp_net_init` / `osp_net_shutdown` (WSAStartup/WSACleanup on Windows;
    no-ops on POSIX).
  - `osp_tcp_listen` (SO_REUSEADDR; port 0 = ephemeral) + `osp_tcp_local_port`,
    `osp_tcp_accept`, `osp_tcp_connect`.
  - `osp_socket_send` (send-all, EINTR-safe), `osp_socket_recv` (single recv;
    0 = orderly peer shutdown), `osp_socket_close`.
  - Backends: `net_posix.c` (BSD sockets; SIGPIPE suppressed via `MSG_NOSIGNAL`
    or `SO_NOSIGPIPE`, chosen by feature-macro presence; libc only) and
    `net_windows.c` (Winsock2 — WSAStartup lifecycle, `SOCKET`/`INVALID_SOCKET`,
    `int`-length send/recv clamped to INT_MAX, `closesocket`; links `ws2_32`).
  - Integration test (`tests/net_test.c`): a single-threaded loopback echo
    round-trip (listen on an ephemeral port → connect → accept → send → echo →
    recv → post-close recv == 0) plus NULL/malformed-address validation.
    Verified under ASan+UBSan with 0 leaks.
