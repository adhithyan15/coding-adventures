"""Test suite: multi-instruction programs for X86Simulator.

These tests exercise the simulator running real algorithms — loops, function
calls, memory operations, BCD arithmetic, I/O ports.  Each program is a
correct 8086 machine-code sequence that should produce a deterministic result.

Programs are assembled by hand.  Comments show the corresponding assembly and
the byte offsets used to derive each jump displacement.

Displacement rule: disp = target_offset - ip_after_jump_instruction.
"""

from __future__ import annotations

from intel_8086_simulator import X86Simulator, X86State

# ── Helpers ───────────────────────────────────────────────────────────────────

HLT = bytes([0xF4])


def mov_ax(val: int) -> bytes:
    """MOV AX, imm16."""
    return bytes([0xB8, val & 0xFF, (val >> 8) & 0xFF])


def mov_bx(val: int) -> bytes:
    """MOV BX, imm16."""
    return bytes([0xBB, val & 0xFF, (val >> 8) & 0xFF])


def mov_cx(val: int) -> bytes:
    """MOV CX, imm16."""
    return bytes([0xB9, val & 0xFF, (val >> 8) & 0xFF])


def mov_dx(val: int) -> bytes:
    """MOV DX, imm16."""
    return bytes([0xBA, val & 0xFF, (val >> 8) & 0xFF])


def mov_si(val: int) -> bytes:
    """MOV SI, imm16."""
    return bytes([0xBE, val & 0xFF, (val >> 8) & 0xFF])


def mov_di(val: int) -> bytes:
    """MOV DI, imm16."""
    return bytes([0xBF, val & 0xFF, (val >> 8) & 0xFF])


def run_prog(
    prog: bytes,
    max_steps: int = 10_000,
    input_ports: dict[int, int] | None = None,
    init_memory: dict[int, int] | None = None,
) -> X86State:
    """Reset, load, inject ports/memory, step to completion.

    Using step-by-step (rather than execute()) means port and memory
    injections survive the reset — execute() calls reset() internally,
    which would wipe any pre-set state.

    Parameters
    ----------
    prog :
        Machine code loaded at physical address 0.
    max_steps :
        Safety ceiling against infinite loops.
    input_ports :
        ``{port_number: byte_value}`` written into input_ports after reset.
    init_memory :
        ``{physical_address: byte_value}`` written into memory after load.
    """
    sim = X86Simulator()
    sim.reset()
    sim.load(prog)
    if input_ports:
        for port, val in input_ports.items():
            sim._input_ports[port] = val
    if init_memory:
        for addr, val in init_memory.items():
            sim._mem[addr] = val
    steps = 0
    while not sim._halted and steps < max_steps:
        sim.step()
        steps += 1
    return sim.get_state()


# ── Sum loop ──────────────────────────────────────────────────────────────────


class TestSumLoop:
    """Compute sum 1+2+…+N using a LOOP instruction.

    Program layout:
        0: MOV AX, 0     (3 bytes)
        3: MOV CX, N     (3 bytes)
        6: ADD AX, CX    (2 bytes)  ← loop body
        8: LOOP -4       (2 bytes)  IP after = 10; target = 6; disp = -4 = FC
       10: HLT
    """

    def _sum_prog(self, n: int) -> bytes:
        return (
            mov_ax(0)
            + mov_cx(n)
            + bytes([
                0x01, 0xC8,  # ADD AX, CX
                0xE2, 0xFC,  # LOOP -4  (back to ADD AX,CX at offset 6)
            ])
            + HLT
        )

    def test_sum_1_to_5(self):
        s = run_prog(self._sum_prog(5))
        assert s.ax == 15   # 5+4+3+2+1 = 15
        assert s.cx == 0

    def test_sum_zero_iterations(self):
        # CX=1: LOOP body runs once (ADD AX,1), then LOOP decrements CX→0, exits.
        s = run_prog(self._sum_prog(1))
        assert s.ax == 1

    def test_sum_1_to_10(self):
        s = run_prog(self._sum_prog(10))
        assert s.ax == 55   # 1+2+…+10 = 55


# ── Factorial (iterative) ─────────────────────────────────────────────────────


