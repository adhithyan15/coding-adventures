"""Multi-instruction program tests for PDP11Simulator.

Tests complete small programs to verify that instruction interactions,
subroutine calls, and loops work correctly end-to-end.
"""

from __future__ import annotations

from pdp11_simulator import PDP11Simulator, PDP11State


def _w(value: int) -> bytes:
    return bytes([value & 0xFF, (value >> 8) & 0xFF])

HALT = _w(0x0000)

def _op(mode: int, reg: int) -> int:
    return (mode << 3) | reg

def _mov(sm, sr, dm, dr): return _w(0x1000 | (_op(sm, sr) << 6) | _op(dm, dr))
def _add(sm, sr, dm, dr): return _w(0x6000 | (_op(sm, sr) << 6) | _op(dm, dr))
def _sub(sm, sr, dm, dr): return _w(0xE000 | (_op(sm, sr) << 6) | _op(dm, dr))
def _cmp(sm, sr, dm, dr): return _w(0x2000 | (_op(sm, sr) << 6) | _op(dm, dr))
def _clr(mode, reg):       return _w(0x0A00 | _op(mode, reg))
def _inc(mode, reg):       return _w(0x0A80 | _op(mode, reg))
def _dec(mode, reg):       return _w(0x0AC0 | _op(mode, reg))
def _asl(mode, reg):       return _w(0x0CC0 | _op(mode, reg))  # octal 006300
def _asr(mode, reg):       return _w(0x0C80 | _op(mode, reg))  # octal 006200
def _bne(offset):          return _w(0x0200 | (offset & 0xFF))
def _beq(offset):          return _w(0x0300 | (offset & 0xFF))
def _blt(offset):          return _w(0x0500 | (offset & 0xFF))
def _br(offset):           return _w(0x0100 | (offset & 0xFF))
def _jsr(link, mode, dst): return _w(0x0800 | (link << 6) | _op(mode, dst))
def _rts(reg):             return _w(0x0080 | reg)   # octal 000 200
def _sob(reg, offset):     return _w(0x7E00 | (reg << 6) | (offset & 0x3F))


def _run(prog: bytes) -> PDP11State:
    sim = PDP11Simulator()
    result = sim.execute(prog)
    assert result.ok, f"Simulation error: {result.error}"
    return result.final_state


