"""Tests for the RISC-V RV32I + M-mode simulator.

Ports all 62 Go tests faithfully.
"""

import pytest

from riscv_simulator.csr import (
    CAUSE_ECALL_M_MODE,
    CSR_MCAUSE,
    CSR_MEPC,
    CSR_MSCRATCH,
    CSR_MSTATUS,
    CSR_MTVEC,
    CSRFile,
    MIE,
)
from riscv_simulator.decode import RiscVDecoder
from riscv_simulator.encoding import (
    assemble,
    encode_add,
    encode_addi,
    encode_and,
    encode_andi,
    encode_auipc,
    encode_beq,
    encode_bge,
    encode_bgeu,
    encode_blt,
    encode_bltu,
    encode_bne,
    encode_csrrc,
    encode_csrrs,
    encode_csrrw,
    encode_ecall,
    encode_jal,
    encode_jalr,
    encode_lb,
    encode_lbu,
    encode_lh,
    encode_lhu,
    encode_lui,
    encode_lw,
    encode_mret,
    encode_or,
    encode_ori,
    encode_sb,
    encode_sh,
    encode_sll,
    encode_slli,
    encode_slt,
    encode_slti,
    encode_sltiu,
    encode_sltu,
    encode_sra,
    encode_srai,
    encode_srl,
    encode_srli,
    encode_sub,
    encode_sw,
    encode_xor,
    encode_xori,
)
from riscv_simulator.simulator import RiscVSimulator


# === Helpers ===

def run_program(instructions: list[int]) -> RiscVSimulator:
    sim = RiscVSimulator(65536)
    program = assemble(instructions)
    sim.run(program)
    return sim


def expect_reg(sim: RiscVSimulator, reg: int, expected: int) -> None:
    got = sim.cpu.registers.read(reg)
    assert got == (expected & 0xFFFFFFFF), (
        f"x{reg}: expected {expected} (0x{expected & 0xFFFFFFFF:08x}), "
        f"got {got} (0x{got:08x})"
    )


def expect_reg_signed(sim: RiscVSimulator, reg: int, expected: int) -> None:
    got = sim.cpu.registers.read(reg)
    got_signed = got if got < 0x80000000 else got - 0x100000000
    assert got_signed == expected, f"x{reg}: expected {expected}, got {got_signed}"


# =============================================================================
# I-type arithmetic instructions
# =============================================================================

class TestITypeArithmetic:
    def test_addi(self) -> None:
        sim = run_program([
            encode_addi(1, 0, 42),
            encode_addi(2, 1, 10),
            encode_addi(3, 0, -5),
            encode_addi(4, 3, 3),
            encode_ecall(),
        ])
        expect_reg(sim, 1, 42)
        expect_reg(sim, 2, 52)
        expect_reg_signed(sim, 3, -5)
        expect_reg_signed(sim, 4, -2)

    def test_slti(self) -> None:
        sim = run_program([
            encode_addi(1, 0, 5),
            encode_slti(2, 1, 10),
            encode_slti(3, 1, 3),
            encode_slti(4, 1, 5),
            encode_addi(5, 0, -1),
            encode_slti(6, 5, 0),
            encode_ecall(),
        ])
        expect_reg(sim, 2, 1)
        expect_reg(sim, 3, 0)
        expect_reg(sim, 4, 0)
        expect_reg(sim, 6, 1)

    def test_sltiu(self) -> None:
        sim = run_program([
            encode_addi(1, 0, 5),
            encode_sltiu(2, 1, 10),
            encode_sltiu(3, 1, 3),
            encode_addi(4, 0, -1),
            encode_sltiu(5, 4, 1),
            encode_ecall(),
        ])
        expect_reg(sim, 2, 1)
        expect_reg(sim, 3, 0)
        expect_reg(sim, 5, 0)

    def test_xori(self) -> None:
        sim = run_program([
            encode_addi(1, 0, 0xFF),
            encode_xori(2, 1, 0x0F),
            encode_ecall(),
        ])
        expect_reg(sim, 2, 0xF0)

    def test_ori(self) -> None:
        sim = run_program([
            encode_addi(1, 0, 0x50),
            encode_ori(2, 1, 0x0F),
            encode_ecall(),
        ])
        expect_reg(sim, 2, 0x5F)

    def test_andi(self) -> None:
        sim = run_program([
            encode_addi(1, 0, 0xFF),
            encode_andi(2, 1, 0x0F),
            encode_ecall(),
        ])
        expect_reg(sim, 2, 0x0F)

    def test_slli(self) -> None:
        sim = run_program([
            encode_addi(1, 0, 1),
            encode_slli(2, 1, 4),
            encode_slli(3, 1, 31),
            encode_ecall(),
        ])
        expect_reg(sim, 2, 16)
        expect_reg(sim, 3, 0x80000000)

    def test_srli(self) -> None:
        sim = run_program([
            encode_addi(1, 0, -1),
            encode_srli(2, 1, 4),
            encode_srli(3, 1, 31),
            encode_ecall(),
        ])
        expect_reg(sim, 2, 0x0FFFFFFF)
        expect_reg(sim, 3, 1)

    def test_srai(self) -> None:
        sim = run_program([
            encode_addi(1, 0, -16),
            encode_srai(2, 1, 2),
            encode_addi(3, 0, 16),
            encode_srai(4, 3, 2),
            encode_ecall(),
        ])
        expect_reg_signed(sim, 2, -4)
        expect_reg(sim, 4, 4)