class TestFactorial:
    """Compute N! using a TEST+JZ+IMUL loop.

    Program layout:
        0: MOV AX, 1        (3)
        3: MOV BX, N        (3)
        6: TEST BX, BX      (2)   ← loop top (checks BX==0)
        8: JZ done  +5      (2)   IP=10, done=15, disp=5
       10: IMUL BX           (2)   AX ← AX×BX
       12: DEC BX             (1)
       13: JMP loop  -9      (2)   IP=15, top=6, disp=-9=F7
       15: HLT                     ← done
    """

    def _factorial_prog(self, n: int) -> bytes:
        return (
            mov_ax(1)
            + mov_bx(n)
            + bytes([
                0x85, 0xDB,  # TEST BX, BX
                0x74, 0x05,  # JZ +5 (to HLT)
                0xF7, 0xEB,  # IMUL BX
                0x4B,        # DEC BX
                0xEB, 0xF7,  # JMP -9 (back to TEST BX,BX)
            ])
            + HLT
        )

    def test_factorial_0(self):
        s = run_prog(self._factorial_prog(0))
        assert s.ax == 1    # 0! = 1

    def test_factorial_1(self):
        s = run_prog(self._factorial_prog(1))
        assert s.ax == 1

    def test_factorial_5(self):
        s = run_prog(self._factorial_prog(5))
        assert s.ax == 120

    def test_factorial_6(self):
        s = run_prog(self._factorial_prog(6))
        assert s.ax == 720

    def test_factorial_7(self):
        s = run_prog(self._factorial_prog(7))
        assert s.ax == 5040


# ── GCD (Euclidean subtraction) ───────────────────────────────────────────────


class TestGCD:
    """Compute GCD(A,B) by repeated subtraction.

    Program layout:
        0: MOV AX, A        (3)
        3: MOV BX, B        (3)
        6: CMP AX, BX       (2)  ← top
        8: JE done  +10     (2)  IP=10, done=20, disp=10=0A
       10: JB b_bigger  +4  (2)  IP=12, b_bigger=16, disp=4
       12: SUB AX, BX       (2)
       14: JMP top  -10     (2)  IP=16, top=6, disp=-10=F6
       16: SUB BX, AX       (2)  ← b_bigger
       18: JMP top  -14     (2)  IP=20, top=6, disp=-14=F2
       20: HLT                   ← done
    """

    def _gcd_prog(self, a: int, b: int) -> bytes:
        return (
            mov_ax(a)
            + mov_bx(b)
            + bytes([
                0x3B, 0xC3,  # CMP AX, BX
                0x74, 0x0A,  # JE +10 (to HLT)
                0x72, 0x04,  # JB +4  (to b_bigger)
                0x2B, 0xC3,  # SUB AX, BX
                0xEB, 0xF6,  # JMP -10 (to CMP)
                0x2B, 0xD8,  # SUB BX, AX  ← b_bigger
                0xEB, 0xF2,  # JMP -14 (to CMP)
            ])
            + HLT
        )

    def test_gcd_12_8(self):
        s = run_prog(self._gcd_prog(12, 8))
        assert s.ax == 4

    def test_gcd_48_36(self):
        s = run_prog(self._gcd_prog(48, 36))
        assert s.ax == 12

    def test_gcd_same(self):
        s = run_prog(self._gcd_prog(7, 7))
        assert s.ax == 7

    def test_gcd_prime(self):
        s = run_prog(self._gcd_prog(13, 7))
        assert s.ax == 1

    def test_gcd_1_and_n(self):
        s = run_prog(self._gcd_prog(1, 100))
        assert s.ax == 1


# ── Memory copy (REP MOVSB) ───────────────────────────────────────────────────


class TestMemoryCopy:
    """Copy N bytes from source to destination using REP MOVSB."""

    def test_copy_5_bytes(self):
        # Source at 0x0200, dest at 0x0300, 5 bytes.
        prog = (
            mov_si(0x0200)
            + mov_di(0x0300)
            + mov_cx(5)
            + bytes([0xF3, 0xA4])  # REP MOVSB
            + HLT
        )
        s = run_prog(prog, init_memory={
            0x0200: 0x10, 0x0201: 0x11, 0x0202: 0x12,
            0x0203: 0x13, 0x0204: 0x14,
        })
        for i in range(5):
            assert s.memory[0x0300 + i] == 0x10 + i
        assert s.cx == 0

    def test_copy_preserves_source(self):
        """After copy, source bytes should be unchanged."""
        prog = (
            mov_si(0x100)
            + mov_di(0x200)
            + mov_cx(4)
            + bytes([0xF3, 0xA4])  # REP MOVSB
            + HLT
        )
        s = run_prog(prog, init_memory={
            0x100: 0xAB, 0x101: 0xAC, 0x102: 0xAD, 0x103: 0xAE,
        })
        for i in range(4):
            assert s.memory[0x100 + i] == 0xAB + i   # source unchanged
            assert s.memory[0x200 + i] == 0xAB + i   # dest copied


# ── String fill (REP STOSB) ───────────────────────────────────────────────────


