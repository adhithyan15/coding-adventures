/*
 * intel4004_simulator.c — Intel 4004 behavioral simulator, pure ISO C17.
 * =====================================================================
 *
 * See intel4004_simulator.h. A direct fetch-decode-execute loop over the 46
 * instructions of the world's first commercial microprocessor. Every data
 * path is 4 bits wide; arithmetic masks to a nibble (`& 0xF`). Two-byte
 * instructions (JCN/FIM/JUN/JMS/ISZ) fetch a second byte during `step`.
 *
 * A faithful port of the Rust crate: identical semantics, including the
 * inverted-carry convention for subtraction, the 3-deep wrapping hardware
 * stack, one-hot KBP decode, and the BCD DAA adjust.
 */
#include "intel4004_simulator.h"

#include <stdio.h>  /* snprintf */
#include <stdlib.h> /* malloc, calloc, realloc, free */
#include <string.h> /* memcpy */

struct I4004Sim {
    uint8_t accumulator;    /* 4-bit */
    uint8_t registers[16];  /* R0-R15, each 4-bit */
    int carry;              /* 0/1 */
    uint8_t *memory;        /* byte-addressable ROM */
    size_t memory_size;
    size_t pc;              /* 12-bit program counter */
    int halted;

    uint16_t hw_stack[3];   /* 3-level 12-bit return-address stack */
    size_t stack_pointer;   /* next free slot, 0-2 */

    uint8_t ram[4][4][16];    /* bank x register x character (nibbles) */
    uint8_t ram_status[4][4][4]; /* bank x register x status nibble */
    uint8_t ram_output[4];    /* one output-port latch per bank */
    size_t ram_bank;          /* selected by DCL (0-3) */
    size_t ram_register;      /* selected by SRC (0-3) */
    size_t ram_character;     /* selected by SRC (0-15) */
    uint8_t rom_port;         /* 4-bit ROM I/O port */

    I4004Trace *traces;
    size_t traces_len, traces_cap;
};

/* ── Construction / lifecycle ───────────────────────────────────────────────*/
I4004Sim *i4004_new(size_t memory_size) {
    I4004Sim *s = (I4004Sim *)calloc(1, sizeof(I4004Sim));
    if (!s) {
        return NULL;
    }
    /* `calloc(0, 1)` may return NULL; guarantee a non-NULL buffer so a
     * zero-size ROM still yields a usable simulator (reads simply fall out of
     * range and return 0). */
    s->memory = (uint8_t *)calloc(memory_size ? memory_size : 1, 1);
    if (!s->memory) {
        free(s);
        return NULL;
    }
    s->memory_size = memory_size;
    return s;
}

void i4004_free(I4004Sim *s) {
    if (!s) {
        return;
    }
    free(s->memory);
    free(s->traces);
    free(s);
}

void i4004_reset(I4004Sim *s) {
    size_t i;
    s->accumulator = 0;
    for (i = 0; i < 16; i++) {
        s->registers[i] = 0;
    }
    s->carry = 0;
    for (i = 0; i < s->memory_size; i++) {
        s->memory[i] = 0;
    }
    s->pc = 0;
    s->halted = 0;
    s->hw_stack[0] = s->hw_stack[1] = s->hw_stack[2] = 0;
    s->stack_pointer = 0;
    memset(s->ram, 0, sizeof s->ram);
    memset(s->ram_status, 0, sizeof s->ram_status);
    s->ram_output[0] = s->ram_output[1] = s->ram_output[2] =
        s->ram_output[3] = 0;
    s->ram_bank = 0;
    s->ram_register = 0;
    s->ram_character = 0;
    s->rom_port = 0;
}

void i4004_load_program(I4004Sim *s, const uint8_t *program, size_t len) {
    if (len > s->memory_size) {
        len = s->memory_size; /* Rust would panic; we clamp defensively. */
    }
    if (len > 0) {
        memcpy(s->memory, program, len);
    }
    s->pc = 0;
    s->halted = 0;
}

