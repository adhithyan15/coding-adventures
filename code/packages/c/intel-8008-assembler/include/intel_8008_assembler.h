/*
 * intel_8008_assembler.h — a two-pass Intel 8008 assembler, pure ISO C17.
 * =======================================================================
 *
 * A faithful port of the Rust `intel-8008-assembler` crate: it turns Intel 8008
 * assembly *text* into raw machine-code bytes.
 *
 * Two passes are needed because of forward references (`JMP loop_end` can appear
 * before `loop_end:` is defined):
 *
 *   Pass 1 — walk every line, track a program counter, record each label's
 *            address in a symbol table.
 *   Pass 2 — walk again and encode each instruction, now that every label's
 *            address is known.
 *
 * Where the Rust crate returns `Result<_, AssemblerError>`, this port returns an
 * `Intel8008Status` and (optionally) writes a diagnostic to a caller buffer.
 *
 * Pure ISO C17: no <math.h>, no compiler extensions.
 */
#ifndef INTEL_8008_ASSEMBLER_H
#define INTEL_8008_ASSEMBLER_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* uint8_t */

#ifdef __cplusplus
extern "C" {
#endif

/* Maximum 14-bit address on the Intel 8008 (16 KB address space). */
#define INTEL8008_MAX_ADDRESS ((size_t)0x3FFF)

typedef enum {
    INTEL8008_OK = 0,
    INTEL8008_ERR, /* an assembly error (message in the caller's errbuf) */
    INTEL8008_ERR_OUT_OF_MEMORY
} Intel8008Status;

/* ── Symbol table (label name -> byte address) ──────────────────────────────*/
typedef struct Intel8008Symbols Intel8008Symbols;

Intel8008Symbols *intel8008_symbols_new(void);
void intel8008_symbols_free(Intel8008Symbols *s);
/* Insert or overwrite. Returns 0 on OOM. */
int intel8008_symbols_set(Intel8008Symbols *s, const char *name, size_t addr);
/* Look up. Returns 1 and fills *addr if present, else 0. */
int intel8008_symbols_get(const Intel8008Symbols *s, const char *name,
                          size_t *addr);

/* ── Low-level API (mirrors the crate's internals; handy for testing) ───────*/

/* Encoded byte size of a mnemonic (0 for ORG). Returns INTEL8008_ERR on an
 * unknown mnemonic. */
Intel8008Status intel8008_instruction_size(const char *mnemonic, size_t *out,
                                           char *errbuf, size_t errlen);

/* Encode one instruction. `operands` is an array of `noperands` NUL-terminated
 * strings. On success stores a malloc'd byte buffer in *out_bytes (free with
 * free) of length *out_len. `symbols` may be NULL (empty table). */
Intel8008Status intel8008_encode_instruction(const char *mnemonic,
                                             const char *const *operands,
                                             size_t noperands,
                                             const Intel8008Symbols *symbols,
                                             size_t pc, uint8_t **out_bytes,
                                             size_t *out_len, char *errbuf,
                                             size_t errlen);

/* ── Public API ─────────────────────────────────────────────────────────────*/

/* Two-pass assemble `text` into machine-code bytes. On success stores a
 * malloc'd buffer in *out_bytes (free with free) of length *out_len (which may
 * be 0, in which case *out_bytes may be NULL). */
Intel8008Status intel8008_assemble(const char *text, uint8_t **out_bytes,
                                   size_t *out_len, char *errbuf, size_t errlen);

#ifdef __cplusplus
}
#endif

#endif /* INTEL_8008_ASSEMBLER_H */