class TestStringFill:
    """Fill a memory region with a constant byte using REP STOSB."""

    def test_fill_10_bytes_with_0xFF(self):
        prog = (
            mov_ax(0x00FF)
            + mov_di(0x0500)
            + mov_cx(10)
            + bytes([0xF3, 0xAA])  # REP STOSB
            + HLT
        )
        s = run_prog(prog)
        for i in range(10):
            assert s.memory[0x0500 + i] == 0xFF
        assert s.memory[0x050A] == 0   # byte past the fill is untouched

    def test_fill_zero(self):
        # Zero out a region that was pre-filled with 0xCC.
        prog = (
            mov_ax(0)
            + mov_di(0x300)
            + mov_cx(8)
            + bytes([0xF3, 0xAA])
            + HLT
        )
        s = run_prog(
            prog,
            init_memory={0x300 + i: 0xCC for i in range(8)},
        )
        for i in range(8):
            assert s.memory[0x300 + i] == 0


# ── I/O port roundtrip ────────────────────────────────────────────────────────


class TestIOPorts:
    """Read from an input port, transform, write to output port."""

    def test_read_add_write(self):
        # Port 0x10 has value 0x30.  Read it, add 5, write to port 0x20.
        prog = bytes([
            0xE4, 0x10,   # IN AL, 0x10
            0x04, 0x05,   # ADD AL, 5
            0xE6, 0x20,   # OUT 0x20, AL
        ]) + HLT
        s = run_prog(prog, input_ports={0x10: 0x30})
        assert s.output_ports[0x20] == 0x35

    def test_port_pass_through(self):
        # Mirror input port 0 to output port 1.
        prog = bytes([
            0xE4, 0x00,  # IN AL, 0x00
            0xE6, 0x01,  # OUT 0x01, AL
        ]) + HLT
        s = run_prog(prog, input_ports={0: 0xBE})
        assert s.output_ports[1] == 0xBE

    def test_dx_based_io(self):
        # IN AL, DX / OUT DX, AL with DX=42
        prog = (
            mov_dx(42)
            + bytes([
                0xEC,        # IN AL, DX
                0x04, 0x01,  # ADD AL, 1
                0xEE,        # OUT DX, AL
            ])
            + HLT
        )
        s = run_prog(prog, input_ports={42: 0x77})
        assert s.output_ports[42] == 0x78


# ── CALL / RET subroutine ─────────────────────────────────────────────────────


class TestSubroutine:
    """Verify CALL pushes the return address and RET returns correctly."""

    def test_call_double_function(self):
        # Layout:
        #  0: MOV AX, 5     (3)
        #  3: CALL +1       (3)   IP after = 6; target = 7; disp = 1
        #  6: HLT
        #  7: ADD AX, AX    (2)
        #  9: RET
        prog = bytes([
            0xB8, 0x05, 0x00,   # MOV AX, 5
            0xE8, 0x01, 0x00,   # CALL +1
            0xF4,               # HLT
            0x01, 0xC0,         # ADD AX, AX
            0xC3,               # RET
        ])
        s = run_prog(prog)
        assert s.ax == 10

    def test_call_ret_restores_ip(self):
        # Layout:
        #  0: MOV AX, 1     (3)
        #  3: CALL +2       (3)   IP after = 6; target = 8; disp = 2
        #  6: INC AX        (1)   ← return site
        #  7: HLT
        #  8: INC AX        (1)   ← subroutine body
        #  9: RET
        prog = bytes([
            0xB8, 0x01, 0x00,   # MOV AX, 1
            0xE8, 0x02, 0x00,   # CALL +2
            0x40,               # INC AX   (after return)
            0xF4,               # HLT
            0x40,               # INC AX   (subroutine)
            0xC3,               # RET
        ])
        s = run_prog(prog)
        assert s.ax == 3    # 1 (initial) + 1 (sub) + 1 (after ret)


# ── Bubble sort ───────────────────────────────────────────────────────────────