/* ── State accessors ────────────────────────────────────────────────────────*/
uint8_t i4004_accumulator(const I4004Sim *s) { return s->accumulator; }
int i4004_carry(const I4004Sim *s) { return s->carry; }
uint8_t i4004_register(const I4004Sim *s, size_t r) {
    return r < 16 ? s->registers[r] : 0;
}
size_t i4004_pc(const I4004Sim *s) { return s->pc; }
int i4004_halted(const I4004Sim *s) { return s->halted; }
uint16_t i4004_hw_stack(const I4004Sim *s, size_t i) {
    return i < 3 ? s->hw_stack[i] : 0;
}
size_t i4004_stack_pointer(const I4004Sim *s) { return s->stack_pointer; }
uint8_t i4004_ram(const I4004Sim *s, size_t bank, size_t reg, size_t chr) {
    if (bank < 4 && reg < 4 && chr < 16) {
        return s->ram[bank][reg][chr];
    }
    return 0;
}
uint8_t i4004_ram_status(const I4004Sim *s, size_t bank, size_t reg,
                         size_t idx) {
    if (bank < 4 && reg < 4 && idx < 4) {
        return s->ram_status[bank][reg][idx];
    }
    return 0;
}
uint8_t i4004_ram_output(const I4004Sim *s, size_t bank) {
    return bank < 4 ? s->ram_output[bank] : 0;
}
size_t i4004_ram_bank(const I4004Sim *s) { return s->ram_bank; }
size_t i4004_ram_register(const I4004Sim *s) { return s->ram_register; }
size_t i4004_ram_character(const I4004Sim *s) { return s->ram_character; }
uint8_t i4004_rom_port(const I4004Sim *s) { return s->rom_port; }

size_t i4004_trace_count(const I4004Sim *s) { return s->traces_len; }
int i4004_trace(const I4004Sim *s, size_t i, I4004Trace *out) {
    if (i >= s->traces_len) {
        return 0;
    }
    if (out) {
        *out = s->traces[i];
    }
    return 1;
}

/* ── Internals ──────────────────────────────────────────────────────────────*/

/* Determine whether a first byte begins a 2-byte instruction:
 *   upper nibble 0x1 (JCN), 0x4 (JUN), 0x5 (JMS), 0x7 (ISZ), or
 *   0x2 with bit0 == 0 (FIM; the odd form 0x2_ is single-byte SRC). */
static int is_two_byte(uint8_t raw) {
    uint8_t upper = (uint8_t)((raw >> 4) & 0xF);
    if (upper == 0x1 || upper == 0x4 || upper == 0x5 || upper == 0x7) {
        return 1;
    }
    return upper == 0x2 && (raw & 0x1) == 0;
}

/* Bounds-checked ROM read: an address past the end reads as 0x00 (NOP),
 * which keeps a runaway PC safe rather than reading out of bounds. */
static uint8_t rom_read(const I4004Sim *s, size_t addr) {
    return addr < s->memory_size ? s->memory[addr] : 0;
}

/* Register-pair helpers: pair p groups (R[2p], R[2p+1]) as high:low nibbles. */
static uint8_t read_pair(const I4004Sim *s, size_t p) {
    uint8_t hi = (uint8_t)(s->registers[p * 2] & 0xF);
    uint8_t lo = (uint8_t)(s->registers[p * 2 + 1] & 0xF);
    return (uint8_t)((hi << 4) | lo);
}
static void write_pair(I4004Sim *s, size_t p, uint8_t val) {
    s->registers[p * 2] = (uint8_t)((val >> 4) & 0xF);
    s->registers[p * 2 + 1] = (uint8_t)(val & 0xF);
}

/* Hardware stack: 3 deep, wraps silently on the 4th push. */
static void stack_push(I4004Sim *s, uint16_t addr) {
    s->hw_stack[s->stack_pointer % 3] = (uint16_t)(addr & 0xFFF);
    s->stack_pointer = (s->stack_pointer + 1) % 3;
}
static uint16_t stack_pop(I4004Sim *s) {
    s->stack_pointer = s->stack_pointer == 0 ? 2 : s->stack_pointer - 1;
    return s->hw_stack[s->stack_pointer % 3];
}

/* Currently addressed RAM nibbles (bank via DCL, register/char via SRC). */
static uint8_t ram_read_main(const I4004Sim *s) {
    return (uint8_t)(s->ram[s->ram_bank][s->ram_register][s->ram_character] &
                     0xF);
}
static void ram_write_main(I4004Sim *s, uint8_t val) {
    s->ram[s->ram_bank][s->ram_register][s->ram_character] =
        (uint8_t)(val & 0xF);
}
static uint8_t ram_read_status(const I4004Sim *s, size_t idx) {
    return (uint8_t)(s->ram_status[s->ram_bank][s->ram_register][idx] & 0xF);
}
static void ram_write_status(I4004Sim *s, size_t idx, uint8_t val) {
    s->ram_status[s->ram_bank][s->ram_register][idx] = (uint8_t)(val & 0xF);
}

