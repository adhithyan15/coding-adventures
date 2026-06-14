"""Tests for complete programs — arithmetic loops, subroutines, string ops."""

import pytest

from intel8086_gatelevel.simulator import Intel8086GateLevelSimulator


def run(program: bytes, max_steps: int = 5000):
    sim = Intel8086GateLevelSimulator()
    return sim.execute(program, max_steps=max_steps)


class TestBasicPrograms:
    def test_mov_hlt(self):
        result = run(bytes([0xB8, 0x0A, 0x00, 0xF4]))
        assert result.final_state.ax == 10
        assert result.halted

    def test_add_two_numbers(self):
        prog = bytes([
            0xB8, 0x05, 0x00,   # MOV AX, 5
            0x05, 0x03, 0x00,   # ADD AX, 3
            0xF4,
        ])
        result = run(prog)
        assert result.final_state.ax == 8

    def test_subtraction(self):
        prog = bytes([
            0xB8, 0x0A, 0x00,   # MOV AX, 10
            0x2D, 0x03, 0x00,   # SUB AX, 3
            0xF4,
        ])
        result = run(prog)
        assert result.final_state.ax == 7

    def test_load_multiple_registers(self):
        prog = bytes([
            0xB8, 0x01, 0x00,   # MOV AX, 1
            0xBB, 0x02, 0x00,   # MOV BX, 2
            0xB9, 0x03, 0x00,   # MOV CX, 3
            0xBA, 0x04, 0x00,   # MOV DX, 4
            0xF4,
        ])
        result = run(prog)
        s = result.final_state
        assert s.ax == 1
        assert s.bx == 2
        assert s.cx == 3
        assert s.dx == 4

    def test_immediate_to_memory(self):
        prog = bytes([
            0xC7, 0x06, 0x00, 0x02, 0x34, 0x12,  # MOV [0x200], 0x1234
            0xA1, 0x00, 0x02,                      # MOV AX, [0x200]
            0xF4,
        ])
        result = run(prog)
        assert result.final_state.ax == 0x1234


class TestArithmeticLoop:
    def test_sum_1_to_5(self):
        """Sum 1+2+3+4+5 = 15 using a LOOP instruction.

        Memory layout:
            0:  B8 00 00  MOV AX, 0   (3 bytes)
            3:  B9 05 00  MOV CX, 5   (3 bytes)
            6:  BB 05 00  MOV BX, 5   (3 bytes)
            9:  01 D8     ADD AX, BX  (2 bytes) ← loop body
            11: 4B        DEC BX      (1 byte)
            12: E2 FB     LOOP -5     (2 bytes) → target = 14 + (-5) = 9
            14: F4        HLT
        """
        prog = bytes([
            0xB8, 0x00, 0x00,   # MOV AX, 0  (accumulator)
            0xB9, 0x05, 0x00,   # MOV CX, 5  (counter)
            0xBB, 0x05, 0x00,   # MOV BX, 5  (current number)
            # Loop body (starts at offset 9):
            0x01, 0xD8,         # ADD AX, BX   (AX += BX)
            0x4B,               # DEC BX        (BX--)
            0xE2, 0xFB,         # LOOP -5 (back to offset 9)
            0xF4,
        ])
        result = run(prog)
        assert result.final_state.ax == 15

    def test_multiply_by_addition(self):
        """Compute 7 * 3 = 21 by adding 7 three times."""
        prog = bytes([
            0xB8, 0x00, 0x00,   # MOV AX, 0
            0xB9, 0x03, 0x00,   # MOV CX, 3
            0x05, 0x07, 0x00,   # ADD AX, 7 ← loop body
            0xE2, 0xFB,         # LOOP -5
            0xF4,
        ])
        result = run(prog)
        assert result.final_state.ax == 21

    def test_countdown(self):
        """Count down from 10 to 0 using DEC and JNZ."""
        prog = bytes([
            0xB8, 0x0A, 0x00,   # MOV AX, 10
            0x48,               # DEC AX ← loop target
            0x75, 0xFD,         # JNZ -3 (back to DEC)
            0xF4,
        ])
        result = run(prog)
        assert result.final_state.ax == 0
        assert result.final_state.zf is True

    def test_power_of_2(self):
        """Compute 2^8 = 256 using SHL."""
        prog = bytes([
            0xB8, 0x01, 0x00,   # MOV AX, 1
            0xB9, 0x08, 0x00,   # MOV CX, 8
            0xD1, 0xE0,         # SHL AX, 1 ← loop body
            0xE2, 0xFC,         # LOOP -4
            0xF4,
        ])
        result = run(prog)
        assert result.final_state.ax == 256


