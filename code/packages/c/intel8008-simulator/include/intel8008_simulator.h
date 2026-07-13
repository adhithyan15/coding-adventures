/*
 * intel8008_simulator.h — Intel 8008 behavioral simulator, pure ISO C17.
 * =====================================================================
 *
 * A faithful port of the Rust `intel8008-simulator` crate: a behavioral
 * simulator for the Intel 8008 (1972), the world's first 8-bit microprocessor.
 *
 * It executes 8008 machine code directly (no gate-level modelling): registers
 * A/B/C/D/E/H/L, the M pseudo-register (memory at [H:L]), four condition flags
 * (carry / zero / sign / parity), a 16 KiB address space, and the 8008's unique
 * 8-level push-down call stack (entry[0] IS the program counter).
 *
 * `i8008_step` executes one instruction and records a `I8008Trace`;
 * `i8008_run` loads a program at address 0 and steps until HLT, an error, or
 * `max_steps`. Traces are retained in the simulator (query via
 * `i8008_trace_count` / `i8008_trace`).
 *
 * Pure ISO C17: no <math.h>, no compiler extensions.
 */
#ifndef INTEL8008_SIMULATOR_H
#define INTEL8008_SIMULATOR_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* uint8_t, uint16_t */

#ifdef __cplusplus
extern "C" {
#endif

typedef struct {
    int carry;
    int zero;
    int sign;
    int parity;
} I8008Flags;

typedef struct {
    uint16_t address;
    uint8_t raw[3];
    size_t raw_len;
    char mnemonic[24];
    uint8_t a_before;
    uint8_t a_after;
    I8008Flags flags_before;
    I8008Flags flags_after;
    int has_mem_address;
    uint16_t mem_address;
    int has_mem_value;
    uint8_t mem_value;
} I8008Trace;

typedef struct I8008Sim I8008Sim;

I8008Sim *i8008_new(void); /* NULL on OOM */
void i8008_free(I8008Sim *s);
void i8008_reset(I8008Sim *s);
void i8008_load_program(I8008Sim *s, const uint8_t *program, size_t len,
                        size_t start);

/* Register / state accessors. */
uint8_t i8008_a(const I8008Sim *s);
uint8_t i8008_b(const I8008Sim *s);
uint8_t i8008_c(const I8008Sim *s);
uint8_t i8008_d(const I8008Sim *s);
uint8_t i8008_e(const I8008Sim *s);
uint8_t i8008_h(const I8008Sim *s);
uint8_t i8008_l(const I8008Sim *s);
uint16_t i8008_pc(const I8008Sim *s);
uint16_t i8008_hl_address(const I8008Sim *s);
I8008Flags i8008_flags(const I8008Sim *s);
size_t i8008_stack_depth(const I8008Sim *s);
int i8008_halted(const I8008Sim *s);

/* I/O ports (port out of range is ignored / returns 0). */
void i8008_set_input_port(I8008Sim *s, size_t port, uint8_t value);
uint8_t i8008_get_output_port(const I8008Sim *s, size_t port);

/* Execute one instruction. Returns 1 on success (fills *out if non-NULL),
 * 0 if the CPU is halted or the opcode is unknown. */
int i8008_step(I8008Sim *s, I8008Trace *out);

/* Reset, load `program` at 0, and step up to `max_steps` (stopping at HLT or an
 * error). Returns the number of traces recorded. */
size_t i8008_run(I8008Sim *s, const uint8_t *program, size_t len,
                 size_t max_steps);

/* Retained traces from the last `i8008_run` (or accumulated `i8008_step`s). */
size_t i8008_trace_count(const I8008Sim *s);
int i8008_trace(const I8008Sim *s, size_t i, I8008Trace *out);

#ifdef __cplusplus
}
#endif

#endif /* INTEL8008_SIMULATOR_H */