# =============================================================================
# R-type arithmetic instructions
# =============================================================================

class TestRTypeArithmetic:
    def test_add_sub(self) -> None:
        sim = run_program([
            encode_addi(1, 0, 10),
            encode_addi(2, 0, 20),
            encode_add(3, 1, 2),
            encode_sub(4, 1, 2),
            encode_ecall(),
        ])
        expect_reg(sim, 3, 30)
        expect_reg_signed(sim, 4, -10)

    def test_sll(self) -> None:
        sim = run_program([
            encode_addi(1, 0, 1),
            encode_addi(2, 0, 8),
            encode_sll(3, 1, 2),
            encode_ecall(),
        ])
        expect_reg(sim, 3, 256)

    def test_slt(self) -> None:
        sim = run_program([
            encode_addi(1, 0, -5),
            encode_addi(2, 0, 3),
            encode_slt(3, 1, 2),
            encode_slt(4, 2, 1),
            encode_ecall(),
        ])
        expect_reg(sim, 3, 1)
        expect_reg(sim, 4, 0)

    def test_sltu(self) -> None:
        sim = run_program([
            encode_addi(1, 0, -1),
            encode_addi(2, 0, 1),
            encode_sltu(3, 2, 1),
            encode_sltu(4, 1, 2),
            encode_ecall(),
        ])
        expect_reg(sim, 3, 1)
        expect_reg(sim, 4, 0)

    def test_xor(self) -> None:
        sim = run_program([
            encode_addi(1, 0, 0xFF),
            encode_addi(2, 0, 0x0F),
            encode_xor(3, 1, 2),
            encode_ecall(),
        ])
        expect_reg(sim, 3, 0xF0)

    def test_srl(self) -> None:
        sim = run_program([
            encode_addi(1, 0, -1),
            encode_addi(2, 0, 4),
            encode_srl(3, 1, 2),
            encode_ecall(),
        ])
        expect_reg(sim, 3, 0x0FFFFFFF)

    def test_sra(self) -> None:
        sim = run_program([
            encode_addi(1, 0, -16),
            encode_addi(2, 0, 2),
            encode_sra(3, 1, 2),
            encode_ecall(),
        ])
        expect_reg_signed(sim, 3, -4)

    def test_or(self) -> None:
        sim = run_program([
            encode_addi(1, 0, 0x50),
            encode_addi(2, 0, 0x0F),
            encode_or(3, 1, 2),
            encode_ecall(),
        ])
        expect_reg(sim, 3, 0x5F)

    def test_and(self) -> None:
        sim = run_program([
            encode_addi(1, 0, 0xFF),
            encode_addi(2, 0, 0x0F),
            encode_and(3, 1, 2),
            encode_ecall(),
        ])
        expect_reg(sim, 3, 0x0F)


# =============================================================================
# Load and store instructions
# =============================================================================

