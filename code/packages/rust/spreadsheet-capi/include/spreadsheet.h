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
#include <stddef.h> /* size_t for the file byte lengths */

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

/* Sort the rows of the inclusive rectangle start..end by the computed values in
   key_col (a 1-based absolute column index inside the rectangle). ascending is a
   flag (0 = descending, non-zero = ascending). Each row moves as a record; moved
   formulas have their relative references shifted with their row, formats ride
   along. Returns 1 when a valid sort is applied (or the range was already sorted),
   0 for a malformed address, an out-of-range key_col, an empty/single-row range,
   or an oversized rectangle. The host re-reads via sc_get_window afterwards. */
int   sc_sort_range(ScSession *s, const char *start, const char *end,
                    uint32_t key_col, int ascending);

/* Find / replace. sc_find_all() returns a heap char* JSON object
   {"matches":["A1",...]} of the A1 addresses whose text contains `query`, in
   (row,col) order — free it with sc_string_free(). `in_formulas` is a flag
   (non-zero searches each cell's source; 0 its computed display value);
   `match_case` is a flag (0 folds ASCII case); an empty query → empty list.
   sc_replace_all() replaces `query` with `replacement` in the source of every
   matching cell (engine rewrites + recomputes) and returns the count of cells
   changed (empty query → 0). The host re-reads via sc_get_window / sc_get_raw. */
char *sc_find_all(ScSession *s, const char *query, int in_formulas, int match_case);
int   sc_replace_all(ScSession *s, const char *query, const char *replacement, int match_case);

/* Multi-sheet workbook. A session has one or more named sheets; bare-A1 cell ops
   address the ACTIVE sheet, and a formula may reference another (=Summary!A1).
   sc_sheet_names() returns a heap char* JSON object
   {"sheets":["Sheet1",...],"active":0} (names in tab order + the active 0-based
   index) — free with sc_string_free(). sc_active_sheet() is the active index.
   The mutators return 1 on success, 0 on rejection (bad index / empty-or-duplicate
   name / can't-delete-last-sheet); the host re-reads cells/raw afterwards.
   sc_set_active_sheet switches by index; sc_add_sheet appends a sheet and makes
   it active; sc_rename_sheet rewrites referencing formulas' qualifiers;
   sc_delete_sheet turns inbound refs into #REF!; sc_move_sheet reorders a tab. */
char    *sc_sheet_names(ScSession *s);
uint32_t sc_active_sheet(ScSession *s);
int      sc_set_active_sheet(ScSession *s, uint32_t index);
int      sc_add_sheet(ScSession *s, const char *name);
int      sc_rename_sheet(ScSession *s, uint32_t index, const char *new_name);
int      sc_delete_sheet(ScSession *s, uint32_t index);
int      sc_move_sheet(ScSession *s, uint32_t index, uint32_t to_index);

/* Save / load. sc_serialize() returns a self-contained JSON document holding the
   workbook's SOURCE (formula text + typed literals) and per-cell formats — not the
   computed values, which recompute on load (small file, can't disagree with itself).
   Free the returned string with sc_string_free(). sc_deserialize() replaces the
   workbook with such a document: returns 1 on success, 0 if the data is malformed or
   an unsupported version (the existing workbook is left untouched on failure). */
char *sc_serialize(ScSession *s);
int   sc_deserialize(ScSession *s, const char *data);

/* Undo / redo (session history). sc_undo() reverts the most recent edit, sc_redo()
   replays the most recently undone one; each returns 1 if it changed the document, 0
   if there was nothing to do (or s == NULL). The host re-reads via sc_get_window /
   sc_get_display_window / sc_get_raw afterwards. sc_can_undo() / sc_can_redo() return
   1/0 so a host can enable/disable its Undo/Redo controls. */
int   sc_undo(ScSession *s);
int   sc_redo(ScSession *s);
int   sc_can_undo(ScSession *s);
int   sc_can_redo(ScSession *s);

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

/* Column widths & row heights on the active sheet (presentation chrome the engine
   stores but never computes with). A 1-based column / row index; a `double` size
   in host units. A returned `0.0` means "no custom size — use the host default"
   (a valid size is always > 0). The setters return 1 if the size changed, 0 if
   rejected (non-finite / <= 0 size, or index 0); they persist through save/load
   and shift with their column/row on an insert/delete. The *_widths/_heights
   readers return a heap JSON array (free with sc_string_free). */
double sc_column_width(ScSession *s, uint32_t col);     /* -> width  | 0.0 if unset/NULL */
double sc_row_height(ScSession *s, uint32_t row);       /* -> height | 0.0 if unset/NULL */
int   sc_set_column_width(ScSession *s, uint32_t col, double width);  /* 1 changed / 0 rejected */
int   sc_set_row_height(ScSession *s, uint32_t row, double height);   /* 1 changed / 0 rejected */
int   sc_clear_column_width(ScSession *s, uint32_t col); /* 1 if a width was removed, else 0 */
int   sc_clear_row_height(ScSession *s, uint32_t row);   /* 1 if a height was removed, else 0 */
char *sc_column_widths(ScSession *s, uint32_t col0, uint32_t col1); /* -> [{"col":N,"w":F},..] */
char *sc_row_heights(ScSession *s, uint32_t row0, uint32_t row1);   /* -> [{"row":N,"h":F},..] */

/* File open / save — bytes in, bytes out. Open a real spreadsheet file the user
   picked and save the current document as one, over the one engine. File bytes
   are binary (a .xlsx is a ZIP, a .xls an OLE2 file) and may contain NUL, so they
   cross as an explicit (ptr, len) pair, never a C string.
     - sc_load_<fmt>(s, bytes, len) -> 1 opened / 0 not a readable file of that
       format (or s NULL). A failed open leaves the current document untouched.
     - sc_save_<fmt>(s, &out_len)   -> heap uint8_t* of length *out_len; free it
       with sc_bytes_free(ptr, out_len). NULL / *out_len = 0 for an empty doc.
   .xlsx keeps live formulas; .xls/.csv/.tsv/.json are lower-fidelity (values). */
int      sc_load_xlsx(ScSession *s, const uint8_t *bytes, size_t len);
uint8_t *sc_save_xlsx(ScSession *s, size_t *out_len);
int      sc_load_xls(ScSession *s, const uint8_t *bytes, size_t len);
uint8_t *sc_save_xls(ScSession *s, size_t *out_len);
int      sc_load_csv(ScSession *s, const uint8_t *bytes, size_t len);
uint8_t *sc_save_csv(ScSession *s, size_t *out_len);
int      sc_load_tsv(ScSession *s, const uint8_t *bytes, size_t len);
uint8_t *sc_save_tsv(ScSession *s, size_t *out_len);
int      sc_load_json(ScSession *s, const uint8_t *bytes, size_t len);
uint8_t *sc_save_json(ScSession *s, size_t *out_len);

/* Free a byte buffer returned by an sc_save_* function. (ptr, len) must match
   what that call returned/wrote. Safe with (NULL, 0). */
void  sc_bytes_free(uint8_t *ptr, size_t len);

/* Free a string returned by any sc_* function. */
void  sc_string_free(char *p);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* SPREADSHEET_CAPI_H */
