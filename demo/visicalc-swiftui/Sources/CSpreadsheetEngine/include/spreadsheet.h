/*
 * spreadsheet.h — C ABI for the spreadsheet engine (spreadsheet-capi).
 *
 * A stable, hand-written C interface over the Rust spreadsheet engine so the
 * native VisiCalc demos can drive it: Qt/C++ include this header directly;
 * Swift via a module map; Kotlin/Android via JNI; Dart via dart:ffi; .NET via
 * P/Invoke.
 *
 * Memory contract:
 *   - sc_session_new() returns an opaque handle; free with sc_session_free().
 *   - Every char* return is a heap-allocated, NUL-terminated UTF-8 string
 *     (JSON for values; raw text for sc_get_raw). The CALLER owns it and must
 *     free it with sc_string_free() — NOT the C free() (different allocator).
 *     A NULL return signals an error (e.g. a NULL session).
 *
 * The JSON value shape matches the TypeScript and WASM engines exactly:
 *   {"kind":"number","value":46.0} | {"kind":"text","value":"x"} |
 *   {"kind":"boolean","value":true} | {"kind":"empty"} |
 *   {"code":"#DIV/0!","kind":"error"}
 */
#ifndef SPREADSHEET_CAPI_H
#define SPREADSHEET_CAPI_H

#ifdef __cplusplus
extern "C" {
#endif

/* Opaque spreadsheet session. */
typedef struct ScSession ScSession;

/* Lifecycle. */
ScSession *sc_session_new(void);
void       sc_session_free(ScSession *s);

/* Operations. Each char* result must be freed with sc_string_free(). */
char *sc_set_cell(ScSession *s, const char *a1, const char *raw); /* -> {"ok":...} */
char *sc_get_value(ScSession *s, const char *a1);                 /* -> value JSON  */
char *sc_get_raw(ScSession *s, const char *a1);                   /* -> typed source */
char *sc_get_values(ScSession *s);                                /* -> {a1: value} */

/* Free a string returned by any sc_* function. */
void  sc_string_free(char *p);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* SPREADSHEET_CAPI_H */
