/*
 * compiler_source_map.h — the source-mapping sidecar for an AOT compiler
 * pipeline, in pure ISO C17. A faithful port of the Rust `compiler-source-map`
 * crate.
 * ===========================================================================
 *
 * WHAT IT IS. As a program is lowered source → AST → IR → (optimiser passes) →
 * machine code, this sidecar records, at each stage, which IDs map to which — so
 * any error, breakpoint, or profiling sample on the machine code can be traced
 * back to the original source position, and vice-versa. Four segments:
 *
 *   1. SourceToAst      : source position → AST node ID
 *   2. AstToIr          : AST node ID → IR instruction IDs (one-to-many)
 *   3. IrToIr (per pass): original IR ID → optimised IR IDs (or deleted)
 *   4. IrToMachineCode  : IR ID → machine-code byte range
 *
 * A SourceMapChain bundles all four and answers the two end-to-end queries,
 * source_to_mc (forward) and mc_to_source (reverse), by composing the segments.
 *
 * OWNERSHIP. Each segment is a malloc'd handle created with `*_new` and released
 * with `*_free`. A SourceMapChain owns its SourceToAst and AstToIr segments (get
 * borrowed pointers to fill them), and TAKES OWNERSHIP of the IrToIr passes and
 * the IrToMachineCode segment handed to it. `smap_chain_source_to_mc` returns a
 * malloc'd array the caller frees. Positions returned by lookups are borrowed.
 *
 * DIVERGENCE FROM RUST. Rust `Option` becomes a status/NULL here. Lookups are
 * linear scans (as in the Rust source), faithful to the "first match wins"
 * semantics.
 *
 * PORTABILITY. Pure ISO C17 — no POSIX strdup, no extensions. Builds clean under
 * GCC, Clang, and MSVC with -pedantic-errors / /permissive- and
 * warnings-as-errors.
 */
#ifndef CA_COMPILER_SOURCE_MAP_H
#define CA_COMPILER_SOURCE_MAP_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* A span of characters in a source file — a "highlighter pen" over a region.
 * On input to `*_add`, `file` is borrowed and deep-copied; when returned from a
 * lookup, `file` borrows the segment's owned copy (do not free it). */
typedef struct {
    const char *file;
    size_t line;   /* 1-based */
    size_t column; /* 1-based */
    size_t length; /* character span */
} SmapPosition;

/* Render "file:line:column (len=N)" into `buf`. Returns the length written
 * (excluding the NUL), or -1 if `buf` is too small. */
int smap_position_to_string(const SmapPosition *p, char *buf, size_t buflen);

/* One machine-code mapping: an IR instruction and its byte range. */
typedef struct {
    int64_t ir_id;
    size_t mc_offset;
    size_t mc_length;
} SmapMcEntry;

/* ── Segment 1: SourceToAst ─────────────────────────────────────────────── */
typedef struct SmapSourceToAst SmapSourceToAst;
SmapSourceToAst *smap_s2a_new(void);
void smap_s2a_free(SmapSourceToAst *s);
/* Record pos → ast_node_id (file deep-copied). Returns 0 or -1 on OOM. */
int smap_s2a_add(SmapSourceToAst *s, const SmapPosition *pos, size_t ast_node_id);
/* The source position for `ast_node_id`, or NULL if not found (borrowed). */
const SmapPosition *smap_s2a_lookup_by_node_id(const SmapSourceToAst *s,
                                               size_t ast_node_id);

/* ── Segment 2: AstToIr ─────────────────────────────────────────────────── */
typedef struct SmapAstToIr SmapAstToIr;
SmapAstToIr *smap_a2i_new(void);
void smap_a2i_free(SmapAstToIr *a);
/* Record that `ast_node_id` produced `ir_ids` (n of them; copied). 0 or -1. */
int smap_a2i_add(SmapAstToIr *a, size_t ast_node_id, const int64_t *ir_ids,
                 size_t n);
/* The IR IDs for `ast_node_id` (borrowed, count via *count_out), or NULL. */
const int64_t *smap_a2i_lookup_by_ast_node_id(const SmapAstToIr *a,
                                              size_t ast_node_id,
                                              size_t *count_out);
