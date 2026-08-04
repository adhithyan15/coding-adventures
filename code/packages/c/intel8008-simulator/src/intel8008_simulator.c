/*
 * intel8008_simulator.c — Intel 8008 behavioral simulator, pure ISO C17.
 * =====================================================================
 *
 * See intel8008_simulator.h. A direct fetch-decode-execute loop over the 8008
 * instruction set: group is `opcode[7:6]`, `ddd = opcode[5:3]`, `sss =
 * opcode[2:0]`. Register index 6 (M) aliases memory at [H:L]. The call stack is
 * an 8-entry push-down where stack[0] is always the live program counter.
 */
#include "intel8008_simulator.h"

#include <stdarg.h> /* va_list */
#include <stdio.h>  /* vsnprintf */
#include <stdlib.h> /* malloc, realloc, free, calloc */
#include <string.h> /* memcpy */

struct I8008Sim {
    uint8_t regs[8];
    uint8_t memory[16384];
    uint16_t stack[8];
    size_t stack_depth;
    I8008Flags flags;
    int halted;
    uint8_t input_ports[8];
    uint8_t output_ports[24];
    I8008Trace *traces;
    size_t traces_len, traces_cap;
};

static const char REG_NAMES[8] = {'B', 'C', 'D', 'E', 'H', 'L', 'M', 'A'};
static const char *ALU_MNEM[8] = {"ADD", "ADC", "SUB", "SBB",
                                  "ANA", "XRA", "ORA", "CMP"};
static const char *ALU_IMM_MNEM[8] = {"ADI", "ACI", "SUI", "SBI",
                                      "ANI", "XRI", "ORI", "CPI"};

static I8008Flags zero_flags(void) {
    I8008Flags f;
    f.carry = f.zero = f.sign = f.parity = 0;
    return f;
}

I8008Sim *i8008_new(void) { return (I8008Sim *)calloc(1, sizeof(I8008Sim)); }
void i8008_free(I8008Sim *s) {
    if (!s) {
        return;
    }
    free(s->traces);
    free(s);
}
void i8008_reset(I8008Sim *s) {
    size_t i;
    for (i = 0; i < 8; i++) {
        s->regs[i] = 0;
        s->stack[i] = 0;
    }
    s->stack_depth = 0;
    s->flags = zero_flags();
    s->halted = 0;
}
void i8008_load_program(I8008Sim *s, const uint8_t *program, size_t len,
                        size_t start) {
    size_t end = start + len;
    if (end > 16384) {
        end = 16384;
    }
    if (start < end) {
        memcpy(s->memory + start, program, end - start);
    }
}

uint8_t i8008_a(const I8008Sim *s) { return s->regs[7]; }
uint8_t i8008_b(const I8008Sim *s) { return s->regs[0]; }
uint8_t i8008_c(const I8008Sim *s) { return s->regs[1]; }
uint8_t i8008_d(const I8008Sim *s) { return s->regs[2]; }
uint8_t i8008_e(const I8008Sim *s) { return s->regs[3]; }
uint8_t i8008_h(const I8008Sim *s) { return s->regs[4]; }
uint8_t i8008_l(const I8008Sim *s) { return s->regs[5]; }
uint16_t i8008_pc(const I8008Sim *s) { return s->stack[0] & 0x3FFF; }
uint16_t i8008_hl_address(const I8008Sim *s) {
    return (uint16_t)((((uint16_t)s->regs[4] & 0x3F) << 8) | s->regs[5]);
}
I8008Flags i8008_flags(const I8008Sim *s) { return s->flags; }
size_t i8008_stack_depth(const I8008Sim *s) { return s->stack_depth; }
int i8008_halted(const I8008Sim *s) { return s->halted; }

void i8008_set_input_port(I8008Sim *s, size_t port, uint8_t value) {
    if (port < 8) {
        s->input_ports[port] = value;
    }
}
uint8_t i8008_get_output_port(const I8008Sim *s, size_t port) {
    return port < 24 ? s->output_ports[port] : 0;
}

