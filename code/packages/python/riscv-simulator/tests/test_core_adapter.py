"""Tests for the RISC-V Core adapter -- port of the Go test suite."""

from __future__ import annotations

from cpu_simulator.sparse_memory import MemoryRegion, SparseMemory
from riscv_simulator.core_adapter import RiscVISADecoder, new_riscv_core
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


# === Helpers ===


class SimpleRegisterFile:
    """Minimal register file for testing the adapter in isolation."""

    def __init__(self, count: int = 32) -> None:
        self._regs = [0] * count

    def read(self, index: int) -> int:
        if index < 0 or index >= len(self._regs):
            return 0
        return self._regs[index]

    def write(self, index: int, value: int) -> None:
        if 0 < index < len(self._regs):
            self._regs[index] = value


def make_token(pc: int = 0) -> dict:
    """Create a fresh pipeline token dict."""
    return {
        "pc": pc,
        "opcode": "",
        "rd": -1,
        "rs1": -1,
        "rs2": -1,
        "immediate": 0,
        "alu_result": 0,
        "write_data": 0,
        "branch_taken": False,
        "branch_target": 0,
        "reg_write": False,
        "mem_read": False,
        "mem_write": False,
        "is_branch": False,
        "is_halt": False,
        "raw_instruction": 0,
    }


# === Test: ISA decoder interface ===


class TestRiscVISADecoder:
    def test_instruction_size(self) -> None:
        decoder = RiscVISADecoder()
        assert decoder.instruction_size() == 4

    def test_csr_accessor(self) -> None:
        decoder = RiscVISADecoder()
        assert decoder.csr is not None

    def test_factory_function(self) -> None:
        decoder = new_riscv_core()
        assert isinstance(decoder, RiscVISADecoder)


# === Test: Decode control signals ===


class TestDecodeControlSignals:
    """Verify that control signals are set correctly for all instruction types."""

    def _check_signals(
        self,
        raw: int,
        *,
        reg_write: bool = False,
        mem_read: bool = False,
        mem_write: bool = False,
        is_branch: bool = False,
        is_halt: bool = False,
    ) -> None:
        decoder = RiscVISADecoder()
        token = make_token()
        decoder.decode(raw, token)
        assert token.get("reg_write", False) == reg_write, f"reg_write mismatch for {token['opcode']}"
        assert token.get("mem_read", False) == mem_read, f"mem_read mismatch for {token['opcode']}"
        assert token.get("mem_write", False) == mem_write, f"mem_write mismatch for {token['opcode']}"
        assert token.get("is_branch", False) == is_branch, f"is_branch mismatch for {token['opcode']}"
        assert token.get("is_halt", False) == is_halt, f"is_halt mismatch for {token['opcode']}"

    def test_r_type_arithmetic(self) -> None:
        for enc in [encode_add, encode_sub, encode_sll, encode_slt, encode_sltu,
                     encode_xor, encode_srl, encode_sra, encode_or, encode_and]:
            self._check_signals(enc(3, 1, 2), reg_write=True)

    def test_i_type_arithmetic(self) -> None:
        for enc in [encode_addi, encode_slti, encode_sltiu, encode_xori, encode_ori, encode_andi]:
            self._check_signals(enc(1, 2, 5), reg_write=True)
        for enc in [encode_slli, encode_srli, encode_srai]:
            self._check_signals(enc(1, 2, 3), reg_write=True)

    def test_upper_immediate(self) -> None:
        self._check_signals(encode_lui(1, 0x12345), reg_write=True)
        self._check_signals(encode_auipc(1, 0x12345), reg_write=True)

    def test_loads(self) -> None:
        for enc in [encode_lb, encode_lh, encode_lw, encode_lbu, encode_lhu]:
            self._check_signals(enc(1, 2, 0), reg_write=True, mem_read=True)

    def test_stores(self) -> None:
        for enc in [encode_sb, encode_sh, encode_sw]:
            self._check_signals(enc(1, 2, 0), mem_write=True)

    def test_branches(self) -> None:
        for enc in [encode_beq, encode_bne, encode_blt, encode_bge, encode_bltu, encode_bgeu]:
            self._check_signals(enc(1, 2, 8), is_branch=True)

    def test_jumps(self) -> None:
        self._check_signals(encode_jal(1, 8), reg_write=True, is_branch=True)
        self._check_signals(encode_jalr(1, 2, 0), reg_write=True, is_branch=True)

    def test_ecall(self) -> None:
        self._check_signals(encode_ecall(), is_halt=True)

    def test_csr_instructions(self) -> None:
        self._check_signals(encode_csrrw(1, 0x300, 2), reg_write=True)
        self._check_signals(encode_csrrs(1, 0x300, 2), reg_write=True)
        self._check_signals(encode_csrrc(1, 0x300, 2), reg_write=True)

    def test_mret(self) -> None:
        self._check_signals(encode_mret(), is_branch=True)


