/*
 * fpga_bitstream.h — emit iCE40 FPGA bitstreams in the Project IceStorm
 * record-stream format, in pure ISO C17. A faithful port of the Rust
 * `fpga-bitstream` crate.
 * ===========================================================================
 *
 * An FPGA holds thousands of programmable blocks (LUTs, flip-flops, routing
 * muxes) whose configuration bits live in CRAM. A bitstream is the binary blob
 * that programs them at power-on. The iCE40 stream is a sequence of
 * variable-length records:
 *
 *     offset  size   field
 *     0       1      total record length (this byte + command byte + payload)
 *     1       1      command code
 *     2..n    n-2    payload
 *
 * framed by the preamble `0xFF 0x00` and the end marker `0xFFFF`.
 *
 * SCOPE (matching the Rust crate). This emits a STRUCTURALLY correct record
 * stream with a stub CRAM image (all zeros) — loadable on real hardware needs
 * the IceStorm chip database to place per-tile bits, which is out of scope.
 *
 * DIVERGENCE FROM RUST. The Rust `cmd` PANICS on a payload longer than 253
 * bytes; this port returns NULL / a status. The `clbs` HashMap becomes an
 * insertion set keyed by (row, col); `emit` sorts by (row, col) exactly as the
 * Rust does, so the output is byte-identical and deterministic.
 *
 * PORTABILITY. Pure ISO C17 — file output uses `<stdio.h>`; no extensions.
 * Builds clean under GCC, Clang, and MSVC with -pedantic-errors / /permissive-
 * and warnings-as-errors.
 */
#ifndef CA_FPGA_BITSTREAM_H
#define CA_FPGA_BITSTREAM_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Supported iCE40 part codes. */
typedef enum { ICE40_HX1K, ICE40_HX8K, ICE40_UP5K, ICE40_LP1K } Ice40Part;

/* Part dimensions (rows, cols, cram_bits_per_tile) for `part`. */
void fpga_part_specs(Ice40Part part, uint32_t *rows, uint32_t *cols,
                     uint32_t *cram_bits);

/* Per-tile CLB configuration. The truth tables are 16-entry 4-input LUTs. In
 * this stub emitter the fields are recorded but do not affect the (zeroed) CRAM
 * image — only the coordinates and CLB count reach the stream/report. */
typedef struct {
    uint8_t lut_a_truth_table[16];
    uint8_t lut_b_truth_table[16];
    int ff_a_enabled;
    int ff_b_enabled;
} FpgaClbConfig;

/* A ClbConfig with zeroed LUTs and disabled flip-flops (the Rust Default). */
FpgaClbConfig fpga_clb_config_default(void);

/* The complete configuration for one FPGA image (a part plus a set of CLBs
 * keyed by (row, col)). Opaque. */
typedef struct FpgaConfig FpgaConfig;

FpgaConfig *fpga_config_new(Ice40Part part); /* NULL on OOM */
void fpga_config_free(FpgaConfig *c);
/* Insert (or overwrite) the CLB at (row, col). Returns 0, or -1 on OOM. */
int fpga_config_insert_clb(FpgaConfig *c, uint32_t row, uint32_t col,
                           const FpgaClbConfig *clb);
size_t fpga_config_clb_count(const FpgaConfig *c);

/* Summary of what `fpga_emit_bitstream` produced. */
typedef struct {
    Ice40Part part;
    size_t bytes_written;
    size_t clb_count;
    size_t cram_size;
} FpgaBitstreamReport;

/* Emit the record stream. Returns a malloc'd byte buffer of `*len_out` bytes
 * (free with free()) and fills `*report`, or NULL on OOM. */
uint8_t *fpga_emit_bitstream(const FpgaConfig *c, size_t *len_out,
                             FpgaBitstreamReport *report);

/* Build one command record `[len, command, payload…]`. Returns a malloc'd
 * buffer of `*out_len` bytes, or NULL if `payload_len` > 253 (the Rust panic)
 * or on OOM. */
uint8_t *fpga_cmd(uint8_t command, const uint8_t *payload, size_t payload_len,
                  size_t *out_len);

/* Emit the bitstream and write it to `path`. Returns 0 (fills `*report`), or -1
 * on emit failure or a file-write error. */
int fpga_write_bin(const char *path, const FpgaConfig *c,
                   FpgaBitstreamReport *report);

#ifdef __cplusplus
}
#endif

#endif /* CA_FPGA_BITSTREAM_H */