class TestLoadStore:
    def test_store_word_load_word(self) -> None:
        sim = run_program([
            encode_addi(1, 0, 0x100),
            encode_addi(2, 0, 0x42),
            encode_sw(2, 1, 0),
            encode_lw(3, 1, 0),
            encode_ecall(),
        ])
        expect_reg(sim, 3, 0x42)

    def test_store_byte_load_byte(self) -> None:
        sim = run_program([
            encode_addi(1, 0, 0x200),
            encode_addi(2, 0, 0xAB),
            encode_sb(2, 1, 0),
            encode_lbu(3, 1, 0),
            encode_ecall(),
        ])
        expect_reg(sim, 3, 0xAB)

    def test_load_byte_sign_extend(self) -> None:
        sim = run_program([
            encode_addi(1, 0, 0x200),
            encode_addi(2, 0, 0xFF),
            encode_sb(2, 1, 0),
            encode_lb(3, 1, 0),
            encode_lbu(4, 1, 0),
            encode_ecall(),
        ])
        expect_reg_signed(sim, 3, -1)
        expect_reg(sim, 4, 0xFF)

    def test_store_half_load_half(self) -> None:
        sim = run_program([
            encode_addi(1, 0, 0x200),
            encode_lui(2, 0),
            encode_addi(2, 0, 0x1FF),
            encode_sh(2, 1, 0),
            encode_lhu(3, 1, 0),
            encode_ecall(),
        ])
        expect_reg(sim, 3, 0x1FF)

    def test_load_half_sign_extend(self) -> None:
        sim = run_program([
            encode_addi(1, 0, 0x200),
            encode_addi(2, 0, -1),
            encode_sh(2, 1, 0),
            encode_lh(3, 1, 0),
            encode_lhu(4, 1, 0),
            encode_ecall(),
        ])
        expect_reg_signed(sim, 3, -1)
        expect_reg(sim, 4, 0xFFFF)

    def test_store_load_with_offset(self) -> None:
        sim = run_program([
            encode_addi(1, 0, 0x200),
            encode_addi(2, 0, 99),
            encode_sw(2, 1, 4),
            encode_lw(3, 1, 4),
            encode_ecall(),
        ])
        expect_reg(sim, 3, 99)


# =============================================================================
# Branch instructions
# =============================================================================

class TestBranches:
    def test_beq_taken(self) -> None:
        sim = run_program([
            encode_addi(1, 0, 5),
            encode_addi(2, 0, 5),
            encode_beq(1, 2, 8),
            encode_addi(3, 0, 999),
            encode_addi(4, 0, 42),
            encode_ecall(),
        ])
        expect_reg(sim, 3, 0)
        expect_reg(sim, 4, 42)

    def test_beq_not_taken(self) -> None:
        sim = run_program([
            encode_addi(1, 0, 5),
            encode_addi(2, 0, 10),
            encode_beq(1, 2, 8),
            encode_addi(3, 0, 42),
            encode_ecall(),
        ])
        expect_reg(sim, 3, 42)

    def test_bne(self) -> None:
        sim = run_program([
            encode_addi(1, 0, 5),
            encode_addi(2, 0, 10),
            encode_bne(1, 2, 8),
            encode_addi(3, 0, 999),
            encode_addi(4, 0, 42),
            encode_ecall(),
        ])
        expect_reg(sim, 3, 0)
        expect_reg(sim, 4, 42)

    def test_blt(self) -> None:
        sim = run_program([
            encode_addi(1, 0, -5),
            encode_addi(2, 0, 3),
            encode_blt(1, 2, 8),
            encode_addi(3, 0, 999),
            encode_addi(4, 0, 42),
            encode_ecall(),
        ])
        expect_reg(sim, 3, 0)
        expect_reg(sim, 4, 42)

    def test_bge(self) -> None:
        sim = run_program([
            encode_addi(1, 0, 5),
            encode_addi(2, 0, 5),
            encode_bge(1, 2, 8),
            encode_addi(3, 0, 999),
            encode_addi(4, 0, 42),
            encode_ecall(),
        ])
        expect_reg(sim, 3, 0)
        expect_reg(sim, 4, 42)

    def test_bltu(self) -> None:
        sim = run_program([
            encode_addi(1, 0, 1),
            encode_addi(2, 0, -1),
            encode_bltu(1, 2, 8),
            encode_addi(3, 0, 999),
            encode_addi(4, 0, 42),
            encode_ecall(),
        ])
        expect_reg(sim, 3, 0)
        expect_reg(sim, 4, 42)

    def test_bgeu(self) -> None:
        sim = run_program([
            encode_addi(1, 0, -1),
            encode_addi(2, 0, 1),
            encode_bgeu(1, 2, 8),
            encode_addi(3, 0, 999),
            encode_addi(4, 0, 42),
            encode_ecall(),
        ])
        expect_reg(sim, 3, 0)
        expect_reg(sim, 4, 42)

    def test_branch_backward(self) -> None:
        sim = run_program([
            encode_addi(1, 0, 0),
            encode_addi(2, 0, 3),
            encode_addi(1, 1, 1),
            encode_bne(1, 2, -4),
            encode_ecall(),
        ])
        expect_reg(sim, 1, 3)


