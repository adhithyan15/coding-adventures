/*
 * tcp_runtime/tcp_runtime.h — a reactor-driven TCP server on net + reactor.
 * ===========================================================================
 *
 * The first consumer of the CCPP02 port campaign: it ties `net` (sockets) and
 * `reactor` (readiness multiplexing) into the reusable server seam that every
 * protocol server sits on — the C port of the Rust `tcp-runtime` crate's core.
 *
 * A classic blocking server spends one thread (and its multi-MB stack) per
 * connection. A tcp_runtime spends ONE thread on a reactor: bind a listener,
 * then loop — the kernel wakes you only for the sockets that are actually ready,
 * you read them, hand the bytes to a handler, and write back its reply. That is
 * the pattern behind nginx, Redis, and Node.js.
 *
 * MODEL.
 *   1. tcp_runtime_bind — listen on host:port and register the listener with an
 *      internal reactor. The handler is called for every chunk a connection
 *      delivers.
 *   2. tcp_runtime_serve — run the accept/read loop until stopped (blocking);
 *      or drive it yourself one step at a time with tcp_runtime_poll.
 *   3. tcp_runtime_stop — ask serve() to return after its current step.
 *   4. tcp_runtime_destroy — close the listener + every live connection, free.
 *
 * THE HANDLER mirrors the Rust `TcpHandlerResult`: given the bytes just read, it
 * fills a reply buffer and returns how many bytes to send and whether to close
 * afterwards. Returning write_len 0 and close 0 keeps the connection idle.
 *
 *      tcp_action echo(uint64_t id, const void *data, size_t len,
 *                      void *out, size_t out_cap, void *user) {
 *          size_t n = len < out_cap ? len : out_cap;
 *          memcpy(out, data, n);
 *          return (tcp_action){ .write_len = n, .close = 0 };
 *      }
 *
 * SCOPE (phase-one core). One listener; many concurrent connections; a stateless
 * handler; cooperative stop; a concurrent-connection cap
 * (tcp_runtime_set_max_connections); and a thread-safe outbound MAILBOX
 * (tcp_runtime_mailbox) so a *worker thread* can push bytes to a connection by
 * id, delivered on the reactor thread's next poll. Deliberately DEFERRED to
 * follow-ups (as in the Rust crate's own phased plan): per-connection state,
 * read-pause/resume backpressure (`defer_read`), socket-option policy
 * (TCP_NODELAY/keepalive), and multi-core reactor sharding.
 *
 * BUILD. Per-OS socket/reactor detail lives in net and reactor, but the mailbox
 * needs a mutex, which is os-platform's `thread` primitive (bucket B). So this
 * one source file is compiled with the net + reactor backends AND the os-platform
 * thread backend for the target OS (`-pthread` on POSIX; the CRT on Windows).
 */
#ifndef TCP_RUNTIME_TCP_RUNTIME_H
#define TCP_RUNTIME_TCP_RUNTIME_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* uint64_t */

#include "os_platform/status.h" /* osp_status */