size_t i8008_trace_count(const I8008Sim *s) { return s->traces_len; }
int i8008_trace(const I8008Sim *s, size_t i, I8008Trace *out) {
    if (i >= s->traces_len) {
        return 0;
    }
    if (out) {
        *out = s->traces[i];
    }
    return 1;
}

/* ── Internals ──────────────────────────────────────────────────────────────*/
static uint8_t mem_read(const I8008Sim *s, uint16_t addr) {
    return s->memory[addr & 0x3FFF];
}
static void mem_write(I8008Sim *s, uint16_t addr, uint8_t v) {
    s->memory[addr & 0x3FFF] = v;
}
static uint8_t reg_read(const I8008Sim *s, unsigned idx) {
    return idx == 6 ? mem_read(s, i8008_hl_address(s)) : s->regs[idx];
}
static void push_and_jump(I8008Sim *s, uint16_t target) {
    int i;
    for (i = 7; i >= 1; i--) {
        s->stack[i] = s->stack[i - 1];
    }
    s->stack[0] = target & 0x3FFF;
    if (s->stack_depth < 7) {
        s->stack_depth++;
    }
}
static void pop_return(I8008Sim *s) {
    int i;
    for (i = 0; i < 7; i++) {
        s->stack[i] = s->stack[i + 1];
    }
    s->stack[7] = 0;
    if (s->stack_depth > 0) {
        s->stack_depth--;
    }
}
static I8008Flags compute_flags(uint8_t result, int carry, int update_carry,
                                I8008Flags prev) {
    int ones = 0, i;
    I8008Flags f;
    for (i = 0; i < 8; i++) {
        ones += (result >> i) & 1;
    }
    f.carry = update_carry ? carry : prev.carry;
    f.zero = result == 0;
    f.sign = (result & 0x80) != 0;
    f.parity = (ones % 2) == 0;
    return f;
}
static int condition_met(const I8008Sim *s, uint8_t ccc, int sense) {
    int v = 0;
    switch (ccc & 0x03) {
        case 0: v = s->flags.carry; break;
        case 1: v = s->flags.zero; break;
        case 2: v = s->flags.sign; break;
        case 3: v = s->flags.parity; break;
    }
    return sense ? v : !v;
}
static void alu_op(uint8_t alu, uint8_t a, uint8_t b, int carry_in,
                   uint8_t *res, int *carry, int *clear_carry) {
    unsigned wide;
    *clear_carry = 0;
    switch (alu) {
        case 0:
            wide = (unsigned)a + b;
            *res = (uint8_t)wide;
            *carry = wide > 0xFF;
            break;
        case 1: {
            unsigned ci = carry_in ? 1u : 0u;
            wide = (unsigned)a + b + ci;
            *res = (uint8_t)wide;
            *carry = wide > 0xFF;
            break;
        }
        case 2:
            *res = (uint8_t)(a - b);
            *carry = a < b;
            break;
        case 3: {
            unsigned bi = carry_in ? 1u : 0u;
            unsigned total = (unsigned)b + bi;
            *res = (uint8_t)((unsigned)a - total);
            *carry = (unsigned)a < total;
            break;
        }
        case 4: *res = a & b; *carry = 0; *clear_carry = 1; break;
        case 5: *res = a ^ b; *carry = 0; *clear_carry = 1; break;
        case 6: *res = a | b; *carry = 0; *clear_carry = 1; break;
        default: /* 7 CMP */
            *res = (uint8_t)(a - b);
            *carry = a < b;
            break;
    }
}
static uint8_t fetch_byte(I8008Sim *s) {
    uint16_t pc = s->stack[0] & 0x3FFF;
    uint8_t byte = s->memory[pc];
    s->stack[0] = (uint16_t)((pc + 1) & 0x3FFF);
    return byte;
}
static char cond_letter(uint8_t ccc) {
    switch (ccc & 0x03) {
        case 0: return 'C';
        case 1: return 'Z';
        case 2: return 'S';
        default: return 'P';
    }
}