/*
 * Execute the decoded instruction, mutating state and writing a NUL-terminated
 * disassembly into `mn`. Mirrors the Rust `execute` dispatch exactly.
 */
static void execute(I4004Sim *s, uint8_t opcode, uint8_t operand, uint8_t raw,
                    int has_raw2, uint8_t raw2, size_t address, char *mn,
                    size_t mn_sz) {
    if (raw == 0x00) {
        snprintf(mn, mn_sz, "NOP");
        return;
    }
    if (raw == 0x01) {
        s->halted = 1;
        snprintf(mn, mn_sz, "HLT");
        return;
    }

    switch (opcode) {
    case 0x1: { /* JCN — jump conditional */
        uint8_t cond = operand;
        uint8_t addr_low = has_raw2 ? raw2 : 0;
        int invert = (cond & 0x8) != 0;
        int test_zero = (cond & 0x4) != 0;
        int test_carry = (cond & 0x2) != 0;
        int test_pin = (cond & 0x1) != 0;
        int result = 0;
        if (test_zero) {
            result = result || (s->accumulator == 0);
        }
        if (test_carry) {
            result = result || s->carry;
        }
        if (test_pin) {
            result = result || 0; /* test pin always low in the simulator */
        }
        if (invert) {
            result = !result;
        }
        if (result) {
            size_t page = (address + 2) & 0xF00;
            s->pc = page | (size_t)addr_low;
        }
        snprintf(mn, mn_sz, "JCN 0x%X,0x%02X", cond, addr_low);
        return;
    }
    case 0x2:
        if ((raw & 1) == 0) { /* FIM — fetch immediate to register pair */
            size_t pair = (size_t)(operand >> 1);
            uint8_t data = has_raw2 ? raw2 : 0;
            write_pair(s, pair, data);
            snprintf(mn, mn_sz, "FIM P%u,0x%02X", (unsigned)pair, data);
        } else { /* SRC — send register control (set RAM address) */
            size_t pair = (size_t)(operand >> 1);
            uint8_t pair_val = read_pair(s, pair);
            s->ram_register = (size_t)((pair_val >> 4) & 0x3);
            s->ram_character = (size_t)(pair_val & 0xF);
            snprintf(mn, mn_sz, "SRC P%u", (unsigned)pair);
        }
        return;
    case 0x3:
        if ((raw & 1) == 0) { /* FIN — fetch indirect from ROM */
            size_t pair = (size_t)(operand >> 1);
            size_t rom_addr = (size_t)read_pair(s, 0);
            size_t page = s->pc & 0xF00;
            uint8_t data = rom_read(s, page | rom_addr);
            write_pair(s, pair, data);
            snprintf(mn, mn_sz, "FIN P%u", (unsigned)pair);
        } else { /* JIN — jump indirect through register pair */
            size_t pair = (size_t)(operand >> 1);
            size_t pair_val = (size_t)read_pair(s, pair);
            size_t page = s->pc & 0xF00;
            s->pc = page | pair_val;
            snprintf(mn, mn_sz, "JIN P%u", (unsigned)pair);
        }
        return;
    case 0x4: { /* JUN — jump unconditional (12-bit) */
        size_t addr_hi = (size_t)operand << 8;
        size_t addr_lo = has_raw2 ? raw2 : 0;
        size_t target = addr_hi | addr_lo;
        s->pc = target;
        snprintf(mn, mn_sz, "JUN 0x%03X", (unsigned)target);
        return;
    }
    case 0x5: { /* JMS — jump to subroutine (push return address) */
        size_t addr_hi = (size_t)operand << 8;
        size_t addr_lo = has_raw2 ? raw2 : 0;
        size_t target = addr_hi | addr_lo;
        stack_push(s, (uint16_t)s->pc);
        s->pc = target;
        snprintf(mn, mn_sz, "JMS 0x%03X", (unsigned)target);
        return;
    }
    case 0x6: { /* INC — increment register (no carry effect) */
        size_t reg = (size_t)operand;
        s->registers[reg] = (uint8_t)((s->registers[reg] + 1) & 0xF);
        snprintf(mn, mn_sz, "INC R%u", (unsigned)reg);
        return;
    }
    case 0x7: { /* ISZ — increment and skip if zero */
        size_t reg = (size_t)operand;
        uint8_t addr_low = has_raw2 ? raw2 : 0;
        s->registers[reg] = (uint8_t)((s->registers[reg] + 1) & 0xF);
        if (s->registers[reg] != 0) {
            size_t page = (address + 2) & 0xF00;
            s->pc = page | (size_t)addr_low;
        }
        snprintf(mn, mn_sz, "ISZ R%u,0x%02X", (unsigned)reg, addr_low);
        return;
    }
    case 0x8: { /* ADD — accumulator + register + carry */
        size_t reg = (size_t)(operand & 0xF);
        unsigned carry_in = s->carry ? 1u : 0u;
        unsigned result = (unsigned)s->accumulator + s->registers[reg] +
                          carry_in;
        s->carry = result > 0xF;
        s->accumulator = (uint8_t)(result & 0xF);
        snprintf(mn, mn_sz, "ADD R%u", (unsigned)reg);
        return;
    }
    case 0x9: { /* SUB — complement-and-add subtraction (inverted carry) */
        size_t reg = (size_t)(operand & 0xF);
        unsigned complement = (unsigned)((~s->registers[reg]) & 0xF);
        unsigned borrow_in = s->carry ? 0u : 1u;
        unsigned result = (unsigned)s->accumulator + complement + borrow_in;
        s->carry = result > 0xF;
        s->accumulator = (uint8_t)(result & 0xF);
        snprintf(mn, mn_sz, "SUB R%u", (unsigned)reg);
        return;
    }
    case 0xA: { /* LD — load register into accumulator */
        size_t reg = (size_t)(operand & 0xF);
        s->accumulator = (uint8_t)(s->registers[reg] & 0xF);
        snprintf(mn, mn_sz, "LD R%u", (unsigned)reg);
        return;
    }
    case 0xB: { /* XCH — exchange accumulator and register */
        size_t reg = (size_t)(operand & 0xF);
        uint8_t old_a = s->accumulator;
        s->accumulator = (uint8_t)(s->registers[reg] & 0xF);
        s->registers[reg] = (uint8_t)(old_a & 0xF);
        snprintf(mn, mn_sz, "XCH R%u", (unsigned)reg);
        return;
    }
    case 0xC: { /* BBL — branch back and load (subroutine return) */
        uint16_t ret_addr = stack_pop(s);
        s->pc = (size_t)ret_addr;
        s->accumulator = (uint8_t)(operand & 0xF);
        snprintf(mn, mn_sz, "BBL %u", (unsigned)operand);
        return;
    }
    case 0xD: /* LDM — load immediate into accumulator */
        s->accumulator = (uint8_t)(operand & 0xF);
        snprintf(mn, mn_sz, "LDM %u", (unsigned)operand);
        return;
    case 0xE: /* I/O and RAM instructions */
        switch (raw) {
        case 0xE0:
            ram_write_main(s, s->accumulator);
            snprintf(mn, mn_sz, "WRM");
            return;
        case 0xE1:
            s->ram_output[s->ram_bank] = (uint8_t)(s->accumulator & 0xF);
            snprintf(mn, mn_sz, "WMP");
            return;
        case 0xE2:
            s->rom_port = (uint8_t)(s->accumulator & 0xF);
            snprintf(mn, mn_sz, "WRR");
            return;
        case 0xE3: /* WPM — write program memory: no-op in the simulator */
            snprintf(mn, mn_sz, "WPM");
            return;
        case 0xE4:
            ram_write_status(s, 0, s->accumulator);
            snprintf(mn, mn_sz, "WR0");
            return;
        case 0xE5:
            ram_write_status(s, 1, s->accumulator);
            snprintf(mn, mn_sz, "WR1");
            return;
        case 0xE6:
            ram_write_status(s, 2, s->accumulator);
            snprintf(mn, mn_sz, "WR2");
            return;
        case 0xE7:
            ram_write_status(s, 3, s->accumulator);
            snprintf(mn, mn_sz, "WR3");
            return;
        case 0xE8: { /* SBM — subtract RAM from accumulator */
            uint8_t mem_val = ram_read_main(s);
            unsigned complement = (unsigned)((~mem_val) & 0xF);
            unsigned borrow_in = s->carry ? 0u : 1u;
            unsigned result = (unsigned)s->accumulator + complement + borrow_in;
            s->carry = result > 0xF;
            s->accumulator = (uint8_t)(result & 0xF);
            snprintf(mn, mn_sz, "SBM");
            return;
        }
        case 0xE9:
            s->accumulator = ram_read_main(s);
            snprintf(mn, mn_sz, "RDM");
            return;
        case 0xEA:
            s->accumulator = (uint8_t)(s->rom_port & 0xF);
            snprintf(mn, mn_sz, "RDR");
            return;
        case 0xEB: { /* ADM — add RAM to accumulator with carry */
            uint8_t mem_val = ram_read_main(s);
            unsigned carry_in = s->carry ? 1u : 0u;
            unsigned result = (unsigned)s->accumulator + mem_val + carry_in;
            s->carry = result > 0xF;
            s->accumulator = (uint8_t)(result & 0xF);
            snprintf(mn, mn_sz, "ADM");
            return;
        }
        case 0xEC:
            s->accumulator = ram_read_status(s, 0);
            snprintf(mn, mn_sz, "RD0");
            return;
        case 0xED:
            s->accumulator = ram_read_status(s, 1);
            snprintf(mn, mn_sz, "RD1");
            return;
        case 0xEE:
            s->accumulator = ram_read_status(s, 2);
            snprintf(mn, mn_sz, "RD2");
            return;
        case 0xEF:
            s->accumulator = ram_read_status(s, 3);
            snprintf(mn, mn_sz, "RD3");
            return;
        default:
            snprintf(mn, mn_sz, "UNKNOWN(0x%02X)", raw);
            return;
        }
    case 0xF: /* accumulator-group instructions */
        switch (raw) {
        case 0xF0:
            s->accumulator = 0;
            s->carry = 0;
            snprintf(mn, mn_sz, "CLB");
            return;
        case 0xF1:
            s->carry = 0;
            snprintf(mn, mn_sz, "CLC");
            return;
        case 0xF2: { /* IAC — increment accumulator */
            unsigned result = (unsigned)s->accumulator + 1;
            s->carry = result > 0xF;
            s->accumulator = (uint8_t)(result & 0xF);
            snprintf(mn, mn_sz, "IAC");
            return;
        }
        case 0xF3:
            s->carry = !s->carry;
            snprintf(mn, mn_sz, "CMC");
            return;
        case 0xF4:
            s->accumulator = (uint8_t)((~s->accumulator) & 0xF);
            snprintf(mn, mn_sz, "CMA");
            return;
        case 0xF5: { /* RAL — rotate left through carry */
            uint8_t old_carry = s->carry ? 1u : 0u;
            s->carry = (s->accumulator & 0x8) != 0;
            s->accumulator =
                (uint8_t)(((s->accumulator << 1) | old_carry) & 0xF);
            snprintf(mn, mn_sz, "RAL");
            return;
        }
        case 0xF6: { /* RAR — rotate right through carry */
            uint8_t old_carry = s->carry ? 0x8u : 0u;
            s->carry = (s->accumulator & 0x1) != 0;
            s->accumulator =
                (uint8_t)(((s->accumulator >> 1) | old_carry) & 0xF);
            snprintf(mn, mn_sz, "RAR");
            return;
        }
        case 0xF7: /* TCC — transfer carry to accumulator, clear carry */
            s->accumulator = (uint8_t)(s->carry ? 1 : 0);
            s->carry = 0;
            snprintf(mn, mn_sz, "TCC");
            return;
        case 0xF8: /* DAC — decrement accumulator (carry = no borrow) */
            s->carry = s->accumulator > 0;
            s->accumulator = (uint8_t)((s->accumulator - 1) & 0xF);
            snprintf(mn, mn_sz, "DAC");
            return;
        case 0xF9: /* TCS — transfer carry subtract, clear carry */
            s->accumulator = (uint8_t)(s->carry ? 10 : 9);
            s->carry = 0;
            snprintf(mn, mn_sz, "TCS");
            return;
        case 0xFA:
            s->carry = 1;
            snprintf(mn, mn_sz, "STC");
            return;
        case 0xFB: /* DAA — decimal adjust accumulator */
            if (s->accumulator > 9 || s->carry) {
                unsigned result = (unsigned)s->accumulator + 6;
                if (result > 0xF) {
                    s->carry = 1;
                }
                s->accumulator = (uint8_t)(result & 0xF);
            }
            snprintf(mn, mn_sz, "DAA");
            return;
        case 0xFC: /* KBP — one-hot keyboard decode */
            switch (s->accumulator) {
            case 0:
                s->accumulator = 0;
                break;
            case 1:
                s->accumulator = 1;
                break;
            case 2:
                s->accumulator = 2;
                break;
            case 4:
                s->accumulator = 3;
                break;
            case 8:
                s->accumulator = 4;
                break;
            default:
                s->accumulator = 15;
                break;
            }
            snprintf(mn, mn_sz, "KBP");
            return;
        case 0xFD: /* DCL — designate command line (select RAM bank) */
            s->ram_bank = (size_t)(s->accumulator & 0x7);
            if (s->ram_bank > 3) {
                s->ram_bank &= 3;
            }
            snprintf(mn, mn_sz, "DCL");
            return;
        default:
            snprintf(mn, mn_sz, "UNKNOWN(0x%02X)", raw);
            return;
        }
    default:
        snprintf(mn, mn_sz, "UNKNOWN(0x%02X)", raw);
        return;
    }
}

