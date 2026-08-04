/*
 * resp_server/resp_server.h — a tiny Redis-style server on tcp-runtime + RESP.
 * ===========================================================================
 *
 * The port campaign's first *protocol* server: it proves the reactor TCP runtime
 * (`tcp-runtime`, itself on `net` + `reactor`) can host a real wire protocol by
 * speaking RESP — the line protocol Redis uses — parsed and encoded by the
 * `resp-protocol` package.
 *
 * It answers a handful of commands against one shared in-memory keyspace:
 *
 *      PING            → +PONG           (PING <msg> echoes <msg> as a bulk string)
 *      ECHO <msg>      → $<msg>
 *      SET  <k> <v>    → +OK             (stores v under k)
 *      GET  <k>        → $<v>  or  $-1   (the value, or the null bulk on a miss)
 *      <anything else> → -ERR unknown command
 *
 * It is a thin wrapper over tcp_runtime: bind installs a RESP handler whose
 * `user` pointer is the shared keystore, so every connection reads and writes the
 * same keyspace. The lifecycle mirrors tcp_runtime (bind / poll / serve / stop /
 * destroy).
 *
 * SCOPE. One command per read chunk (the phase-one stateless handler cannot yet
 * reassemble a frame split across reads, nor pipeline several per chunk — that
 * needs tcp_runtime's stateful-handler follow-up). A value larger than the
 * runtime's per-read buffer is truncated. Keys and values are arbitrary bytes.
 */
#ifndef RESP_SERVER_RESP_SERVER_H
#define RESP_SERVER_RESP_SERVER_H

#include "os_platform/status.h" /* osp_status */

#ifdef __cplusplus
extern "C" {
#endif

/* Opaque server. Created by resp_server_bind, freed by resp_server_destroy. */
typedef struct resp_server resp_server;

/*
 * resp_server_bind — listen on `host`:`port` (numeric IPv4; port 0 = ephemeral)
 * with an empty keyspace. OSP_ERR_INVAL / OSP_ERR_NOMEM / OSP_ERR_OS.
 */
osp_status resp_server_bind(resp_server **out, const char *host,
                            unsigned short port);

/* The bound port (useful for port 0). OSP_ERR_INVAL / OSP_ERR_OS. */
osp_status resp_server_local_port(resp_server *s, unsigned short *out_port);

/* One reactor step (accept + service ready connections). OSP_ERR_INVAL / OSP_ERR_OS. */
osp_status resp_server_poll(resp_server *s, int timeout_ms, int *out_handled);

/* Loop poll until stopped (blocks). OSP_ERR_INVAL / OSP_ERR_OS. */
osp_status resp_server_serve(resp_server *s);

/* Ask a running serve() to return. Idempotent. */
void resp_server_stop(resp_server *s);

/* Close everything and free the server + keyspace. OSP_ERR_INVAL if s is NULL. */
osp_status resp_server_destroy(resp_server *s);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* RESP_SERVER_RESP_SERVER_H */