static void set_mn(I8008Trace *t, const char *fmt, ...) {
    va_list ap;
    va_start(ap, fmt);
    vsnprintf(t->mnemonic, sizeof t->mnemonic, fmt, ap);
    va_end(ap);
}

int i8008_step(I8008Sim *s, I8008Trace *out_opt) {
    I8008Trace tr;
    uint16_t fetch_pc;
    uint8_t opcode, group, ddd, sss, a_before;
    I8008Flags flags_before;

    if (s->halted) {
        return 0;
    }
    fetch_pc = s->stack[0] & 0x3FFF;
    a_before = s->regs[7];
    flags_before = s->flags;

    memset(&tr, 0, sizeof tr);
    opcode = fetch_byte(s);
    tr.raw[tr.raw_len++] = opcode;
    group = (opcode >> 6) & 0x03;
    ddd = (opcode >> 3) & 0x07;
    sss = opcode & 0x07;

    if (group == 0) {
        switch (sss) {
            case 0: { /* INR */
                uint8_t result = (uint8_t)(reg_read(s, ddd) + 1);
                if (ddd == 6) {
                    uint16_t addr = i8008_hl_address(s);
                    mem_write(s, addr, result);
                    tr.has_mem_address = 1;
                    tr.mem_address = addr;
                    tr.has_mem_value = 1;
                    tr.mem_value = result;
                } else {
                    s->regs[ddd] = result;
                }
                s->flags = compute_flags(result, 0, 0, s->flags);
                set_mn(&tr, "INR %c", REG_NAMES[ddd]);
                break;
            }
            case 1: { /* DCR */
                uint8_t result = (uint8_t)(reg_read(s, ddd) - 1);
                if (ddd == 6) {
                    uint16_t addr = i8008_hl_address(s);
                    mem_write(s, addr, result);
                    tr.has_mem_address = 1;
                    tr.mem_address = addr;
                    tr.has_mem_value = 1;
                    tr.mem_value = result;
                } else {
                    s->regs[ddd] = result;
                }
                s->flags = compute_flags(result, 0, 0, s->flags);
                set_mn(&tr, "DCR %c", REG_NAMES[ddd]);
                break;
            }
            case 2: { /* rotates or OUT */
                uint8_t a = s->regs[7], r;
                int cy;
                switch (ddd) {
                    case 0:
                        cy = (a >> 7) & 1;
                        r = (uint8_t)((a << 1) | ((a >> 7) & 1));
                        s->regs[7] = r;
                        s->flags.carry = cy;
                        set_mn(&tr, "RLC");
                        break;
                    case 1:
                        cy = a & 1;
                        r = (uint8_t)((a >> 1) | ((a & 1) << 7));
                        s->regs[7] = r;
                        s->flags.carry = cy;
                        set_mn(&tr, "RRC");
                        break;
                    case 2:
                        cy = (a >> 7) & 1;
                        r = (uint8_t)((a << 1) | (s->flags.carry ? 1 : 0));
                        s->regs[7] = r;
                        s->flags.carry = cy;
                        set_mn(&tr, "RAL");
                        break;
                    case 3:
                        cy = a & 1;
                        r = (uint8_t)((s->flags.carry ? 0x80 : 0) | (a >> 1));
                        s->regs[7] = r;
                        s->flags.carry = cy;
                        set_mn(&tr, "RAR");
                        break;
                    default: {
                        size_t port = (opcode >> 1) & 0x1F;
                        if (port < 24) {
                            s->output_ports[port] = s->regs[7];
                        }
                        set_mn(&tr, "OUT %zu", port);
                        break;
                    }
                }
                break;
            }
            case 3: { /* return if false */
                uint8_t ccc = ddd & 0x03;
                set_mn(&tr, "RF%c", cond_letter(ccc));
                if (condition_met(s, ccc, 0)) {
                    pop_return(s);
                }
                break;
            }
            case 5: { /* RST */
                uint16_t target = (uint16_t)(ddd << 3);
                set_mn(&tr, "RST %u", (unsigned)ddd);
                push_and_jump(s, target);
                break;
            }
            case 6: { /* MVI */
                uint8_t data = fetch_byte(s);
                tr.raw[tr.raw_len++] = data;
                if (ddd == 6) {
                    uint16_t addr = i8008_hl_address(s);
                    mem_write(s, addr, data);
                    tr.has_mem_address = 1;
                    tr.mem_address = addr;
                    tr.has_mem_value = 1;
                    tr.mem_value = data;
                    set_mn(&tr, "MVI M, 0x%02X", data);
                } else {
                    s->regs[ddd] = data;
                    set_mn(&tr, "MVI %c, 0x%02X", REG_NAMES[ddd], data);
                }
                break;
            }
            case 7: { /* return if true / RET */
                if (opcode == 0x3F) {
                    pop_return(s);
                    set_mn(&tr, "RET");
                } else {
                    uint8_t ccc = ddd & 0x03;
                    set_mn(&tr, "RT%c", cond_letter(ccc));
                    if (condition_met(s, ccc, 1)) {
                        pop_return(s);
                    }
                }
                break;
            }
            default:
                if (out_opt) {
                    *out_opt = tr;
                }
                return 0; /* unknown opcode */
        }
    } else if (group == 1) {
        if (opcode == 0x76) {
            s->halted = 1;
            set_mn(&tr, "HLT");
        } else if (opcode == 0x7C) { /* JMP */
            uint8_t lo = fetch_byte(s), hi = fetch_byte(s);
            uint16_t target;
            tr.raw[tr.raw_len++] = lo;
            tr.raw[tr.raw_len++] = hi;
            target = (uint16_t)((((uint16_t)hi & 0x3F) << 8) | lo);
            s->stack[0] = target;
            set_mn(&tr, "JMP 0x%04X", target);
        } else if (opcode == 0x7E) { /* CAL */
            uint8_t lo = fetch_byte(s), hi = fetch_byte(s);
            uint16_t target;
            tr.raw[tr.raw_len++] = lo;
            tr.raw[tr.raw_len++] = hi;
            target = (uint16_t)((((uint16_t)hi & 0x3F) << 8) | lo);
            push_and_jump(s, target);
            set_mn(&tr, "CAL 0x%04X", target);
        } else if (sss == 1) { /* IN */
            size_t port = ddd;
            s->regs[7] = s->input_ports[port < 7 ? port : 7];
            set_mn(&tr, "IN %zu", port);
        } else if ((sss == 0 || sss == 4) && ddd <= 3) { /* cond jump */
            uint8_t lo = fetch_byte(s), hi = fetch_byte(s);
            uint16_t target;
            int sense = sss == 4;
            tr.raw[tr.raw_len++] = lo;
            tr.raw[tr.raw_len++] = hi;
            target = (uint16_t)((((uint16_t)hi & 0x3F) << 8) | lo);
            set_mn(&tr, "J%c%c 0x%04X", sense ? 'T' : 'F', cond_letter(ddd),
                   target);
            if (condition_met(s, ddd, sense)) {
                s->stack[0] = target;
            }
        } else if ((sss == 2 || sss == 6) && ddd <= 3) { /* cond call */
            uint8_t lo = fetch_byte(s), hi = fetch_byte(s);
            uint16_t target;
            int sense = sss == 6;
            tr.raw[tr.raw_len++] = lo;
            tr.raw[tr.raw_len++] = hi;
            target = (uint16_t)((((uint16_t)hi & 0x3F) << 8) | lo);
            set_mn(&tr, "C%c%c 0x%04X", sense ? 'T' : 'F', cond_letter(ddd),
                   target);
            if (condition_met(s, ddd, sense)) {
                push_and_jump(s, target);
            }
        } else { /* MOV */
            uint8_t src_val = reg_read(s, sss);
            if (sss == 6) {
                tr.has_mem_address = 1;
                tr.mem_address = i8008_hl_address(s);
                tr.has_mem_value = 1;
                tr.mem_value = src_val;
            }
            if (ddd == 6) {
                uint16_t addr = i8008_hl_address(s);
                mem_write(s, addr, src_val);
                tr.has_mem_address = 1;
                tr.mem_address = addr;
                tr.has_mem_value = 1;
                tr.mem_value = src_val;
            } else {
                s->regs[ddd] = src_val;
            }
            set_mn(&tr, "MOV %c, %c", REG_NAMES[ddd], REG_NAMES[sss]);
        }
    } else if (group == 2) { /* ALU register */
        uint8_t src_val = reg_read(s, sss), result;
        int carry, clear_carry;
        if (sss == 6) {
            tr.has_mem_address = 1;
            tr.mem_address = i8008_hl_address(s);
            tr.has_mem_value = 1;
            tr.mem_value = src_val;
        }
        alu_op(ddd, s->regs[7], src_val, s->flags.carry, &result, &carry,
               &clear_carry);
        s->flags = compute_flags(result, clear_carry ? 0 : carry, 1, s->flags);
        if (ddd != 7) {
            s->regs[7] = result;
        }
        set_mn(&tr, "%.3s %c", ALU_MNEM[ddd], REG_NAMES[sss]);
    } else { /* group 3: ALU immediate / HLT */
        if (opcode == 0xFF) {
            s->halted = 1;
            set_mn(&tr, "HLT");
        } else if (sss == 4) {
            uint8_t data = fetch_byte(s), result;
            int carry, clear_carry;
            tr.raw[tr.raw_len++] = data;
            alu_op(ddd, s->regs[7], data, s->flags.carry, &result, &carry,
                   &clear_carry);
            s->flags =
                compute_flags(result, clear_carry ? 0 : carry, 1, s->flags);
            if (ddd != 7) {
                s->regs[7] = result;
            }
            set_mn(&tr, "%.3s 0x%02X", ALU_IMM_MNEM[ddd], data);
        } else {
            if (out_opt) {
                *out_opt = tr;
            }
            return 0; /* unknown opcode */
        }
    }

    tr.address = fetch_pc;
    tr.a_before = a_before;
    tr.a_after = s->regs[7];
    tr.flags_before = flags_before;
    tr.flags_after = s->flags;
    if (out_opt) {
        *out_opt = tr;
    }
    return 1;
}

