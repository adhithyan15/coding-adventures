/*
 * arm1_simulator.c — ARM1 behavioral CPU simulator, pure ISO C17.
 * =====================================================================
 *
 * See arm1_simulator.h. A faithful port of the Rust `arm1-simulator` crate:
 * condition evaluation, the barrel shifter, the 16-op ALU, the PLA-style
 * decoder, disassembly, and the fetch-decode-execute loop with a 27-register
 * banked file and byte-addressable little-endian memory.
 */
#include "arm1_simulator.h"

#include <stdio.h>  /* snprintf */
#include <stdlib.h> /* calloc, free */
#include <string.h> /* memcpy, strcmp, strlen */

/* ── Small bounded string builder (avoids format-truncation on %s) ──────────*/
typedef struct {
    char *buf;
    size_t cap;
    size_t pos;
} SB;

static void sb_init(SB *s, char *buf, size_t cap) {
    s->buf = buf;
    s->cap = cap;
    s->pos = 0;
    if (cap > 0) {
        buf[0] = '\0';
    }
}
static void sb_str(SB *s, const char *str) {
    while (*str && s->pos + 1 < s->cap) {
        s->buf[s->pos++] = *str++;
    }
    if (s->cap > 0) {
        s->buf[s->pos] = '\0';
    }
}
static void sb_u32(SB *s, uint32_t v) {
    char tmp[12];
    snprintf(tmp, sizeof tmp, "%lu", (unsigned long)v);
    sb_str(s, tmp);
}
static void sb_i32(SB *s, int32_t v) {
    char tmp[12];
    snprintf(tmp, sizeof tmp, "%ld", (long)v);
    sb_str(s, tmp);
}
static void sb_hex(SB *s, uint32_t v) {
    char tmp[12];
    snprintf(tmp, sizeof tmp, "%lX", (unsigned long)v);
    sb_str(s, tmp);
}
static void sb_reg(SB *s, size_t r) {
    sb_str(s, "R");
    sb_u32(s, (uint32_t)r);
}

/* ── Enum → string helpers ──────────────────────────────────────────────────*/
const char *arm1_mode_string(uint32_t mode) {
    switch (mode) {
    case ARM1_MODE_USR:
        return "USR";
    case ARM1_MODE_FIQ:
        return "FIQ";
    case ARM1_MODE_IRQ:
        return "IRQ";
    case ARM1_MODE_SVC:
        return "SVC";
    default:
        return "???";
    }
}
const char *arm1_cond_string(uint32_t cond) {
    static const char *N[16] = {"EQ", "NE", "CS", "CC", "MI", "PL",
                                "VS", "VC", "HI", "LS", "GE", "LT",
                                "GT", "LE", "", "NV"};
    return cond < 16 ? N[cond] : "??";
}
const char *arm1_op_string(uint32_t opcode) {
    static const char *N[16] = {"AND", "EOR", "SUB", "RSB", "ADD", "ADC",
                                "SBC", "RSC", "TST", "TEQ", "CMP", "CMN",
                                "ORR", "MOV", "BIC", "MVN"};
    return opcode < 16 ? N[opcode] : "???";
}
int arm1_is_test_op(uint32_t opcode) {
    return opcode >= ARM1_OP_TST && opcode <= ARM1_OP_CMN;
}
int arm1_is_logical_op(uint32_t opcode) {
    return opcode == ARM1_OP_AND || opcode == ARM1_OP_EOR ||
           opcode == ARM1_OP_TST || opcode == ARM1_OP_TEQ ||
           opcode == ARM1_OP_ORR || opcode == ARM1_OP_MOV ||
           opcode == ARM1_OP_BIC || opcode == ARM1_OP_MVN;
}
const char *arm1_shift_string(uint32_t shift_type) {
    switch (shift_type) {
    case ARM1_SHIFT_LSL:
        return "LSL";
    case ARM1_SHIFT_LSR:
        return "LSR";
    case ARM1_SHIFT_ASR:
        return "ASR";
    case ARM1_SHIFT_ROR:
        return "ROR";
    default:
        return "???";
    }
}

/* ── Condition evaluator ────────────────────────────────────────────────────*/
int arm1_evaluate_condition(uint32_t cond, Arm1Flags f) {
    switch (cond) {
    case ARM1_COND_EQ:
        return f.z;
    case ARM1_COND_NE:
        return !f.z;
    case ARM1_COND_CS:
        return f.c;
    case ARM1_COND_CC:
        return !f.c;
    case ARM1_COND_MI:
        return f.n;
    case ARM1_COND_PL:
        return !f.n;
    case ARM1_COND_VS:
        return f.v;
    case ARM1_COND_VC:
        return !f.v;
    case ARM1_COND_HI:
        return f.c && !f.z;
    case ARM1_COND_LS:
        return !f.c || f.z;
    case ARM1_COND_GE:
        return (f.n != 0) == (f.v != 0);
    case ARM1_COND_LT:
        return (f.n != 0) != (f.v != 0);
    case ARM1_COND_GT:
        return !f.z && ((f.n != 0) == (f.v != 0));
    case ARM1_COND_LE:
        return f.z || ((f.n != 0) != (f.v != 0));
    case ARM1_COND_AL:
        return 1;
    case ARM1_COND_NV:
        return 0;
    default:
        return 0;
    }
}

