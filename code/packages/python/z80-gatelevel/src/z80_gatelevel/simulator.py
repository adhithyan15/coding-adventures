"""Z80GateLevelSimulator — gate-level Z80 implementing Simulator[Z80State].

This is the top-level simulator class. It implements the
`Simulator[Z80State]` protocol from `simulator_protocol`.

It delegates execution to the internal _execute_one() method which uses:
  - DecoderZ80 for combinational instruction decode
  - ALU functions (add8, sub8, and8, etc.) for gate-level arithmetic/logic
  - RegisterFile (Register8/Register16 flip-flop arrays) for register storage
  - Register16 for PC and SP

The output type is `Z80State` — the same frozen dataclass used by the
behavioral Z80 simulator. Both simulators implement `Simulator[Z80State]`
and produce bit-for-bit identical output for all covered instructions.

=== Gate count for a simple ADD A, B instruction ===

Fetch opcode:        0 gates (memory bus, no logic)
Decode (group10):    6 gates (2 NOT + 4 AND for group detect)
Read registers:      0 gates (flip-flop read, no logic)
8-bit ALU (add8):
  - 8 × full_adder:  ~40 gates (each FA = 2 XOR + 2 AND + 1 OR = 5 gates)
  - Overflow XOR:     1 gate
  - Zero NOR tree:   ~8 gates
  - Sign:             0 gates (just a wire = bit 7)
  - H carry:          0 extra (already computed in adder chain)
  - Total ALU:       ~49 gates
Write result:        0 gates (flip-flop write, no logic)
─────────────────
Total for ADD A,B:  ~55 gate operations

Real Z80 uses ~8500 transistors total; ADD A,B activates perhaps 150–200
transistors after all decoder/mux propagation. Our Python model captures
the logical depth but not the transistor-level detail.

=== Halt condition ===

HALT (opcode 0x76) sets halted=True. In real hardware, the Z80 repeats
NOP internally until an interrupt arrives. We simply stop execution.
Calling step() when halted raises RuntimeError.
"""

from __future__ import annotations

from simulator_protocol import ExecutionResult, Simulator, StepTrace
from z80_simulator import Z80State

from z80_gatelevel.alu import (
    ALUResultZ80,
    adc16,
    add8,
    add16,
    and8,
    bit_test,
    cpl8,
    daa8,
    dec8,
    inc8,
    neg8,
    or8,
    res_bit,
    rl8,
    rla8,
    rlc8,
    rlca8,
    rr8,
    rra8,
    rrc8,
    rrca8,
    sbc16,
    set_bit,
    sla8,
    sra8,
    srl8,
    sub8,
    xor8,
)
from z80_gatelevel.bits import add_16bit
from z80_gatelevel.decoder import DecoderZ80
from z80_gatelevel.register_file import (
    REG_A,
    REG_B,
    REG_C,
    REG_D,
    REG_E,
    REG_H,
    REG_L,
    REG_MEM,
    Register16,
    RegisterFile,
    pack_f,
    unpack_f,
)

_NUM_PORTS = 256

# Condition code names for mnemonic generation
_COND_NAMES = ["NZ", "Z", "NC", "C", "PO", "PE", "P", "M"]
_REG_NAMES = ["B", "C", "D", "E", "H", "L", "(HL)", "A"]
_PAIR_NAMES = ["BC", "DE", "HL", "SP"]
_ALU_NAMES = ["ADD", "ADC", "SUB", "SBC", "AND", "XOR", "OR", "CP"]