class TestArithmeticPrograms:
    def test_sum_1_to_10(self):
        """Compute sum 1+2+…+10 = 55 using a countdown loop.

        Layout:
          R0 = accumulator (sum)
          R1 = loop counter (10 down to 1)

          MOV #10, R1       ; counter = 10
          CLR R0            ; sum = 0
        loop:
          ADD R1, R0        ; sum += counter
          DEC R1            ; counter--
          BNE loop          ; if counter != 0, repeat
          HALT
        """
        prog = (
            _mov(2, 7, 0, 1) + _w(10)  # MOV #10, R1
            + _clr(0, 0)                # CLR R0
            + _add(0, 1, 0, 0)          # ADD R1, R0   ← loop starts here (offset=3)
            + _dec(0, 1)                # DEC R1
            + _bne(0xFD)                # BNE -3 (3 words back = 6 bytes back)
            + HALT
        )
        # Verify the offset: BNE is at offset 10 from program start.
        # PC after fetching BNE = 0x100C.
        # Loop body starts at 0x1008.
        # EA = 0x100C + 2 * offset_signed
        # We want EA = 0x1008 → offset = (0x1008 - 0x100C) / 2 = -4/2 = -2
        # So offset byte = 0xFE (signed -2)
        prog = (
            _mov(2, 7, 0, 1) + _w(10)  # 0x1000: MOV #10, R1 (4 bytes)
            + _clr(0, 0)                # 0x1004: CLR R0        (2 bytes)
            + _add(0, 1, 0, 0)          # 0x1006: ADD R1, R0    (2 bytes) ← loop
            + _dec(0, 1)                # 0x1008: DEC R1         (2 bytes)
            + _bne(0xFE)                # 0x100A: BNE -2 → 0x100A+2 + 2*(-2) = 0x100C-4 = 0x1006? wait
            + HALT
        )
        # Recalculate: PC after fetch of BNE word at 0x100A = 0x100C
        # EA = 0x100C + 2 * sign_extend(0xFE, 8)
        # sign_extend(0xFE, 8) = -2
        # EA = 0x100C + 2*(-2) = 0x100C - 4 = 0x1008? That's DEC R1, not ADD R1,R0
        # We want to branch back to ADD R1, R0 at 0x1006
        # EA = 0x1006 → offset = (0x1006 - 0x100C) / 2 = -6/2 = -3
        # offset byte = 0xFD
        prog = (
            _mov(2, 7, 0, 1) + _w(10)  # 0x1000: MOV #10, R1
            + _clr(0, 0)                # 0x1004: CLR R0
            + _add(0, 1, 0, 0)          # 0x1006: ADD R1, R0   ← loop top
            + _dec(0, 1)                # 0x1008: DEC R1
            + _bne(0xFD)                # 0x100A: BNE → 0x100C + 2*(-3) = 0x1006 ✓
            + HALT                      # 0x100C
        )
        s = _run(prog)
        assert s.r[0] == 55

    def test_multiply_by_repeated_addition(self):
        """5 * 7 = 35 via repeated addition.

          R0 = result (accumulator)
          R1 = 5 (one operand, counts down)
          R2 = 7 (other operand, added each time)

          MOV #5, R1
          MOV #7, R2
          CLR R0
        loop:
          ADD R2, R0
          SOB R1, loop
          HALT
        """
        # SOB R1, offset: decrement R1; if R1!=0 branch backward by offset words
        # loop body (ADD + SOB) = 2 words = 4 bytes
        # SOB offset = 2 (branch back over ADD = 1 word, SOB itself = 1 word → 2 words total)
        # Wait: after SOB fetches, PC points past SOB. EA = PC - 2*offset.
        # If SOB is at 0x100A, PC after fetch = 0x100C.
        # ADD is at 0x1008. EA = 0x100C - 2*2 = 0x1008 ✓
        prog = (
            _mov(2, 7, 0, 1) + _w(5)   # 0x1000: MOV #5, R1
            + _mov(2, 7, 0, 2) + _w(7) # 0x1004: MOV #7, R2
            + _clr(0, 0)                # 0x1008: CLR R0
            + _add(0, 2, 0, 0)          # 0x100A: ADD R2, R0  ← loop top
            + _sob(1, 2)                # 0x100C: SOB R1, 2 → PC(0x100E) - 4 = 0x100A ✓
            + HALT                      # 0x100E
        )
        s = _run(prog)
        assert s.r[0] == 35

    def test_power_of_2(self):
        """Compute 2^8 = 256 by left-shifting.

          R0 = 1
          R1 = 8 (shift count)
        loop:
          ASL R0        (multiply by 2)
          SOB R1, loop
          HALT
        """
        prog = (
            _mov(2, 7, 0, 0) + _w(1)   # MOV #1, R0
            + _mov(2, 7, 0, 1) + _w(8) # MOV #8, R1
            + _asl(0, 0)                # ASL R0  ← loop top (0x1008)
            + _sob(1, 1)                # SOB R1, 1 → PC(0x100C) - 2 = 0x100A? wait
        )
        # ASL at 0x1008, SOB at 0x100A
        # PC after SOB fetch = 0x100C
        # EA = 0x100C - 2*2 = 0x1008 ✓ (offset=2)
        prog = (
            _mov(2, 7, 0, 0) + _w(1)   # 0x1000: MOV #1, R0
            + _mov(2, 7, 0, 1) + _w(8) # 0x1004: MOV #8, R1
            + _asl(0, 0)                # 0x1008: ASL R0
            + _sob(1, 2)                # 0x100A: SOB R1, 2 → 0x100C - 4 = 0x1008 ✓
            + HALT                      # 0x100C
        )
        s = _run(prog)
        assert s.r[0] == 256