/* ── Barrel shifter ─────────────────────────────────────────────────────────*/
static uint32_t rotr32(uint32_t v, uint32_t amount) {
    amount &= 31;
    if (amount == 0) {
        return v;
    }
    return (v >> amount) | (v << (32 - amount));
}

static uint32_t shift_lsl(uint32_t value, uint32_t amount, int carry_in,
                          int *carry_out) {
    if (amount == 0) {
        *carry_out = carry_in;
        return value;
    }
    if (amount >= 32) {
        if (amount == 32) {
            *carry_out = (value & 1) != 0;
        } else {
            *carry_out = 0;
        }
        return 0;
    }
    *carry_out = ((value >> (32 - amount)) & 1) != 0;
    return value << amount;
}
static uint32_t shift_lsr(uint32_t value, uint32_t amount, int carry_in,
                          int by_register, int *carry_out) {
    if (amount == 0 && !by_register) {
        *carry_out = (value >> 31) != 0;
        return 0;
    }
    if (amount == 0) {
        *carry_out = carry_in;
        return value;
    }
    if (amount >= 32) {
        *carry_out = (amount == 32) ? ((value >> 31) != 0) : 0;
        return 0;
    }
    *carry_out = ((value >> (amount - 1)) & 1) != 0;
    return value >> amount;
}
static uint32_t shift_asr(uint32_t value, uint32_t amount, int carry_in,
                          int by_register, int *carry_out) {
    int sign_bit = (value >> 31) != 0;
    if (amount == 0 && !by_register) {
        if (sign_bit) {
            *carry_out = 1;
            return 0xFFFFFFFFu;
        }
        *carry_out = 0;
        return 0;
    }
    if (amount == 0) {
        *carry_out = carry_in;
        return value;
    }
    if (amount >= 32) {
        if (sign_bit) {
            *carry_out = 1;
            return 0xFFFFFFFFu;
        }
        *carry_out = 0;
        return 0;
    }
    {
        int32_t signed_v = (int32_t)value;
        uint32_t result = (uint32_t)(signed_v >> amount);
        *carry_out = ((value >> (amount - 1)) & 1) != 0;
        return result;
    }
}
static uint32_t shift_ror(uint32_t value, uint32_t amount, int carry_in,
                          int by_register, int *carry_out) {
    if (amount == 0 && !by_register) {
        /* RRX — rotate right through carry. */
        int carry = (value & 1) != 0;
        uint32_t result = value >> 1;
        if (carry_in) {
            result |= 0x80000000u;
        }
        *carry_out = carry;
        return result;
    }
    if (amount == 0) {
        *carry_out = carry_in;
        return value;
    }
    amount &= 31;
    if (amount == 0) {
        *carry_out = (value >> 31) != 0;
        return value;
    }
    {
        uint32_t result = rotr32(value, amount);
        *carry_out = ((result >> 31) & 1) != 0;
        return result;
    }
}

uint32_t arm1_barrel_shift(uint32_t value, uint32_t shift_type, uint32_t amount,
                           int carry_in, int by_register, int *carry_out) {
    if (by_register && amount == 0) {
        *carry_out = carry_in;
        return value;
    }
    switch (shift_type) {
    case ARM1_SHIFT_LSL:
        return shift_lsl(value, amount, carry_in, carry_out);
    case ARM1_SHIFT_LSR:
        return shift_lsr(value, amount, carry_in, by_register, carry_out);
    case ARM1_SHIFT_ASR:
        return shift_asr(value, amount, carry_in, by_register, carry_out);
    case ARM1_SHIFT_ROR:
        return shift_ror(value, amount, carry_in, by_register, carry_out);
    default:
        *carry_out = carry_in;
        return value;
    }
}

uint32_t arm1_decode_immediate(uint32_t imm8, uint32_t rotate, int *carry_out) {
    uint32_t rotate_amount = rotate * 2;
    uint32_t value;
    if (rotate_amount == 0) {
        *carry_out = 0;
        return imm8;
    }
    value = rotr32(imm8, rotate_amount);
    *carry_out = (value >> 31) != 0;
    return value;
}

/* ── ALU ────────────────────────────────────────────────────────────────────*/
static uint32_t add32(uint32_t a, uint32_t b, int carry_in, int *carry_out,
                      int *overflow_out) {
    uint64_t cin = carry_in ? 1 : 0;
    uint64_t sum = (uint64_t)a + (uint64_t)b + cin;
    uint32_t result = (uint32_t)sum;
    *carry_out = (sum >> 32) != 0;
    *overflow_out = ((((a ^ result) & (b ^ result)) >> 31) & 1) != 0;
    return result;
}