# === Test: Execute via direct decoder call ===


class TestExecuteDirectly:
    def test_alu_operations(self) -> None:
        decoder = RiscVISADecoder()
        regs = SimpleRegisterFile()
        regs.write(1, 10)
        regs.write(2, 3)

        cases = [
            ("add", encode_add(3, 1, 2), 13),
            ("sub", encode_sub(3, 1, 2), 7),
            ("sll", encode_sll(3, 1, 2), 80),
            ("srl", encode_srl(3, 1, 2), 1),
            ("xor", encode_xor(3, 1, 2), 10 ^ 3),
            ("or", encode_or(3, 1, 2), 10 | 3),
            ("and", encode_and(3, 1, 2), 10 & 3),
            ("addi", encode_addi(3, 1, 5), 15),
            ("slli", encode_slli(3, 1, 2), 40),
            ("srli", encode_srli(3, 1, 1), 5),
            ("lui", encode_lui(3, 1), 4096),
            ("lw", encode_lw(3, 1, 4), 14),
            ("sw", encode_sw(2, 1, 8), 18),
        ]

        for name, raw, expected_alu in cases:
            token = make_token()
            decoder.decode(raw, token)
            decoder.execute(token, regs)
            assert token["alu_result"] == expected_alu, (
                f"{name}: expected ALU={expected_alu}, got {token['alu_result']}"
            )

    def test_beq_not_taken(self) -> None:
        decoder = RiscVISADecoder()
        regs = SimpleRegisterFile()
        regs.write(1, 10)
        regs.write(2, 3)

        token = make_token()
        decoder.decode(encode_beq(1, 2, 100), token)
        decoder.execute(token, regs)
        assert token["alu_result"] == 4  # PC+4 since not taken


# === Test: Branch resolution ===


class TestBranchExecution:
    def test_branches(self) -> None:
        decoder = RiscVISADecoder()
        regs = SimpleRegisterFile()
        regs.write(1, 5)
        regs.write(2, 5)
        regs.write(3, 10)

        cases = [
            ("beq_taken", encode_beq(1, 2, 20), True, 20),
            ("beq_not_taken", encode_beq(1, 3, 20), False, 0),
            ("bne_taken", encode_bne(1, 3, 20), True, 20),
            ("bne_not_taken", encode_bne(1, 2, 20), False, 0),
            ("blt_taken", encode_blt(1, 3, 20), True, 20),
            ("blt_not_taken", encode_blt(3, 1, 20), False, 0),
            ("bge_taken", encode_bge(3, 1, 20), True, 20),
            ("bge_not_taken", encode_bge(1, 3, 20), False, 0),
            ("bltu_taken", encode_bltu(1, 3, 20), True, 20),
            ("bgeu_taken", encode_bgeu(3, 1, 20), True, 20),
        ]

        for name, raw, expected_taken, expected_target in cases:
            token = make_token()
            decoder.decode(raw, token)
            decoder.execute(token, regs)
            assert token["branch_taken"] == expected_taken, (
                f"{name}: expected taken={expected_taken}"
            )
            if expected_taken:
                assert token["branch_target"] == expected_target, (
                    f"{name}: expected target={expected_target}"
                )


# === Test: JAL/JALR execution ===


