/*
 * net/tcp.h — blocking TCP sockets over IPv4, portably (POSIX / Winsock).
 * ===========================================================================
 *
 * CCPP02 Phase 3, built on top of the os-platform bucket-B core. Networking is
 * pure OS territory — ISO C has no sockets at all — and the two families differ
 * enough (int fds vs the SOCKET handle, close vs closesocket, no init vs
 * WSAStartup) that a thin portable layer earns its keep:
 *
 *      step        POSIX (macOS/Linux)   Windows (Winsock2)
 *      ──────────  ────────────────────  ─────────────────────────
 *      init        (nothing)             WSAStartup
 *      create      socket                socket
 *      server      bind + listen         bind + listen
 *      accept      accept                accept
 *      client      connect               connect
 *      transfer    send / recv           send / recv
 *      close       close                 closesocket
 *      shutdown    (nothing)             WSACleanup
 *
 * This first cut is deliberately small and blocking: enough to open a real
 * connection and exchange bytes (proved by a loopback echo test on every OS).
 * Non-blocking I/O and readiness notification are the job of the `reactor`
 * primitive (epoll/kqueue/iocp) that follows.
 *
 * Addresses are numeric IPv4 dotted-quads (e.g. "127.0.0.1"); no DNS resolution
 * is performed here. It shares the os-platform error vocabulary (osp_status).
 *
 * LIFECYCLE. Call osp_net_init once before any socket call and osp_net_shutdown
 * once at the end (both are no-ops on POSIX; on Windows they are WSAStartup /
 * WSACleanup). Every socket returned by listen/accept/connect must be released
 * with osp_socket_close, which frees the handle — nothing leaks.
 *
 * BUILD. Compiled by platform-harness; the POSIX backend links no extra library,
 * the Windows backend links ws2_32. Per-OS source selection is done by the BUILD.
 */
#ifndef NET_TCP_H
#define NET_TCP_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* uintptr_t */

#include "os_platform/status.h" /* osp_status — shared platform error codes */

#ifdef __cplusplus
extern "C" {
#endif

/* Opaque socket handle (a connection or a listener). Freed by osp_socket_close. */
typedef struct osp_socket osp_socket;

/* Process-wide networking start/stop. Call init once up front, shutdown once at
 * the end. No-ops on POSIX; WSAStartup / WSACleanup on Windows. */
osp_status osp_net_init(void);
osp_status osp_net_shutdown(void);

/*
 * osp_tcp_listen — create a listening socket bound to `host`:`port`.
 * `host` is a numeric IPv4 string (e.g. "127.0.0.1"). A `port` of 0 asks the OS
 * for an ephemeral port (read it back with osp_tcp_local_port). `backlog` is the
 * pending-connection queue length. Sets SO_REUSEADDR. Returns OSP_ERR_INVAL for
 * NULL/malformed args, OSP_ERR_NOMEM, OSP_ERR_OS.
 */
osp_status osp_tcp_listen(osp_socket **out, const char *host,
                          unsigned short port, int backlog);

/*
 * osp_tcp_local_port — the actual local port `s` is bound to. Useful after
 * listening on port 0. Returns OSP_ERR_INVAL / OSP_ERR_OS.
 */
osp_status osp_tcp_local_port(osp_socket *s, unsigned short *out_port);

/*
 * osp_tcp_accept — block until a client connects to `listener`, returning the
 * new connection through *out_conn. OSP_ERR_INVAL / OSP_ERR_NOMEM / OSP_ERR_OS.
 */
osp_status osp_tcp_accept(osp_socket *listener, osp_socket **out_conn);

/*
 * osp_tcp_connect — open a blocking connection to `host`:`port` (numeric IPv4).
 * OSP_ERR_INVAL / OSP_ERR_NOMEM / OSP_ERR_OS.
 */
osp_status osp_tcp_connect(osp_socket **out, const char *host,
                           unsigned short port);

/*
 * osp_socket_send — send ALL `len` bytes from `buf` (looping over partial sends,
 * retrying EINTR). On success *out_n (if non-NULL) is set to `len`. OSP_ERR_*.
 */
osp_status osp_socket_send(osp_socket *s, const void *buf, size_t len,
                           size_t *out_n);

/*
 * osp_socket_recv — receive up to `len` bytes into `buf` (a single recv). Writes
 * the count to *out_n: 0 means the peer performed an orderly shutdown. OSP_ERR_*.
 */
osp_status osp_socket_recv(osp_socket *s, void *buf, size_t len, size_t *out_n);

/*
 * osp_socket_close — close the socket and free the handle (freed even if the OS
 * close call reports an error). OSP_ERR_INVAL if s is NULL, OSP_ERR_OS on a
 * close failure.
 */
osp_status osp_socket_close(osp_socket *s);

/*
 * osp_socket_fd — expose the underlying OS descriptor (an int fd on POSIX, a
 * SOCKET on Windows), widened to uintptr_t so one signature serves both. This
 * lets an external event loop — e.g. the `reactor` — watch this socket for
 * readiness instead of blocking in accept/recv. The descriptor stays owned by
 * the socket; do not close it directly (use osp_socket_close). A consumer feeds
 * it to the reactor by casting to the reactor's osp_fd (an int on POSIX, a
 * uintptr_t on Windows). OSP_ERR_INVAL if s or out is NULL.
 */
osp_status osp_socket_fd(const osp_socket *s, uintptr_t *out);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* NET_TCP_H */