Arm1ALUResult arm1_alu_execute(uint32_t opcode, uint32_t a, uint32_t b,
                               int carry_in, int shifter_carry, int old_v) {
    Arm1ALUResult r;
    uint32_t result = 0;
    int carry = 0, overflow = 0;
    switch (opcode) {
    case ARM1_OP_AND:
    case ARM1_OP_TST:
        result = a & b;
        carry = shifter_carry;
        overflow = old_v;
        break;
    case ARM1_OP_EOR:
    case ARM1_OP_TEQ:
        result = a ^ b;
        carry = shifter_carry;
        overflow = old_v;
        break;
    case ARM1_OP_ORR:
        result = a | b;
        carry = shifter_carry;
        overflow = old_v;
        break;
    case ARM1_OP_MOV:
        result = b;
        carry = shifter_carry;
        overflow = old_v;
        break;
    case ARM1_OP_BIC:
        result = a & ~b;
        carry = shifter_carry;
        overflow = old_v;
        break;
    case ARM1_OP_MVN:
        result = ~b;
        carry = shifter_carry;
        overflow = old_v;
        break;
    case ARM1_OP_ADD:
    case ARM1_OP_CMN:
        result = add32(a, b, 0, &carry, &overflow);
        break;
    case ARM1_OP_ADC:
        result = add32(a, b, carry_in, &carry, &overflow);
        break;
    case ARM1_OP_SUB:
    case ARM1_OP_CMP:
        result = add32(a, ~b, 1, &carry, &overflow);
        break;
    case ARM1_OP_SBC:
        result = add32(a, ~b, carry_in, &carry, &overflow);
        break;
    case ARM1_OP_RSB:
        result = add32(b, ~a, 1, &carry, &overflow);
        break;
    case ARM1_OP_RSC:
        result = add32(b, ~a, carry_in, &carry, &overflow);
        break;
    default:
        result = 0;
        carry = 0;
        overflow = 0;
        break;
    }
    r.result = result;
    r.n = (result >> 31) != 0;
    r.z = result == 0;
    r.c = carry;
    r.v = overflow;
    r.write_result = !arm1_is_test_op(opcode);
    return r;
}

/* ── Decoder ────────────────────────────────────────────────────────────────*/
static void decode_dp(Arm1DecodedInstruction *d, uint32_t inst) {
    d->immediate = ((inst >> 25) & 1) == 1;
    d->opcode = (inst >> 21) & 0xF;
    d->s = ((inst >> 20) & 1) == 1;
    d->rn = (inst >> 16) & 0xF;
    d->rd = (inst >> 12) & 0xF;
    if (d->immediate) {
        d->imm8 = inst & 0xFF;
        d->rotate = (inst >> 8) & 0xF;
    } else {
        d->rm = inst & 0xF;
        d->shift_type = (inst >> 5) & 0x3;
        d->shift_by_reg = ((inst >> 4) & 1) == 1;
        if (d->shift_by_reg) {
            d->rs = (inst >> 8) & 0xF;
        } else {
            d->shift_imm = (inst >> 7) & 0x1F;
        }
    }
}
static void decode_ls(Arm1DecodedInstruction *d, uint32_t inst) {
    d->immediate = ((inst >> 25) & 1) == 1;
    d->pre_index = ((inst >> 24) & 1) == 1;
    d->up = ((inst >> 23) & 1) == 1;
    d->byte = ((inst >> 22) & 1) == 1;
    d->write_back = ((inst >> 21) & 1) == 1;
    d->load = ((inst >> 20) & 1) == 1;
    d->rn = (inst >> 16) & 0xF;
    d->rd = (inst >> 12) & 0xF;
    if (d->immediate) {
        d->rm = inst & 0xF;
        d->shift_type = (inst >> 5) & 0x3;
        d->shift_imm = (inst >> 7) & 0x1F;
    } else {
        d->offset12 = inst & 0xFFF;
    }
}
static void decode_bt(Arm1DecodedInstruction *d, uint32_t inst) {
    d->pre_index = ((inst >> 24) & 1) == 1;
    d->up = ((inst >> 23) & 1) == 1;
    d->force_user = ((inst >> 22) & 1) == 1;
    d->write_back = ((inst >> 21) & 1) == 1;
    d->load = ((inst >> 20) & 1) == 1;
    d->rn = (inst >> 16) & 0xF;
    d->register_list = (uint16_t)(inst & 0xFFFF);
}
static void decode_br(Arm1DecodedInstruction *d, uint32_t inst) {
    uint32_t offset = inst & 0x00FFFFFF;
    d->link = ((inst >> 24) & 1) == 1;
    if ((offset >> 23) != 0) {
        offset |= 0xFF000000u;
    }
    d->branch_offset = (int32_t)(offset << 2);
}

Arm1DecodedInstruction arm1_decode(uint32_t instruction) {
    Arm1DecodedInstruction d;
    uint32_t bits2726, bit25;
    memset(&d, 0, sizeof d);
    d.raw = instruction;
    d.inst_type = ARM1_INST_UNDEFINED;
    d.cond = (instruction >> 28) & 0xF;

    bits2726 = (instruction >> 26) & 0x3;
    bit25 = (instruction >> 25) & 0x1;

    if (bits2726 == 0) {
        d.inst_type = ARM1_INST_DATA_PROCESSING;
        decode_dp(&d, instruction);
    } else if (bits2726 == 1) {
        d.inst_type = ARM1_INST_LOAD_STORE;
        decode_ls(&d, instruction);
    } else if (bits2726 == 2 && bit25 == 0) {
        d.inst_type = ARM1_INST_BLOCK_TRANSFER;
        decode_bt(&d, instruction);
    } else if (bits2726 == 2 && bit25 == 1) {
        d.inst_type = ARM1_INST_BRANCH;
        decode_br(&d, instruction);
    } else if (bits2726 == 3) {
        if (((instruction >> 24) & 0xF) == 0xF) {
            d.inst_type = ARM1_INST_SWI;
            d.swi_comment = instruction & 0x00FFFFFF;
        } else {
            d.inst_type = ARM1_INST_COPROCESSOR;
        }
    }
    return d;
}