int i4004_step(I4004Sim *s, I4004Trace *out) {
    I4004Trace t;
    uint8_t raw, opcode, operand;
    if (s->halted) {
        return 0; /* the Rust `step` asserts !halted; we decline gracefully */
    }

    t.address = s->pc;
    raw = rom_read(s, s->pc);
    s->pc += 1;

    t.raw = raw;
    if (is_two_byte(raw)) {
        t.has_raw2 = 1;
        t.raw2 = rom_read(s, s->pc);
        s->pc += 1;
    } else {
        t.has_raw2 = 0;
        t.raw2 = 0;
    }

    t.accumulator_before = s->accumulator;
    t.carry_before = s->carry;

    opcode = (uint8_t)((raw >> 4) & 0xF);
    operand = (uint8_t)(raw & 0xF);
    execute(s, opcode, operand, raw, t.has_raw2, t.raw2, t.address, t.mnemonic,
            sizeof t.mnemonic);

    t.accumulator_after = s->accumulator;
    t.carry_after = s->carry;

    if (out) {
        *out = t;
    }
    return 1;
}

/* Grow guard for the trace vector — guards `size_t` overflow both when
 * doubling the capacity and when scaling it by the element size. */
static int grow_traces(I4004Sim *s) {
    size_t nc;
    I4004Trace *nd;
    if (s->traces_len < s->traces_cap) {
        return 1;
    }
    nc = s->traces_cap ? s->traces_cap : 8;
    if (nc > (size_t)-1 / 2) {
        return 0;
    }
    nc *= 2;
    if (nc > (size_t)-1 / sizeof(I4004Trace)) {
        return 0;
    }
    nd = (I4004Trace *)realloc(s->traces, nc * sizeof(I4004Trace));
    if (!nd) {
        return 0;
    }
    s->traces = nd;
    s->traces_cap = nc;
    return 1;
}

