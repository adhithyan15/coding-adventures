/*
 * arm1_simulator.h — ARM1 behavioral CPU simulator, pure ISO C17.
 * =====================================================================
 *
 * A faithful port of the Rust `arm1-simulator` crate: a complete behavioral
 * simulator for the ARM1 (Sophie Wilson & Steve Furber, Acorn, 1985) — the
 * first ARM chip. Implements the full ARMv1 instruction set: 16 data-processing
 * ops, load/store (LDR/STR/LDRB/STRB), block transfer (LDM/STM), branch (B/BL),
 * software interrupt (SWI), conditional execution, the inline barrel shifter,
 * and 4 processor modes with banked registers.
 *
 * ARMv1's signature: the program counter and status flags share one 32-bit
 * register (R15). Every instruction is conditional on the 4-bit code in bits
 * 31:28.
 *
 * Pure ISO C17: no <math.h>, no compiler extensions.
 */
#ifndef ARM1_SIMULATOR_H
#define ARM1_SIMULATOR_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* uint8_t, uint16_t, uint32_t, int32_t */

#ifdef __cplusplus
extern "C" {
#endif

/* Processor modes. */
#define ARM1_MODE_USR 0u
#define ARM1_MODE_FIQ 1u
#define ARM1_MODE_IRQ 2u
#define ARM1_MODE_SVC 3u

/* Condition codes (bits 31:28). */
#define ARM1_COND_EQ 0x0u
#define ARM1_COND_NE 0x1u
#define ARM1_COND_CS 0x2u
#define ARM1_COND_CC 0x3u
#define ARM1_COND_MI 0x4u
#define ARM1_COND_PL 0x5u
#define ARM1_COND_VS 0x6u
#define ARM1_COND_VC 0x7u
#define ARM1_COND_HI 0x8u
#define ARM1_COND_LS 0x9u
#define ARM1_COND_GE 0xAu
#define ARM1_COND_LT 0xBu
#define ARM1_COND_GT 0xCu
#define ARM1_COND_LE 0xDu
#define ARM1_COND_AL 0xEu
#define ARM1_COND_NV 0xFu

/* ALU opcodes (bits 24:21). */
#define ARM1_OP_AND 0x0u
#define ARM1_OP_EOR 0x1u
#define ARM1_OP_SUB 0x2u
#define ARM1_OP_RSB 0x3u
#define ARM1_OP_ADD 0x4u
#define ARM1_OP_ADC 0x5u
#define ARM1_OP_SBC 0x6u
#define ARM1_OP_RSC 0x7u
#define ARM1_OP_TST 0x8u
#define ARM1_OP_TEQ 0x9u
#define ARM1_OP_CMP 0xAu
#define ARM1_OP_CMN 0xBu
#define ARM1_OP_ORR 0xCu
#define ARM1_OP_MOV 0xDu
#define ARM1_OP_BIC 0xEu
#define ARM1_OP_MVN 0xFu

/* Shift types (bits 6:5). */
#define ARM1_SHIFT_LSL 0u
#define ARM1_SHIFT_LSR 1u
#define ARM1_SHIFT_ASR 2u
#define ARM1_SHIFT_ROR 3u

/* R15 bit positions. */
#define ARM1_FLAG_N (1u << 31)
#define ARM1_FLAG_Z (1u << 30)
#define ARM1_FLAG_C (1u << 29)
#define ARM1_FLAG_V (1u << 28)
#define ARM1_FLAG_I (1u << 27)
#define ARM1_FLAG_F (1u << 26)
#define ARM1_PC_MASK 0x03FFFFFCu
#define ARM1_MODE_MASK 0x3u
#define ARM1_HALT_SWI 0x123456u

#define ARM1_MNEMONIC_CAP 128
#define ARM1_MAX_MEM_ACCESS 16 /* a block transfer touches at most 16 regs */

const char *arm1_mode_string(uint32_t mode);
const char *arm1_cond_string(uint32_t cond);
const char *arm1_op_string(uint32_t opcode);
int arm1_is_test_op(uint32_t opcode);
int arm1_is_logical_op(uint32_t opcode);
const char *arm1_shift_string(uint32_t shift_type);

/* Condition flags. */
typedef struct {
    int n, z, c, v;
} Arm1Flags;

int arm1_evaluate_condition(uint32_t cond, Arm1Flags flags);

/* Barrel shift. Returns the shifted value; writes the carry-out to *carry_out. */
uint32_t arm1_barrel_shift(uint32_t value, uint32_t shift_type, uint32_t amount,
                           int carry_in, int by_register, int *carry_out);
/* Decode a rotated immediate. Returns the value; writes carry-out. */
uint32_t arm1_decode_immediate(uint32_t imm8, uint32_t rotate, int *carry_out);

typedef struct {
    uint32_t result;
    int n, z, c, v;
    int write_result;
} Arm1ALUResult;

Arm1ALUResult arm1_alu_execute(uint32_t opcode, uint32_t a, uint32_t b,
                               int carry_in, int shifter_carry, int old_v);

/* Instruction class (bits 27:25). */
typedef enum {
    ARM1_INST_DATA_PROCESSING,
    ARM1_INST_LOAD_STORE,
    ARM1_INST_BLOCK_TRANSFER,
    ARM1_INST_BRANCH,
    ARM1_INST_SWI,
    ARM1_INST_COPROCESSOR,
    ARM1_INST_UNDEFINED
} Arm1InstType;

typedef struct {
    uint32_t raw;
    Arm1InstType inst_type;
    uint32_t cond;
    uint32_t opcode;
    int s;
    size_t rn, rd;
    int immediate;
    uint32_t imm8, rotate;
    size_t rm;
    uint32_t shift_type;
    int shift_by_reg;
    uint32_t shift_imm;
    size_t rs;
    int load, byte, pre_index, up, write_back;
    uint32_t offset12;
    uint16_t register_list;
    int force_user;
    int link;
    int32_t branch_offset;
    uint32_t swi_comment;
} Arm1DecodedInstruction;

Arm1DecodedInstruction arm1_decode(uint32_t instruction);
void arm1_disassemble(const Arm1DecodedInstruction *d, char *out, size_t n);

typedef struct {
    uint32_t address;
    uint32_t value;
} Arm1MemoryAccess;

/* One instruction's before/after state snapshot. Plain value type. */
typedef struct {
    uint32_t address;
    uint32_t raw;
    char mnemonic[ARM1_MNEMONIC_CAP];
    char condition[4];
    int condition_met;
    uint32_t regs_before[16];
    uint32_t regs_after[16];
    Arm1Flags flags_before;
    Arm1Flags flags_after;
    Arm1MemoryAccess memory_reads[ARM1_MAX_MEM_ACCESS];
    size_t memory_read_count;
    Arm1MemoryAccess memory_writes[ARM1_MAX_MEM_ACCESS];
    size_t memory_write_count;
} Arm1Trace;

/* Opaque CPU. */
typedef struct ARM1 ARM1;

ARM1 *arm1_new(size_t memory_size); /* NULL on OOM */
void arm1_free(ARM1 *cpu);
void arm1_reset(ARM1 *cpu);

uint32_t arm1_read_register(const ARM1 *cpu, size_t index);
void arm1_write_register(ARM1 *cpu, size_t index, uint32_t value);
uint32_t arm1_pc(const ARM1 *cpu);
void arm1_set_pc(ARM1 *cpu, uint32_t addr);
Arm1Flags arm1_flags(const ARM1 *cpu);
void arm1_set_flags(ARM1 *cpu, Arm1Flags f);
uint32_t arm1_mode(const ARM1 *cpu);
int arm1_halted(const ARM1 *cpu);
uint32_t arm1_r15_raw(const ARM1 *cpu);

uint32_t arm1_read_word(const ARM1 *cpu, uint32_t addr);
void arm1_write_word(ARM1 *cpu, uint32_t addr, uint32_t value);
uint8_t arm1_read_byte(const ARM1 *cpu, uint32_t addr);
void arm1_write_byte(ARM1 *cpu, uint32_t addr, uint8_t value);
void arm1_load_program(ARM1 *cpu, const uint8_t *code, size_t len,
                       uint32_t start_addr);
void arm1_load_program_words(ARM1 *cpu, const uint32_t *instructions,
                             size_t count, uint32_t start_addr);

/* Execute one instruction; writes the trace to *out (if non-NULL). */
void arm1_step(ARM1 *cpu, Arm1Trace *out);
/* Run up to max_steps (stopping at halt); writes each trace to out[0..],
 * capped at out_cap. Returns the number of traces produced. */
size_t arm1_run(ARM1 *cpu, size_t max_steps, Arm1Trace *out, size_t out_cap);

/* ── Encoding helpers ──────────────────────────────────────────────────────*/
uint32_t arm1_encode_data_processing(uint32_t cond, uint32_t opcode, uint32_t s,
                                     uint32_t rn, uint32_t rd,
                                     uint32_t operand2);
uint32_t arm1_encode_mov_imm(uint32_t cond, uint32_t rd, uint32_t imm8);
uint32_t arm1_encode_alu_reg(uint32_t cond, uint32_t opcode, uint32_t s,
                             uint32_t rd, uint32_t rn, uint32_t rm);
uint32_t arm1_encode_branch(uint32_t cond, int link, int32_t offset);
uint32_t arm1_encode_halt(void);
uint32_t arm1_encode_ldr(uint32_t cond, uint32_t rd, uint32_t rn,
                         int32_t offset, int pre_index);
uint32_t arm1_encode_str(uint32_t cond, uint32_t rd, uint32_t rn,
                         int32_t offset, int pre_index);
/* mode is "IA"/"IB"/"DA"/"DB". */
uint32_t arm1_encode_ldm(uint32_t cond, uint32_t rn, uint16_t reg_list,
                         int write_back, const char *mode);
uint32_t arm1_encode_stm(uint32_t cond, uint32_t rn, uint16_t reg_list,
                         int write_back, const char *mode);

#ifdef __cplusplus
}
#endif

#endif /* ARM1_SIMULATOR_H */