/* ── Disassembly ────────────────────────────────────────────────────────────*/
static void disasm_operand2(const Arm1DecodedInstruction *d, SB *s) {
    if (d->immediate) {
        int c;
        uint32_t val = arm1_decode_immediate(d->imm8, d->rotate, &c);
        sb_str(s, "#");
        sb_u32(s, val);
        return;
    }
    if (!d->shift_by_reg && d->shift_imm == 0 &&
        d->shift_type == ARM1_SHIFT_LSL) {
        sb_reg(s, d->rm);
        return;
    }
    if (d->shift_by_reg) {
        sb_reg(s, d->rm);
        sb_str(s, ", ");
        sb_str(s, arm1_shift_string(d->shift_type));
        sb_str(s, " ");
        sb_reg(s, d->rs);
        return;
    }
    {
        uint32_t amount = d->shift_imm;
        if (amount == 0) {
            if (d->shift_type == ARM1_SHIFT_LSR ||
                d->shift_type == ARM1_SHIFT_ASR) {
                amount = 32;
            } else if (d->shift_type == ARM1_SHIFT_ROR) {
                sb_reg(s, d->rm);
                sb_str(s, ", RRX");
                return;
            }
        }
        sb_reg(s, d->rm);
        sb_str(s, ", ");
        sb_str(s, arm1_shift_string(d->shift_type));
        sb_str(s, " #");
        sb_u32(s, amount);
    }
}

static void disasm_reg_list(uint16_t list, SB *s) {
    int i, first = 1;
    for (i = 0; i < 16; i++) {
        if (((list >> i) & 1) == 0) {
            continue;
        }
        if (!first) {
            sb_str(s, ", ");
        }
        first = 0;
        if (i == 15) {
            sb_str(s, "PC");
        } else if (i == 14) {
            sb_str(s, "LR");
        } else if (i == 13) {
            sb_str(s, "SP");
        } else {
            sb_str(s, "R");
            sb_u32(s, (uint32_t)i);
        }
    }
}

void arm1_disassemble(const Arm1DecodedInstruction *d, char *out, size_t n) {
    SB s;
    const char *cond = arm1_cond_string(d->cond);
    sb_init(&s, out, n);

    switch (d->inst_type) {
    case ARM1_INST_DATA_PROCESSING: {
        const char *op = arm1_op_string(d->opcode);
        int suf = d->s && !arm1_is_test_op(d->opcode);
        if (d->opcode == ARM1_OP_MOV || d->opcode == ARM1_OP_MVN) {
            sb_str(&s, op);
            sb_str(&s, cond);
            if (suf) {
                sb_str(&s, "S");
            }
            sb_str(&s, " ");
            sb_reg(&s, d->rd);
            sb_str(&s, ", ");
            disasm_operand2(d, &s);
        } else if (arm1_is_test_op(d->opcode)) {
            sb_str(&s, op);
            sb_str(&s, cond);
            sb_str(&s, " ");
            sb_reg(&s, d->rn);
            sb_str(&s, ", ");
            disasm_operand2(d, &s);
        } else {
            sb_str(&s, op);
            sb_str(&s, cond);
            if (suf) {
                sb_str(&s, "S");
            }
            sb_str(&s, " ");
            sb_reg(&s, d->rd);
            sb_str(&s, ", ");
            sb_reg(&s, d->rn);
            sb_str(&s, ", ");
            disasm_operand2(d, &s);
        }
        break;
    }
    case ARM1_INST_LOAD_STORE: {
        sb_str(&s, d->load ? "LDR" : "STR");
        sb_str(&s, cond);
        if (d->byte) {
            sb_str(&s, "B");
        }
        sb_str(&s, " ");
        sb_reg(&s, d->rd);
        sb_str(&s, ", [");
        sb_reg(&s, d->rn);
        if (d->pre_index) {
            sb_str(&s, ", ");
            if (!d->up) {
                sb_str(&s, "-");
            }
            if (d->immediate) {
                sb_reg(&s, d->rm);
                if (d->shift_imm != 0) {
                    sb_str(&s, ", ");
                    sb_str(&s, arm1_shift_string(d->shift_type));
                    sb_str(&s, " #");
                    sb_u32(&s, d->shift_imm);
                }
            } else {
                sb_str(&s, "#");
                sb_u32(&s, d->offset12);
            }
            sb_str(&s, "]");
            if (d->write_back) {
                sb_str(&s, "!");
            }
        } else {
            sb_str(&s, "], ");
            if (!d->up) {
                sb_str(&s, "-");
            }
            if (d->immediate) {
                sb_reg(&s, d->rm);
                if (d->shift_imm != 0) {
                    sb_str(&s, ", ");
                    sb_str(&s, arm1_shift_string(d->shift_type));
                    sb_str(&s, " #");
                    sb_u32(&s, d->shift_imm);
                }
            } else {
                sb_str(&s, "#");
                sb_u32(&s, d->offset12);
            }
        }
        break;
    }
    case ARM1_INST_BLOCK_TRANSFER: {
        const char *mode = d->pre_index ? (d->up ? "IB" : "DB")
                                        : (d->up ? "IA" : "DA");
        sb_str(&s, d->load ? "LDM" : "STM");
        sb_str(&s, cond);
        sb_str(&s, mode);
        sb_str(&s, " ");
        sb_reg(&s, d->rn);
        if (d->write_back) {
            sb_str(&s, "!");
        }
        sb_str(&s, ", {");
        disasm_reg_list(d->register_list, &s);
        sb_str(&s, "}");
        break;
    }
    case ARM1_INST_BRANCH:
        sb_str(&s, d->link ? "BL" : "B");
        sb_str(&s, cond);
        sb_str(&s, " #");
        sb_i32(&s, d->branch_offset);
        break;
    case ARM1_INST_SWI:
        if (d->swi_comment == ARM1_HALT_SWI) {
            sb_str(&s, "HLT");
            sb_str(&s, cond);
        } else {
            sb_str(&s, "SWI");
            sb_str(&s, cond);
            sb_str(&s, " #0x");
            sb_hex(&s, d->swi_comment);
        }
        break;
    case ARM1_INST_COPROCESSOR:
        sb_str(&s, "CDP");
        sb_str(&s, cond);
        sb_str(&s, " (undefined)");
        break;
    case ARM1_INST_UNDEFINED:
    default: {
        char tmp[12];
        sb_str(&s, "UND");
        sb_str(&s, cond);
        sb_str(&s, " #0x");
        snprintf(tmp, sizeof tmp, "%08lX", (unsigned long)d->raw);
        sb_str(&s, tmp);
        break;
    }
    }
}

