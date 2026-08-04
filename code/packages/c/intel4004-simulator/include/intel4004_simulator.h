/*
 * intel4004_simulator.h — Intel 4004 behavioral simulator, pure ISO C17.
 * =====================================================================
 *
 * A faithful port of the Rust `intel4004-simulator` crate: a behavioral
 * simulator for the Intel 4004 (1971), the world's first commercial
 * single-chip microprocessor. The entire chip held just 2,300 transistors.
 *
 * ## Why 4-bit?
 *
 * The 4004 is natively 4-bit: every data value is 4 bits wide (0-15) and all
 * arithmetic is forcibly masked to a nibble (`& 0xF`). This suited its first
 * job — pocket calculators — where a decimal digit (0-9) fits in 4 bits and
 * BCD arithmetic works one digit at a time.
 *
 * ## Accumulator architecture
 *
 * Unlike register-to-register machines, the 4004 funnels every operation
 * through a single Accumulator. To add two registers you must LDM/XCH one
 * operand into a register, LDM the other into the accumulator, then ADD.
 *
 * ## Memory hierarchy
 *
 *   - ROM: byte-addressable program memory (up to 4096 bytes, 12-bit address).
 *   - RAM: 4 banks x 4 registers x 16 characters (nibbles) of data.
 *   - RAM status: 4 banks x 4 registers x 4 status nibbles.
 *   - Hardware stack: 3 levels deep, holding 12-bit return addresses. Nesting
 *     a 4th call silently overwrites the oldest entry (the real chip wraps).
 *
 * ## Instruction encoding
 *
 * Instructions are 1 or 2 bytes. The upper nibble of the first byte is the
 * opcode, the lower nibble an operand. Six opcode classes take a second byte:
 * JCN (0x1_), FIM (0x2_ even), JUN (0x4_), JMS (0x5_), ISZ (0x7_).
 *
 * `i4004_step` executes one instruction and records an `I4004Trace`;
 * `i4004_run` resets, loads a program at address 0, and steps until HLT or
 * `max_steps`. Traces are retained in the simulator (query via
 * `i4004_trace_count` / `i4004_trace`).
 *
 * Pure ISO C17: no <math.h>, no compiler extensions.
 */
#ifndef INTEL4004_SIMULATOR_H
#define INTEL4004_SIMULATOR_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* uint8_t, uint16_t */

