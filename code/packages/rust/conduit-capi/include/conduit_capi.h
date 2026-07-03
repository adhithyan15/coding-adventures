/*
 * conduit_capi.h — the reusable C ABI for the Conduit web framework.
 * ============================================================================
 *
 * Conduit is a Sinatra/Express-style web framework over the Rust `web-core`
 * engine (WEB08 facade). This header is the single, stable C interface that
 * every C-capable host language binds against: Swift (WEB12), C++ (WEB13),
 * Go/cgo (WEB14), C# P/Invoke (WEB15), F# (WEB16), Dart FFI (WEB17), and
 * Haskell FFI (WEB18). The trust boundary — header-injection defense, status
 * clamping, UTF-8 validation, panic isolation — is enforced once, in the Rust
 * `conduit-capi` crate behind this header.
 *
 * DISPATCH MODEL
 * --------------
 * A handler is a C function pointer plus an opaque `ctx` (the host boxes its
 * closure and hands us the pointer) and a `ctx_free` destructor we call when the
 * owning app/server is freed. On each request the library builds a read-only
 * ConduitRequest view, calls the handler, and takes ownership of the returned
 * ConduitResponse. Returning NULL means "no response": for a before-filter that
 * is "continue"; for a route it routes through the error handler using the
 * message the host stashed via conduit_capi_report_error().
 *
 * MEMORY OWNERSHIP
 * ----------------
 *   - ConduitApp*    : created by conduit_app_new; consumed by conduit_server_bind
 *                      (or freed by conduit_app_free if never bound).
 *   - ConduitServer* : created by conduit_server_bind; freed by conduit_server_free.
 *   - ConduitRequest*: borrowed; valid ONLY for the duration of one handler call.
 *   - ConduitResponse*: created by conduit_response_new; the library takes
 *                      ownership when you RETURN it from a handler. Any response
 *                      you build but do not return must be conduit_response_free'd.
 *   - char* from conduit_app_get_setting must be conduit_string_free'd.
 *
 * THREADING
 * ---------
 * Foreground conduit_server_serve dispatches handlers on the calling thread.
 * conduit_server_serve_background spawns one OS thread. Host closures must be
 * thread-safe.
 */

#ifndef CONDUIT_CAPI_H
#define CONDUIT_CAPI_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Opaque handles. */
typedef struct ConduitApp ConduitApp;
typedef struct ConduitServer ConduitServer;
typedef struct ConduitRequest ConduitRequest;
typedef struct ConduitResponse ConduitResponse;

/* Callback typedefs. */

/* Returns a NEW response (route / not_found / on_error), or NULL.
 * For a before-filter, NULL = continue; non-NULL = short-circuit. */
typedef ConduitResponse *(*ConduitHandler)(void *ctx, const ConduitRequest *req);

/* Transforming after-hook: receives and owns `current`, returns a response
 * (may be `current` mutated, or a freshly built one). */
typedef ConduitResponse *(*ConduitAfter)(void *ctx, const ConduitRequest *req,
                                         ConduitResponse *current);

/* Destructor for a handler ctx, called when the owning app/server is freed. */
typedef void (*ConduitCtxFree)(void *ctx);

/* ── Error channels ─────────────────────────────────────────────────────── */

/* Host calls this from inside a handler before returning NULL, to record why a
 * route failed; the error handler then receives this message. */
void conduit_capi_report_error(const char *msg);

/* Thread-local last-error string (e.g. after a failed bind). Valid until the
 * next conduit_capi call on this thread. */
const char *conduit_last_error(void);

/* ── App lifecycle ──────────────────────────────────────────────────────── */

ConduitApp *conduit_app_new(void);
void conduit_app_free(ConduitApp *app); /* only if never bound */

void conduit_app_set_setting(ConduitApp *app, const char *key, const char *value);
char *conduit_app_get_setting(ConduitApp *app, const char *key); /* owned; conduit_string_free */

void conduit_app_add_route(ConduitApp *app, const char *method, const char *pattern,
                           ConduitHandler handler, void *ctx, ConduitCtxFree ctx_free);
void conduit_app_add_before(ConduitApp *app, ConduitHandler handler, void *ctx,
                            ConduitCtxFree ctx_free);
void conduit_app_add_after(ConduitApp *app, ConduitAfter handler, void *ctx,
                           ConduitCtxFree ctx_free);
void conduit_app_set_not_found(ConduitApp *app, ConduitHandler handler, void *ctx,
                               ConduitCtxFree ctx_free);
void conduit_app_set_error_handler(ConduitApp *app, ConduitHandler handler, void *ctx,
                                   ConduitCtxFree ctx_free);

/* ── Server ─────────────────────────────────────────────────────────────── */

/* Bind host:port and CONSUME `app`. Returns NULL on error (see conduit_last_error). */
ConduitServer *conduit_server_bind(const char *host, uint16_t port, ConduitApp *app);
int conduit_server_serve(ConduitServer *srv);            /* 0 ok; blocks until stopped */
int conduit_server_serve_background(ConduitServer *srv); /* 0 ok */
void conduit_server_stop(ConduitServer *srv);
uint16_t conduit_server_local_port(ConduitServer *srv);
int conduit_server_running(ConduitServer *srv); /* 0/1 */
void conduit_server_free(ConduitServer *srv);

/* ── Request accessors (valid only during the handler call) ─────────────── */

const char *conduit_request_method(const ConduitRequest *req);
const char *conduit_request_path(const ConduitRequest *req);
const char *conduit_request_query_string(const ConduitRequest *req);
const char *conduit_request_content_type(const ConduitRequest *req); /* "" if none */
const char *conduit_request_remote_addr(const ConduitRequest *req);
const char *conduit_request_error(const ConduitRequest *req); /* for on_error; "" otherwise */
const uint8_t *conduit_request_body(const ConduitRequest *req, size_t *out_len);
const char *conduit_request_param(const ConduitRequest *req, const char *name);  /* NULL if absent */
const char *conduit_request_query(const ConduitRequest *req, const char *name);  /* NULL if absent */
const char *conduit_request_header(const ConduitRequest *req, const char *name); /* case-insensitive */

/* ── Response builder / reader ──────────────────────────────────────────── */

ConduitResponse *conduit_response_new(uint16_t status, const uint8_t *body,
                                      size_t body_len); /* status clamped 100–599 */
void conduit_response_set_header(ConduitResponse *resp, const char *name,
                                 const char *value); /* CR/LF/CTL/':'-in-name dropped */
uint16_t conduit_response_status(const ConduitResponse *resp);
const uint8_t *conduit_response_body(const ConduitResponse *resp, size_t *out_len);
size_t conduit_response_header_count(const ConduitResponse *resp);
const char *conduit_response_header_name(const ConduitResponse *resp, size_t i);  /* NULL if out of range */
const char *conduit_response_header_value(const ConduitResponse *resp, size_t i); /* NULL if out of range */
void conduit_response_free(ConduitResponse *resp); /* for responses you build but don't return */

void conduit_string_free(char *s);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* CONDUIT_CAPI_H */