/* ── CPU ────────────────────────────────────────────────────────────────────*/
struct ARM1 {
    uint32_t regs[27];
    uint8_t *memory;
    size_t memory_size;
    int halted;
};

ARM1 *arm1_new(size_t memory_size) {
    ARM1 *cpu;
    if (memory_size == 0) {
        memory_size = 1024 * 1024;
    }
    cpu = (ARM1 *)calloc(1, sizeof(ARM1));
    if (!cpu) {
        return NULL;
    }
    cpu->memory = (uint8_t *)calloc(memory_size, 1);
    if (!cpu->memory) {
        free(cpu);
        return NULL;
    }
    cpu->memory_size = memory_size;
    arm1_reset(cpu);
    return cpu;
}
void arm1_free(ARM1 *cpu) {
    if (cpu) {
        free(cpu->memory);
        free(cpu);
    }
}
void arm1_reset(ARM1 *cpu) {
    memset(cpu->regs, 0, sizeof cpu->regs);
    cpu->regs[15] = ARM1_FLAG_I | ARM1_FLAG_F | ARM1_MODE_SVC;
    cpu->halted = 0;
}

uint32_t arm1_mode(const ARM1 *cpu) { return cpu->regs[15] & ARM1_MODE_MASK; }

static size_t physical_reg(const ARM1 *cpu, size_t index) {
    uint32_t mode = arm1_mode(cpu);
    if (mode == ARM1_MODE_FIQ && index >= 8 && index <= 14) {
        return 16 + (index - 8);
    }
    if (mode == ARM1_MODE_IRQ && index >= 13 && index <= 14) {
        return 23 + (index - 13);
    }
    if (mode == ARM1_MODE_SVC && index >= 13 && index <= 14) {
        return 25 + (index - 13);
    }
    return index;
}

uint32_t arm1_read_register(const ARM1 *cpu, size_t index) {
    if (index > 15) {
        return 0; /* only R0-R15 are addressable (Rust would panic) */
    }
    return cpu->regs[physical_reg(cpu, index)];
}
void arm1_write_register(ARM1 *cpu, size_t index, uint32_t value) {
    if (index > 15) {
        return;
    }
    cpu->regs[physical_reg(cpu, index)] = value;
}
uint32_t arm1_pc(const ARM1 *cpu) { return cpu->regs[15] & ARM1_PC_MASK; }
void arm1_set_pc(ARM1 *cpu, uint32_t addr) {
    cpu->regs[15] = (cpu->regs[15] & ~ARM1_PC_MASK) | (addr & ARM1_PC_MASK);
}
Arm1Flags arm1_flags(const ARM1 *cpu) {
    uint32_t r15 = cpu->regs[15];
    Arm1Flags f;
    f.n = (r15 & ARM1_FLAG_N) != 0;
    f.z = (r15 & ARM1_FLAG_Z) != 0;
    f.c = (r15 & ARM1_FLAG_C) != 0;
    f.v = (r15 & ARM1_FLAG_V) != 0;
    return f;
}
void arm1_set_flags(ARM1 *cpu, Arm1Flags f) {
    uint32_t r15 =
        cpu->regs[15] & ~(ARM1_FLAG_N | ARM1_FLAG_Z | ARM1_FLAG_C | ARM1_FLAG_V);
    if (f.n) {
        r15 |= ARM1_FLAG_N;
    }
    if (f.z) {
        r15 |= ARM1_FLAG_Z;
    }
    if (f.c) {
        r15 |= ARM1_FLAG_C;
    }
    if (f.v) {
        r15 |= ARM1_FLAG_V;
    }
    cpu->regs[15] = r15;
}
int arm1_halted(const ARM1 *cpu) { return cpu->halted; }
uint32_t arm1_r15_raw(const ARM1 *cpu) { return cpu->regs[15]; }