class TestBubbleSort:
    """Sort a small array in memory using bubble-sort outer/inner loops.

    Program layout (base = 0x0400):
         0: MOV CX, 3            (3)   outer loop count = N-1 passes
         3: MOV BX, 0            (3)   ← outer (reset index)
         6: MOV AL, [BX+base]    (4)   ← inner
        10: MOV AH, [BX+base+1]  (4)
        14: CMP AL, AH           (2)
        16: JBE +8               (2)   IP=18; skip=26; disp=8
        18: MOV [BX+base],   AH  (4)   swap low
        22: MOV [BX+base+1], AL  (4)   swap high
        26: INC BX               (1)   ← skip
        27: CMP BX, 3            (3)
        30: JL -26               (2)   IP=32; inner=6; disp=-26=E6
        32: LOOP -31             (2)   IP=34; outer=3; disp=-31=E1
        34: HLT
    """

    def test_sort_4_bytes(self):
        base = 0x0400

        prog = bytes([
            # MOV CX, 3
            0xB9, 0x03, 0x00,
            # outer: MOV BX, 0
            0xBB, 0x00, 0x00,
            # inner: MOV AL, [BX + base]
            0x8A, 0x87, base & 0xFF, (base >> 8) & 0xFF,
            # MOV AH, [BX + base+1]
            0x8A, 0xA7, (base + 1) & 0xFF, ((base + 1) >> 8) & 0xFF,
            # CMP AL, AH
            0x3A, 0xC4,
            # JBE +8 (skip swap)
            0x76, 0x08,
            # MOV [BX+base], AH
            0x88, 0xA7, base & 0xFF, (base >> 8) & 0xFF,
            # MOV [BX+base+1], AL
            0x88, 0x87, (base + 1) & 0xFF, ((base + 1) >> 8) & 0xFF,
            # skip: INC BX
            0x43,
            # CMP BX, 3
            0x83, 0xFB, 0x03,
            # JL -26 (back to inner at offset 6)
            0x7C, 0xE6,
            # LOOP -31 (back to outer at offset 3)
            0xE2, 0xE1,
            # HLT
            0xF4,
        ])

        s = run_prog(prog, init_memory={
            base + 0: 4, base + 1: 2, base + 2: 7, base + 3: 1,
        })
        assert list(s.memory[base:base + 4]) == [1, 2, 4, 7]


# ── BCD arithmetic (DAA/DAS/AAA) ─────────────────────────────────────────────


class TestBCDArithmetic:
    """BCD addition and subtraction via DAA/DAS/AAA."""

    def test_daa_39_plus_1(self):
        # 0x39 (BCD 39) + 1 → 0x3A → after DAA → 0x40 (BCD 40)
        prog = bytes([
            0xB0, 0x39,   # MOV AL, 0x39
            0x04, 0x01,   # ADD AL, 1
            0x27,         # DAA
        ]) + HLT
        s = run_prog(prog)
        assert s.ax & 0xFF == 0x40

    def test_daa_99_plus_1(self):
        # 0x99 (BCD 99) + 1 → BCD 00 with carry
        prog = bytes([
            0xB0, 0x99,
            0x04, 0x01,
            0x27,
        ]) + HLT
        s = run_prog(prog)
        assert s.ax & 0xFF == 0x00
        assert s.cf is True

    def test_das_50_minus_1(self):
        # 0x50 (BCD 50) - 1 → 0x4F → after DAS → 0x49 (BCD 49)
        prog = bytes([
            0xB0, 0x50,
            0x2C, 0x01,   # SUB AL, 1
            0x2F,         # DAS
        ]) + HLT
        s = run_prog(prog)
        assert s.ax & 0xFF == 0x49

    def test_aaa_with_carry(self):
        # 8 + 5 = 0xD in AL; low nibble 0xD > 9 → AAA adjusts.
        # After AAA: AL = (0xD+6)&0xF = 3; AH++; CF=AF=1.
        prog = bytes([
            0xB8, 0x08, 0x00,  # MOV AX, 0x0008
            0x04, 0x05,        # ADD AL, 5   → AL=0x0D
            0x37,              # AAA
        ]) + HLT
        s = run_prog(prog)
        assert (s.ax & 0xFF) == 3   # low digit
        assert (s.ax >> 8) == 1     # AH incremented
        assert s.cf is True

    def test_aaa_no_carry(self):
        # 1 + 2 = 3; low nibble 3 ≤ 9 → no adjust; AL=3, AH=0.
        prog = bytes([
            0xB8, 0x01, 0x00,  # MOV AX, 0x0001
            0x04, 0x02,        # ADD AL, 2
            0x37,              # AAA
        ]) + HLT
        s = run_prog(prog)
        assert (s.ax & 0xFF) == 3
        assert (s.ax >> 8) == 0
        assert s.cf is False


# ── CBW / CWD sign extension ──────────────────────────────────────────────────