class Z80GateLevelSimulator(Simulator[Z80State]):
    """Gate-level simulator for the Zilog Z80 microprocessor.

    Every arithmetic/logic operation routes through real gate functions
    from the `logic_gates` and `arithmetic` packages. No host arithmetic
    shortcuts in the ALU path.

    Implements `Simulator[Z80State]` from `simulator_protocol`.
    Cross-validates against `Z80Simulator` (behavioral) in tests.

    Usage:
        >>> sim = Z80GateLevelSimulator()
        >>> result = sim.execute(bytes([
        ...     0x3E, 0x0A,  # LD A, 10
        ...     0xC6, 0x05,  # ADD A, 5  (routes through ripple-carry adder)
        ...     0x76,        # HALT
        ... ]))
        >>> result.final_state.a
        15
    """

    def __init__(self) -> None:
        """Create a gate-level simulator at reset state."""
        self._memory: bytearray = bytearray(65536)
        self._rf = RegisterFile()
        self._pc = Register16()
        self._sp = Register16()
        self._i = 0    # Interrupt vector base (8-bit)
        self._r = 0    # Memory refresh counter (8-bit)
        self._iff1 = False
        self._iff2 = False
        self._im = 0
        self._halted = False
        self._dec = DecoderZ80()
        self._input_ports: list[int] = [0] * _NUM_PORTS
        self._output_ports: list[int] = [0] * _NUM_PORTS

    # ── SIM00 Protocol ────────────────────────────────────────────────────────

    def reset(self) -> None:
        """Reset to power-on state.

        Registers: all 0. F = 0xFF (all flags set). Memory zeroed.
        """
        self._memory = bytearray(65536)
        self._rf = RegisterFile()
        self._pc = Register16()
        self._sp = Register16()
        self._i = 0
        self._r = 0
        self._iff1 = False
        self._iff2 = False
        self._im = 0
        self._halted = False
        # Z80 power-on: F = 0xFF (all flags set)
        self._rf.write_flags(1, 1, 1, 1, 1, 1)

    def load(self, program: bytes, origin: int = 0x0000) -> None:
        """Write program bytes to memory and set PC to origin.

        Args:
            program: Machine code bytes.
            origin:  Load address (default 0x0000).
        """
        if not (0 <= origin <= 0xFFFF):
            msg = f"origin {origin:#06x} out of range"
            raise ValueError(msg)
        for i, byte in enumerate(program):
            self._memory[(origin + i) & 0xFFFF] = byte & 0xFF
        self._pc.write(origin)
        self._halted = False

    def step(self) -> StepTrace:
        """Execute one instruction and return a StepTrace.

        Returns:
            StepTrace with pc_before, pc_after, mnemonic, description.

        Raises:
            RuntimeError: If already halted.
        """
        if self._halted:
            msg = "CPU is halted — call reset() or load() first"
            raise RuntimeError(msg)
        pc_before = self._pc.read()
        mnemonic, description = self._fetch_and_execute()
        return StepTrace(
            pc_before=pc_before,
            pc_after=self._pc.read(),
            mnemonic=mnemonic,
            description=description,
        )

    def execute(
        self,
        program: bytes,
        origin: int = 0x0000,
        max_steps: int = 100_000,
    ) -> ExecutionResult[Z80State]:
        """Load and run until HALT or max_steps.

        Preserves I/O port values across internal reset.

        Args:
            program:   Bytecode to execute.
            origin:    Load address (default 0x0000).
            max_steps: Safety limit (default 100,000).

        Returns:
            ExecutionResult with halted, steps, final_state, traces, error.
        """
        saved_in = list(self._input_ports)
        saved_out = list(self._output_ports)
        self.reset()
        self._input_ports = saved_in
        self._output_ports = saved_out
        self.load(program, origin)

        traces: list[StepTrace] = []
        steps = 0
        error: str | None = None

        try:
            while not self._halted and steps < max_steps:
                traces.append(self.step())
                steps += 1
        except Exception as exc:  # noqa: BLE001
            error = str(exc)

        return ExecutionResult(
            halted=self._halted,
            steps=steps,
            final_state=self.get_state(),
            traces=traces,
            error=error,
        )

    def get_state(self) -> Z80State:
        """Return an immutable snapshot of the current CPU state."""
        rf = self._rf
        flags = rf.read_flags()
        return Z80State(
            a=rf.read8(REG_A),
            b=rf.read8(REG_B),
            c=rf.read8(REG_C),
            d=rf.read8(REG_D),
            e=rf.read8(REG_E),
            h=rf.read8(REG_H),
            l=rf.read8(REG_L),
            a_prime=rf.read_alt8(REG_A),
            f_prime=rf.read_f_prime(),
            b_prime=rf.read_alt8(REG_B),
            c_prime=rf.read_alt8(REG_C),
            d_prime=rf.read_alt8(REG_D),
            e_prime=rf.read_alt8(REG_E),
            h_prime=rf.read_alt8(REG_H),
            l_prime=rf.read_alt8(REG_L),
            ix=rf.read_ix(),
            iy=rf.read_iy(),
            sp=self._sp.read(),
            pc=self._pc.read(),
            i=self._i,
            r=self._r,
            flag_s=bool(flags['s']),
            flag_z=bool(flags['z']),
            flag_h=bool(flags['h']),
            flag_pv=bool(flags['pv']),
            flag_n=bool(flags['n']),
            flag_c=bool(flags['c']),
            iff1=self._iff1,
            iff2=self._iff2,
            im=self._im,
            halted=self._halted,
            memory=tuple(self._memory),
        )

    def set_input_port(self, port: int, value: int) -> None:
        """Set the value returned when reading from port `port` (0–255)."""
        if not (0 <= port < _NUM_PORTS):
            msg = f"port {port} out of range 0–{_NUM_PORTS - 1}"
            raise ValueError(msg)
        if not (0 <= value <= 255):
            msg = f"value {value} out of range 0–255"
            raise ValueError(msg)
        self._input_ports[port] = value

    def get_output_port(self, port: int) -> int:
        """Return the last value written to output port `port` (0–255)."""
        if not (0 <= port < _NUM_PORTS):
            msg = f"port {port} out of range 0–{_NUM_PORTS - 1}"
            raise ValueError(msg)
        return self._output_ports[port]

    # ── Memory helpers ────────────────────────────────────────────────────────

    def _read(self, addr: int) -> int:
        return self._memory[addr & 0xFFFF]

    def _write(self, addr: int, value: int) -> None:
        self._memory[addr & 0xFFFF] = value & 0xFF

    def _read16(self, addr: int) -> int:
        lo = self._memory[addr & 0xFFFF]
        hi = self._memory[(addr + 1) & 0xFFFF]
        return (hi << 8) | lo

    def _write16(self, addr: int, value: int) -> None:
        self._memory[addr & 0xFFFF] = value & 0xFF
        self._memory[(addr + 1) & 0xFFFF] = (value >> 8) & 0xFF

    def _fetch(self) -> int:
        """Read byte at PC, advance PC, increment R (low 7 bits)."""
        val = self._memory[self._pc.read()]
        self._pc.inc(1)
        # R register: auto-increment low 7 bits, bit 7 preserved
        self._r = ((self._r + 1) & 0x7F) | (self._r & 0x80)
        return val

    def _fetch_signed(self) -> int:
        """Read signed 8-bit displacement byte at PC."""
        b = self._fetch()
        return b - 256 if b >= 0x80 else b

    def _fetch16(self) -> int:
        lo = self._fetch()
        hi = self._fetch()
        return (hi << 8) | lo

    # ── Stack helpers ─────────────────────────────────────────────────────────

    def _push16(self, value: int) -> None:
        self._sp.dec(1)
        self._write(self._sp.read(), (value >> 8) & 0xFF)
        self._sp.dec(1)
        self._write(self._sp.read(), value & 0xFF)

    def _pop16(self) -> int:
        lo = self._read(self._sp.read())
        self._sp.inc(1)
        hi = self._read(self._sp.read())
        self._sp.inc(1)
        return (hi << 8) | lo

    # ── Register read/write by 3-bit code ────────────────────────────────────

    def _get_r(self, code: int) -> int:
        """Read 8-bit register by Z80 3-bit code (0=B..7=A, 6=(HL))."""
        if code == REG_MEM:
            hl = (self._rf.read8(REG_H) << 8) | self._rf.read8(REG_L)
            return self._read(hl)
        return self._rf.read8(code)

    def _set_r(self, code: int, value: int) -> None:
        """Write 8-bit register by Z80 3-bit code."""
        if code == REG_MEM:
            hl = (self._rf.read8(REG_H) << 8) | self._rf.read8(REG_L)
            self._write(hl, value)
        else:
            self._rf.write8(code, value)

    def _get_rp(self, code: int) -> int:
        """Read 16-bit register pair (0=BC, 1=DE, 2=HL, 3=SP)."""
        return self._rf.read16_pair(code, self._sp)

    def _set_rp(self, code: int, value: int) -> None:
        """Write 16-bit register pair."""
        self._rf.write16_pair(code, value, self._sp)

    def _get_rp_af(self, code: int) -> int:
        """Get register pair for PUSH/POP (code 3 = AF)."""
        if code == 3:
            flags = self._rf.read_flags()
            f = pack_f(
                flags['s'], flags['z'], flags['h'],
                flags['pv'], flags['n'], flags['c']
            )
            return (self._rf.read8(REG_A) << 8) | f
        return self._get_rp(code)

    def _set_rp_af(self, code: int, value: int) -> None:
        """Set register pair for PUSH/POP (code 3 = AF)."""
        if code == 3:
            self._rf.write8(REG_A, (value >> 8) & 0xFF)
            s, z, h, pv, n, c = unpack_f(value & 0xFF)
            self._rf.write_flags(s, z, h, pv, n, c)
        else:
            self._set_rp(code, value)

    # ── Flag access helpers ───────────────────────────────────────────────────

    def _flags(self) -> dict[str, int]:
        return self._rf.read_flags()

    def _set_flags(self, **kwargs: int) -> None:
        """Update specific flags, leaving others unchanged."""
        current = self._rf.read_flags()
        current.update(kwargs)
        self._rf.write_flags(
            current['s'], current['z'], current['h'],
            current['pv'], current['n'], current['c']
        )

    def _apply_alu(self, res: ALUResultZ80, *, update_c: bool = True) -> None:
        """Apply ALU result to flag register.

        Args:
            res:      ALU result with flag bits.
            update_c: If False, preserve existing C flag (for INC/DEC).
        """
        current = self._rf.read_flags()
        c = res.flag_c if update_c else current['c']
        self._rf.write_flags(
            res.flag_s, res.flag_z, res.flag_h, res.flag_pv, res.flag_n, c
        )

    def _cond(self, cc: int) -> bool:
        """Evaluate 3-bit condition code."""
        f = self._flags()
        if cc == 0:
            return not f['z']      # NZ
        if cc == 1:
            return bool(f['z'])    # Z
        if cc == 2:
            return not f['c']      # NC
        if cc == 3:
            return bool(f['c'])    # C
        if cc == 4:
            return not f['pv']     # PO (parity odd)
        if cc == 5:
            return bool(f['pv'])   # PE (parity even)
        if cc == 6:
            return not f['s']      # P (positive / sign clear)
        return bool(f['s'])        # M (minus / sign set)

    # ── Main dispatch ─────────────────────────────────────────────────────────

    def _fetch_and_execute(self) -> tuple[str, str]:
        """Fetch and execute one instruction. Returns (mnemonic, description)."""
        b = self._fetch()

        if b == 0xCB:
            return self._exec_cb()
        if b == 0xED:
            return self._exec_ed()
        if b == 0xDD:
            return self._exec_ddfd(ix=True)
        if b == 0xFD:
            return self._exec_ddfd(ix=False)

        return self._exec_main(b)

    # ── Main (unprefixed) instruction set ─────────────────────────────────────

    def _exec_main(self, op: int) -> tuple[str, str]:  # noqa: PLR0911, PLR0912, PLR0915
        """Execute an unprefixed opcode."""

        if op == 0x00:
            return "NOP", "No operation"

        if op == 0x76:
            self._halted = True
            return "HALT", "Halt CPU"

        # ── LD r, r' (group 01, excluding HALT) ─────────────────────────────
        if 0x40 <= op <= 0x7F:
            dst = (op >> 3) & 0x07
            src = op & 0x07
            self._set_r(dst, self._get_r(src))
            return "LD r,r'", f"LD {_REG_NAMES[dst]},{_REG_NAMES[src]}"

        # ── LD r, n ─────────────────────────────────────────────────────────
        if op & 0xC7 == 0x06:
            dst = (op >> 3) & 0x07
            n = self._fetch()
            self._set_r(dst, n)
            return "LD r,n", f"LD {_REG_NAMES[dst]},{n:#04x}"

        # ── 8-bit ALU with register operand (0x80–0xBF) ─────────────────────
        if 0x80 <= op <= 0xBF:
            alu_op = (op >> 3) & 0x07
            src = op & 0x07
            operand = self._get_r(src)
            self._alu8(alu_op, operand)
            return _ALU_NAMES[alu_op], f"{_ALU_NAMES[alu_op]} A,{_REG_NAMES[src]}"

        # ── ALU with immediate operand ───────────────────────────────────────
        if op & 0xC7 == 0xC6:
            alu_op = (op >> 3) & 0x07
            n = self._fetch()
            self._alu8(alu_op, n)
            return _ALU_NAMES[alu_op], f"{_ALU_NAMES[alu_op]} A,{n:#04x}"

        # ── INC r ────────────────────────────────────────────────────────────
        if op & 0xC7 == 0x04:
            r_code = (op >> 3) & 0x07
            v = self._get_r(r_code)
            res = inc8(v)
            self._set_r(r_code, res.result)
            self._apply_alu(res, update_c=False)
            return "INC", f"INC {_REG_NAMES[r_code]}"

        # ── DEC r ────────────────────────────────────────────────────────────
        if op & 0xC7 == 0x05:
            r_code = (op >> 3) & 0x07
            v = self._get_r(r_code)
            res = dec8(v)
            self._set_r(r_code, res.result)
            self._apply_alu(res, update_c=False)
            return "DEC", f"DEC {_REG_NAMES[r_code]}"

        # ── LD rp, nn ────────────────────────────────────────────────────────
        if op & 0xCF == 0x01:
            rp = (op >> 4) & 0x03
            nn = self._fetch16()
            self._set_rp(rp, nn)
            return "LD rp,nn", f"LD {_PAIR_NAMES[rp]},{nn:#06x}"

        # ── ADD HL, rp ───────────────────────────────────────────────────────
        if op & 0xCF == 0x09:
            rp = (op >> 4) & 0x03
            hl = (self._rf.read8(REG_H) << 8) | self._rf.read8(REG_L)
            rp_val = self._get_rp(rp)
            res = add16(hl, rp_val)
            self._rf.write8(REG_H, (res.result >> 8) & 0xFF)
            self._rf.write8(REG_L, res.result & 0xFF)
            # ADD HL,rp: only H, N, C affected (preserve S, Z, PV)
            self._set_flags(h=res.flag_h, n=0, c=res.flag_c)
            return "ADD HL,rp", f"ADD HL,{_PAIR_NAMES[rp]}"

        # ── INC rp ───────────────────────────────────────────────────────────
        if op & 0xCF == 0x03:
            rp = (op >> 4) & 0x03
            val, _, _ = add_16bit(self._get_rp(rp), 1, 0)
            self._set_rp(rp, val & 0xFFFF)
            return "INC rp", f"INC {_PAIR_NAMES[rp]}"

        # ── DEC rp ───────────────────────────────────────────────────────────
        if op & 0xCF == 0x0B:
            rp = (op >> 4) & 0x03
            val, _, _ = add_16bit(self._get_rp(rp), 0xFFFF, 0)  # +0xFFFF = -1 mod 2^16
            self._set_rp(rp, val & 0xFFFF)
            return "DEC rp", f"DEC {_PAIR_NAMES[rp]}"

        # ── LD SP, HL ────────────────────────────────────────────────────────
        if op == 0xF9:
            hl = (self._rf.read8(REG_H) << 8) | self._rf.read8(REG_L)
            self._sp.write(hl)
            return "LD SP,HL", "LD SP,HL"

        # ── LD HL, (nn) ──────────────────────────────────────────────────────
        if op == 0x2A:
            nn = self._fetch16()
            self._rf.write8(REG_L, self._read(nn))
            self._rf.write8(REG_H, self._read((nn + 1) & 0xFFFF))
            return "LD HL,(nn)", f"LD HL,({nn:#06x})"

        # ── LD (nn), HL ──────────────────────────────────────────────────────
        if op == 0x22:
            nn = self._fetch16()
            self._write(nn, self._rf.read8(REG_L))
            self._write((nn + 1) & 0xFFFF, self._rf.read8(REG_H))
            return "LD (nn),HL", f"LD ({nn:#06x}),HL"

        # ── LD A, (nn) ───────────────────────────────────────────────────────
        if op == 0x3A:
            nn = self._fetch16()
            self._rf.write8(REG_A, self._read(nn))
            return "LD A,(nn)", f"LD A,({nn:#06x})"

        # ── LD (nn), A ───────────────────────────────────────────────────────
        if op == 0x32:
            nn = self._fetch16()
            self._write(nn, self._rf.read8(REG_A))
            return "LD (nn),A", f"LD ({nn:#06x}),A"

        # ── LD A, (BC) / LD A, (DE) ──────────────────────────────────────────
        if op == 0x0A:
            bc = (self._rf.read8(REG_B) << 8) | self._rf.read8(REG_C)
            self._rf.write8(REG_A, self._read(bc))
            return "LD A,(BC)", "LD A,(BC)"

        if op == 0x1A:
            de = (self._rf.read8(REG_D) << 8) | self._rf.read8(REG_E)
            self._rf.write8(REG_A, self._read(de))
            return "LD A,(DE)", "LD A,(DE)"

        # ── LD (BC), A / LD (DE), A ──────────────────────────────────────────
        if op == 0x02:
            bc = (self._rf.read8(REG_B) << 8) | self._rf.read8(REG_C)
            self._write(bc, self._rf.read8(REG_A))
            return "LD (BC),A", "LD (BC),A"

        if op == 0x12:
            de = (self._rf.read8(REG_D) << 8) | self._rf.read8(REG_E)
            self._write(de, self._rf.read8(REG_A))
            return "LD (DE),A", "LD (DE),A"

        # ── PUSH / POP ───────────────────────────────────────────────────────
        if op & 0xCF == 0xC5:
            rp = (op >> 4) & 0x03
            self._push16(self._get_rp_af(rp))
            return "PUSH", f"PUSH {_PAIR_NAMES[rp] if rp != 3 else 'AF'}"

        if op & 0xCF == 0xC1:
            rp = (op >> 4) & 0x03
            self._set_rp_af(rp, self._pop16())
            return "POP", f"POP {_PAIR_NAMES[rp] if rp != 3 else 'AF'}"

        # ── Exchange ──────────────────────────────────────────────────────────
        if op == 0xEB:   # EX DE, HL
            d, h = self._rf.read8(REG_D), self._rf.read8(REG_H)
            e, lo = self._rf.read8(REG_E), self._rf.read8(REG_L)
            self._rf.write8(REG_H, d)
            self._rf.write8(REG_L, e)
            self._rf.write8(REG_D, h)
            self._rf.write8(REG_E, lo)
            return "EX DE,HL", "EX DE,HL"

        if op == 0x08:   # EX AF, AF'
            self._rf.exchange_af()
            return "EX AF,AF'", "EX AF,AF'"

        if op == 0xD9:   # EXX
            self._rf.exchange_bank()
            return "EXX", "EXX"

        if op == 0xE3:   # EX (SP), HL
            lo = self._read(self._sp.read())
            hi = self._read((self._sp.read() + 1) & 0xFFFF)
            self._write(self._sp.read(), self._rf.read8(REG_L))
            self._write((self._sp.read() + 1) & 0xFFFF, self._rf.read8(REG_H))
            self._rf.write8(REG_H, hi)
            self._rf.write8(REG_L, lo)
            return "EX (SP),HL", "EX (SP),HL"

        # ── Jumps ─────────────────────────────────────────────────────────────
        if op == 0xC3:   # JP nn
            nn = self._fetch16()
            self._pc.write(nn)
            return "JP", f"JP {nn:#06x}"

        if op & 0xC7 == 0xC2:   # JP cc, nn
            cc = (op >> 3) & 0x07
            nn = self._fetch16()
            if self._cond(cc):
                self._pc.write(nn)
            return "JP cc", f"JP {_COND_NAMES[cc]},{nn:#06x}"

        if op == 0xE9:   # JP (HL)
            hl = (self._rf.read8(REG_H) << 8) | self._rf.read8(REG_L)
            self._pc.write(hl)
            return "JP (HL)", "JP (HL)"

        if op == 0x18:   # JR e
            e = self._fetch_signed()
            self._pc.write((self._pc.read() + e) & 0xFFFF)
            return "JR", f"JR {e:+d}"

        if op == 0x20:   # JR NZ, e
            e = self._fetch_signed()
            if not self._flags()['z']:
                self._pc.write((self._pc.read() + e) & 0xFFFF)
            return "JR NZ", f"JR NZ,{e:+d}"

        if op == 0x28:   # JR Z, e
            e = self._fetch_signed()
            if self._flags()['z']:
                self._pc.write((self._pc.read() + e) & 0xFFFF)
            return "JR Z", f"JR Z,{e:+d}"

        if op == 0x30:   # JR NC, e
            e = self._fetch_signed()
            if not self._flags()['c']:
                self._pc.write((self._pc.read() + e) & 0xFFFF)
            return "JR NC", f"JR NC,{e:+d}"

        if op == 0x38:   # JR C, e
            e = self._fetch_signed()
            if self._flags()['c']:
                self._pc.write((self._pc.read() + e) & 0xFFFF)
            return "JR C", f"JR C,{e:+d}"

        if op == 0x10:   # DJNZ e
            e = self._fetch_signed()
            b = (self._rf.read8(REG_B) - 1) & 0xFF
            self._rf.write8(REG_B, b)
            if b != 0:
                self._pc.write((self._pc.read() + e) & 0xFFFF)
            return "DJNZ", f"DJNZ {e:+d}"

        # ── Call / Return ─────────────────────────────────────────────────────
        if op == 0xCD:   # CALL nn
            nn = self._fetch16()
            self._push16(self._pc.read())
            self._pc.write(nn)
            return "CALL", f"CALL {nn:#06x}"

        if op & 0xC7 == 0xC4:   # CALL cc, nn
            cc = (op >> 3) & 0x07
            nn = self._fetch16()
            if self._cond(cc):
                self._push16(self._pc.read())
                self._pc.write(nn)
            return "CALL cc", f"CALL {_COND_NAMES[cc]},{nn:#06x}"

        if op == 0xC9:   # RET
            self._pc.write(self._pop16())
            return "RET", "RET"

        if op & 0xC7 == 0xC0:   # RET cc
            cc = (op >> 3) & 0x07
            if self._cond(cc):
                self._pc.write(self._pop16())
            return "RET cc", f"RET {_COND_NAMES[cc]}"

        # ── RST ───────────────────────────────────────────────────────────────
        if op & 0xC7 == 0xC7:
            p = op & 0x38
            self._push16(self._pc.read())
            self._pc.write(p)
            return "RST", f"RST {p:#04x}"

        # ── Accumulator rotates ───────────────────────────────────────────────
        if op == 0x07:   # RLCA
            a = self._rf.read8(REG_A)
            res = rlca8(a)
            self._rf.write8(REG_A, res.result)
            self._set_flags(h=0, n=0, c=res.flag_c)
            return "RLCA", "RLCA"

        if op == 0x0F:   # RRCA
            a = self._rf.read8(REG_A)
            res = rrca8(a)
            self._rf.write8(REG_A, res.result)
            self._set_flags(h=0, n=0, c=res.flag_c)
            return "RRCA", "RRCA"

        if op == 0x17:   # RLA
            a = self._rf.read8(REG_A)
            res = rla8(a, self._flags()['c'])
            self._rf.write8(REG_A, res.result)
            self._set_flags(h=0, n=0, c=res.flag_c)
            return "RLA", "RLA"

        if op == 0x1F:   # RRA
            a = self._rf.read8(REG_A)
            res = rra8(a, self._flags()['c'])
            self._rf.write8(REG_A, res.result)
            self._set_flags(h=0, n=0, c=res.flag_c)
            return "RRA", "RRA"

        # ── DAA ───────────────────────────────────────────────────────────────
        if op == 0x27:
            f = self._flags()
            res = daa8(self._rf.read8(REG_A), f['n'], f['h'], f['c'])
            self._rf.write8(REG_A, res.result)
            self._rf.write_flags(
                res.flag_s, res.flag_z, res.flag_h, res.flag_pv, res.flag_n, res.flag_c
            )
            return "DAA", "DAA"

        # ── CPL ───────────────────────────────────────────────────────────────
        if op == 0x2F:
            a = self._rf.read8(REG_A)
            res = cpl8(a)
            self._rf.write8(REG_A, res.result)
            self._set_flags(h=1, n=1)
            return "CPL", "CPL"

        # ── CCF / SCF ─────────────────────────────────────────────────────────
        if op == 0x3F:   # CCF (complement carry)
            f = self._flags()
            self._set_flags(h=f['c'], n=0, c=1 - f['c'])
            return "CCF", "CCF"

        if op == 0x37:   # SCF (set carry)
            self._set_flags(h=0, n=0, c=1)
            return "SCF", "SCF"

        # ── I/O ───────────────────────────────────────────────────────────────
        if op == 0xD3:   # OUT (n), A
            n = self._fetch()
            self._output_ports[n] = self._rf.read8(REG_A)
            return "OUT", f"OUT ({n:#04x}),A"

        if op == 0xDB:   # IN A, (n)
            n = self._fetch()
            self._rf.write8(REG_A, self._input_ports[n])
            return "IN", f"IN A,({n:#04x})"

        # ── Interrupt control ─────────────────────────────────────────────────
        if op == 0xF3:   # DI
            self._iff1 = False
            self._iff2 = False
            return "DI", "DI"

        if op == 0xFB:   # EI
            self._iff1 = True
            self._iff2 = True
            return "EI", "EI"

        return f"??{op:#04x}", f"Unknown opcode {op:#04x}"

    # ── 8-bit ALU dispatch ────────────────────────────────────────────────────

    def _alu8(self, op: int, operand: int) -> None:
        """Execute 8-bit ALU operation on A via gate-level functions.

        op codes: 0=ADD, 1=ADC, 2=SUB, 3=SBC, 4=AND, 5=XOR, 6=OR, 7=CP
        """
        a = self._rf.read8(REG_A)
        f = self._flags()
        c = f['c']

        if op == 0:    # ADD A, m
            res = add8(a, operand, 0)
            self._rf.write8(REG_A, res.result)
            self._apply_alu(res)
        elif op == 1:  # ADC A, m
            res = add8(a, operand, c)
            self._rf.write8(REG_A, res.result)
            self._apply_alu(res)
        elif op == 2:  # SUB m
            res = sub8(a, operand, 0)
            self._rf.write8(REG_A, res.result)
            self._apply_alu(res)
        elif op == 3:  # SBC A, m
            res = sub8(a, operand, c)
            self._rf.write8(REG_A, res.result)
            self._apply_alu(res)
        elif op == 4:  # AND m
            res = and8(a, operand)
            self._rf.write8(REG_A, res.result)
            self._apply_alu(res)
        elif op == 5:  # XOR m
            res = xor8(a, operand)
            self._rf.write8(REG_A, res.result)
            self._apply_alu(res)
        elif op == 6:  # OR m
            res = or8(a, operand)
            self._rf.write8(REG_A, res.result)
            self._apply_alu(res)
        else:          # CP m (compare: like SUB but A unchanged)
            res = sub8(a, operand, 0)
            self._apply_alu(res)

    # ── CB-prefix: bit manipulation and rotate/shift ──────────────────────────

    def _exec_cb(self) -> tuple[str, str]:
        """Execute CB-prefixed instruction."""
        op = self._fetch()
        r_code = op & 0x07
        v = self._get_r(r_code)
        rot_op = (op >> 3) & 0x07
        bit = (op >> 3) & 0x07

        if op < 0x40:  # Rotate/shift (00xxxxxx)
            if rot_op == 0:
                res = rlc8(v)
            elif rot_op == 1:
                res = rrc8(v)
            elif rot_op == 2:
                res = rl8(v, self._flags()['c'])
            elif rot_op == 3:
                res = rr8(v, self._flags()['c'])
            elif rot_op == 4:
                res = sla8(v)
            elif rot_op == 5:
                res = sra8(v)
            elif rot_op == 6:  # SLL (undocumented: shift left, 1 into bit 0)
                # Treat like SLA but set bit 0
                bits_v = [1] + [((v >> i) & 1) for i in range(7)]
                from z80_gatelevel.bits import bits_to_int
                r_val = bits_to_int(bits_v)
                from z80_gatelevel.alu import (
                    ALUResultZ80,
                    compute_parity,
                    compute_zero,
                    int_to_bits,
                )
                r_bits = int_to_bits(r_val, 8)
                res = ALUResultZ80(
                    result=r_val,
                    flag_s=r_bits[7],
                    flag_z=compute_zero(r_bits),
                    flag_h=0,
                    flag_pv=compute_parity(r_bits),
                    flag_n=0,
                    flag_c=(v >> 7) & 1,
                )
            else:  # rot_op == 7: SRL
                res = srl8(v)
            self._set_r(r_code, res.result)
            self._apply_alu(res)
            return f"ROT{rot_op}", f"CB rot{rot_op} {_REG_NAMES[r_code]}"

        elif op < 0x80:  # BIT (01xxxxxx)
            res = bit_test(v, bit)
            # BIT only updates Z, H, N; S/PV/C from tested bit
            self._set_flags(z=res.flag_z, h=1, n=0)
            return "BIT", f"BIT {bit},{_REG_NAMES[r_code]}"

        elif op < 0xC0:  # RES (10xxxxxx)
            r_val = res_bit(v, bit)
            self._set_r(r_code, r_val)
            return "RES", f"RES {bit},{_REG_NAMES[r_code]}"

        else:  # SET (11xxxxxx)
            r_val = set_bit(v, bit)
            self._set_r(r_code, r_val)
            return "SET", f"SET {bit},{_REG_NAMES[r_code]}"

    # ── ED-prefix: extended instructions ─────────────────────────────────────

    def _exec_ed(self) -> tuple[str, str]:  # noqa: PLR0912, PLR0915
        """Execute ED-prefixed instruction."""
        op = self._fetch()

        # ── LD A, I / LD A, R ────────────────────────────────────────────────
        if op == 0x57:
            self._rf.write8(REG_A, self._i)
            s = (self._i >> 7) & 1
            z = 1 if self._i == 0 else 0
            pv = int(self._iff2)
            self._set_flags(s=s, z=z, h=0, pv=pv, n=0)
            return "LD A,I", "LD A,I"

        if op == 0x5F:
            self._rf.write8(REG_A, self._r)
            s = (self._r >> 7) & 1
            z = 1 if self._r == 0 else 0
            pv = int(self._iff2)
            self._set_flags(s=s, z=z, h=0, pv=pv, n=0)
            return "LD A,R", "LD A,R"

        if op == 0x47:
            self._i = self._rf.read8(REG_A)
            return "LD I,A", "LD I,A"

        if op == 0x4F:
            self._r = self._rf.read8(REG_A)
            return "LD R,A", "LD R,A"

        # ── 16-bit register load via memory ──────────────────────────────────
        if op & 0xCF == 0x4B:   # LD rp, (nn)
            rp = (op >> 4) & 0x03
            nn = self._fetch16()
            val = self._read16(nn)
            self._set_rp(rp, val)
            return "LD rp,(nn)", f"LD {_PAIR_NAMES[rp]},({nn:#06x})"

        if op & 0xCF == 0x43:   # LD (nn), rp
            rp = (op >> 4) & 0x03
            nn = self._fetch16()
            self._write16(nn, self._get_rp(rp))
            return "LD (nn),rp", f"LD ({nn:#06x}),{_PAIR_NAMES[rp]}"

        # ── ADC HL, rp ───────────────────────────────────────────────────────
        if op & 0xCF == 0x4A:
            rp = (op >> 4) & 0x03
            hl = (self._rf.read8(REG_H) << 8) | self._rf.read8(REG_L)
            rp_v = self._get_rp(rp)
            c = self._flags()['c']
            res = adc16(hl, rp_v, c)
            self._rf.write8(REG_H, (res.result >> 8) & 0xFF)
            self._rf.write8(REG_L, res.result & 0xFF)
            self._apply_alu(res)
            return "ADC HL,rp", f"ADC HL,{_PAIR_NAMES[rp]}"

        # ── SBC HL, rp ───────────────────────────────────────────────────────
        if op & 0xCF == 0x42:
            rp = (op >> 4) & 0x03
            hl = (self._rf.read8(REG_H) << 8) | self._rf.read8(REG_L)
            rp_v = self._get_rp(rp)
            c = self._flags()['c']
            res = sbc16(hl, rp_v, c)
            self._rf.write8(REG_H, (res.result >> 8) & 0xFF)
            self._rf.write8(REG_L, res.result & 0xFF)
            self._apply_alu(res)
            return "SBC HL,rp", f"SBC HL,{_PAIR_NAMES[rp]}"

        # ── NEG ───────────────────────────────────────────────────────────────
        if op == 0x44:
            a = self._rf.read8(REG_A)
            res = neg8(a)
            self._rf.write8(REG_A, res.result)
            self._apply_alu(res)
            return "NEG", "NEG"

        # ── Interrupt mode ────────────────────────────────────────────────────
        if op == 0x46:
            self._im = 0
            return "IM 0", "IM 0"
        if op == 0x56:
            self._im = 1
            return "IM 1", "IM 1"
        if op == 0x5E:
            self._im = 2
            return "IM 2", "IM 2"

        # ── RETI / RETN ───────────────────────────────────────────────────────
        if op == 0x4D:
            self._iff1 = self._iff2
            self._pc.write(self._pop16())
            return "RETI", "RETI"

        if op == 0x45:
            self._iff1 = self._iff2
            self._pc.write(self._pop16())
            return "RETN", "RETN"

        # ── Block operations ──────────────────────────────────────────────────
        if op == 0xA0:
            return self._ldi()
        if op == 0xA8:
            return self._ldd()
        if op == 0xB0:
            return self._ldir()
        if op == 0xB8:
            return self._lddr()
        if op == 0xA1:
            return self._cpi_op()
        if op == 0xA9:
            return self._cpd_op()
        if op == 0xB1:
            return self._cpir_op()
        if op == 0xB9:
            return self._cpdr_op()

        # ── IN r, (C) / OUT (C), r ────────────────────────────────────────────
        if op & 0xC7 == 0x40:
            r_code = (op >> 3) & 0x07
            val = self._input_ports[self._rf.read8(REG_C)]
            if r_code != 6:
                self._set_r(r_code, val)
            self._set_flags(h=0, n=0)
            return "IN r,(C)", "IN r,(C)"

        if op & 0xC7 == 0x41:
            r_code = (op >> 3) & 0x07
            val = self._get_r(r_code) if r_code != 6 else 0
            self._output_ports[self._rf.read8(REG_C)] = val
            return "OUT (C),r", "OUT (C),r"

        return f"ED {op:#04x}", f"ED unknown {op:#04x}"

    # ── DD/FD prefix: index register instructions ─────────────────────────────

    def _exec_ddfd(self, ix: bool) -> tuple[str, str]:  # noqa: PLR0912, PLR0915
        """Handle DD-prefixed (IX) or FD-prefixed (IY) instructions."""
        idx_val = self._rf.read_ix() if ix else self._rf.read_iy()
        prefix_name = "IX" if ix else "IY"
        op = self._fetch()

        def write_idx(val: int) -> None:
            if ix:
                self._rf.write_ix(val & 0xFFFF)
            else:
                self._rf.write_iy(val & 0xFFFF)

        # DDCB / FDCB
        if op == 0xCB:
            return self._exec_ddcb(idx_val, prefix_name, ix)

        # LD (IX+d), n
        if op == 0x36:
            d = self._fetch_signed()
            n = self._fetch()
            self._write((idx_val + d) & 0xFFFF, n)
            return "LD (IX+d),n", f"LD ({prefix_name}{d:+d}),{n:#04x}"

        # LD IX, nn
        if op == 0x21:
            nn = self._fetch16()
            write_idx(nn)
            return f"LD {prefix_name},nn", f"LD {prefix_name},{nn:#06x}"

        # LD IX, (nn)
        if op == 0x2A:
            nn = self._fetch16()
            val = self._read16(nn)
            write_idx(val)
            return f"LD {prefix_name},(nn)", f"LD {prefix_name},({nn:#06x})"

        # LD (nn), IX
        if op == 0x22:
            nn = self._fetch16()
            self._write16(nn, idx_val)
            return f"LD (nn),{prefix_name}", f"LD ({nn:#06x}),{prefix_name}"

        # LD SP, IX
        if op == 0xF9:
            self._sp.write(idx_val)
            return f"LD SP,{prefix_name}", f"LD SP,{prefix_name}"

        # PUSH IX / POP IX
        if op == 0xE5:
            self._push16(idx_val)
            return f"PUSH {prefix_name}", f"PUSH {prefix_name}"

        if op == 0xE1:
            write_idx(self._pop16())
            return f"POP {prefix_name}", f"POP {prefix_name}"

        # ADD IX, rp
        if op & 0xCF == 0x09:
            rp = (op >> 4) & 0x03
            rp_val = self._get_rp(rp) if rp != 2 else idx_val
            res = add16(idx_val, rp_val)
            write_idx(res.result)
            self._set_flags(h=res.flag_h, n=0, c=res.flag_c)
            return f"ADD {prefix_name},rp", f"ADD {prefix_name},{_PAIR_NAMES[rp]}"

        # INC IX / DEC IX
        if op == 0x23:
            write_idx((idx_val + 1) & 0xFFFF)
            return f"INC {prefix_name}", f"INC {prefix_name}"

        if op == 0x2B:
            write_idx((idx_val - 1) & 0xFFFF)
            return f"DEC {prefix_name}", f"DEC {prefix_name}"

        # LD r, (IX+d) or LD (IX+d), r
        if 0x40 <= op <= 0x7F and op != 0x76:
            dst = (op >> 3) & 0x07
            src = op & 0x07
            if src == 6:  # source is (IX+d)
                d = self._fetch_signed()
                val = self._read((idx_val + d) & 0xFFFF)
                self._set_r(dst, val)
                return "LD r,(IX+d)", f"LD {_REG_NAMES[dst]},({prefix_name}{d:+d})"
            if dst == 6:  # destination is (IX+d)
                d = self._fetch_signed()
                val = self._get_r(src)
                self._write((idx_val + d) & 0xFFFF, val)
                return "LD (IX+d),r", f"LD ({prefix_name}{d:+d}),{_REG_NAMES[src]}"

        # ALU ops with (IX+d)
        if 0x86 <= op <= 0xBE and (op & 0x07) == 0x06:
            alu_op = (op >> 3) & 0x07
            d = self._fetch_signed()
            val = self._read((idx_val + d) & 0xFFFF)
            self._alu8(alu_op, val)
            mn = f"{_ALU_NAMES[alu_op]} (IX+d)"
            desc = f"{_ALU_NAMES[alu_op]} ({prefix_name}{d:+d})"
            return mn, desc

        # INC/DEC (IX+d)
        if op == 0x34:
            d = self._fetch_signed()
            addr = (idx_val + d) & 0xFFFF
            v = self._read(addr)
            res = inc8(v)
            self._write(addr, res.result)
            self._apply_alu(res, update_c=False)
            return "INC (IX+d)", f"INC ({prefix_name}{d:+d})"

        if op == 0x35:
            d = self._fetch_signed()
            addr = (idx_val + d) & 0xFFFF
            v = self._read(addr)
            res = dec8(v)
            self._write(addr, res.result)
            self._apply_alu(res, update_c=False)
            return "DEC (IX+d)", f"DEC ({prefix_name}{d:+d})"

        # JP (IX)
        if op == 0xE9:
            self._pc.write(idx_val)
            return f"JP ({prefix_name})", f"JP ({prefix_name})"

        # EX (SP), IX
        if op == 0xE3:
            lo = self._read(self._sp.read())
            hi = self._read((self._sp.read() + 1) & 0xFFFF)
            self._write(self._sp.read(), idx_val & 0xFF)
            self._write((self._sp.read() + 1) & 0xFFFF, (idx_val >> 8) & 0xFF)
            write_idx((hi << 8) | lo)
            return f"EX (SP),{prefix_name}", f"EX (SP),{prefix_name}"

        return f"DD/FD {op:#04x}", f"DD/FD unknown {op:#04x}"

    def _exec_ddcb(self, idx_val: int, prefix_name: str, ix: bool) -> tuple[str, str]:
        """Handle DDCB / FDCB prefixed bit instructions on (IX+d)/(IY+d)."""
        d = self._fetch_signed()
        op = self._fetch()
        addr = (idx_val + d) & 0xFFFF
        v = self._read(addr)
        bit = (op >> 3) & 0x07
        r_code = op & 0x07
        rot_op = (op >> 3) & 0x07

        if op < 0x40:  # rotate/shift (IX+d)
            if rot_op == 0:
                res = rlc8(v)
            elif rot_op == 1:
                res = rrc8(v)
            elif rot_op == 2:
                res = rl8(v, self._flags()['c'])
            elif rot_op == 3:
                res = rr8(v, self._flags()['c'])
            elif rot_op == 4:
                res = sla8(v)
            elif rot_op == 5:
                res = sra8(v)
            else:
                res = srl8(v)
            self._write(addr, res.result)
            if r_code != 6:
                self._set_r(r_code, res.result)
            self._apply_alu(res)
            return f"ROT ({prefix_name}+d)", f"ROT ({prefix_name}{d:+d})"

        elif op < 0x80:  # BIT
            res = bit_test(v, bit)
            self._set_flags(z=res.flag_z, h=1, n=0)
            return f"BIT ({prefix_name}+d)", f"BIT {bit},({prefix_name}{d:+d})"

        elif op < 0xC0:  # RES
            r_val = res_bit(v, bit)
            self._write(addr, r_val)
            if r_code != 6:
                self._set_r(r_code, r_val)
            return f"RES ({prefix_name}+d)", f"RES {bit},({prefix_name}{d:+d})"

        else:  # SET
            r_val = set_bit(v, bit)
            self._write(addr, r_val)
            if r_code != 6:
                self._set_r(r_code, r_val)
            return f"SET ({prefix_name}+d)", f"SET {bit},({prefix_name}{d:+d})"

    # ── Block operations ──────────────────────────────────────────────────────

    def _ldi(self) -> tuple[str, str]:
        """LDI: (DE) ← (HL); HL++; DE++; BC--. PV set if BC≠0 after.

        BC decrement uses explicit parentheses to avoid Python precedence
        issues: `A | B - 1` would parse as `A | (B - 1)` due to `-` binding
        tighter than `|`. We always compute the full 16-bit pair first, then
        subtract 1.
        """
        src = (self._rf.read8(REG_H) << 8) | self._rf.read8(REG_L)
        dst = (self._rf.read8(REG_D) << 8) | self._rf.read8(REG_E)
        self._write(dst, self._read(src))
        hl = (src + 1) & 0xFFFF
        self._rf.write8(REG_H, hl >> 8)
        self._rf.write8(REG_L, hl & 0xFF)
        de = (dst + 1) & 0xFFFF
        self._rf.write8(REG_D, de >> 8)
        self._rf.write8(REG_E, de & 0xFF)
        bc_val = (self._rf.read8(REG_B) << 8) | self._rf.read8(REG_C)
        bc = (bc_val - 1) & 0xFFFF
        self._rf.write8(REG_B, bc >> 8)
        self._rf.write8(REG_C, bc & 0xFF)
        self._set_flags(h=0, n=0, pv=1 if bc != 0 else 0)
        return "LDI", "LDI"

    def _ldd(self) -> tuple[str, str]:
        """LDD: (DE) ← (HL); HL--; DE--; BC--.

        See _ldi() for the BC decrement parenthesisation note.
        """
        src = (self._rf.read8(REG_H) << 8) | self._rf.read8(REG_L)
        dst = (self._rf.read8(REG_D) << 8) | self._rf.read8(REG_E)
        self._write(dst, self._read(src))
        hl = (src - 1) & 0xFFFF
        self._rf.write8(REG_H, hl >> 8)
        self._rf.write8(REG_L, hl & 0xFF)
        de = (dst - 1) & 0xFFFF
        self._rf.write8(REG_D, de >> 8)
        self._rf.write8(REG_E, de & 0xFF)
        bc_val = (self._rf.read8(REG_B) << 8) | self._rf.read8(REG_C)
        bc = (bc_val - 1) & 0xFFFF
        self._rf.write8(REG_B, bc >> 8)
        self._rf.write8(REG_C, bc & 0xFF)
        self._set_flags(h=0, n=0, pv=1 if bc != 0 else 0)
        return "LDD", "LDD"

    # Maximum iterations for block operations. BC is 16-bit so the worst
    # case is 65535 iterations. We cap to the full address space size to
    # prevent an unbounded inner loop from bypassing the outer max_steps
    # guard in execute(). In practice a legitimate Z80 program never sets
    # BC=0xFFFF for a block copy, but a crafted input could.
    _BLOCK_MAX: int = 65536

    def _ldir(self) -> tuple[str, str]:
        """LDIR: repeat LDI until BC=0. Capped at _BLOCK_MAX iterations."""
        for _ in range(self._BLOCK_MAX):
            self._ldi()
            bc = (self._rf.read8(REG_B) << 8) | self._rf.read8(REG_C)
            if bc == 0:
                break
        return "LDIR", "LDIR"

    def _lddr(self) -> tuple[str, str]:
        """LDDR: repeat LDD until BC=0. Capped at _BLOCK_MAX iterations."""
        for _ in range(self._BLOCK_MAX):
            self._ldd()
            bc = (self._rf.read8(REG_B) << 8) | self._rf.read8(REG_C)
            if bc == 0:
                break
        return "LDDR", "LDDR"

    def _cpi_op(self) -> tuple[str, str]:
        """CPI: compare A with (HL); HL++; BC--.

        See _ldi() for the BC decrement parenthesisation note.
        """
        hl = (self._rf.read8(REG_H) << 8) | self._rf.read8(REG_L)
        m = self._read(hl)
        res = sub8(self._rf.read8(REG_A), m, 0)
        hl = (hl + 1) & 0xFFFF
        self._rf.write8(REG_H, hl >> 8)
        self._rf.write8(REG_L, hl & 0xFF)
        bc_val = (self._rf.read8(REG_B) << 8) | self._rf.read8(REG_C)
        bc = (bc_val - 1) & 0xFFFF
        self._rf.write8(REG_B, bc >> 8)
        self._rf.write8(REG_C, bc & 0xFF)
        pv = 1 if bc != 0 else 0
        self._set_flags(s=res.flag_s, z=res.flag_z, h=res.flag_h, pv=pv, n=1)
        return "CPI", "CPI"

    def _cpd_op(self) -> tuple[str, str]:
        """CPD: compare A with (HL); HL--; BC--.

        See _ldi() for the BC decrement parenthesisation note.
        """
        hl = (self._rf.read8(REG_H) << 8) | self._rf.read8(REG_L)
        m = self._read(hl)
        res = sub8(self._rf.read8(REG_A), m, 0)
        hl = (hl - 1) & 0xFFFF
        self._rf.write8(REG_H, hl >> 8)
        self._rf.write8(REG_L, hl & 0xFF)
        bc_val = (self._rf.read8(REG_B) << 8) | self._rf.read8(REG_C)
        bc = (bc_val - 1) & 0xFFFF
        self._rf.write8(REG_B, bc >> 8)
        self._rf.write8(REG_C, bc & 0xFF)
        pv = 1 if bc != 0 else 0
        self._set_flags(s=res.flag_s, z=res.flag_z, h=res.flag_h, pv=pv, n=1)
        return "CPD", "CPD"

    def _cpir_op(self) -> tuple[str, str]:
        """CPIR: repeat CPI until match or BC=0. Capped at _BLOCK_MAX."""
        for _ in range(self._BLOCK_MAX):
            self._cpi_op()
            bc = (self._rf.read8(REG_B) << 8) | self._rf.read8(REG_C)
            if self._flags()["z"] or bc == 0:
                break
        return "CPIR", "CPIR"

    def _cpdr_op(self) -> tuple[str, str]:
        """CPDR: repeat CPD until match or BC=0. Capped at _BLOCK_MAX."""
        for _ in range(self._BLOCK_MAX):
            self._cpd_op()
            bc = (self._rf.read8(REG_B) << 8) | self._rf.read8(REG_C)
            if self._flags()["z"] or bc == 0:
                break
        return "CPDR", "CPDR"