#ifdef __cplusplus
extern "C" {
#endif

/*
 * What a handler decides after seeing a chunk: how many bytes it wrote into the
 * reply buffer, and whether to close the connection once that reply is sent.
 * (Mirrors the Rust TcpHandlerResult's { write, close }.)
 */
typedef struct {
    size_t write_len; /* bytes placed in the handler's `out` buffer (0 = none) */
    int close;        /* nonzero → close the connection after sending the reply */
} tcp_action;

/*
 * A connection handler. Called with the bytes just read (`data`[0..`len`)) for
 * connection `conn_id` (stable for that connection's life); it fills its reply
 * into `out` (capacity `out_cap`) and returns a tcp_action. `user` is the opaque
 * pointer passed to tcp_runtime_bind.
 */
typedef tcp_action (*tcp_handler)(uint64_t conn_id, const void *data, size_t len,
                                  void *out, size_t out_cap, void *user);

/* Opaque server. Created by tcp_runtime_bind, freed by tcp_runtime_destroy. */
typedef struct tcp_runtime tcp_runtime;

/*
 * tcp_runtime_bind — listen on `host`:`port` (numeric IPv4; port 0 = ephemeral)
 * and prepare the reactor. `handler`/`user` are invoked per read. Writes the
 * server through *out. OSP_ERR_INVAL (NULL out/host/handler), OSP_ERR_NOMEM, or
 * OSP_ERR_OS (listen failed).
 */
osp_status tcp_runtime_bind(tcp_runtime **out, const char *host,
                            unsigned short port, tcp_handler handler, void *user);

/* The actual bound port (useful when binding to port 0). OSP_ERR_INVAL / OSP_ERR_OS. */
osp_status tcp_runtime_local_port(tcp_runtime *rt, unsigned short *out_port);

/*
 * tcp_runtime_set_max_connections — cap the number of concurrent connections.
 * 0 means unlimited (the default). While at the cap, a newly accepted connection
 * is closed immediately (the client is refused) rather than tracked. The limit
 * applies to future accepts; connections already open are left in place.
 * OSP_ERR_INVAL if rt is NULL.
 */
osp_status tcp_runtime_set_max_connections(tcp_runtime *rt,
                                           size_t max_connections);

/* The current number of live connections. OSP_ERR_INVAL if rt/out_count NULL. */
osp_status tcp_runtime_connection_count(tcp_runtime *rt, size_t *out_count);

/*
 * tcp_runtime_poll — run ONE reactor step: wait up to `timeout_ms` (negative =
 * forever) for readiness, then accept new connections and service readable ones
 * (reading, dispatching to the handler, writing replies, closing as asked).
 * Writes the number of ready descriptors serviced to *out_handled (may be 0 on
 * timeout). OSP_ERR_INVAL / OSP_ERR_OS.
 */
osp_status tcp_runtime_poll(tcp_runtime *rt, int timeout_ms, int *out_handled);

/*
 * tcp_runtime_serve — call tcp_runtime_poll in a loop until tcp_runtime_stop is
 * requested. Blocks. Returns OSP_OK on a clean stop, or the first fatal
 * OSP_ERR_OS from a poll step.
 */
osp_status tcp_runtime_serve(tcp_runtime *rt);

/*
 * tcp_runtime_stop — request that a running serve() return after its current
 * poll step. Idempotent. (The flag is observed within one poll timeout.)
 */
void tcp_runtime_stop(tcp_runtime *rt);

/*
 * tcp_runtime_destroy — close the listener and every live connection, drain and
 * free any commands still queued in the mailbox, then free the server. The caller
 * must ensure no producer thread is still using the mailbox (see tcp_mailbox
 * below) when this runs. OSP_ERR_INVAL if rt is NULL.
 */
osp_status tcp_runtime_destroy(tcp_runtime *rt);

/* ── Outbound mailbox (post bytes to a connection from another thread) ─────── */

/*
 * The mailbox is the answer to "how does a worker thread reply to a connection?"
 * A tcp_runtime services sockets on ONE reactor thread, so only that thread may
 * touch a socket. A worker instead hands the runtime a COMMAND — send these bytes
 * to connection N, and/or close it — which the reactor thread executes on its
 * next poll. Mirrors the Rust crate's `TcpMailbox`.
 *
 * Each command is enqueued under a mutex (the queue is the only shared state; the
 * connection table stays private to the reactor thread), the payload is COPIED,
 * and tcp_runtime_poll drains the whole queue after servicing readiness — never
 * holding the lock across a socket write. A command for an unknown or
 * already-closed connection id is silently dropped.
 *
 * DELIVERY LATENCY. There is no cross-thread wakeup (a self-pipe/eventfd is a
 * documented follow-up), so a posted command is delivered on the reactor thread's
 * NEXT poll — within one poll timeout when driven by tcp_runtime_serve (100 ms),
 * or immediately if you drive tcp_runtime_poll yourself.
 *
 * LIFETIME (caller precondition). The mailbox lives inside its runtime. The send
 * functions are safe to call CONCURRENTLY with each other and with the reactor
 * thread's poll — but NOT concurrently with tcp_runtime_destroy, which tears the
 * mailbox (and its mutex) down. Quiesce every producer thread before destroying
 * the runtime, exactly as you would before freeing any shared object.
 */
typedef struct tcp_mailbox tcp_mailbox;

/*
 * tcp_runtime_mailbox — the runtime's outbound mailbox. The returned handle is
 * owned by the runtime (freed by tcp_runtime_destroy); do not free it. Its send
 * functions are safe to call from any thread. Returns NULL if rt is NULL.
 */
tcp_mailbox *tcp_runtime_mailbox(tcp_runtime *rt);

/*
 * tcp_mailbox_send — queue `len` bytes of `data` to be sent to connection
 * `conn_id` on the next poll. The bytes are copied, so `data` need not outlive
 * the call. OSP_ERR_INVAL (mb NULL, or data NULL with len > 0), OSP_ERR_NOMEM.
 */
osp_status tcp_mailbox_send(tcp_mailbox *mb, uint64_t conn_id, const void *data,
                            size_t len);

/*
 * tcp_mailbox_send_and_close — like tcp_mailbox_send, but the connection is closed
 * once the bytes are written (on the same poll). OSP_ERR_INVAL / OSP_ERR_NOMEM.
 */
osp_status tcp_mailbox_send_and_close(tcp_mailbox *mb, uint64_t conn_id,
                                      const void *data, size_t len);

/*
 * tcp_mailbox_close — queue a close of connection `conn_id` (no bytes sent),
 * executed on the next poll. OSP_ERR_INVAL (mb NULL), OSP_ERR_NOMEM.
 */
osp_status tcp_mailbox_close(tcp_mailbox *mb, uint64_t conn_id);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* TCP_RUNTIME_TCP_RUNTIME_H */