# =============================================================================
# Jump instructions
# =============================================================================

class TestJumps:
    def test_jal(self) -> None:
        sim = run_program([
            encode_jal(1, 8),
            encode_addi(2, 0, 999),
            encode_addi(3, 0, 42),
            encode_ecall(),
        ])
        expect_reg(sim, 1, 4)
        expect_reg(sim, 2, 0)
        expect_reg(sim, 3, 42)

    def test_jalr(self) -> None:
        sim = run_program([
            encode_addi(5, 0, 12),
            encode_jalr(1, 5, 0),
            encode_addi(2, 0, 999),
            encode_addi(3, 0, 42),
            encode_ecall(),
        ])
        expect_reg(sim, 1, 8)
        expect_reg(sim, 2, 0)
        expect_reg(sim, 3, 42)

    def test_jalr_with_offset(self) -> None:
        sim = run_program([
            encode_addi(5, 0, 8),
            encode_jalr(1, 5, 4),
            encode_addi(2, 0, 999),
            encode_addi(3, 0, 42),
            encode_ecall(),
        ])
        expect_reg(sim, 1, 8)
        expect_reg(sim, 2, 0)
        expect_reg(sim, 3, 42)

    def test_call_and_return(self) -> None:
        sim = run_program([
            encode_jal(1, 12),
            encode_addi(11, 0, 99),
            encode_ecall(),
            encode_addi(10, 0, 42),
            encode_jalr(0, 1, 0),
        ])
        expect_reg(sim, 1, 4)
        expect_reg(sim, 10, 42)
        expect_reg(sim, 11, 99)


# =============================================================================
# LUI and AUIPC
# =============================================================================

class TestUpperImmediate:
    def test_lui(self) -> None:
        sim = run_program([
            encode_lui(1, 0x12345),
            encode_ecall(),
        ])
        expect_reg(sim, 1, 0x12345000)

    def test_lui_plus_addi(self) -> None:
        sim = run_program([
            encode_lui(1, 0x12345),
            encode_addi(1, 1, 0x678),
            encode_ecall(),
        ])
        expect_reg(sim, 1, 0x12345678)

    def test_auipc(self) -> None:
        sim = run_program([
            encode_auipc(1, 1),
            encode_ecall(),
        ])
        expect_reg(sim, 1, 0x1000)

    def test_auipc_non_zero_pc(self) -> None:
        sim = run_program([
            encode_addi(0, 0, 0),  # nop
            encode_auipc(1, 2),
            encode_ecall(),
        ])
        expect_reg(sim, 1, 0x2004)


# =============================================================================
# Register x0 hardwired to zero
# =============================================================================

class TestRegisterZero:
    def test_register_zero_hardwired(self) -> None:
        sim = run_program([
            encode_addi(0, 0, 42),
            encode_ecall(),
        ])
        expect_reg(sim, 0, 0)

    def test_register_zero_on_r_type(self) -> None:
        sim = run_program([
            encode_addi(1, 0, 5),
            encode_addi(2, 0, 10),
            encode_add(0, 1, 2),
            encode_ecall(),
        ])
        expect_reg(sim, 0, 0)


# =============================================================================
# CSR operations
# =============================================================================

