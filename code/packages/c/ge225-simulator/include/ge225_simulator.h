/*
 * ge225_simulator.h — a GE-225 CPU simulator, pure ISO C17.
 * ========================================================
 *
 * A faithful port of the Rust `ge225-simulator` crate: a fetch-decode-execute
 * simulator for the GE-225 (1959), the mainframe Dartmouth BASIC was designed
 * on. It models the 20-bit word machine — accumulator `A`, extension `Q`,
 * index-register groups, a bit-addressed memory, the console typewriter and card
 * reader, and the full memory-reference / fixed / shift instruction set.
 *
 * A memory-reference word is `[opcode:5][modifier:2][address:13]`; "fixed" and
 * "shift" instructions occupy dedicated 20-bit encodings decoded by table. Words
 * pack into 3 bytes each.
 *
 * Numbers are signed with a 20-bit (and 40-bit double-word) two's-complement
 * representation; sign extension uses unsigned masks so there is no
 * signed-overflow UB.
 *
 * Pure ISO C17: no <math.h>, no compiler extensions, no 128-bit integers.
 */
#ifndef GE225_SIMULATOR_H
#define GE225_SIMULATOR_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* int32_t */

#ifdef __cplusplus
extern "C" {
#endif

/* Status codes (the Rust API returns Result<_, String>). */
typedef enum {
    GE_OK = 0,
    GE_ERR_ADDRESS_OUT_OF_RANGE,
    GE_ERR_HALTED,
    GE_ERR_DECODE,          /* unknown opcode/instruction */
    GE_ERR_RANGE,           /* opcode/modifier/address/shift out of range */
    GE_ERR_DIVIDE_BY_ZERO,
    GE_ERR_NO_CARD_RECORD,
    GE_ERR_INVALID_TYPEWRITER_CODE,
    GE_ERR_ODD_BYTE_LENGTH, /* byte stream not a multiple of 3 */
    GE_ERR_UNKNOWN_MNEMONIC,
    GE_ERR_OUT_OF_MEMORY
} GeStatus;

/* ── Instruction assembly / encoding (free functions) ──────────────────────*/

/* Encode `opcode`(0..0o37) / `modifier`(0..3) / `address`(0..0x1fff) into a
 * 20-bit word. Returns GE_ERR_RANGE if a field is out of range. */
GeStatus ge225_encode_instruction(int32_t opcode, int32_t modifier,
                                  int32_t address, int32_t *out_word);
/* Decode a 20-bit word into its opcode / modifier / address fields. */
void ge225_decode_instruction(int32_t word, int32_t *out_opcode,
                              int32_t *out_modifier, int32_t *out_address);
/* Look up a fixed instruction's 20-bit word by mnemonic (e.g. "NOP", "LDZ"). */
GeStatus ge225_assemble_fixed(const char *mnemonic, int32_t *out_word);
/* Assemble a shift instruction (e.g. "SAN") with a 5-bit count (0..0o37). */
GeStatus ge225_assemble_shift(const char *mnemonic, int32_t count,
                              int32_t *out_word);

/* Pack `n` 20-bit words into `3*n` big-endian bytes (malloc'd; caller frees). */
uint8_t *ge225_pack_words(const int32_t *words, size_t n, size_t *out_len);
/* Unpack a byte stream (length multiple of 3) into words (malloc'd; caller
 * frees). Returns GE_ERR_ODD_BYTE_LENGTH otherwise. */
GeStatus ge225_unpack_words(const uint8_t *program, size_t len,
                            int32_t **out_words, size_t *out_n);

/* ── The simulator ─────────────────────────────────────────────────────────*/

typedef struct Ge225Simulator Ge225Simulator;

/* Create a simulator with `memory_words` words of memory (> 0; NULL if 0 or
 * on allocation failure). */
Ge225Simulator *ge225_new(int32_t memory_words);
void ge225_free(Ge225Simulator *sim);
void ge225_reset(Ge225Simulator *sim);

void ge225_set_control_switches(Ge225Simulator *sim, int32_t value);
/* Queue a card-reader record (copied) that a later RCD instruction consumes. */
GeStatus ge225_queue_card_reader_record(Ge225Simulator *sim,
                                        const int32_t *words, size_t n);
/* Copy the accumulated typewriter output into `buf` (NUL-terminated, truncated
 * to `cap`). Returns the full length (excluding NUL) regardless of truncation. */
size_t ge225_typewriter_output(const Ge225Simulator *sim, char *buf,
                               size_t cap);

GeStatus ge225_load_words(Ge225Simulator *sim, const int32_t *words, size_t n,
                          int32_t start_address);
GeStatus ge225_read_word(const Ge225Simulator *sim, int32_t address,
                         int32_t *out_value);
GeStatus ge225_write_word(Ge225Simulator *sim, int32_t address, int32_t value);

/* Disassemble a 20-bit word into `buf` (NUL-terminated, truncated to `cap`). */
GeStatus ge225_disassemble_word(const Ge225Simulator *sim, int32_t word,
                                char *buf, size_t cap);

/* One decoded/executed instruction (numeric fields; the disassembly string is
 * available via ge225_disassemble_word on `instruction_word`). */
typedef struct {
    int32_t address;
    int32_t instruction_word;
    int32_t a_before;
    int32_t a_after;
    int32_t q_before;
    int32_t q_after;
    int has_effective_address;
    int32_t effective_address;
} Ge225Trace;

/* Execute one instruction. Fills `*trace` (may be NULL). Returns GE_ERR_HALTED
 * if the machine is already halted. */
GeStatus ge225_step(Ge225Simulator *sim, Ge225Trace *trace);
/* Step up to `max_steps` times, stopping early if the machine halts. */
GeStatus ge225_run(Ge225Simulator *sim, size_t max_steps);

/* ── State accessors ───────────────────────────────────────────────────────*/

int32_t ge225_get_a(const Ge225Simulator *sim);
int32_t ge225_get_q(const Ge225Simulator *sim);
int32_t ge225_get_m(const Ge225Simulator *sim);
int32_t ge225_get_n(const Ge225Simulator *sim);
int32_t ge225_get_pc(const Ge225Simulator *sim);
int32_t ge225_get_ir(const Ge225Simulator *sim);
int ge225_get_overflow(const Ge225Simulator *sim);
int ge225_get_parity_error(const Ge225Simulator *sim);
int ge225_get_decimal_mode(const Ge225Simulator *sim);
int ge225_get_automatic_interrupt_mode(const Ge225Simulator *sim);
size_t ge225_get_selected_x_group(const Ge225Simulator *sim);
int ge225_get_n_ready(const Ge225Simulator *sim);
int ge225_get_typewriter_power(const Ge225Simulator *sim);
int ge225_get_halted(const Ge225Simulator *sim);
/* Index-register word `slot` (0..3) of the selected group. */
int32_t ge225_get_x_word(const Ge225Simulator *sim, size_t slot);

#ifdef __cplusplus
}
#endif

#endif /* GE225_SIMULATOR_H */
