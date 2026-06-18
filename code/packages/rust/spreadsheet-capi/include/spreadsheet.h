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

#include <stdint.h> /* uint32_t / uint64_t for the viewport calls */

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

/* Cell display formats — an Excel-style code per cell (e.g. "#,##0.00",
   "yyyy-mm-dd") decides how its value reads. set with an empty code to clear.
   char* results must be freed with sc_string_free(). */
void  sc_set_format(ScSession *s, const char *a1, const char *code);
char *sc_get_format(ScSession *s, const char *a1);                /* -> code | ""   */
char *sc_get_display(ScSession *s, const char *a1);               /* -> display str */

/* Structural edits — insert/delete rows & columns. 1-based `at`, `count` lines.
   The engine relocates cells and rewrites formula references (a reference to a
   deleted line becomes #REF!); the formula echo stays in step. No return — the
   host re-reads via sc_get_window / sc_get_raw afterwards. */
void  sc_insert_rows(ScSession *s, uint32_t at, uint32_t count);
void  sc_delete_rows(ScSession *s, uint32_t at, uint32_t count);
void  sc_insert_cols(ScSession *s, uint32_t at, uint32_t count);
void  sc_delete_cols(ScSession *s, uint32_t at, uint32_t count);

/* Fill / replicate (drag-fill): copy the `src` cell across the inclusive
   rectangle `dst_start`..`dst_end`. Relative references shift per target,
   absolute ($) refs pin, the source's format carries along, an empty source
   clears each target; a malformed address is a no-op. No return — the host
   re-reads via sc_get_window / sc_get_display_window / sc_get_raw afterwards. */
void  sc_fill(ScSession *s, const char *src, const char *dst_start, const char *dst_end);

/* Clipboard — cut / copy / paste. copy/cut capture the inclusive rectangle
   `start`..`end` (content + format + the typed source). A copy's buffer survives
   any number of pastes; a cut is a one-shot move whose paste clears the source it
   didn't overwrite. paste places the block so its top-left lands at `dst_start`,
   shifting the whole block's references by the destination's offset; it returns 1
   when applied, 0 for a no-op (empty clipboard / malformed address / off-grid).
   No char* results — the host re-reads via sc_get_window / sc_get_display_window /
   sc_get_raw afterwards. A malformed/oversized range on copy/cut is a no-op. */
void  sc_copy(ScSession *s, const char *start, const char *end);
void  sc_cut(ScSession *s, const char *start, const char *end);
int   sc_paste(ScSession *s, const char *dst_start);

/* Viewport primitive — read just the visible window of the unbounded sheet.
   Coordinates are 1-based and inclusive. Each char* result must be freed with
   sc_string_free(). */
/* -> {"row0":..,"col0":..,"rows":R,"cols":C,"values":[[value,..],..]} | {"error":".."} */
char *sc_get_window(ScSession *s, uint32_t row0, uint32_t col0,
                    uint32_t row1, uint32_t col1);
/* Like sc_get_window, but each cell is its display string (value rendered through
   its format code); empty cells are "". The one read a virtualized grid needs. */
/* -> {"row0":..,"col0":..,"rows":R,"cols":C,"cells":[["1,234.50",..],..]} | {"error":".."} */
char *sc_get_display_window(ScSession *s, uint32_t row0, uint32_t col0,
                            uint32_t row1, uint32_t col1);
char *sc_used_range(ScSession *s);              /* -> {"minRow":..,..} | null            */
char *sc_column_letters(ScSession *s, uint32_t index); /* 1-based index -> "A"/"AA"/...   */
uint64_t sc_current_revision(ScSession *s);     /* per-edit revision clock (0 if s==NULL) */
char *sc_changed_since(ScSession *s, uint64_t since);  /* -> {"revision":N,"changed":[..]} |
                                                            {"revision":N,"stale":true}    */

/* Free a string returned by any sc_* function. */
void  sc_string_free(char *p);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* SPREADSHEET_CAPI_H */
