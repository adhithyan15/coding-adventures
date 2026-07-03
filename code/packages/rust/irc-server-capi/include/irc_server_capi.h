/*
 * irc_server_capi.h — the reusable C ABI for the all-Rust IRC engine.
 * ============================================================================
 *
 * `irc-net-reactor` is a complete IRC server (protocol + state machine) running
 * on the home-grown kqueue/epoll/IOCP reactor. This header is the single, stable
 * C interface that any C-capable host language binds against. The Swift package
 * `code/packages/swift/IrcServerNative` is the first consumer; the same header
 * works for C, C++, Go/cgo, C# P/Invoke, Dart FFI, Zig, and friends.
 *
 * CONTROL SURFACE (no callbacks)
 * ------------------------------
 * Because ALL IRC and TCP logic lives in Rust, the binding is a pure lifecycle
 * controller — create, serve, stop. There is no per-message callback back into
 * the host language, so the trust boundary (UTF-8 validation, number clamping,
 * panic isolation) is enforced once, in the Rust `irc-server-capi` crate.
 *
 * MEMORY OWNERSHIP
 * ----------------
 *   - IrcServer* : created by irc_server_new (NULL on bind failure); freed by
 *                  irc_server_free exactly once. Double-free is undefined.
 *   - char* from irc_server_local_host must be released with
 *                  irc_server_string_free.
 *
 * THREADING
 * ---------
 * irc_server_serve runs the event loop on the CALLING thread and blocks until
 * irc_server_stop. irc_server_serve_background runs it on one background Rust
 * OS thread and returns immediately. The loop runs an owned clone of the engine,
 * so irc_server_stop / irc_server_running / irc_server_local_* may be called
 * from a DIFFERENT thread than the one blocked in irc_server_serve.
 *
 * irc_server_free is the exception: it takes ownership and must happen-AFTER
 * every other call on the handle has returned (in particular, a foreground
 * irc_server_serve must have returned first — call irc_server_stop, then free).
 * This is the standard C contract: do not free an object another thread is still
 * inside a call on.
 */

#ifndef IRC_SERVER_CAPI_H
#define IRC_SERVER_CAPI_H

#include <stdbool.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Opaque handle to a bound IRC server. */
typedef struct CapiServer IrcServer;

/* Bind host:port and return a handle, or NULL if the socket could not be bound.
 * `motd` is a single newline-joined string (split into lines inside Rust). Any
 * NULL or non-UTF-8 string argument falls back to a safe default;
 * `max_connections` is clamped to at least 1. */
IrcServer *irc_server_new(const char *host, uint16_t port, const char *server_name,
                          const char *motd, const char *oper_password,
                          uint32_t max_connections);

/* Run the loop on the CALLING thread; blocks until stopped. 0 ok, -1 on error. */
int irc_server_serve(IrcServer *srv);

/* Run the loop on a background Rust thread; returns immediately. 0 ok, -1 error. */
int irc_server_serve_background(IrcServer *srv);

/* Signal the loop to stop and join the background thread (if any). */
void irc_server_stop(IrcServer *srv);

/* Whether the event loop is currently running. */
bool irc_server_running(IrcServer *srv);

/* The bound IP address as a heap C string; release with irc_server_string_free.
 * Returns NULL on a NULL handle or allocation failure. */
char *irc_server_local_host(IrcServer *srv);

/* The bound TCP port (the OS-assigned port when bound with port == 0). */
uint16_t irc_server_local_port(IrcServer *srv);

/* Free a string returned by this library (e.g. irc_server_local_host). NULL ok. */
void irc_server_string_free(char *s);

/* Stop, join, and free a handle from irc_server_new. NULL ok; free exactly once. */
void irc_server_free(IrcServer *srv);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* IRC_SERVER_CAPI_H */