class TestSignExtension:
    """CBW (AL→AX) and CWD (AX→DX:AX) sign extension."""

    def test_cbw_positive(self):
        prog = bytes([0xB0, 0x7F, 0x98]) + HLT   # MOV AL,0x7F; CBW
        s = run_prog(prog)
        assert s.ax == 0x007F

    def test_cbw_negative(self):
        prog = bytes([0xB0, 0x80, 0x98]) + HLT
        s = run_prog(prog)
        assert s.ax == 0xFF80

    def test_cwd_positive(self):
        prog = mov_ax(0x7FFF) + bytes([0x99]) + HLT  # CWD
        s = run_prog(prog)
        assert s.dx == 0x0000
        assert s.ax == 0x7FFF

    def test_cwd_negative(self):
        prog = mov_ax(0x8000) + bytes([0x99]) + HLT
        s = run_prog(prog)
        assert s.dx == 0xFFFF
        assert s.ax == 0x8000


# ── XLAT lookup table ─────────────────────────────────────────────────────────


class TestXLAT:
    """XLAT: AL ← DS:[BX + AL].  Classic lookup-table instruction."""

    def test_xlat_nibble_to_ascii(self):
        # Table at 0x0100: "0123456789ABCDEF"; BX=0x100, AL=3 → AL=ord('3')=0x33
        table = b"0123456789ABCDEF"
        prog = (
            mov_bx(0x0100)
            + bytes([0xB0, 0x03])  # MOV AL, 3
            + bytes([0xD7])        # XLAT
            + HLT
        )
        s = run_prog(
            prog,
            init_memory={0x0100 + i: b for i, b in enumerate(table)},
        )
        assert s.ax & 0xFF == ord('3')

    def test_xlat_hex_digit_a(self):
        table = b"0123456789ABCDEF"
        prog = (
            mov_bx(0x0200)
            + bytes([0xB0, 10])   # MOV AL, 10
            + bytes([0xD7])       # XLAT
            + HLT
        )
        s = run_prog(
            prog,
            init_memory={0x0200 + i: b for i, b in enumerate(table)},
        )
        assert s.ax & 0xFF == ord('A')


# ── Segment override ──────────────────────────────────────────────────────────


class TestSegmentOverride:
    """CS: override reads from code-segment memory."""

    def test_cs_override_read(self):
        # CS: prefix + MOV AX, [0x0010] reads from CS:0x10 = physical 0x0010
        # (CS=0 at reset, so physical = 16×0 + 0x10).
        prog = bytes([
            0x2E,               # CS: prefix
            0xA1, 0x10, 0x00,   # MOV AX, [0x0010]
        ]) + HLT
        s = run_prog(
            prog,
            init_memory={0x0010: 0xCD, 0x0011: 0xAB},
        )
        assert s.ax == 0xABCD


# ── Max steps guard ───────────────────────────────────────────────────────────


class TestMaxSteps:
    """Verify infinite loops are caught by the max_steps guard."""

    def test_infinite_jmp_short(self):
        # JMP $ (EB FE) jumps to itself — infinite loop.
        sim = X86Simulator()
        result = sim.execute(bytes([0xEB, 0xFE]), max_steps=100)
        assert result.halted is False
        assert result.steps == 100
        assert result.ok is False
        assert "max_steps" in result.error

    def test_tight_loop_with_decrement(self):
        # MOV CX,200; loop: DEC CX; JNZ loop; HLT — capped at 50 steps.
        prog = (
            mov_cx(200)
            + bytes([
                0x49,        # DEC CX
                0x75, 0xFD,  # JNZ -3
            ])
            + HLT
        )
        sim = X86Simulator()
        result = sim.execute(prog, max_steps=50)
        assert result.steps == 50
        assert result.ok is False


# ── Multi-step trace verification ────────────────────────────────────────────


class TestTraces:
    """Verify that execute() records a trace for every step."""

    def test_trace_count_equals_steps(self):
        prog = mov_ax(1) + mov_bx(2) + bytes([0x01, 0xD8]) + HLT
        sim = X86Simulator()
        result = sim.execute(prog)
        assert len(result.traces) == result.steps

    def test_trace_mnemonics_sequence(self):
        prog = bytes([0x90, 0x90]) + HLT   # NOP; NOP; HLT
        sim = X86Simulator()
        result = sim.execute(prog)
        assert result.traces[0].mnemonic == "NOP"
        assert result.traces[1].mnemonic == "NOP"
        assert result.traces[2].mnemonic == "HLT"

    def test_trace_pc_sequence_is_monotone(self):
        # Straight-line program: pc_before should increase monotonically.
        prog = mov_ax(5) + mov_bx(3) + HLT
        sim = X86Simulator()
        result = sim.execute(prog)
        pcs = [t.pc_before for t in result.traces]
        assert pcs == sorted(pcs)

    def test_trace_description_not_empty(self):
        sim = X86Simulator()
        result = sim.execute(HLT)
        for trace in result.traces:
            assert trace.description