size_t i4004_run(I4004Sim *s, const uint8_t *program, size_t len,
                 size_t max_steps) {
    size_t i;
    i4004_reset(s);
    i4004_load_program(s, program, len);
    s->traces_len = 0;
    for (i = 0; i < max_steps; i++) {
        I4004Trace t;
        if (!i4004_step(s, &t)) {
            break; /* halted */
        }
        if (!grow_traces(s)) {
            break; /* OOM: stop recording, keep what we have */
        }
        s->traces[s->traces_len++] = t;
    }
    return s->traces_len;
}

/* ── Encoding helpers ──────────────────────────────────────────────────────*/
uint8_t i4004_encode_nop(void) { return 0x00; }
uint8_t i4004_encode_hlt(void) { return 0x01; }
uint8_t i4004_encode_ldm(uint8_t n) { return (uint8_t)((0xD << 4) | (n & 0xF)); }
uint8_t i4004_encode_ld(uint8_t r) { return (uint8_t)((0xA << 4) | (r & 0xF)); }
uint8_t i4004_encode_xch(uint8_t r) { return (uint8_t)((0xB << 4) | (r & 0xF)); }
uint8_t i4004_encode_add(uint8_t r) { return (uint8_t)((0x8 << 4) | (r & 0xF)); }
uint8_t i4004_encode_sub(uint8_t r) { return (uint8_t)((0x9 << 4) | (r & 0xF)); }
uint8_t i4004_encode_inc(uint8_t r) { return (uint8_t)((0x6 << 4) | (r & 0xF)); }
uint8_t i4004_encode_bbl(uint8_t n) { return (uint8_t)((0xC << 4) | (n & 0xF)); }
uint8_t i4004_encode_src(uint8_t pair) {
    return (uint8_t)((0x2 << 4) | ((pair & 0x7) << 1) | 1);
}
uint8_t i4004_encode_fin(uint8_t pair) {
    return (uint8_t)((0x3 << 4) | ((pair & 0x7) << 1));
}
uint8_t i4004_encode_jin(uint8_t pair) {
    return (uint8_t)((0x3 << 4) | ((pair & 0x7) << 1) | 1);
}

