/*
 * conduit_dart_bridge.c — C shims for Dart DL API function-pointer variables.
 *
 * dart_api_dl.h declares Dart_PostCObject_DL as a *variable* with a function-
 * pointer type (not as a function).  Calling it via `extern "C" { fn }` in
 * Rust would jump to the DATA address of the variable, which is not executable
 * on ARM64 Apple Silicon (W^X enforcement) → SIGBUS BUS_ADRALN.
 *
 * These C shims are compiled with full knowledge of the dart_api_dl.h types,
 * so they correctly dereference the function-pointer variable before calling.
 */

#include "dart_api_dl.h"  /* NOLINT */

#include <stdbool.h>
#include <stddef.h>   /* NULL */
#include <stdint.h>

/*
 * Safe post: returns false if the Dart DL API is not yet initialised
 * (Dart_PostCObject_DL == NULL) instead of crashing.
 */
bool conduit_dart_post_cobject(int64_t port_id, Dart_CObject* message) {
    if (Dart_PostCObject_DL == NULL) return false;
    return Dart_PostCObject_DL(port_id, message);
}