uint32_t arm1_read_word(const ARM1 *cpu, uint32_t addr) {
    size_t a = (size_t)(addr & ARM1_PC_MASK) & ~(size_t)3;
    if (a + 3 >= cpu->memory_size) {
        return 0;
    }
    return (uint32_t)cpu->memory[a] | ((uint32_t)cpu->memory[a + 1] << 8) |
           ((uint32_t)cpu->memory[a + 2] << 16) |
           ((uint32_t)cpu->memory[a + 3] << 24);
}
void arm1_write_word(ARM1 *cpu, uint32_t addr, uint32_t value) {
    size_t a = (size_t)(addr & ARM1_PC_MASK) & ~(size_t)3;
    if (a + 3 >= cpu->memory_size) {
        return;
    }
    cpu->memory[a] = (uint8_t)value;
    cpu->memory[a + 1] = (uint8_t)(value >> 8);
    cpu->memory[a + 2] = (uint8_t)(value >> 16);
    cpu->memory[a + 3] = (uint8_t)(value >> 24);
}
uint8_t arm1_read_byte(const ARM1 *cpu, uint32_t addr) {
    size_t a = (size_t)(addr & ARM1_PC_MASK);
    return a < cpu->memory_size ? cpu->memory[a] : 0;
}
void arm1_write_byte(ARM1 *cpu, uint32_t addr, uint8_t value) {
    size_t a = (size_t)(addr & ARM1_PC_MASK);
    if (a < cpu->memory_size) {
        cpu->memory[a] = value;
    }
}
void arm1_load_program(ARM1 *cpu, const uint8_t *code, size_t len,
                       uint32_t start_addr) {
    size_t i;
    for (i = 0; i < len; i++) {
        size_t addr = (size_t)start_addr + i;
        if (addr < cpu->memory_size) {
            cpu->memory[addr] = code[i];
        }
    }
}
void arm1_load_program_words(ARM1 *cpu, const uint32_t *instructions,
                             size_t count, uint32_t start_addr) {
    size_t i;
    for (i = 0; i < count; i++) {
        uint32_t inst = instructions[i];
        uint8_t b[4];
        b[0] = (uint8_t)inst;
        b[1] = (uint8_t)(inst >> 8);
        b[2] = (uint8_t)(inst >> 16);
        b[3] = (uint8_t)(inst >> 24);
        arm1_load_program(cpu, b, 4, start_addr + (uint32_t)(i * 4));
    }
}

/* R15 reads as PC + 8; step() already advanced PC by 4, so add 4 more. */
static uint32_t read_reg_for_exec(const ARM1 *cpu, size_t index) {
    if (index == 15) {
        return cpu->regs[15] + 4;
    }
    return arm1_read_register(cpu, index);
}

static void exec_dp(ARM1 *cpu, const Arm1DecodedInstruction *d) {
    uint32_t a = (d->opcode != ARM1_OP_MOV && d->opcode != ARM1_OP_MVN)
                     ? read_reg_for_exec(cpu, d->rn)
                     : 0;
    Arm1Flags flags = arm1_flags(cpu);
    uint32_t b;
    int shifter_carry;
    Arm1ALUResult result;

    if (d->immediate) {
        int c;
        uint32_t val = arm1_decode_immediate(d->imm8, d->rotate, &c);
        b = val;
        shifter_carry = (d->rotate == 0) ? flags.c : c;
    } else {
        uint32_t rm_val = read_reg_for_exec(cpu, d->rm);
        uint32_t shift_amount = d->shift_by_reg
                                    ? (read_reg_for_exec(cpu, d->rs) & 0xFF)
                                    : d->shift_imm;
        b = arm1_barrel_shift(rm_val, d->shift_type, shift_amount, flags.c,
                              d->shift_by_reg, &shifter_carry);
    }

    result = arm1_alu_execute(d->opcode, a, b, flags.c, shifter_carry, flags.v);

    if (result.write_result) {
        if (d->rd == 15) {
            if (d->s) {
                cpu->regs[15] = result.result;
            } else {
                arm1_set_pc(cpu, result.result & ARM1_PC_MASK);
            }
        } else {
            arm1_write_register(cpu, d->rd, result.result);
        }
    }
    if (d->s && d->rd != 15) {
        Arm1Flags nf = {result.n, result.z, result.c, result.v};
        arm1_set_flags(cpu, nf);
    }
    if (arm1_is_test_op(d->opcode)) {
        Arm1Flags nf = {result.n, result.z, result.c, result.v};
        arm1_set_flags(cpu, nf);
    }
}

static void trace_push_read(Arm1Trace *t, uint32_t addr, uint32_t value) {
    if (t->memory_read_count < ARM1_MAX_MEM_ACCESS) {
        t->memory_reads[t->memory_read_count].address = addr;
        t->memory_reads[t->memory_read_count].value = value;
        t->memory_read_count++;
    }
}
static void trace_push_write(Arm1Trace *t, uint32_t addr, uint32_t value) {
    if (t->memory_write_count < ARM1_MAX_MEM_ACCESS) {
        t->memory_writes[t->memory_write_count].address = addr;
        t->memory_writes[t->memory_write_count].value = value;
        t->memory_write_count++;
    }
}