class TestSubroutines:
    def test_simple_subroutine(self):
        """Call a subroutine that doubles AX."""
        prog = bytes([
            0xB8, 0x07, 0x00,   # 0: MOV AX, 7
            0xE8, 0x03, 0x00,   # 3: CALL +3 → target at 9
            0xF4,               # 6: HLT
            0x90, 0x90,         # 7,8: NOP
            0xD1, 0xE0,         # 9: SHL AX, 1
            0xC3,               # 11: RET
        ])
        result = run(prog)
        assert result.final_state.ax == 14

    def test_nested_subroutines(self):
        """Two levels of nested CALL/RET."""
        prog = bytes([
            0xB8, 0x01, 0x00,   # 0: MOV AX, 1
            0xE8, 0x07, 0x00,   # 3: CALL outer (offset 13)
            0xF4,               # 6: HLT
            # Padding to align targets
            0x90, 0x90, 0x90, 0x90, 0x90, 0x90,  # 7-12: NOP
            # outer (offset 13): calls inner
            0x05, 0x01, 0x00,   # 13: ADD AX, 1
            0xE8, 0x03, 0x00,   # 16: CALL inner (offset 22)
            0xC3,               # 19: RET
            0x90, 0x90,         # 20, 21: NOP
            # inner (offset 22): add 10
            0x05, 0x0A, 0x00,   # 22: ADD AX, 10
            0xC3,               # 25: RET
        ])
        result = run(prog)
        assert result.final_state.ax == 12  # 1 + 1 + 10

    def test_ret_with_stack_cleanup(self):
        """RET n: pop and adjust SP."""
        prog = bytes([
            0xB8, 0x05, 0x00,   # 0: MOV AX, 5
            0x50,               # 3: PUSH AX
            0xE8, 0x03, 0x00,   # 4: CALL target (offset 10)
            0xF4,               # 7: HLT
            0x90, 0x90,         # 8, 9: NOP
            # target (offset 10): add [SP] then ret 2
            0x8B, 0x04,         # 10: MOV AX, [SI] – actually use imm
            0xC2, 0x02, 0x00,   # 10: RET 2
        ])
        # Simpler test: just verify RET 2 pops SP+2
        sim = Intel8086GateLevelSimulator()
        sim.reset()
        # Set SP to 0xFFF8, push two bytes at SP
        # Manual approach: patch SP to 0x100, put return address at 0x100
        sim._rf.write16("sp", 0x100)
        sim._mem[0x100] = 0x05; sim._mem[0x101] = 0x00  # return IP=5
        sim._mem[5] = 0xF4  # HLT at IP=5
        # Write RET 2 at IP=0
        sim._mem[0] = 0xC2; sim._mem[1] = 0x02; sim._mem[2] = 0x00
        # Step once
        sim.step()
        assert sim._rf.read16("ip") == 5
        assert sim._rf.read16("sp") == 0x104  # 0x100 + 2 (pop) + 2 (RET n)


