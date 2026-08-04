/*
 * assembler.h — ARM assembly parser and binary encoder, pure ISO C17.
 * ==================================================================
 *
 * A faithful port of the Rust `assembler` crate. Parses a subset of ARM
 * assembly source text into structured instructions, then encodes each into
 * its 32-bit ARM machine-code word.
 *
 * ## Instruction word (data processing)
 *
 *   31-28  27-26  25  24-21     20  19-16  15-12  11-0
 *   [cond] [00]   [I] [opcode]  [S] [Rn]   [Rd]   [operand2]
 *
 * cond=0xE (always); I=1 for an immediate operand2; opcode per ArmOpcode;
 * S set by the `S` suffix. LDR/STR use the load/store format; NOP encodes as
 * MOV R0,R0 (0xE1A00000).
 *
 * Supported mnemonics: MOV(S), ADD(S), SUB(S), AND(S), ORR(S), EOR(S), RSB(S),
 * CMP, LDR, STR, NOP, and labels (`name:`).
 *
 * ## Divergences from Rust (documented)
 *
 *   - Rust `Result<_, AssemblerError>` -> an `AsmStatus` code plus an optional
 *     `AsmError` out-parameter whose `message` reproduces the Rust
 *     `Display` text (e.g. "Unknown mnemonic: BLAH").
 *   - Rust `Vec<ArmInstruction>` / `Vec<u32>` -> malloc'd arrays the caller
 *     frees (`asm_instructions_free` / plain `free`).
 *   - Rust `HashMap<String,usize>` labels -> an assoc-array with
 *     `asm_label_lookup`.
 *
 * Pure ISO C17: compiles under GCC, Clang and MSVC with -pedantic-errors /
 * /permissive- and warnings-as-errors; no <math.h>, no compiler extensions.
 */
#ifndef CA_ASSEMBLER_H
#define CA_ASSEMBLER_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* uint32_t */

#ifdef __cplusplus
extern "C" {
#endif

/* ARM data-processing opcodes (bits 24-21). Values match the ARM ARM. */
typedef enum {
    ARM_AND = 0x0,
    ARM_EOR = 0x1,
    ARM_SUB = 0x2,
    ARM_RSB = 0x3,
    ARM_ADD = 0x4,
    ARM_CMP = 0xA,
    ARM_ORR = 0xC,
    ARM_MOV = 0xD
} ArmOpcode;

/* Second operand: a register index or an immediate value. */
typedef enum { ASM_OPERAND2_REGISTER, ASM_OPERAND2_IMMEDIATE } AsmOperand2Kind;
typedef struct {
    AsmOperand2Kind kind;
    uint32_t value;
} AsmOperand2;

/* A parsed instruction. `label` is owned only for ASM_INSTR_LABEL. */
typedef enum {
    ASM_INSTR_DATA_PROCESSING,
    ASM_INSTR_LOAD,
    ASM_INSTR_STORE,
    ASM_INSTR_NOP,
    ASM_INSTR_LABEL
} AsmInstrKind;

typedef struct {
    AsmInstrKind kind;
    ArmOpcode opcode; /* DATA_PROCESSING */
    int has_rd;       /* rd present? (false only for CMP's destination) */
    uint32_t rd;
    int has_rn; /* rn present? (false only for MOV's first source) */
    uint32_t rn;
    AsmOperand2 operand2; /* DATA_PROCESSING */
    int set_flags;        /* the `S` suffix */
    char *label;          /* owned (ASM_INSTR_LABEL); NULL otherwise */
} ArmInstruction;

/* Free an instruction array (releasing any owned label names). */
void asm_instructions_free(ArmInstruction *instrs, size_t n);

/* Status / error codes (mirror the Rust `AssemblerError` variants). */
typedef enum {
    ASM_OK = 0,
    ASM_ERR_UNKNOWN_MNEMONIC,
    ASM_ERR_INVALID_REGISTER,
    ASM_ERR_INVALID_IMMEDIATE,
    ASM_ERR_INVALID_OPERAND_COUNT,
    ASM_ERR_PARSE,
    ASM_ERR_ALLOC
} AsmStatus;

/* A detailed error: a code plus a message reproducing the Rust Display text. */
typedef struct {
    AsmStatus code;
    char message[128];
} AsmError;

/* An assembler holds the label table across a parse. */
typedef struct {
    struct AsmLabel *labels; /* opaque; use asm_label_lookup */
    size_t num_labels;
    size_t cap_labels;
} Assembler;

void asm_init(Assembler *asmr);
void asm_free(Assembler *asmr);

/* Look up a label's address. Returns 1 and writes *out_addr if found. */
int asm_label_lookup(const Assembler *asmr, const char *name, size_t *out_addr);

/* Parse `source` into a malloc'd instruction array. On ASM_OK, *out owns the
 * array (free with asm_instructions_free) of *out_len entries. On error *out is
 * NULL and (if `err` is non-NULL) *err carries the code and message. Labels are
 * recorded in `asmr`. */
AsmStatus asm_parse(Assembler *asmr, const char *source, ArmInstruction **out,
                    size_t *out_len, AsmError *err);

/* Encode instructions into a malloc'd array of 32-bit words (labels emit
 * nothing). On ASM_OK, *out owns the array of *out_len words (free with
 * `free`). Only ASM_ERR_ALLOC can be returned. */
AsmStatus asm_encode(const ArmInstruction *instrs, size_t n, uint32_t **out,
                     size_t *out_len);

#ifdef __cplusplus
}
#endif

#endif /* CA_ASSEMBLER_H */