static void exec_ls(ARM1 *cpu, const Arm1DecodedInstruction *d, Arm1Trace *t) {
    uint32_t offset;
    uint32_t base, addr, transfer_addr;

    if (d->immediate) {
        uint32_t rm_val = read_reg_for_exec(cpu, d->rm);
        if (d->shift_imm != 0) {
            int c;
            rm_val = arm1_barrel_shift(rm_val, d->shift_type, d->shift_imm,
                                       arm1_flags(cpu).c, 0, &c);
        }
        offset = rm_val;
    } else {
        offset = d->offset12;
    }

    base = read_reg_for_exec(cpu, d->rn);
    addr = d->up ? (base + offset) : (base - offset);
    transfer_addr = d->pre_index ? addr : base;

    if (d->load) {
        uint32_t value;
        if (d->byte) {
            value = arm1_read_byte(cpu, transfer_addr);
        } else {
            uint32_t v = arm1_read_word(cpu, transfer_addr);
            uint32_t rotation = (transfer_addr & 3) * 8;
            if (rotation != 0) {
                v = rotr32(v, rotation);
            }
            value = v;
        }
        trace_push_read(t, transfer_addr, value);
        if (d->rd == 15) {
            cpu->regs[15] = value;
        } else {
            arm1_write_register(cpu, d->rd, value);
        }
    } else {
        uint32_t value = read_reg_for_exec(cpu, d->rd);
        if (d->byte) {
            arm1_write_byte(cpu, transfer_addr, (uint8_t)(value & 0xFF));
        } else {
            arm1_write_word(cpu, transfer_addr, value);
        }
        trace_push_write(t, transfer_addr, value);
    }

    if ((d->write_back || !d->pre_index) && d->rn != 15) {
        arm1_write_register(cpu, d->rn, addr);
    }
}

static void exec_bt(ARM1 *cpu, const Arm1DecodedInstruction *d, Arm1Trace *t) {
    uint32_t base = arm1_read_register(cpu, d->rn);
    uint16_t reg_list = d->register_list;
    uint32_t count = 0, start_addr, addr;
    int i;

    for (i = 0; i < 16; i++) {
        if ((reg_list >> i) & 1) {
            count++;
        }
    }
    if (count == 0) {
        return;
    }

    if (!d->pre_index && d->up) {
        start_addr = base; /* IA */
    } else if (d->pre_index && d->up) {
        start_addr = base + 4; /* IB */
    } else if (!d->pre_index && !d->up) {
        start_addr = base - (count * 4) + 4; /* DA */
    } else {
        start_addr = base - (count * 4); /* DB */
    }

    addr = start_addr;
    for (i = 0; i < 16; i++) {
        if (((reg_list >> i) & 1) == 0) {
            continue;
        }
        if (d->load) {
            uint32_t value = arm1_read_word(cpu, addr);
            trace_push_read(t, addr, value);
            if (i == 15) {
                cpu->regs[15] = value;
            } else {
                arm1_write_register(cpu, (size_t)i, value);
            }
        } else {
            uint32_t value = (i == 15) ? (cpu->regs[15] + 4)
                                       : arm1_read_register(cpu, (size_t)i);
            arm1_write_word(cpu, addr, value);
            trace_push_write(t, addr, value);
        }
        addr += 4;
    }

    if (d->write_back) {
        uint32_t new_base = d->up ? (base + count * 4) : (base - count * 4);
        arm1_write_register(cpu, d->rn, new_base);
    }
}

static void exec_branch(ARM1 *cpu, const Arm1DecodedInstruction *d) {
    uint32_t branch_base = arm1_pc(cpu) + 4;
    uint32_t target;
    if (d->link) {
        arm1_write_register(cpu, 14, cpu->regs[15]);
    }
    target = (uint32_t)((int32_t)branch_base + d->branch_offset);
    arm1_set_pc(cpu, target & ARM1_PC_MASK);
}

static void exec_swi(ARM1 *cpu, const Arm1DecodedInstruction *d) {
    uint32_t r15;
    if (d->swi_comment == ARM1_HALT_SWI) {
        cpu->halted = 1;
        return;
    }
    cpu->regs[25] = cpu->regs[15];
    cpu->regs[26] = cpu->regs[15];
    r15 = cpu->regs[15];
    r15 = (r15 & ~ARM1_MODE_MASK) | ARM1_MODE_SVC;
    r15 |= ARM1_FLAG_I;
    cpu->regs[15] = r15;
    arm1_set_pc(cpu, 0x08);
}

static void trap_undefined(ARM1 *cpu) {
    uint32_t r15;
    cpu->regs[26] = cpu->regs[15];
    r15 = cpu->regs[15];
    r15 = (r15 & ~ARM1_MODE_MASK) | ARM1_MODE_SVC;
    r15 |= ARM1_FLAG_I;
    cpu->regs[15] = r15;
    arm1_set_pc(cpu, 0x04);
}