class TestStringOps:
    def test_stosb_single(self):
        """STOSB writes AL to ES:DI and increments DI."""
        prog = bytes([
            0xB0, 0x42,         # MOV AL, 0x42
            0xBF, 0x00, 0x02,   # MOV DI, 0x200
            0xAA,               # STOSB
            0xF4,
        ])
        result = run(prog)
        assert result.final_state.memory[0x200] == 0x42
        assert result.final_state.di == 0x201  # DI incremented

    def test_stosw_single(self):
        """STOSW writes AX to ES:DI."""
        prog = bytes([
            0xB8, 0x34, 0x12,   # MOV AX, 0x1234
            0xBF, 0x00, 0x02,   # MOV DI, 0x200
            0xAB,               # STOSW
            0xF4,
        ])
        result = run(prog)
        s = result.final_state
        assert s.memory[0x200] == 0x34
        assert s.memory[0x201] == 0x12

    def test_rep_stosb(self):
        """REP STOSB fills memory with a value."""
        prog = bytes([
            0xB0, 0xAA,         # MOV AL, 0xAA
            0xBF, 0x00, 0x02,   # MOV DI, 0x200
            0xB9, 0x04, 0x00,   # MOV CX, 4
            0xF3, 0xAA,         # REP STOSB
            0xF4,
        ])
        result = run(prog)
        s = result.final_state
        assert s.memory[0x200] == 0xAA
        assert s.memory[0x201] == 0xAA
        assert s.memory[0x202] == 0xAA
        assert s.memory[0x203] == 0xAA
        assert s.cx == 0  # CX exhausted

    def test_lodsb(self):
        """LODSB loads a byte from memory into AL."""
        prog = bytes([
            0xC6, 0x06, 0x00, 0x02, 0x55,  # MOV [0x200], 0x55
            0xBE, 0x00, 0x02,               # MOV SI, 0x200
            0xAC,                           # LODSB
            0xF4,
        ])
        result = run(prog)
        assert result.final_state.al == 0x55
        assert result.final_state.si == 0x201

    def test_movsb(self):
        """MOVSB copies a byte from DS:SI to ES:DI."""
        prog = bytes([
            0xC6, 0x06, 0x00, 0x01, 0x77,  # MOV [0x100], 0x77
            0xBE, 0x00, 0x01,               # MOV SI, 0x100
            0xBF, 0x00, 0x02,               # MOV DI, 0x200
            0xA4,                           # MOVSB
            0xF4,
        ])
        result = run(prog)
        assert result.final_state.memory[0x200] == 0x77

    def test_scasb_found(self):
        """REPNE SCASB: scan memory for a byte value."""
        prog = bytes([
            # Write search target to memory
            0xC6, 0x06, 0x00, 0x02, 0x00,  # MOV [0x200], 0
            0xC6, 0x06, 0x01, 0x02, 0x00,  # MOV [0x201], 0
            0xC6, 0x06, 0x02, 0x02, 0x42,  # MOV [0x202], 0x42
            0xB0, 0x42,                    # MOV AL, 0x42
            0xBF, 0x00, 0x02,              # MOV DI, 0x200
            0xB9, 0x03, 0x00,              # MOV CX, 3
            0xF2, 0xAE,                    # REPNE SCASB
            0xF4,
        ])
        result = run(prog)
        assert result.final_state.zf is True  # found
        assert result.final_state.cx == 0     # exhausted CX at exact position

    def test_std_stos_decrements(self):
        """STD sets direction flag; STOSB decrements DI."""
        prog = bytes([
            0xB0, 0x55,         # MOV AL, 0x55
            0xBF, 0x05, 0x02,   # MOV DI, 0x205
            0xFD,               # STD
            0xAA,               # STOSB
            0xFC,               # CLD
            0xF4,
        ])
        result = run(prog)
        assert result.final_state.memory[0x205] == 0x55
        assert result.final_state.di == 0x204   # decremented


class TestSegmentAddressing:
    def test_cs_ip_physical_address(self):
        """Verify CS:IP physical address computation."""
        sim = Intel8086GateLevelSimulator()
        sim.reset()
        sim._rf.write16("cs", 0x1000)
        sim._rf.write16("ip", 0x0100)
        # Place HLT at physical address CS×16 + IP = 0x10100
        sim._mem[0x10100] = 0xF4
        sim.step()
        assert sim._halted

    def test_ds_segment_memory_access(self):
        """DS segment used for default data access.

        DS=0, read from [0x0200] to avoid overlapping with program at 0.
        Physical address = DS×16 + 0x200 = 0x0200.
        Use load()+step() to avoid execute() resetting memory.
        """
        prog = bytes([
            0xA1, 0x00, 0x02,   # MOV AX, [0x0200]  (DS:0x200, DS=0)
            0xF4,
        ])
        sim = Intel8086GateLevelSimulator()
        sim.reset()
        sim._mem[0x200] = 0x34; sim._mem[0x201] = 0x12   # 0x1234 little-endian
        sim.load(prog)
        for _ in range(10):
            if sim._halted:
                break
            sim.step()
        assert sim.get_state().ax == 0x1234

    def test_segment_override(self):
        """ES: override changes which segment is used.

        Use load()+step() to avoid execute() resetting registers and memory.
        ES=0x0100, read from ES:[0x0000] → physical 0x1000.
        """
        prog = bytes([
            0x26,               # ES: prefix
            0xA1, 0x00, 0x00,   # MOV AX, ES:[0x0000]
            0xF4,
        ])
        sim = Intel8086GateLevelSimulator()
        sim.reset()
        sim._rf.write16("es", 0x0100)
        # Write 0x5678 at physical ES×16 + 0 = 0x01000
        sim._mem[0x1000] = 0x78; sim._mem[0x1001] = 0x56
        sim.load(prog)
        for _ in range(10):
            if sim._halted:
                break
            sim.step()
        assert sim.get_state().ax == 0x5678
