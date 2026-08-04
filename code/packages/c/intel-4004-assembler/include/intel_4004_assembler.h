/*
 * intel_4004_assembler.h — a two-pass assembler for the Intel 4004 (the first
 * commercial microprocessor, 1971), in pure ISO C17. A faithful port of the Rust
 * `intel-4004-assembler` crate.
 * ===========================================================================
 *
 * The assembler turns 4004 assembly text into a byte array of machine code:
 *   - Pass 1 walks the lines building a symbol table (label -> program counter),
 *     honouring `ORG` to set the origin.
 *   - Pass 2 encodes each instruction, padding with zeros for forward `ORG`s.
 *
 * A line is `[label:] [mnemonic [operands]] [; comment]`. Mnemonics are
 * case-insensitive; operands are comma-separated. Registers are `Rn`, register
 * pairs `Pn`, numbers decimal or `0x`-hex, and any bare identifier is looked up
 * as a symbol.
 *
 * OWNERSHIP. `i4004_assemble` writes the machine code to a malloc'd buffer the
 * caller frees. On an assembly error it fills the caller's `err` buffer with a
 * message and returns I4004_ERROR.
 *
 * Portability: pure ISO C17 — GCC, Clang, and MSVC with -pedantic-errors /
 * /permissive- and warnings-as-errors.
 */
#ifndef INTEL_4004_ASSEMBLER_H
#define INTEL_4004_ASSEMBLER_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* uint8_t */

typedef enum {
    I4004_OK = 0,      /* success; *out / *out_len written */
    I4004_ERROR,       /* assembly error; message written to `err` */
    I4004_ALLOC_ERROR  /* out of memory */
} I4004Status;

/* i4004_assemble — assemble `text` into machine code.
 *
 * On success writes a malloc'd buffer to *out (caller frees) and its length to
 * *out_len, and returns I4004_OK. On an assembly error returns I4004_ERROR and,
 * if `err`/`err_len` are provided, writes a NUL-terminated message. Returns
 * I4004_ALLOC_ERROR on out of memory. */
I4004Status i4004_assemble(const char *text, uint8_t **out, size_t *out_len,
                           char *err, size_t err_len);

#endif /* INTEL_4004_ASSEMBLER_H */