void arm1_step(ARM1 *cpu, Arm1Trace *out) {
    Arm1Trace t;
    uint32_t pc = arm1_pc(cpu);
    uint32_t instruction;
    Arm1DecodedInstruction decoded;
    Arm1Flags flags_before = arm1_flags(cpu);
    int i;

    memset(&t, 0, sizeof t);
    for (i = 0; i < 16; i++) {
        t.regs_before[i] = arm1_read_register(cpu, (size_t)i);
    }
    t.flags_before = flags_before;
    t.address = pc;

    instruction = arm1_read_word(cpu, pc);
    t.raw = instruction;
    decoded = arm1_decode(instruction);
    arm1_disassemble(&decoded, t.mnemonic, sizeof t.mnemonic);
    strncpy(t.condition, arm1_cond_string(decoded.cond), sizeof t.condition - 1);
    t.condition[sizeof t.condition - 1] = '\0';
    t.condition_met = arm1_evaluate_condition(decoded.cond, flags_before);

    arm1_set_pc(cpu, pc + 4);

    if (t.condition_met) {
        switch (decoded.inst_type) {
        case ARM1_INST_DATA_PROCESSING:
            exec_dp(cpu, &decoded);
            break;
        case ARM1_INST_LOAD_STORE:
            exec_ls(cpu, &decoded, &t);
            break;
        case ARM1_INST_BLOCK_TRANSFER:
            exec_bt(cpu, &decoded, &t);
            break;
        case ARM1_INST_BRANCH:
            exec_branch(cpu, &decoded);
            break;
        case ARM1_INST_SWI:
            exec_swi(cpu, &decoded);
            break;
        case ARM1_INST_COPROCESSOR:
        case ARM1_INST_UNDEFINED:
        default:
            trap_undefined(cpu);
            break;
        }
    }

    for (i = 0; i < 16; i++) {
        t.regs_after[i] = arm1_read_register(cpu, (size_t)i);
    }
    t.flags_after = arm1_flags(cpu);

    if (out) {
        *out = t;
    }
}

size_t arm1_run(ARM1 *cpu, size_t max_steps, Arm1Trace *out, size_t out_cap) {
    size_t n = 0, i;
    for (i = 0; i < max_steps; i++) {
        Arm1Trace t;
        if (cpu->halted) {
            break;
        }
        arm1_step(cpu, &t);
        if (out && n < out_cap) {
            out[n] = t;
        }
        n++;
    }
    return n;
}

/* ── Encoding helpers ──────────────────────────────────────────────────────*/
uint32_t arm1_encode_data_processing(uint32_t cond, uint32_t opcode, uint32_t s,
                                     uint32_t rn, uint32_t rd,
                                     uint32_t operand2) {
    return (cond << 28) | operand2 | (opcode << 21) | (s << 20) | (rn << 16) |
           (rd << 12);
}
uint32_t arm1_encode_mov_imm(uint32_t cond, uint32_t rd, uint32_t imm8) {
    return arm1_encode_data_processing(cond, ARM1_OP_MOV, 0, 0, rd,
                                       (1u << 25) | imm8);
}
uint32_t arm1_encode_alu_reg(uint32_t cond, uint32_t opcode, uint32_t s,
                             uint32_t rd, uint32_t rn, uint32_t rm) {
    return arm1_encode_data_processing(cond, opcode, s, rn, rd, rm);
}
uint32_t arm1_encode_branch(uint32_t cond, int link, int32_t offset) {
    uint32_t inst = (cond << 28) | 0x0A000000u;
    if (link) {
        inst |= 0x01000000u;
    }
    inst |= ((uint32_t)(offset >> 2)) & 0x00FFFFFFu;
    return inst;
}
uint32_t arm1_encode_halt(void) {
    return (ARM1_COND_AL << 28) | 0x0F000000u | ARM1_HALT_SWI;
}
static uint32_t encode_ls(uint32_t base_opc, uint32_t cond, uint32_t rd,
                          uint32_t rn, int32_t offset, int pre_index) {
    uint32_t inst = (cond << 28) | base_opc;
    inst |= rd << 12;
    inst |= rn << 16;
    if (pre_index) {
        inst |= 1u << 24;
    }
    if (offset >= 0) {
        inst |= 1u << 23;
        inst |= ((uint32_t)offset) & 0xFFF;
    } else {
        inst |= ((uint32_t)(-offset)) & 0xFFF;
    }
    return inst;
}
uint32_t arm1_encode_ldr(uint32_t cond, uint32_t rd, uint32_t rn,
                         int32_t offset, int pre_index) {
    return encode_ls(0x04100000u, cond, rd, rn, offset, pre_index);
}
uint32_t arm1_encode_str(uint32_t cond, uint32_t rd, uint32_t rn,
                         int32_t offset, int pre_index) {
    return encode_ls(0x04000000u, cond, rd, rn, offset, pre_index);
}
uint32_t arm1_encode_ldm(uint32_t cond, uint32_t rn, uint16_t reg_list,
                         int write_back, const char *mode) {
    uint32_t inst = (cond << 28) | 0x08100000u;
    inst |= rn << 16;
    inst |= reg_list;
    if (write_back) {
        inst |= 1u << 21;
    }
    if (strcmp(mode, "IA") == 0) {
        inst |= 1u << 23;
    } else if (strcmp(mode, "IB") == 0) {
        inst |= (1u << 24) | (1u << 23);
    } else if (strcmp(mode, "DB") == 0) {
        inst |= 1u << 24;
    }
    /* "DA" adds nothing. */
    return inst;
}
uint32_t arm1_encode_stm(uint32_t cond, uint32_t rn, uint16_t reg_list,
                         int write_back, const char *mode) {
    return arm1_encode_ldm(cond, rn, reg_list, write_back, mode) & ~(1u << 20);
}
