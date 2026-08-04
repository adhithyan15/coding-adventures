/*
 * vcd_writer.h — a streaming Value Change Dump (VCD) writer, in pure ISO C17.
 * A faithful port of the Rust `vcd-writer` crate.
 * ===========================================================================
 *
 * VCD (IEEE 1364-2005 §18) is the text format every waveform viewer (GTKWave,
 * Surfer, ModelSim, ...) reads. The writer produces a complete VCD document in
 * an internal buffer in two phases:
 *
 *   1. Header — vcd_open_scope / vcd_declare / vcd_close_scope /
 *      vcd_end_definitions. Each `declare` returns a compact printable-ASCII
 *      identifier used later to reference the variable.
 *   2. Body — vcd_time(t) then vcd_value_change(id, value) pairs.
 *
 * Read the accumulated text with vcd_text (borrowed) before freeing the writer.
 *
 * Identifiers are allocated in a bijective base-94 scheme over '!'..'~': the
 * first 94 variables get one character, the next 94^2 two characters, etc.
 *
 * Portability: pure ISO C17 — GCC, Clang, and MSVC with -pedantic-errors /
 * /permissive- and warnings-as-errors.
 */
#ifndef VCD_WRITER_H
#define VCD_WRITER_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* uint32_t, uint64_t, int64_t */

typedef struct VcdWriter VcdWriter;

/* vcd_new — create a writer with the given timescale (e.g. "1ps", "1ns"),
 * emitting the header preamble. NULL on allocation failure. */
VcdWriter *vcd_new(const char *timescale);

/* vcd_free — release the writer and its buffer (safe with NULL). */
void vcd_free(VcdWriter *w);

/* vcd_ok — 0 if a previous allocation failed (further output is dropped). */
int vcd_ok(const VcdWriter *w);

/* ---- header ----------------------------------------------------------- */

void vcd_open_scope(VcdWriter *w, const char *name); /* kind = "module" */
void vcd_open_scope_kind(VcdWriter *w, const char *name, const char *kind);
void vcd_close_scope(VcdWriter *w);

/* vcd_declare — declare a variable of `width` bits and kind `kind` (e.g.
 * "wire", "reg", "real"). Writes the compact VCD identifier (NUL-terminated)
 * into `id_out` (capacity `id_out_len`, 16 bytes is always enough) and returns
 * 1; returns 0 on allocation failure or too-small buffer. */
int vcd_declare(VcdWriter *w, const char *name, uint32_t width, const char *kind,
                char *id_out, size_t id_out_len);

/* vcd_end_definitions — close any open scopes and end the definitions section.
 * Called automatically before the first vcd_time if not done manually. */
void vcd_end_definitions(VcdWriter *w);

/* ---- body ------------------------------------------------------------- */

/* vcd_time — advance to simulation time `t` (should be non-decreasing). */
void vcd_time(VcdWriter *w, uint64_t t);

/* vcd_value_change — emit one value change for `var_id`; skips silently if the
 * value is unchanged since the last emit. */
void vcd_value_change(VcdWriter *w, const char *var_id, int64_t value);

/* vcd_value_change_at — advance time then emit a value change. */
void vcd_value_change_at(VcdWriter *w, uint64_t t, const char *var_id,
                         int64_t value);

/* vcd_dump_initial — emit a $dumpvars block with an initial value for every
 * declared variable (in declaration order). `ids`/`values` (length `n`) supply
 * overrides; any variable not present defaults to 0. Pass n = 0 for all-zeros. */
void vcd_dump_initial(VcdWriter *w, const char *const *ids, const int64_t *values,
                      size_t n);

/* ---- output ----------------------------------------------------------- */

/* vcd_text — the accumulated VCD text so far (borrowed; valid until vcd_free).
 * Returns "" if a previous allocation failed. */
const char *vcd_text(const VcdWriter *w);

#endif /* VCD_WRITER_H */