/* The AST node that produced `ir_id` (first match). 1 (sets *out) or 0. */
int smap_a2i_lookup_by_ir_id(const SmapAstToIr *a, int64_t ir_id, size_t *out);

/* ── Segment 3: IrToIr (one per optimiser pass) ─────────────────────────── */
typedef struct SmapIrToIr SmapIrToIr;
SmapIrToIr *smap_i2i_new(const char *pass_name); /* pass_name deep-copied */
void smap_i2i_free(SmapIrToIr *m);
/* Record original_id → new_ids (n; copied). 0 or -1. */
int smap_i2i_add_mapping(SmapIrToIr *m, int64_t original_id,
                         const int64_t *new_ids, size_t n);
/* Record that original_id was deleted (adds it to the deleted set and an
 * empty-new_ids entry, mirroring the Rust behaviour). 0 or -1. */
int smap_i2i_add_deletion(SmapIrToIr *m, int64_t original_id);
int smap_i2i_is_deleted(const SmapIrToIr *m, int64_t original_id);
/* New IDs for original_id, or NULL if deleted or not found. */
const int64_t *smap_i2i_lookup_by_original_id(const SmapIrToIr *m,
                                             int64_t original_id,
                                             size_t *count_out);
/* Original ID that produced new_id (first match). 1 (sets *out) or 0. */
int smap_i2i_lookup_by_new_id(const SmapIrToIr *m, int64_t new_id,
                              int64_t *out);
const char *smap_i2i_pass_name(const SmapIrToIr *m);

/* ── Segment 4: IrToMachineCode ─────────────────────────────────────────── */
typedef struct SmapIrToMc SmapIrToMc;
SmapIrToMc *smap_i2mc_new(void);
void smap_i2mc_free(SmapIrToMc *mc);
int smap_i2mc_add(SmapIrToMc *mc, int64_t ir_id, size_t mc_offset,
                  size_t mc_length);
/* (offset, length) for ir_id (first match). 1 (sets outs) or 0. */
int smap_i2mc_lookup_by_ir_id(const SmapIrToMc *mc, int64_t ir_id,
                              size_t *offset_out, size_t *length_out);
/* IR ID whose byte range [offset, offset+length) contains `offset`. 1/0. */
int smap_i2mc_lookup_by_mc_offset(const SmapIrToMc *mc, size_t offset,
                                  int64_t *ir_id_out);

/* ── SourceMapChain ─────────────────────────────────────────────────────── */
typedef struct SmapChain SmapChain;
SmapChain *smap_chain_new(void); /* empty s2a + a2i, no passes, no backend */
void smap_chain_free(SmapChain *c);
/* Borrowed handles to the chain's own segments (fill them in place). */
SmapSourceToAst *smap_chain_source_to_ast(SmapChain *c);
SmapAstToIr *smap_chain_ast_to_ir(SmapChain *c);
/* Set the machine-code backend segment, TAKING OWNERSHIP (frees any prior). */
void smap_chain_set_machine_code(SmapChain *c, SmapIrToMc *mc);
/* Append an optimiser-pass segment, TAKING OWNERSHIP. Returns 0 or -1 on OOM
 * (segment freed on failure). */
int smap_chain_add_optimizer_pass(SmapChain *c, SmapIrToIr *segment);

/* Forward: source position → machine-code entries. Composes all segments.
 * Returns 1 (fills *out malloc'd, count via *count_out — free with free()),
 * 0 if the chain is incomplete / no mapping, or -1 on OOM. */
int smap_chain_source_to_mc(const SmapChain *c, const SmapPosition *pos,
                            SmapMcEntry **out, size_t *count_out);
/* Reverse: machine-code offset → source position (borrowed), or NULL. */
const SmapPosition *smap_chain_mc_to_source(const SmapChain *c,
                                            size_t mc_offset);

#ifdef __cplusplus
}
#endif

#endif /* CA_COMPILER_SOURCE_MAP_H */