class TestCSR:
    def test_csrrw(self) -> None:
        sim = RiscVSimulator(65536)
        program = assemble([
            encode_addi(1, 0, 0x100),
            encode_csrrw(2, CSR_MSCRATCH, 1),
            encode_csrrw(3, CSR_MSCRATCH, 0),
            encode_ecall(),
        ])
        sim.run(program)
        expect_reg(sim, 2, 0)
        expect_reg(sim, 3, 0x100)

    def test_csrrs(self) -> None:
        sim = RiscVSimulator(65536)
        program = assemble([
            encode_addi(1, 0, 8),
            encode_csrrs(2, CSR_MSTATUS, 1),
            encode_csrrs(3, CSR_MSTATUS, 0),
            encode_ecall(),
        ])
        sim.run(program)
        expect_reg(sim, 2, 0)
        expect_reg(sim, 3, 8)

    def test_csrrc(self) -> None:
        sim = RiscVSimulator(65536)
        program = assemble([
            encode_addi(1, 0, 0xFF),
            encode_csrrw(0, CSR_MSCRATCH, 1),
            encode_addi(2, 0, 0x0F),
            encode_csrrc(3, CSR_MSCRATCH, 2),
            encode_csrrs(4, CSR_MSCRATCH, 0),
            encode_ecall(),
        ])
        sim.run(program)
        expect_reg(sim, 3, 0xFF)
        expect_reg(sim, 4, 0xF0)


# =============================================================================
# ecall trap behavior
# =============================================================================