class TestJumpExecution:
    def test_jal(self) -> None:
        decoder = RiscVISADecoder()
        regs = SimpleRegisterFile()

        token = make_token(pc=8)
        decoder.decode(encode_jal(1, 20), token)
        decoder.execute(token, regs)

        assert token["branch_taken"] is True
        assert token["branch_target"] == 28  # PC(8) + 20
        assert token["write_data"] == 12  # PC(8) + 4

    def test_jalr(self) -> None:
        decoder = RiscVISADecoder()
        regs = SimpleRegisterFile()
        regs.write(5, 100)

        token = make_token(pc=16)
        decoder.decode(encode_jalr(1, 5, 8), token)
        decoder.execute(token, regs)

        assert token["branch_taken"] is True
        assert token["branch_target"] == 108  # (100 + 8) & ~1
        assert token["write_data"] == 20  # PC(16) + 4


# === Test: SLT comparison operations ===


class TestSetLessThan:
    def test_slt(self) -> None:
        decoder = RiscVISADecoder()
        regs = SimpleRegisterFile()
        regs.write(1, 5)
        regs.write(2, 10)

        token = make_token()
        decoder.decode(encode_slt(3, 1, 2), token)
        decoder.execute(token, regs)
        assert token["alu_result"] == 1

    def test_sltu(self) -> None:
        decoder = RiscVISADecoder()
        regs = SimpleRegisterFile()
        regs.write(1, 5)
        regs.write(2, 10)

        token = make_token()
        decoder.decode(encode_sltu(3, 1, 2), token)
        decoder.execute(token, regs)
        assert token["alu_result"] == 1


# === Test: SRA (arithmetic right shift) ===


class TestSRA:
    def test_sra_negative(self) -> None:
        decoder = RiscVISADecoder()
        regs = SimpleRegisterFile()
        # -16 in 32-bit two's complement
        regs.write(1, 0xFFFFFFF0)
        regs.write(2, 2)

        token = make_token()
        decoder.decode(encode_sra(3, 1, 2), token)
        decoder.execute(token, regs)
        assert token["alu_result"] == -4  # -16 >> 2 = -4 (arithmetic)


# === Test: getField helper ===


class TestGetField:
    def test_existing_key(self) -> None:
        from riscv_simulator.core_adapter import _get_field
        assert _get_field({"rd": 5, "rs1": 3}, "rd", -1) == 5

    def test_missing_key(self) -> None:
        from riscv_simulator.core_adapter import _get_field
        assert _get_field({"rd": 5}, "rs2", -1) == -1

    def test_empty_map(self) -> None:
        from riscv_simulator.core_adapter import _get_field
        assert _get_field({}, "rd", 42) == 42


# === Test: SparseMemory used as program storage ===


class TestSparseMemoryIntegration:
    def test_sparse_memory_program_storage(self) -> None:
        mem = SparseMemory([
            MemoryRegion(base=0x00000000, size=0x10000, name="RAM"),
            MemoryRegion(base=0xFFFF0000, size=0x100, name="ROM", read_only=True),
        ])

        program = assemble([encode_addi(1, 0, 42), encode_ecall()])
        mem.load_bytes(0, program)

        assert mem.read_word(0) != 0
        assert mem.read_byte(0xFFFF0000) == 0


# === Test: Unknown instruction does not crash ===


class TestUnknownInstruction:
    def test_no_crash(self) -> None:
        decoder = RiscVISADecoder()
        regs = SimpleRegisterFile()

        token = make_token()
        token["opcode"] = "SOMETHING_UNKNOWN"
        # Should not raise
        decoder.execute(token, regs)


# === Test: CSR extraction from token ===


class TestCSRExtraction:
    def test_csrrw_csr_address(self) -> None:
        decoder = RiscVISADecoder()
        regs = SimpleRegisterFile()
        regs.write(2, 42)

        raw = encode_csrrw(1, 0x300, 2)
        token = make_token()
        token["raw_instruction"] = raw
        decoder.decode(raw, token)
        decoder.execute(token, regs)

        # After csrrw, the old value of CSR 0x300 (which was 0) should be in alu_result
        assert token["alu_result"] == 0
        # And the CSR should now contain 42
        assert decoder.csr.read(0x300) == 42
