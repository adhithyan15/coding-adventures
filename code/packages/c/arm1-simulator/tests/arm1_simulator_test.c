/*
 * Tests for arm1-simulator, mirroring the Rust crate's unit tests: condition
 * evaluation, barrel shifter, ALU, decode/disassemble, and full ARM1 program
 * execution. Uses the header-only iso_test.h harness (pure ISO C17).
 */
#include "iso_test.h"

#include "arm1_simulator.h"

#include <string.h>

static Arm1Flags mkflags(int n, int z, int c, int v) {
    Arm1Flags f;
    f.n = n;
    f.z = z;
    f.c = c;
    f.v = v;
    return f;
}

int main(void) {
    /* ── constants / strings ─────────────────────────────────────────────── */
    ISO_CHECK_STR_EQ(arm1_mode_string(ARM1_MODE_USR), "USR");
    ISO_CHECK_STR_EQ(arm1_mode_string(ARM1_MODE_SVC), "SVC");
    ISO_CHECK_STR_EQ(arm1_mode_string(99), "???");
    ISO_CHECK_STR_EQ(arm1_op_string(ARM1_OP_ADD), "ADD");
    ISO_CHECK_STR_EQ(arm1_op_string(ARM1_OP_MOV), "MOV");
    ISO_CHECK_STR_EQ(arm1_op_string(99), "???");
    ISO_CHECK(arm1_is_test_op(ARM1_OP_TST) && arm1_is_test_op(ARM1_OP_CMP));
    ISO_CHECK(!arm1_is_test_op(ARM1_OP_ADD));
    ISO_CHECK(arm1_is_logical_op(ARM1_OP_AND) &&
              arm1_is_logical_op(ARM1_OP_MOV));
    ISO_CHECK(!arm1_is_logical_op(ARM1_OP_ADD));

    /* ── condition evaluator (comprehensive) ─────────────────────────────── */
    ISO_CHECK(arm1_evaluate_condition(ARM1_COND_EQ, mkflags(0, 1, 0, 0)));
    ISO_CHECK(!arm1_evaluate_condition(ARM1_COND_EQ, mkflags(0, 0, 0, 0)));
    ISO_CHECK(arm1_evaluate_condition(ARM1_COND_NE, mkflags(0, 0, 0, 0)));
    ISO_CHECK(arm1_evaluate_condition(ARM1_COND_CS, mkflags(0, 0, 1, 0)));
    ISO_CHECK(arm1_evaluate_condition(ARM1_COND_CC, mkflags(0, 0, 0, 0)));
    ISO_CHECK(arm1_evaluate_condition(ARM1_COND_MI, mkflags(1, 0, 0, 0)));
    ISO_CHECK(arm1_evaluate_condition(ARM1_COND_PL, mkflags(0, 0, 0, 0)));
    ISO_CHECK(arm1_evaluate_condition(ARM1_COND_VS, mkflags(0, 0, 0, 1)));
    ISO_CHECK(arm1_evaluate_condition(ARM1_COND_HI, mkflags(0, 0, 1, 0)));
    ISO_CHECK(!arm1_evaluate_condition(ARM1_COND_HI, mkflags(0, 1, 1, 0)));
    ISO_CHECK(arm1_evaluate_condition(ARM1_COND_LS, mkflags(0, 0, 0, 0)));
    ISO_CHECK(arm1_evaluate_condition(ARM1_COND_LS, mkflags(0, 1, 1, 0)));
    ISO_CHECK(arm1_evaluate_condition(ARM1_COND_GE, mkflags(0, 0, 0, 0)));
    ISO_CHECK(arm1_evaluate_condition(ARM1_COND_GE, mkflags(1, 0, 0, 1)));
    ISO_CHECK(!arm1_evaluate_condition(ARM1_COND_GE, mkflags(1, 0, 0, 0)));
    ISO_CHECK(arm1_evaluate_condition(ARM1_COND_LT, mkflags(1, 0, 0, 0)));
    ISO_CHECK(arm1_evaluate_condition(ARM1_COND_GT, mkflags(0, 0, 0, 0)));
    ISO_CHECK(!arm1_evaluate_condition(ARM1_COND_GT, mkflags(0, 1, 0, 0)));
    ISO_CHECK(arm1_evaluate_condition(ARM1_COND_LE, mkflags(0, 1, 0, 0)));
    ISO_CHECK(arm1_evaluate_condition(ARM1_COND_LE, mkflags(1, 0, 0, 0)));
    ISO_CHECK(arm1_evaluate_condition(ARM1_COND_AL, mkflags(0, 0, 0, 0)));
    ISO_CHECK(!arm1_evaluate_condition(ARM1_COND_NV, mkflags(0, 0, 0, 0)));

    /* ── barrel shifter ──────────────────────────────────────────────────── */
    {
        int c;
        ISO_CHECK_EQ_UINT(arm1_barrel_shift(0xFF, ARM1_SHIFT_LSL, 0, 0, 0, &c),
                          0xFF);
        ISO_CHECK_EQ_UINT(arm1_barrel_shift(0xFF, ARM1_SHIFT_LSL, 1, 0, 0, &c),
                          0x1FE);
        ISO_CHECK_EQ_UINT(arm1_barrel_shift(1, ARM1_SHIFT_LSL, 31, 0, 0, &c),
                          0x80000000u);
        ISO_CHECK(arm1_barrel_shift(1, ARM1_SHIFT_LSL, 32, 0, 0, &c) == 0 && c);
        ISO_CHECK(arm1_barrel_shift(1, ARM1_SHIFT_LSL, 33, 0, 0, &c) == 0 && !c);
        /* LSR */
        ISO_CHECK(arm1_barrel_shift(0xFF, ARM1_SHIFT_LSR, 1, 0, 0, &c) == 0x7F &&
                  c);
        ISO_CHECK(arm1_barrel_shift(0x80000000u, ARM1_SHIFT_LSR, 0, 0, 0, &c) ==
                      0 &&
                  c); /* LSR #0 == #32 */
        ISO_CHECK(arm1_barrel_shift(0x80000000u, ARM1_SHIFT_LSR, 32, 0, 1, &c) ==
                      0 &&
                  c);
        /* ASR */
        ISO_CHECK_EQ_UINT(
            arm1_barrel_shift(0x7FFFFFFEu, ARM1_SHIFT_ASR, 1, 0, 0, &c),
            0x3FFFFFFFu);
        ISO_CHECK_EQ_UINT(
            arm1_barrel_shift(0x80000000u, ARM1_SHIFT_ASR, 1, 0, 0, &c),
            0xC0000000u);
        ISO_CHECK(arm1_barrel_shift(0x80000000u, ARM1_SHIFT_ASR, 0, 0, 0, &c) ==
                      0xFFFFFFFFu &&
                  c);
        ISO_CHECK(arm1_barrel_shift(0x7FFFFFFFu, ARM1_SHIFT_ASR, 0, 0, 0, &c) ==
                      0 &&
                  !c);
        /* ROR / RRX */
        ISO_CHECK_EQ_UINT(
            arm1_barrel_shift(0x0000000Fu, ARM1_SHIFT_ROR, 4, 0, 0, &c),
            0xF0000000u);
        ISO_CHECK(arm1_barrel_shift(1, ARM1_SHIFT_ROR, 0, 1, 0, &c) ==
                      0x80000000u &&
                  c);
        /* by-register, amount 0: pass through, carry unchanged */
        ISO_CHECK(arm1_barrel_shift(0xDEADBEEFu, ARM1_SHIFT_LSL, 0, 1, 1, &c) ==
                      0xDEADBEEFu &&
                  c);
    }

    /* ── decode immediate ────────────────────────────────────────────────── */
    {
        int c;
        ISO_CHECK_EQ_UINT(arm1_decode_immediate(0xFF, 0, &c), 0xFF);
        ISO_CHECK_EQ_UINT(arm1_decode_immediate(0xFF, 4, &c), 0xFF000000u);
    }

    /* ── ALU ─────────────────────────────────────────────────────────────── */
    {
        Arm1ALUResult r = arm1_alu_execute(ARM1_OP_ADD, 1, 2, 0, 0, 0);
        ISO_CHECK(r.result == 3 && !r.n && !r.z && !r.c && !r.v);
        r = arm1_alu_execute(ARM1_OP_SUB, 5, 5, 0, 0, 0);
        ISO_CHECK(r.result == 0 && r.z && r.c);
        r = arm1_alu_execute(ARM1_OP_SUB, 3, 5, 0, 0, 0);
        ISO_CHECK(r.n && !r.c);
        r = arm1_alu_execute(ARM1_OP_AND, 0xFF00FF00u, 0x0FF00FF0u, 0, 0, 0);
        ISO_CHECK_EQ_UINT(r.result, 0x0F000F00u);
        r = arm1_alu_execute(ARM1_OP_EOR, 0xFF00FF00u, 0x0FF00FF0u, 0, 0, 0);
        ISO_CHECK_EQ_UINT(r.result, 0xF0F0F0F0u);
        r = arm1_alu_execute(ARM1_OP_ORR, 0xFF00FF00u, 0x0FF00FF0u, 0, 0, 0);
        ISO_CHECK_EQ_UINT(r.result, 0xFFF0FFF0u);
        r = arm1_alu_execute(ARM1_OP_MOV, 0, 42, 0, 0, 0);
        ISO_CHECK_EQ_UINT(r.result, 42);
        r = arm1_alu_execute(ARM1_OP_MVN, 0, 0, 0, 0, 0);
        ISO_CHECK_EQ_UINT(r.result, 0xFFFFFFFFu);
        r = arm1_alu_execute(ARM1_OP_BIC, 0xFFFFFFFFu, 0x000000FFu, 0, 0, 0);
        ISO_CHECK_EQ_UINT(r.result, 0xFFFFFF00u);
        r = arm1_alu_execute(ARM1_OP_TST, 0xFF, 0x0F, 0, 0, 0);
        ISO_CHECK(!r.write_result && r.result == 0x0F);
        r = arm1_alu_execute(ARM1_OP_CMP, 5, 5, 0, 0, 0);
        ISO_CHECK(!r.write_result && r.z);
        r = arm1_alu_execute(ARM1_OP_ADC, 0, 0, 1, 0, 0);
        ISO_CHECK_EQ_UINT(r.result, 1);
        r = arm1_alu_execute(ARM1_OP_SBC, 5, 3, 0, 0, 0);
        ISO_CHECK_EQ_UINT(r.result, 1);
        r = arm1_alu_execute(ARM1_OP_RSB, 3, 10, 0, 0, 0);
        ISO_CHECK_EQ_UINT(r.result, 7);
        r = arm1_alu_execute(ARM1_OP_RSC, 3, 10, 1, 0, 0);
        ISO_CHECK_EQ_UINT(r.result, 7);
        r = arm1_alu_execute(ARM1_OP_ADD, 0x7FFFFFFFu, 1, 0, 0, 0);
        ISO_CHECK(r.v && r.n);
    }

    /* ── decoder ─────────────────────────────────────────────────────────── */
    {
        Arm1DecodedInstruction d =
            arm1_decode(arm1_encode_mov_imm(ARM1_COND_AL, 0, 42));
        ISO_CHECK(d.inst_type == ARM1_INST_DATA_PROCESSING &&
                  d.opcode == ARM1_OP_MOV && d.immediate && d.imm8 == 42 &&
                  d.rd == 0);
        d = arm1_decode(arm1_encode_alu_reg(ARM1_COND_AL, ARM1_OP_ADD, 1, 2, 0,
                                            1));
        ISO_CHECK(d.opcode == ARM1_OP_ADD && d.s && d.rd == 2 && d.rn == 0 &&
                  d.rm == 1);
        d = arm1_decode(arm1_encode_branch(ARM1_COND_NE, 0, -16));
        ISO_CHECK(d.inst_type == ARM1_INST_BRANCH && !d.link &&
                  d.branch_offset == -16);
        d = arm1_decode(arm1_encode_branch(ARM1_COND_AL, 1, 8));
        ISO_CHECK(d.inst_type == ARM1_INST_BRANCH && d.link);
        d = arm1_decode(arm1_encode_halt());
        ISO_CHECK(d.inst_type == ARM1_INST_SWI &&
                  d.swi_comment == ARM1_HALT_SWI);
        d = arm1_decode(arm1_encode_ldr(ARM1_COND_AL, 0, 1, 4, 1));
        ISO_CHECK(d.inst_type == ARM1_INST_LOAD_STORE && d.load && d.pre_index &&
                  d.up && d.rd == 0 && d.rn == 1 && d.offset12 == 4);
        d = arm1_decode(arm1_encode_str(ARM1_COND_AL, 2, 3, -8, 1));
        ISO_CHECK(!d.load && !d.up && d.offset12 == 8);
        d = arm1_decode(arm1_encode_ldm(ARM1_COND_AL, 13, 0x000F, 1, "IA"));
        ISO_CHECK(d.inst_type == ARM1_INST_BLOCK_TRANSFER && d.load &&
                  d.write_back && d.register_list == 0x000F);
    }

    /* ── disassembly ─────────────────────────────────────────────────────── */
    {
        char buf[ARM1_MNEMONIC_CAP];
        Arm1DecodedInstruction d =
            arm1_decode(arm1_encode_mov_imm(ARM1_COND_AL, 0, 42));
        arm1_disassemble(&d, buf, sizeof buf);
        ISO_CHECK_STR_EQ(buf, "MOV R0, #42");
        d = arm1_decode(arm1_encode_halt());
        arm1_disassemble(&d, buf, sizeof buf);
        ISO_CHECK_STR_EQ(buf, "HLT");
    }

    /* ── CPU state / reset / banking / memory ────────────────────────────── */
    {
        ARM1 *cpu = arm1_new(1024);
        ISO_CHECK(arm1_mode(cpu) == ARM1_MODE_SVC && arm1_pc(cpu) == 0 &&
                  !arm1_halted(cpu));
        arm1_write_register(cpu, 0, 42);
        arm1_set_pc(cpu, 100);
        arm1_reset(cpu);
        ISO_CHECK(arm1_read_register(cpu, 0) == 0 && arm1_pc(cpu) == 0 &&
                  arm1_mode(cpu) == ARM1_MODE_SVC);
        arm1_free(cpu);
    }
    {
        ARM1 *cpu = arm1_new(4096);
        arm1_write_word(cpu, 0x100, 0xDEADBEEFu);
        ISO_CHECK_EQ_UINT(arm1_read_word(cpu, 0x100), 0xDEADBEEFu);
        arm1_write_byte(cpu, 0x50, 0xAB);
        ISO_CHECK_EQ_INT(arm1_read_byte(cpu, 0x50), 0xAB);
        arm1_free(cpu);
    }
    {
        ARM1 *cpu = arm1_new(256);
        ISO_CHECK_EQ_UINT(arm1_read_word(cpu, 0x1000), 0);
        ISO_CHECK_EQ_INT(arm1_read_byte(cpu, 0x1000), 0);
        arm1_free(cpu);
    }
    { /* PC masks to 26 bits */
        ARM1 *cpu = arm1_new(4096);
        arm1_set_pc(cpu, 0xFFFFFFFCu);
        ISO_CHECK_EQ_UINT(arm1_pc(cpu), 0x03FFFFFCu);
        arm1_free(cpu);
    }

    /* ── full program execution ──────────────────────────────────────────── */
    {
        ARM1 *cpu = arm1_new(4096);
        uint32_t prog[] = {arm1_encode_mov_imm(ARM1_COND_AL, 0, 42),
                           arm1_encode_halt()};
        Arm1Trace traces[100];
        size_t n;
        arm1_load_program_words(cpu, prog, 2, 0);
        n = arm1_run(cpu, 100, traces, 100);
        ISO_CHECK_EQ_UINT(n, 2);
        ISO_CHECK_EQ_UINT(arm1_read_register(cpu, 0), 42);
        ISO_CHECK(arm1_halted(cpu));
        arm1_free(cpu);
    }
    {
        ARM1 *cpu = arm1_new(4096);
        uint32_t prog[] = {arm1_encode_mov_imm(ARM1_COND_AL, 0, 10),
                           arm1_encode_mov_imm(ARM1_COND_AL, 1, 20),
                           arm1_encode_alu_reg(ARM1_COND_AL, ARM1_OP_ADD, 0, 2, 0,
                                               1),
                           arm1_encode_halt()};
        arm1_load_program_words(cpu, prog, 4, 0);
        arm1_run(cpu, 100, NULL, 0);
        ISO_CHECK_EQ_UINT(arm1_read_register(cpu, 2), 30);
        arm1_free(cpu);
    }
    {
        ARM1 *cpu = arm1_new(4096);
        uint32_t prog[] = {arm1_encode_mov_imm(ARM1_COND_AL, 0, 5),
                           arm1_encode_mov_imm(ARM1_COND_AL, 1, 5),
                           arm1_encode_alu_reg(ARM1_COND_AL, ARM1_OP_SUB, 1, 2, 0,
                                               1),
                           arm1_encode_halt()};
        arm1_load_program_words(cpu, prog, 4, 0);
        arm1_run(cpu, 100, NULL, 0);
        ISO_CHECK_EQ_UINT(arm1_read_register(cpu, 2), 0);
        ISO_CHECK(arm1_flags(cpu).z);
        arm1_free(cpu);
    }
    {
        /* conditional execution */
        ARM1 *cpu = arm1_new(4096);
        uint32_t prog[] = {arm1_encode_mov_imm(ARM1_COND_AL, 0, 5),
                           arm1_encode_mov_imm(ARM1_COND_AL, 1, 5),
                           arm1_encode_alu_reg(ARM1_COND_AL, ARM1_OP_SUB, 1, 2, 0,
                                               1),
                           arm1_encode_mov_imm(ARM1_COND_NE, 3, 99),
                           arm1_encode_mov_imm(ARM1_COND_EQ, 4, 42),
                           arm1_encode_halt()};
        arm1_load_program_words(cpu, prog, 6, 0);
        arm1_run(cpu, 100, NULL, 0);
        ISO_CHECK_EQ_UINT(arm1_read_register(cpu, 3), 0);
        ISO_CHECK_EQ_UINT(arm1_read_register(cpu, 4), 42);
        arm1_free(cpu);
    }
    {
        /* branch skips an instruction */
        ARM1 *cpu = arm1_new(4096);
        uint32_t prog[] = {arm1_encode_mov_imm(ARM1_COND_AL, 0, 1),
                           arm1_encode_branch(ARM1_COND_AL, 0, 4),
                           arm1_encode_mov_imm(ARM1_COND_AL, 0, 99),
                           arm1_encode_halt()};
        arm1_load_program_words(cpu, prog, 4, 0);
        arm1_run(cpu, 100, NULL, 0);
        ISO_CHECK_EQ_UINT(arm1_read_register(cpu, 0), 1);
        arm1_free(cpu);
    }
    {
        /* loop sum 1..10 = 55 */
        ARM1 *cpu = arm1_new(4096);
        uint32_t prog[] = {
            arm1_encode_mov_imm(ARM1_COND_AL, 0, 0),
            arm1_encode_mov_imm(ARM1_COND_AL, 1, 10),
            arm1_encode_alu_reg(ARM1_COND_AL, ARM1_OP_ADD, 0, 0, 0, 1),
            arm1_encode_data_processing(ARM1_COND_AL, ARM1_OP_SUB, 1, 1, 1,
                                        (1u << 25) | 1),
            arm1_encode_branch(ARM1_COND_NE, 0, -16),
            arm1_encode_halt()};
        arm1_load_program_words(cpu, prog, 6, 0);
        arm1_run(cpu, 200, NULL, 0);
        ISO_CHECK_EQ_UINT(arm1_read_register(cpu, 0), 55);
        arm1_free(cpu);
    }
    {
        /* LDR/STR round-trip */
        ARM1 *cpu = arm1_new(4096);
        uint32_t prog[] = {
            arm1_encode_mov_imm(ARM1_COND_AL, 0, 42),
            arm1_encode_data_processing(ARM1_COND_AL, ARM1_OP_MOV, 0, 0, 1,
                                        (1u << 25) | (12u << 8) | 1),
            arm1_encode_str(ARM1_COND_AL, 0, 1, 0, 1),
            arm1_encode_mov_imm(ARM1_COND_AL, 0, 0),
            arm1_encode_ldr(ARM1_COND_AL, 0, 1, 0, 1),
            arm1_encode_halt()};
        arm1_load_program_words(cpu, prog, 6, 0);
        arm1_run(cpu, 100, NULL, 0);
        ISO_CHECK_EQ_UINT(arm1_read_register(cpu, 0), 42);
        arm1_free(cpu);
    }
    {
        /* STM/LDM round-trip */
        ARM1 *cpu = arm1_new(4096);
        uint32_t setr5 = arm1_encode_data_processing(
            ARM1_COND_AL, ARM1_OP_MOV, 0, 0, 5, (1u << 25) | (12u << 8) | 1);
        uint32_t prog[] = {arm1_encode_mov_imm(ARM1_COND_AL, 0, 10),
                           arm1_encode_mov_imm(ARM1_COND_AL, 1, 20),
                           arm1_encode_mov_imm(ARM1_COND_AL, 2, 30),
                           arm1_encode_mov_imm(ARM1_COND_AL, 3, 40),
                           setr5,
                           arm1_encode_stm(ARM1_COND_AL, 5, 0x000F, 1, "IA"),
                           arm1_encode_mov_imm(ARM1_COND_AL, 0, 0),
                           arm1_encode_mov_imm(ARM1_COND_AL, 1, 0),
                           arm1_encode_mov_imm(ARM1_COND_AL, 2, 0),
                           arm1_encode_mov_imm(ARM1_COND_AL, 3, 0),
                           setr5,
                           arm1_encode_ldm(ARM1_COND_AL, 5, 0x000F, 1, "IA"),
                           arm1_encode_halt()};
        arm1_load_program_words(cpu, prog, 13, 0);
        arm1_run(cpu, 100, NULL, 0);
        ISO_CHECK_EQ_UINT(arm1_read_register(cpu, 0), 10);
        ISO_CHECK_EQ_UINT(arm1_read_register(cpu, 1), 20);
        ISO_CHECK_EQ_UINT(arm1_read_register(cpu, 2), 30);
        ISO_CHECK_EQ_UINT(arm1_read_register(cpu, 3), 40);
        arm1_free(cpu);
    }
    {
        /* branch and link */
        ARM1 *cpu = arm1_new(4096);
        uint32_t prog[] = {
            arm1_encode_mov_imm(ARM1_COND_AL, 0, 7),
            arm1_encode_branch(ARM1_COND_AL, 1, 4),
            arm1_encode_halt(),
            0,
            arm1_encode_alu_reg(ARM1_COND_AL, ARM1_OP_ADD, 0, 0, 0, 0),
            arm1_encode_data_processing(ARM1_COND_AL, ARM1_OP_MOV, 1, 0, 15,
                                        14)};
        arm1_load_program_words(cpu, prog, 6, 0);
        arm1_run(cpu, 20, NULL, 0);
        ISO_CHECK_EQ_UINT(arm1_read_register(cpu, 0), 14);
        arm1_free(cpu);
    }
    {
        /* barrel shifter in instruction: ADD R1, R0, R0, LSL #2 -> 35 */
        ARM1 *cpu = arm1_new(4096);
        uint32_t add_shift = ((ARM1_COND_AL << 28) | (ARM1_OP_ADD << 21)) |
                             (1u << 12) | (2u << 7) | (ARM1_SHIFT_LSL << 5);
        uint32_t prog[] = {arm1_encode_mov_imm(ARM1_COND_AL, 0, 7), add_shift,
                           arm1_encode_halt()};
        arm1_load_program_words(cpu, prog, 3, 0);
        arm1_run(cpu, 100, NULL, 0);
        ISO_CHECK_EQ_UINT(arm1_read_register(cpu, 1), 35);
        arm1_free(cpu);
    }
    {
        /* CMP + conditional branch: R0>R1 -> R2=1 */
        ARM1 *cpu = arm1_new(4096);
        uint32_t prog[] = {arm1_encode_mov_imm(ARM1_COND_AL, 0, 10),
                           arm1_encode_mov_imm(ARM1_COND_AL, 1, 5),
                           arm1_encode_alu_reg(ARM1_COND_AL, ARM1_OP_CMP, 1, 0, 0,
                                               1),
                           arm1_encode_mov_imm(ARM1_COND_GT, 2, 1),
                           arm1_encode_mov_imm(ARM1_COND_LE, 2, 0),
                           arm1_encode_halt()};
        arm1_load_program_words(cpu, prog, 6, 0);
        arm1_run(cpu, 100, NULL, 0);
        ISO_CHECK_EQ_UINT(arm1_read_register(cpu, 2), 1);
        arm1_free(cpu);
    }
    {
        /* trace memory tracking on STR */
        ARM1 *cpu = arm1_new(4096);
        uint32_t prog[] = {
            arm1_encode_mov_imm(ARM1_COND_AL, 0, 99),
            arm1_encode_data_processing(ARM1_COND_AL, ARM1_OP_MOV, 0, 0, 1,
                                        (1u << 25) | (12u << 8) | 1),
            arm1_encode_str(ARM1_COND_AL, 0, 1, 0, 1),
            arm1_encode_halt()};
        Arm1Trace traces[100];
        size_t n;
        arm1_load_program_words(cpu, prog, 4, 0);
        n = arm1_run(cpu, 100, traces, 100);
        ISO_CHECK(n >= 3);
        ISO_CHECK(traces[2].memory_write_count > 0);
        ISO_CHECK_EQ_UINT(traces[2].memory_writes[0].value, 99);
        arm1_free(cpu);
    }
    {
        /* SWI non-halt vectors to 0x08 */
        ARM1 *cpu = arm1_new(4096);
        uint32_t halt = arm1_encode_halt();
        uint32_t swi = (ARM1_COND_AL << 28) | 0x0F000000u | 0x42;
        arm1_load_program_words(cpu, &halt, 1, 0x08);
        arm1_load_program_words(cpu, &swi, 1, 0);
        arm1_run(cpu, 10, NULL, 0);
        ISO_CHECK(arm1_halted(cpu));
        arm1_free(cpu);
    }

    /* ── register banking (SVC banks R13/R14) ────────────────────────────── */
    {
        ARM1 *cpu = arm1_new(1024);
        arm1_write_register(cpu, 13, 0xAAAA);
        arm1_write_register(cpu, 14, 0xBBBB);
        ISO_CHECK_EQ_UINT(arm1_read_register(cpu, 13), 0xAAAA);
        ISO_CHECK_EQ_UINT(arm1_read_register(cpu, 14), 0xBBBB);
        ISO_CHECK_EQ_UINT(arm1_r15_raw(cpu) & ARM1_MODE_MASK, ARM1_MODE_SVC);
        arm1_free(cpu);
    }

    return ISO_TEST_RESULT();
}