class TestSubroutineCalls:
    def test_simple_subroutine(self):
        """Call a doubling subroutine and return.

        Main:
          MOV #7, R0
          JSR PC, @#double_addr
          HALT

        double:
          ADD R0, R0
          RTS PC
        """
        # Layout:
        # 0x1000: MOV #7, R0      (4 bytes)
        # 0x1004: JSR PC, @#sub   (2 bytes opcode + 2 bytes absolute addr)
        # 0x1008: HALT
        # 0x100A: padding (2 bytes to align sub at 0x100C)
        # 0x100A: ADD R0, R0      (2 bytes)
        # 0x100C: RTS PC          (2 bytes)
        sub_addr = 0x100A
        prog = (
            _mov(2, 7, 0, 0) + _w(7)          # 0x1000: MOV #7, R0
            + _jsr(7, 3, 7) + _w(sub_addr)    # 0x1004: JSR PC, @#sub_addr
            + HALT                              # 0x1008: return here
            + _w(0)                             # 0x100A: padding
            + _add(0, 0, 0, 0)                 # 0x100C: ADD R0, R0  ← sub
            + _rts(7)                           # 0x100E: RTS PC
        )
        # Hmm, if sub_addr = 0x100A that's the NOP pad, not ADD.
        # Let's recalculate: sub is ADD + RTS
        # 0x1000: MOV #7, R0 = 4 bytes (0x1000-0x1003)
        # 0x1004: JSR = 2 bytes + addr word = 4 bytes (0x1004-0x1007)
        # 0x1008: HALT = 2 bytes
        # sub at 0x100A:
        sub_addr = 0x100A
        prog = (
            _mov(2, 7, 0, 0) + _w(7)          # 0x1000
            + _jsr(7, 3, 7) + _w(sub_addr)    # 0x1004
            + HALT                              # 0x1008
            + _add(0, 0, 0, 0)                 # 0x100A: sub entry
            + _rts(7)                           # 0x100C: RTS PC
        )
        s = _run(prog)
        assert s.r[0] == 14

    def test_nested_subroutine_calls(self):
        """Nested calls: main calls double, double calls add1.

        main:
          MOV #3, R0
          JSR PC, @#double    → R0 = 6
          HALT

        double:
          JSR PC, @#add1      → R0 = R0 + 1
          JSR PC, @#add1      → R0 = R0 + 1
          RTS PC

        add1:
          INC R0
          RTS PC
        """
        # Assemble carefully with correct addresses:
        # 0x1000: MOV #3, R0         (4 bytes, ends 0x1003)
        # 0x1004: JSR PC, @#double   (4 bytes, ends 0x1007)
        # 0x1008: HALT               (2 bytes)
        # double at 0x100A:
        # 0x100A: JSR PC, @#add1     (4 bytes)
        # 0x100E: JSR PC, @#add1     (4 bytes)
        # 0x1012: RTS PC             (2 bytes)
        # add1 at 0x1014:
        # 0x1014: INC R0             (2 bytes)
        # 0x1016: RTS PC             (2 bytes)

        double_addr = 0x100A
        add1_addr   = 0x1014

        prog = (
            _mov(2, 7, 0, 0) + _w(3)              # 0x1000
            + _jsr(7, 3, 7) + _w(double_addr)     # 0x1004
            + HALT                                  # 0x1008
            + _jsr(7, 3, 7) + _w(add1_addr)       # 0x100A: double entry
            + _jsr(7, 3, 7) + _w(add1_addr)       # 0x100E
            + _rts(7)                               # 0x1012
            + _inc(0, 0)                            # 0x1014: add1 entry
            + _rts(7)                               # 0x1016
        )
        s = _run(prog)
        assert s.r[0] == 5   # 3 + 1 + 1 = 5

    def test_stack_balanced_after_call(self):
        """Stack pointer must be restored after JSR/RTS."""
        sub_addr = 0x1008
        prog = (
            _jsr(7, 3, 7) + _w(sub_addr)   # 0x1000: JSR PC, @#sub
            + HALT                           # 0x1004
            + _w(0)                          # 0x1006: padding
            + _rts(7)                        # 0x1008: sub just returns
        )
        sim = PDP11Simulator()
        result = sim.execute(prog)
        assert result.ok
        sp_before = 0xF000
        assert result.final_state.r[6] == sp_before   # SP restored


