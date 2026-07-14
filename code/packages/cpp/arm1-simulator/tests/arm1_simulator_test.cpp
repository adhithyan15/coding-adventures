// Tests for arm1-simulator, mirroring the Rust crate's unit tests, using the
// header-only iso_test.h harness (pure ISO C++17).
#include "iso_test.h"

#include <cstdint>
#include <vector>

#include "arm1_simulator.hpp"

namespace a1 = ca::arm1_simulator;

static a1::Flags mkf(bool n, bool z, bool c, bool v) { return {n, z, c, v}; }

int main() {
    // ── constants / strings ──────────────────────────────────────────────
    ISO_CHECK(a1::mode_string(a1::MODE_USR) == "USR");
    ISO_CHECK(a1::mode_string(99) == "???");
    ISO_CHECK(a1::op_string(a1::OP_ADD) == "ADD");
    ISO_CHECK(a1::op_string(99) == "???");
    ISO_CHECK(a1::is_test_op(a1::OP_TST) && !a1::is_test_op(a1::OP_ADD));
    ISO_CHECK(a1::is_logical_op(a1::OP_AND) && !a1::is_logical_op(a1::OP_ADD));

    // ── condition evaluator ──────────────────────────────────────────────
    ISO_CHECK(a1::evaluate_condition(a1::COND_EQ, mkf(0, 1, 0, 0)));
    ISO_CHECK(!a1::evaluate_condition(a1::COND_EQ, mkf(0, 0, 0, 0)));
    ISO_CHECK(a1::evaluate_condition(a1::COND_NE, mkf(0, 0, 0, 0)));
    ISO_CHECK(a1::evaluate_condition(a1::COND_HI, mkf(0, 0, 1, 0)));
    ISO_CHECK(!a1::evaluate_condition(a1::COND_HI, mkf(0, 1, 1, 0)));
    ISO_CHECK(a1::evaluate_condition(a1::COND_GE, mkf(1, 0, 0, 1)));
    ISO_CHECK(!a1::evaluate_condition(a1::COND_GE, mkf(1, 0, 0, 0)));
    ISO_CHECK(a1::evaluate_condition(a1::COND_LE, mkf(1, 0, 0, 0)));
    ISO_CHECK(a1::evaluate_condition(a1::COND_AL, mkf(0, 0, 0, 0)));
    ISO_CHECK(!a1::evaluate_condition(a1::COND_NV, mkf(0, 0, 0, 0)));

    // ── barrel shifter ───────────────────────────────────────────────────
    {
        ISO_CHECK(a1::barrel_shift(0xFF, a1::SHIFT_LSL, 1, false, false).value ==
                  0x1FE);
        auto r = a1::barrel_shift(1, a1::SHIFT_LSL, 32, false, false);
        ISO_CHECK(r.value == 0 && r.carry);
        r = a1::barrel_shift(0xFF, a1::SHIFT_LSR, 1, false, false);
        ISO_CHECK(r.value == 0x7F && r.carry);
        r = a1::barrel_shift(0x80000000u, a1::SHIFT_LSR, 0, false, false);
        ISO_CHECK(r.value == 0 && r.carry);
        ISO_CHECK(a1::barrel_shift(0x80000000u, a1::SHIFT_ASR, 1, false, false)
                      .value == 0xC0000000u);
        r = a1::barrel_shift(0x80000000u, a1::SHIFT_ASR, 0, false, false);
        ISO_CHECK(r.value == 0xFFFFFFFFu && r.carry);
        ISO_CHECK(a1::barrel_shift(0x0000000Fu, a1::SHIFT_ROR, 4, false, false)
                      .value == 0xF0000000u);
        r = a1::barrel_shift(1, a1::SHIFT_ROR, 0, true, false);  // RRX
        ISO_CHECK(r.value == 0x80000000u && r.carry);
        r = a1::barrel_shift(0xDEADBEEFu, a1::SHIFT_LSL, 0, true, true);
        ISO_CHECK(r.value == 0xDEADBEEFu && r.carry);
    }

    // ── decode immediate ─────────────────────────────────────────────────
    ISO_CHECK(a1::decode_immediate(0xFF, 0).value == 0xFF);
    ISO_CHECK(a1::decode_immediate(0xFF, 4).value == 0xFF000000u);

    // ── ALU ──────────────────────────────────────────────────────────────
    {
        auto r = a1::alu_execute(a1::OP_ADD, 1, 2, false, false, false);
        ISO_CHECK(r.result == 3 && !r.n && !r.z && !r.c && !r.v);
        r = a1::alu_execute(a1::OP_SUB, 5, 5, false, false, false);
        ISO_CHECK(r.result == 0 && r.z && r.c);
        r = a1::alu_execute(a1::OP_SUB, 3, 5, false, false, false);
        ISO_CHECK(r.n && !r.c);
        ISO_CHECK(a1::alu_execute(a1::OP_AND, 0xFF00FF00u, 0x0FF00FF0u, false,
                                  false, false)
                      .result == 0x0F000F00u);
        ISO_CHECK(a1::alu_execute(a1::OP_MVN, 0, 0, false, false, false)
                      .result == 0xFFFFFFFFu);
        r = a1::alu_execute(a1::OP_TST, 0xFF, 0x0F, false, false, false);
        ISO_CHECK(!r.write_result && r.result == 0x0F);
        ISO_CHECK(a1::alu_execute(a1::OP_ADC, 0, 0, true, false, false).result ==
                  1);
        ISO_CHECK(a1::alu_execute(a1::OP_SBC, 5, 3, false, false, false)
                      .result == 1);
        ISO_CHECK(a1::alu_execute(a1::OP_RSB, 3, 10, false, false, false)
                      .result == 7);
        r = a1::alu_execute(a1::OP_ADD, 0x7FFFFFFFu, 1, false, false, false);
        ISO_CHECK(r.v && r.n);
    }

    // ── decoder ──────────────────────────────────────────────────────────
    {
        auto d = a1::decode(a1::encode_mov_imm(a1::COND_AL, 0, 42));
        ISO_CHECK(d.inst_type == a1::InstType::DataProcessing &&
                  d.opcode == a1::OP_MOV && d.immediate && d.imm8 == 42 &&
                  d.rd == 0);
        d = a1::decode(a1::encode_alu_reg(a1::COND_AL, a1::OP_ADD, 1, 2, 0, 1));
        ISO_CHECK(d.opcode == a1::OP_ADD && d.s && d.rd == 2 && d.rn == 0 &&
                  d.rm == 1);
        d = a1::decode(a1::encode_branch(a1::COND_NE, false, -16));
        ISO_CHECK(d.inst_type == a1::InstType::Branch && !d.link &&
                  d.branch_offset == -16);
        d = a1::decode(a1::encode_halt());
        ISO_CHECK(d.inst_type == a1::InstType::SWI &&
                  d.swi_comment == a1::HALT_SWI);
        d = a1::decode(a1::encode_ldr(a1::COND_AL, 0, 1, 4, true));
        ISO_CHECK(d.inst_type == a1::InstType::LoadStore && d.load &&
                  d.pre_index && d.up && d.offset12 == 4);
        d = a1::decode(a1::encode_ldm(a1::COND_AL, 13, 0x000F, true, "IA"));
        ISO_CHECK(d.inst_type == a1::InstType::BlockTransfer && d.load &&
                  d.write_back && d.register_list == 0x000F);
    }

    // ── disassembly ──────────────────────────────────────────────────────
    ISO_CHECK(a1::decode(a1::encode_mov_imm(a1::COND_AL, 0, 42)).disassemble() ==
              "MOV R0, #42");
    ISO_CHECK(a1::decode(a1::encode_halt()).disassemble() == "HLT");

    // ── CPU state / reset / memory ───────────────────────────────────────
    {
        a1::ARM1 cpu(1024);
        ISO_CHECK(cpu.mode() == a1::MODE_SVC && cpu.pc() == 0 && !cpu.halted());
        cpu.write_register(0, 42);
        cpu.set_pc(100);
        cpu.reset();
        ISO_CHECK(cpu.read_register(0) == 0 && cpu.pc() == 0 &&
                  cpu.mode() == a1::MODE_SVC);
    }
    {
        a1::ARM1 cpu(4096);
        cpu.write_word(0x100, 0xDEADBEEFu);
        ISO_CHECK_EQ_UINT(cpu.read_word(0x100), 0xDEADBEEFu);
        cpu.write_byte(0x50, 0xAB);
        ISO_CHECK_EQ_INT(cpu.read_byte(0x50), 0xAB);
    }
    {
        a1::ARM1 cpu(256);
        ISO_CHECK_EQ_UINT(cpu.read_word(0x1000), 0u);
        ISO_CHECK_EQ_INT(cpu.read_byte(0x1000), 0);
    }
    {
        a1::ARM1 cpu(4096);
        cpu.set_pc(0xFFFFFFFCu);
        ISO_CHECK_EQ_UINT(cpu.pc(), 0x03FFFFFCu);
    }

    // ── full program execution ───────────────────────────────────────────
    {
        a1::ARM1 cpu(4096);
        cpu.load_program_words(
            {a1::encode_mov_imm(a1::COND_AL, 0, 42), a1::encode_halt()}, 0);
        auto traces = cpu.run(100);
        ISO_CHECK_EQ_UINT(traces.size(), 2u);
        ISO_CHECK_EQ_UINT(cpu.read_register(0), 42u);
        ISO_CHECK(cpu.halted());
    }
    {
        a1::ARM1 cpu(4096);
        cpu.load_program_words({a1::encode_mov_imm(a1::COND_AL, 0, 10),
                                a1::encode_mov_imm(a1::COND_AL, 1, 20),
                                a1::encode_alu_reg(a1::COND_AL, a1::OP_ADD, 0, 2,
                                                   0, 1),
                                a1::encode_halt()},
                               0);
        cpu.run(100);
        ISO_CHECK_EQ_UINT(cpu.read_register(2), 30u);
    }
    {
        a1::ARM1 cpu(4096);
        cpu.load_program_words({a1::encode_mov_imm(a1::COND_AL, 0, 5),
                                a1::encode_mov_imm(a1::COND_AL, 1, 5),
                                a1::encode_alu_reg(a1::COND_AL, a1::OP_SUB, 1, 2,
                                                   0, 1),
                                a1::encode_halt()},
                               0);
        cpu.run(100);
        ISO_CHECK_EQ_UINT(cpu.read_register(2), 0u);
        ISO_CHECK(cpu.flags().z);
    }
    {
        a1::ARM1 cpu(4096);
        cpu.load_program_words({a1::encode_mov_imm(a1::COND_AL, 0, 5),
                                a1::encode_mov_imm(a1::COND_AL, 1, 5),
                                a1::encode_alu_reg(a1::COND_AL, a1::OP_SUB, 1, 2,
                                                   0, 1),
                                a1::encode_mov_imm(a1::COND_NE, 3, 99),
                                a1::encode_mov_imm(a1::COND_EQ, 4, 42),
                                a1::encode_halt()},
                               0);
        cpu.run(100);
        ISO_CHECK_EQ_UINT(cpu.read_register(3), 0u);
        ISO_CHECK_EQ_UINT(cpu.read_register(4), 42u);
    }
    {
        a1::ARM1 cpu(4096);
        cpu.load_program_words({a1::encode_mov_imm(a1::COND_AL, 0, 1),
                                a1::encode_branch(a1::COND_AL, false, 4),
                                a1::encode_mov_imm(a1::COND_AL, 0, 99),
                                a1::encode_halt()},
                               0);
        cpu.run(100);
        ISO_CHECK_EQ_UINT(cpu.read_register(0), 1u);
    }
    {
        a1::ARM1 cpu(4096);
        cpu.load_program_words(
            {a1::encode_mov_imm(a1::COND_AL, 0, 0),
             a1::encode_mov_imm(a1::COND_AL, 1, 10),
             a1::encode_alu_reg(a1::COND_AL, a1::OP_ADD, 0, 0, 0, 1),
             a1::encode_data_processing(a1::COND_AL, a1::OP_SUB, 1, 1, 1,
                                        (1u << 25) | 1),
             a1::encode_branch(a1::COND_NE, false, -16), a1::encode_halt()},
            0);
        cpu.run(200);
        ISO_CHECK_EQ_UINT(cpu.read_register(0), 55u);
    }
    {
        a1::ARM1 cpu(4096);
        cpu.load_program_words(
            {a1::encode_mov_imm(a1::COND_AL, 0, 42),
             a1::encode_data_processing(a1::COND_AL, a1::OP_MOV, 0, 0, 1,
                                        (1u << 25) | (12u << 8) | 1),
             a1::encode_str(a1::COND_AL, 0, 1, 0, true),
             a1::encode_mov_imm(a1::COND_AL, 0, 0),
             a1::encode_ldr(a1::COND_AL, 0, 1, 0, true), a1::encode_halt()},
            0);
        cpu.run(100);
        ISO_CHECK_EQ_UINT(cpu.read_register(0), 42u);
    }
    {
        a1::ARM1 cpu(4096);
        std::uint32_t setr5 = a1::encode_data_processing(
            a1::COND_AL, a1::OP_MOV, 0, 0, 5, (1u << 25) | (12u << 8) | 1);
        cpu.load_program_words({a1::encode_mov_imm(a1::COND_AL, 0, 10),
                                a1::encode_mov_imm(a1::COND_AL, 1, 20),
                                a1::encode_mov_imm(a1::COND_AL, 2, 30),
                                a1::encode_mov_imm(a1::COND_AL, 3, 40),
                                setr5,
                                a1::encode_stm(a1::COND_AL, 5, 0x000F, true,
                                               "IA"),
                                a1::encode_mov_imm(a1::COND_AL, 0, 0),
                                a1::encode_mov_imm(a1::COND_AL, 1, 0),
                                a1::encode_mov_imm(a1::COND_AL, 2, 0),
                                a1::encode_mov_imm(a1::COND_AL, 3, 0),
                                setr5,
                                a1::encode_ldm(a1::COND_AL, 5, 0x000F, true,
                                               "IA"),
                                a1::encode_halt()},
                               0);
        cpu.run(100);
        ISO_CHECK_EQ_UINT(cpu.read_register(0), 10u);
        ISO_CHECK_EQ_UINT(cpu.read_register(1), 20u);
        ISO_CHECK_EQ_UINT(cpu.read_register(2), 30u);
        ISO_CHECK_EQ_UINT(cpu.read_register(3), 40u);
    }
    {
        a1::ARM1 cpu(4096);
        cpu.load_program_words(
            {a1::encode_mov_imm(a1::COND_AL, 0, 7),
             a1::encode_branch(a1::COND_AL, true, 4), a1::encode_halt(), 0,
             a1::encode_alu_reg(a1::COND_AL, a1::OP_ADD, 0, 0, 0, 0),
             a1::encode_data_processing(a1::COND_AL, a1::OP_MOV, 1, 0, 15, 14)},
            0);
        cpu.run(20);
        ISO_CHECK_EQ_UINT(cpu.read_register(0), 14u);
    }
    {
        a1::ARM1 cpu(4096);
        std::uint32_t add_shift =
            ((a1::COND_AL << 28) | (a1::OP_ADD << 21)) | (1u << 12) |
            (2u << 7) | (a1::SHIFT_LSL << 5);
        cpu.load_program_words({a1::encode_mov_imm(a1::COND_AL, 0, 7), add_shift,
                                a1::encode_halt()},
                               0);
        cpu.run(100);
        ISO_CHECK_EQ_UINT(cpu.read_register(1), 35u);
    }
    {
        a1::ARM1 cpu(4096);
        cpu.load_program_words({a1::encode_mov_imm(a1::COND_AL, 0, 10),
                                a1::encode_mov_imm(a1::COND_AL, 1, 5),
                                a1::encode_alu_reg(a1::COND_AL, a1::OP_CMP, 1, 0,
                                                   0, 1),
                                a1::encode_mov_imm(a1::COND_GT, 2, 1),
                                a1::encode_mov_imm(a1::COND_LE, 2, 0),
                                a1::encode_halt()},
                               0);
        cpu.run(100);
        ISO_CHECK_EQ_UINT(cpu.read_register(2), 1u);
    }
    {
        a1::ARM1 cpu(4096);
        cpu.load_program_words(
            {a1::encode_mov_imm(a1::COND_AL, 0, 99),
             a1::encode_data_processing(a1::COND_AL, a1::OP_MOV, 0, 0, 1,
                                        (1u << 25) | (12u << 8) | 1),
             a1::encode_str(a1::COND_AL, 0, 1, 0, true), a1::encode_halt()},
            0);
        auto traces = cpu.run(100);
        ISO_CHECK(traces.size() >= 3);
        ISO_CHECK(!traces[2].memory_writes.empty());
        ISO_CHECK_EQ_UINT(traces[2].memory_writes[0].value, 99u);
    }
    {
        a1::ARM1 cpu(4096);
        cpu.load_program_words({a1::encode_halt()}, 0x08);
        std::uint32_t swi = (a1::COND_AL << 28) | 0x0F000000u | 0x42;
        cpu.load_program_words({swi}, 0);
        cpu.run(10);
        ISO_CHECK(cpu.halted());
    }

    // ── register banking (SVC banks R13/R14) ─────────────────────────────
    {
        a1::ARM1 cpu(1024);
        cpu.write_register(13, 0xAAAA);
        cpu.write_register(14, 0xBBBB);
        ISO_CHECK_EQ_UINT(cpu.read_register(13), 0xAAAAu);
        ISO_CHECK_EQ_UINT(cpu.read_register(14), 0xBBBBu);
        ISO_CHECK_EQ_UINT(cpu.r15_raw() & a1::MODE_MASK, a1::MODE_SVC);
    }

    return ISO_TEST_RESULT();
}