/* Grow guard for the trace vector. */
static int grow_traces(I8008Sim *s) {
    size_t nc;
    I8008Trace *nd;
    if (s->traces_len < s->traces_cap) {
        return 1;
    }
    nc = s->traces_cap ? s->traces_cap : 8;
    if (nc > (size_t)-1 / 2) {
        return 0;
    }
    nc *= 2;
    if (nc > (size_t)-1 / sizeof(I8008Trace)) {
        return 0;
    }
    nd = (I8008Trace *)realloc(s->traces, nc * sizeof(I8008Trace));
    if (!nd) {
        return 0;
    }
    s->traces = nd;
    s->traces_cap = nc;
    return 1;
}

size_t i8008_run(I8008Sim *s, const uint8_t *program, size_t len,
                 size_t max_steps) {
    size_t i;
    i8008_reset(s);
    i8008_load_program(s, program, len, 0);
    s->traces_len = 0;
    for (i = 0; i < max_steps; i++) {
        I8008Trace t;
        int halted;
        if (!i8008_step(s, &t)) {
            break;
        }
        halted = s->halted;
        if (!grow_traces(s)) {
            break; /* OOM: stop recording, keep what we have */
        }
        s->traces[s->traces_len++] = t;
        if (halted) {
            break;
        }
    }
    return s->traces_len;
}