class TestMemoryPrograms:
    def test_copy_array(self):
        """Copy 4 words from src to dst using autoincrement.

        Layout:
          R0 = src pointer (autoincrement)
          R1 = dst pointer (autoincrement)
          R2 = count

          MOV #src, R0
          MOV #dst, R1
          MOV #4, R2
        loop:
          MOV (R0)+, (R1)+
          SOB R2, loop
          HALT
        """
        # Data: src at 0x2000, dst at 0x2100
        # But we need to write src data into memory before running.
        # We'll use the program itself to set up src data.
        # simpler: write src data using MOV #val, @#addr

        src = 0x2000
        dst = 0x2100

        def _mov_abs(val, addr):
            """MOV #val, @#addr"""
            return _mov(2, 7, 3, 7) + _w(val) + _w(addr)

        prog = (
            _mov_abs(0x0001, src + 0)
            + _mov_abs(0x0002, src + 2)
            + _mov_abs(0x0003, src + 4)
            + _mov_abs(0x0004, src + 6)
            + _mov(2, 7, 0, 0) + _w(src)  # R0 = src
            + _mov(2, 7, 0, 1) + _w(dst)  # R1 = dst
            + _mov(2, 7, 0, 2) + _w(4)    # R2 = 4
            + _mov(2, 0, 2, 1)             # MOV (R0)+, (R1)+  ← loop
            + _sob(2, 1)                   # SOB R2, 1 → PC_after - 2 = loop
            + HALT
        )
        # Let me figure out the addresses:
        # Each _mov_abs = 6 bytes → 4 of them = 24 bytes (0x1000 to 0x1017)
        # 0x1018: MOV #src, R0 (4 bytes)
        # 0x101C: MOV #dst, R1 (4 bytes)
        # 0x1020: MOV #4, R2   (4 bytes)
        # 0x1024: MOV (R0)+,(R1)+  (2 bytes) ← loop
        # 0x1026: SOB R2, 2    → PC(0x1028) - 2*2 = 0x1024 ✓ (offset=2)
        # 0x1028: HALT
        prog = (
            _mov_abs(0x0001, src + 0)
            + _mov_abs(0x0002, src + 2)
            + _mov_abs(0x0003, src + 4)
            + _mov_abs(0x0004, src + 6)
            + _mov(2, 7, 0, 0) + _w(src)
            + _mov(2, 7, 0, 1) + _w(dst)
            + _mov(2, 7, 0, 2) + _w(4)
            + _mov(2, 0, 2, 1)              # MOV (R0)+, (R1)+
            + _sob(2, 2)
            + HALT
        )
        s = _run(prog)
        for i in range(4):
            addr = dst + i * 2
            word = s.memory[addr] | (s.memory[addr + 1] << 8)
            assert word == i + 1, f"dst[{i}] = {word}, expected {i+1}"

    def test_reverse_array(self):
        """Reverse a 3-element array in place.

        Use two pointers: left (autoincrement) and right (autodecrement).
        Swap elements while left < right.

        For simplicity, just hardcode the 3 swaps using MOV with deferred modes.
        """
        base = 0x2000
        # Write [10, 20, 30] to memory
        # Then swap element[0] and element[2], leaving element[1]

        def _mov_abs(val, addr):
            return _mov(2, 7, 3, 7) + _w(val) + _w(addr)

        def _mov_mem2reg(addr, reg):
            """MOV @#addr, Rn"""
            return _mov(3, 7, 0, reg) + _w(addr)

        def _mov_reg2mem(reg, addr):
            """MOV Rn, @#addr"""
            return _mov(0, reg, 3, 7) + _w(addr)

        prog = (
            _mov_abs(10, base + 0)
            + _mov_abs(20, base + 2)
            + _mov_abs(30, base + 4)
            # Swap [0] and [2]: use R3 as temp
            + _mov_mem2reg(base + 0, 3)   # R3 = M[base]
            + _mov_mem2reg(base + 4, 4)   # R4 = M[base+4]
            + _mov_reg2mem(4, base + 0)   # M[base] = R4
            + _mov_reg2mem(3, base + 4)   # M[base+4] = R3
            + HALT
        )
        s = _run(prog)
        def word(a): return s.memory[a] | (s.memory[a+1] << 8)
        assert word(base + 0) == 30
        assert word(base + 2) == 20
        assert word(base + 4) == 10


class TestRTI:
    def test_rti_restores_pc_and_psw(self):
        """RTI pops PC then PSW from the stack.

        We set up the stack manually then execute RTI.
        Target: after RTI, PC = 0x1010, PSW = 0x000F (all flags set).
        """
        sim = PDP11Simulator()
        sim.reset()
        # Place RTI at 0x1000
        sim._mem[0x1000] = 0x02   # RTI low byte
        sim._mem[0x1001] = 0x00   # RTI high byte (0x0002)
        # Place HALT at 0x1010 (return target)
        sim._mem[0x1010] = 0x00
        sim._mem[0x1011] = 0x00

        # Set up stack: push PSW=0x000F then PC=0x1010
        # Stack grows down; RTI pops PC first, then PSW
        # So stack (from SP upward): PC=0x1010, PSW=0x000F
        # push PSW first (lower address), then PC
        sp = 0xF000 - 4
        sim._r[6] = sp
        sim._mem[sp]   = 0x10; sim._mem[sp+1] = 0x10   # PC = 0x1010 (low byte first)
        sim._mem[sp+2] = 0x0F; sim._mem[sp+3] = 0x00   # PSW = 0x000F

        sim._r[7] = 0x1000   # Execute RTI at 0x1000

        sim.step()   # RTI

        assert sim._r[7] == 0x1010
        assert sim._psw == 0x000F
        assert sim._r[6] == 0xF000  # SP restored