class TestEcallTrap:
    def test_ecall_halt_when_no_trap_handler(self) -> None:
        sim = run_program([
            encode_addi(1, 0, 42),
            encode_ecall(),
        ])
        assert sim.cpu.halted is True
        expect_reg(sim, 1, 42)

    def test_ecall_trap_with_handler(self) -> None:
        sim = RiscVSimulator(65536)

        main_code = [
            encode_addi(1, 0, 0x100),
            encode_csrrw(0, CSR_MTVEC, 1),
            encode_ecall(),
            encode_addi(11, 0, 77),
            encode_csrrw(0, CSR_MTVEC, 0),
            encode_ecall(),
        ]

        pad_count = (0x100 // 4) - len(main_code)
        padded = main_code + [encode_addi(0, 0, 0)] * pad_count

        trap_handler = [
            encode_addi(10, 0, 99),
            encode_csrrs(20, CSR_MEPC, 0),
            encode_addi(20, 20, 4),
            encode_csrrw(0, CSR_MEPC, 20),
            encode_mret(),
        ]
        padded += trap_handler

        program = assemble(padded)
        sim.run(program)

        expect_reg(sim, 10, 99)
        expect_reg(sim, 11, 77)
        assert sim.cpu.halted is True

    def test_ecall_sets_csrs(self) -> None:
        sim = RiscVSimulator(65536)

        main_code = [
            encode_addi(1, 0, 0x200),
            encode_csrrw(0, CSR_MTVEC, 1),
            encode_addi(2, 0, 8),
            encode_csrrs(0, CSR_MSTATUS, 2),
            encode_ecall(),
        ]

        pad_count = (0x200 // 4) - len(main_code)
        padded = main_code + [encode_addi(0, 0, 0)] * pad_count

        trap_handler = [
            encode_csrrs(20, CSR_MEPC, 0),
            encode_csrrs(21, CSR_MCAUSE, 0),
            encode_csrrs(22, CSR_MSTATUS, 0),
            encode_csrrw(0, CSR_MTVEC, 0),
            encode_ecall(),
        ]
        padded += trap_handler

        program = assemble(padded)
        sim.run(program)

        mepc = sim.cpu.registers.read(20)
        mcause = sim.cpu.registers.read(21)
        mstatus = sim.cpu.registers.read(22)

        assert mepc == 16
        assert mcause == CAUSE_ECALL_M_MODE
        assert mstatus & MIE == 0


# =============================================================================
# mret
# =============================================================================

class TestMret:
    def test_mret(self) -> None:
        sim = RiscVSimulator(65536)
        sim.csr.write(CSR_MEPC, 12)

        program = assemble([
            encode_mret(),
            encode_addi(1, 0, 999),
            encode_addi(2, 0, 999),
            encode_addi(3, 0, 42),
            encode_ecall(),
        ])
        sim.run(program)

        expect_reg(sim, 1, 0)
        expect_reg(sim, 2, 0)
        expect_reg(sim, 3, 42)

    def test_mret_reenables_interrupts(self) -> None:
        sim = RiscVSimulator(65536)
        sim.csr.write(CSR_MSTATUS, 0)
        sim.csr.write(CSR_MEPC, 4)

        program = assemble([
            encode_mret(),
            encode_ecall(),
        ])
        sim.run(program)

        mstatus = sim.csr.read(CSR_MSTATUS)
        assert mstatus & MIE != 0


# =============================================================================
# Unknown instruction handling
# =============================================================================

class TestUnknown:
    def test_unknown_instruction(self) -> None:
        sim = run_program([
            0xFFFFFFFF,
            encode_ecall(),
        ])
        assert sim.cpu.registers.read(1) == 0


# =============================================================================
# Negative immediate decoding
# =============================================================================

class TestNegativeImmediate:
    def test_negative_immediate(self) -> None:
        sim = run_program([
            encode_addi(1, 0, -5),
            encode_ecall(),
        ])
        expect_reg_signed(sim, 1, -5)


# =============================================================================
# Integration tests
# =============================================================================

class TestIntegration:
    def test_fibonacci(self) -> None:
        sim = run_program([
            encode_addi(1, 0, 0),
            encode_addi(2, 0, 1),
            encode_addi(4, 0, 2),
            encode_addi(5, 0, 11),
            encode_add(3, 1, 2),
            encode_addi(1, 2, 0),
            encode_addi(2, 3, 0),
            encode_addi(4, 4, 1),
            encode_bne(4, 5, -16),
            encode_ecall(),
        ])
        expect_reg(sim, 2, 55)

    def test_memcpy(self) -> None:
        sim = RiscVSimulator(65536)

        sim.cpu.memory.write_byte(0x200, 0xDE)
        sim.cpu.memory.write_byte(0x201, 0xAD)
        sim.cpu.memory.write_byte(0x202, 0xBE)
        sim.cpu.memory.write_byte(0x203, 0xEF)

        program = assemble([
            encode_addi(1, 0, 0x200),
            encode_addi(2, 0, 0x300),
            encode_lw(3, 1, 0),
            encode_sw(3, 2, 0),
            encode_ecall(),
        ])
        sim.run(program)

        for i in range(4):
            src = sim.cpu.memory.read_byte(0x200 + i)
            dst = sim.cpu.memory.read_byte(0x300 + i)
            assert src == dst, f"Byte {i}: src=0x{src:02x}, dst=0x{dst:02x}"

    def test_stack_operations(self) -> None:
        sim = run_program([
            encode_addi(2, 0, 0x400),
            encode_addi(10, 0, 42),
            encode_addi(11, 0, 99),
            encode_addi(2, 2, -4),
            encode_sw(10, 2, 0),
            encode_addi(2, 2, -4),
            encode_sw(11, 2, 0),
            encode_lw(12, 2, 0),
            encode_addi(2, 2, 4),
            encode_lw(13, 2, 0),
            encode_addi(2, 2, 4),
            encode_ecall(),
        ])
        expect_reg(sim, 12, 99)
        expect_reg(sim, 13, 42)
        expect_reg(sim, 2, 0x400)


# =============================================================================
# Step-by-step execution
# =============================================================================

class TestStep:
    def test_step(self) -> None:
        sim = RiscVSimulator(65536)
        program = assemble([
            encode_addi(1, 0, 1),
            encode_addi(2, 0, 2),
            encode_ecall(),
        ])
        sim.cpu.load_program(program)

        trace1 = sim.step()
        assert trace1.decode.mnemonic == "addi"
        expect_reg(sim, 1, 1)

        trace2 = sim.step()
        assert trace2.decode.mnemonic == "addi"
        expect_reg(sim, 2, 2)


# =============================================================================
# Encoding round-trip tests
# =============================================================================

class TestEncodeDecodeRoundTrip:
    def test_round_trip(self) -> None:
        decoder = RiscVDecoder()
        cases = [
            ("addi", encode_addi(1, 2, 42)),
            ("slti", encode_slti(1, 2, -5)),
            ("sltiu", encode_sltiu(1, 2, 5)),
            ("xori", encode_xori(1, 2, 0xFF)),
            ("ori", encode_ori(1, 2, 0xFF)),
            ("andi", encode_andi(1, 2, 0xFF)),
            ("slli", encode_slli(1, 2, 5)),
            ("srli", encode_srli(1, 2, 5)),
            ("srai", encode_srai(1, 2, 5)),
            ("add", encode_add(1, 2, 3)),
            ("sub", encode_sub(1, 2, 3)),
            ("sll", encode_sll(1, 2, 3)),
            ("slt", encode_slt(1, 2, 3)),
            ("sltu", encode_sltu(1, 2, 3)),
            ("xor", encode_xor(1, 2, 3)),
            ("srl", encode_srl(1, 2, 3)),
            ("sra", encode_sra(1, 2, 3)),
            ("or", encode_or(1, 2, 3)),
            ("and", encode_and(1, 2, 3)),
            ("lb", encode_lb(1, 2, 4)),
            ("lh", encode_lh(1, 2, 4)),
            ("lw", encode_lw(1, 2, 4)),
            ("lbu", encode_lbu(1, 2, 4)),
            ("lhu", encode_lhu(1, 2, 4)),
            ("sb", encode_sb(3, 2, 4)),
            ("sh", encode_sh(3, 2, 4)),
            ("sw", encode_sw(3, 2, 4)),
            ("beq", encode_beq(1, 2, 8)),
            ("bne", encode_bne(1, 2, 8)),
            ("blt", encode_blt(1, 2, 8)),
            ("bge", encode_bge(1, 2, 8)),
            ("bltu", encode_bltu(1, 2, 8)),
            ("bgeu", encode_bgeu(1, 2, 8)),
            ("jal", encode_jal(1, 8)),
            ("jalr", encode_jalr(1, 2, 4)),
            ("lui", encode_lui(1, 0x12345)),
            ("auipc", encode_auipc(1, 0x12345)),
            ("ecall", encode_ecall()),
            ("mret", encode_mret()),
            ("csrrw", encode_csrrw(1, 0x300, 2)),
            ("csrrs", encode_csrrs(1, 0x300, 2)),
            ("csrrc", encode_csrrc(1, 0x300, 2)),
        ]
        for name, encoded in cases:
            result = decoder.decode(encoded, 0)
            assert result.mnemonic == name, f"Decode({name}): expected {name}, got {result.mnemonic}"


# =============================================================================
# CSR file unit tests
# =============================================================================

class TestCSRFile:
    def test_read_write(self) -> None:
        csr = CSRFile()
        assert csr.read(CSR_MSTATUS) == 0
        csr.write(CSR_MSTATUS, 0x1234)
        assert csr.read(CSR_MSTATUS) == 0x1234

    def test_read_write_atomic(self) -> None:
        csr = CSRFile()
        csr.write(CSR_MSCRATCH, 42)
        old = csr.read_write(CSR_MSCRATCH, 99)
        assert old == 42
        assert csr.read(CSR_MSCRATCH) == 99

    def test_read_set(self) -> None:
        csr = CSRFile()
        csr.write(CSR_MSTATUS, 0xF0)
        old = csr.read_set(CSR_MSTATUS, 0x0F)
        assert old == 0xF0
        assert csr.read(CSR_MSTATUS) == 0xFF

    def test_read_clear(self) -> None:
        csr = CSRFile()
        csr.write(CSR_MSTATUS, 0xFF)
        old = csr.read_clear(CSR_MSTATUS, 0x0F)
        assert old == 0xFF
        assert csr.read(CSR_MSTATUS) == 0xF0


# =============================================================================
# Edge cases
# =============================================================================

class TestEdgeCases:
    def test_shift_amount_masking(self) -> None:
        sim = run_program([
            encode_addi(1, 0, 1),
            encode_addi(2, 0, 33),
            encode_sll(3, 1, 2),
            encode_ecall(),
        ])
        expect_reg(sim, 3, 2)

    def test_assemble(self) -> None:
        data = assemble([0x12345678])
        assert len(data) == 4
        assert data[0] == 0x78
        assert data[1] == 0x56
        assert data[2] == 0x34
        assert data[3] == 0x12