#ifdef __cplusplus
extern "C" {
#endif

/*
 * One executed instruction's record. Because the 4004 is accumulator-based,
 * we capture the accumulator and carry both before and after. For a two-byte
 * instruction `has_raw2` is 1 and `raw2` holds the second byte.
 *
 * `mnemonic` is a NUL-terminated disassembly (longest form "UNKNOWN(0xFE)").
 */
typedef struct {
    size_t address;
    uint8_t raw;
    int has_raw2;
    uint8_t raw2;
    char mnemonic[16];
    uint8_t accumulator_before;
    uint8_t accumulator_after;
    int carry_before;
    int carry_after;
} I4004Trace;

typedef struct I4004Sim I4004Sim;

/* Construct with `memory_size` bytes of ROM (typically 4096). NULL on OOM. */
I4004Sim *i4004_new(size_t memory_size);
void i4004_free(I4004Sim *s);
void i4004_reset(I4004Sim *s);
/* Copy `program` to ROM starting at address 0 (clamped to memory size),
 * reset PC to 0, and clear the halted flag. */
void i4004_load_program(I4004Sim *s, const uint8_t *program, size_t len);

/* ── State accessors ───────────────────────────────────────────────────────*/
uint8_t i4004_accumulator(const I4004Sim *s);
int i4004_carry(const I4004Sim *s);
/* Register r (0-15); out-of-range returns 0. */
uint8_t i4004_register(const I4004Sim *s, size_t r);
size_t i4004_pc(const I4004Sim *s);
int i4004_halted(const I4004Sim *s);
/* Hardware stack slot i (0-2); out-of-range returns 0. */
uint16_t i4004_hw_stack(const I4004Sim *s, size_t i);
size_t i4004_stack_pointer(const I4004Sim *s);
/* Data RAM nibble at [bank][reg][chr]; out-of-range returns 0. */
uint8_t i4004_ram(const I4004Sim *s, size_t bank, size_t reg, size_t chr);
/* RAM status nibble at [bank][reg][idx]; out-of-range returns 0. */
uint8_t i4004_ram_status(const I4004Sim *s, size_t bank, size_t reg,
                         size_t idx);
/* RAM output-port latch for a bank (0-3); out-of-range returns 0. */
uint8_t i4004_ram_output(const I4004Sim *s, size_t bank);
size_t i4004_ram_bank(const I4004Sim *s);
size_t i4004_ram_register(const I4004Sim *s);
size_t i4004_ram_character(const I4004Sim *s);
uint8_t i4004_rom_port(const I4004Sim *s);

/* Execute one instruction. Returns 1 on success (fills *out if non-NULL),
 * 0 if the CPU is already halted. Unknown opcodes do NOT halt — they record a
 * "UNKNOWN(0xXX)" mnemonic and execution continues, faithful to the Rust. */
int i4004_step(I4004Sim *s, I4004Trace *out);

/* Reset, load `program` at 0, then step up to `max_steps` (stopping at HLT).
 * Returns the number of traces recorded. */
size_t i4004_run(I4004Sim *s, const uint8_t *program, size_t len,
                 size_t max_steps);

/* Retained traces from the last `i4004_run` (or accumulated `i4004_step`s). */
size_t i4004_trace_count(const I4004Sim *s);
int i4004_trace(const I4004Sim *s, size_t i, I4004Trace *out);

/* ── Encoding helpers ──────────────────────────────────────────────────────*/
/* Single-byte instructions return the opcode byte directly. */
uint8_t i4004_encode_nop(void);
uint8_t i4004_encode_hlt(void);
uint8_t i4004_encode_ldm(uint8_t n);
uint8_t i4004_encode_ld(uint8_t r);
uint8_t i4004_encode_xch(uint8_t r);
uint8_t i4004_encode_add(uint8_t r);
uint8_t i4004_encode_sub(uint8_t r);
uint8_t i4004_encode_inc(uint8_t r);
uint8_t i4004_encode_bbl(uint8_t n);
uint8_t i4004_encode_src(uint8_t pair);
uint8_t i4004_encode_fin(uint8_t pair);
uint8_t i4004_encode_jin(uint8_t pair);

/* Two-byte instructions return the first byte and write the second via *lo. */
uint8_t i4004_encode_jcn(uint8_t cond, uint8_t addr, uint8_t *lo);
uint8_t i4004_encode_fim(uint8_t pair, uint8_t data, uint8_t *lo);
uint8_t i4004_encode_jun(uint16_t addr, uint8_t *lo);
uint8_t i4004_encode_jms(uint16_t addr, uint8_t *lo);
uint8_t i4004_encode_isz(uint8_t r, uint8_t addr, uint8_t *lo);

/* I/O and RAM instructions (fixed single-byte encodings). */
uint8_t i4004_encode_wrm(void);
uint8_t i4004_encode_wmp(void);
uint8_t i4004_encode_wrr(void);
uint8_t i4004_encode_wpm(void);
uint8_t i4004_encode_wr0(void);
uint8_t i4004_encode_wr1(void);
uint8_t i4004_encode_wr2(void);
uint8_t i4004_encode_wr3(void);
uint8_t i4004_encode_sbm(void);
uint8_t i4004_encode_rdm(void);
uint8_t i4004_encode_rdr(void);
uint8_t i4004_encode_adm(void);
uint8_t i4004_encode_rd0(void);
uint8_t i4004_encode_rd1(void);
uint8_t i4004_encode_rd2(void);
uint8_t i4004_encode_rd3(void);

/* Accumulator-group instructions (fixed single-byte encodings). */
uint8_t i4004_encode_clb(void);
uint8_t i4004_encode_clc(void);
uint8_t i4004_encode_iac(void);
uint8_t i4004_encode_cmc(void);
uint8_t i4004_encode_cma(void);
uint8_t i4004_encode_ral(void);
uint8_t i4004_encode_rar(void);
uint8_t i4004_encode_tcc(void);
uint8_t i4004_encode_dac(void);
uint8_t i4004_encode_tcs(void);
uint8_t i4004_encode_stc(void);
uint8_t i4004_encode_daa(void);
uint8_t i4004_encode_kbp(void);
uint8_t i4004_encode_dcl(void);

#ifdef __cplusplus
}
#endif

#endif /* INTEL4004_SIMULATOR_H */