uint8_t i4004_encode_jcn(uint8_t cond, uint8_t addr, uint8_t *lo) {
    *lo = addr;
    return (uint8_t)((0x1 << 4) | (cond & 0xF));
}
uint8_t i4004_encode_fim(uint8_t pair, uint8_t data, uint8_t *lo) {
    *lo = data;
    return (uint8_t)((0x2 << 4) | ((pair & 0x7) << 1));
}
uint8_t i4004_encode_jun(uint16_t addr, uint8_t *lo) {
    *lo = (uint8_t)(addr & 0xFF);
    return (uint8_t)((0x4 << 4) | ((addr >> 8) & 0xF));
}
uint8_t i4004_encode_jms(uint16_t addr, uint8_t *lo) {
    *lo = (uint8_t)(addr & 0xFF);
    return (uint8_t)((0x5 << 4) | ((addr >> 8) & 0xF));
}
uint8_t i4004_encode_isz(uint8_t r, uint8_t addr, uint8_t *lo) {
    *lo = addr;
    return (uint8_t)((0x7 << 4) | (r & 0xF));
}

uint8_t i4004_encode_wrm(void) { return 0xE0; }
uint8_t i4004_encode_wmp(void) { return 0xE1; }
uint8_t i4004_encode_wrr(void) { return 0xE2; }
uint8_t i4004_encode_wpm(void) { return 0xE3; }
uint8_t i4004_encode_wr0(void) { return 0xE4; }
uint8_t i4004_encode_wr1(void) { return 0xE5; }
uint8_t i4004_encode_wr2(void) { return 0xE6; }
uint8_t i4004_encode_wr3(void) { return 0xE7; }
uint8_t i4004_encode_sbm(void) { return 0xE8; }
uint8_t i4004_encode_rdm(void) { return 0xE9; }
uint8_t i4004_encode_rdr(void) { return 0xEA; }
uint8_t i4004_encode_adm(void) { return 0xEB; }
uint8_t i4004_encode_rd0(void) { return 0xEC; }
uint8_t i4004_encode_rd1(void) { return 0xED; }
uint8_t i4004_encode_rd2(void) { return 0xEE; }
uint8_t i4004_encode_rd3(void) { return 0xEF; }

uint8_t i4004_encode_clb(void) { return 0xF0; }
uint8_t i4004_encode_clc(void) { return 0xF1; }
uint8_t i4004_encode_iac(void) { return 0xF2; }
uint8_t i4004_encode_cmc(void) { return 0xF3; }
uint8_t i4004_encode_cma(void) { return 0xF4; }
uint8_t i4004_encode_ral(void) { return 0xF5; }
uint8_t i4004_encode_rar(void) { return 0xF6; }
uint8_t i4004_encode_tcc(void) { return 0xF7; }
uint8_t i4004_encode_dac(void) { return 0xF8; }
uint8_t i4004_encode_tcs(void) { return 0xF9; }
uint8_t i4004_encode_stc(void) { return 0xFA; }
uint8_t i4004_encode_daa(void) { return 0xFB; }
uint8_t i4004_encode_kbp(void) { return 0xFC; }
uint8_t i4004_encode_dcl(void) { return 0xFD; }
