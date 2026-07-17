/*
 * http_server/http_server.h — a tiny HTTP/1.1 server on tcp-runtime + http-core.
 * ===========================================================================
 *
 * The port campaign's second protocol server: it proves the reactor TCP runtime
 * (`tcp-runtime` → `net` + `reactor`) can host HTTP. A request handler parses the
 * request line + headers, interprets them with the `http-core` package
 * (version, request-target/path splitting, query values, header lookup), routes,
 * and writes an HTTP/1.1 response.
 *
 * Routes:
 *      GET /            → 200  "hello from http-server\n"
 *      GET /echo?msg=X  → 200  the value of the `msg` query parameter
 *      GET /headers     → 200  the request's own headers, one per line
 *      (other path)     → 404
 *      (other method)   → 405
 *      (malformed)      → 400
 *
 * Every response carries `Connection: close`, so it is one request/response per
 * connection (HTTP/1.0 style) — keep-alive is a follow-up. The lifecycle mirrors
 * tcp_runtime (bind / poll / serve / stop / destroy).
 *
 * SCOPE. The request must arrive in one read (the phase-one tcp_runtime handler
 * is stateless and cannot reassemble a request split across reads); a request
 * over the runtime's 8 KiB per-read buffer is rejected with 400. No request body
 * is read (GET only), no chunked/keep-alive. http-core supplies the syntax-level
 * helpers; the byte-level request framing here is a minimal in-server parser (a
 * standalone `http1` wire crate would be its own follow-up).
 */
#ifndef HTTP_SERVER_HTTP_SERVER_H
#define HTTP_SERVER_HTTP_SERVER_H

#include "os_platform/status.h" /* osp_status */

#ifdef __cplusplus
extern "C" {
#endif

/* Opaque server. Created by http_server_bind, freed by http_server_destroy. */
typedef struct http_server http_server;

/*
 * http_server_bind — listen on `host`:`port` (numeric IPv4; port 0 = ephemeral).
 * OSP_ERR_INVAL / OSP_ERR_NOMEM / OSP_ERR_OS.
 */
osp_status http_server_bind(http_server **out, const char *host,
                            unsigned short port);

/* The bound port (useful for port 0). OSP_ERR_INVAL / OSP_ERR_OS. */
osp_status http_server_local_port(http_server *s, unsigned short *out_port);

/* One reactor step (accept + service ready connections). OSP_ERR_INVAL / OSP_ERR_OS. */
osp_status http_server_poll(http_server *s, int timeout_ms, int *out_handled);

/* Loop poll until stopped (blocks). OSP_ERR_INVAL / OSP_ERR_OS. */
osp_status http_server_serve(http_server *s);

/* Ask a running serve() to return. Idempotent. */
void http_server_stop(http_server *s);

/* Close everything and free the server. OSP_ERR_INVAL if s is NULL. */
osp_status http_server_destroy(http_server *s);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* HTTP_SERVER_HTTP_SERVER_H */
