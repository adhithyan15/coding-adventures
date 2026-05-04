"""MOS 6502 behavioral simulator.

This module implements the full MOS Technology 6502 (NMOS) instruction set
as it was documented in the MOS Technology MCS6500 Microcomputer Family
Programming Manual (1976) and corrected by later references including the
6502.org community documentation.

Key design decisions
--------------------
*Behavioral model*: operations use Python integer arithmetic, not gate
functions. For a gate-level implementation see the companion package
``mos6502-gatelevel`` (Layer 07j2).

*Halt condition*: BRK (opcode 0x00) is treated as HALT — the simulator stops
and sets ``halted=True`` in the state. This matches the convention used
throughout the simulator stack (HLT for 8080, TRAP for IBM 704, etc.).
Real 6502 BRK is a software interrupt; this distinction only matters for
programs expecting real interrupt-driven BRK behaviour.

*Memory-mapped I/O*: the 6502 has no IN/OUT instructions. Instead, reads and
writes to addresses 0xFF00–0xFFEF are intercepted and routed to input_ports
and output_ports arrays (ports 0–239). This lets callers inject sensor data
and observe output data without changing the instruction set.

*The indirect JMP bug*: ``JMP ($10FF)`` reads the high byte from ``$1000``,
not ``$1100``. This is a documented hardware bug in all NMOS 6502 chips.
This simulator replicates it exactly.

*Decimal mode*: ADC and SBC in decimal mode (D=1) use BCD correction per
NMOS 6502 behaviour. N/V/Z flags in decimal mode reflect the *binary* result
before BCD correction (the 65C02 fixes this; NMOS does not).
"""

from __future__ import annotations

from simulator_protocol import ExecutionResult, Simulator, StepTrace

from mos6502_simulator.flags import (
    bcd_add,
    bcd_sub,
    compute_nz,
    compute_overflow_add,
    compute_overflow_sub,
    unpack_p,
)
from mos6502_simulator.state import MOS6502State

# ── Power-on defaults ────────────────────────────────────────────────────────
_RESET_S = 0xFD          # Stack pointer after reset
_RESET_P = 0x24          # P = bit5 always 1, I=1

# ── Memory-mapped I/O range ──────────────────────────────────────────────────
_IO_BASE = 0xFF00        # Reads/writes here → port arrays
_IO_END  = 0xFFEF        # Last port address (port 239)
_NUM_PORTS = 240

# ── Addressing mode codes (internal) ────────────────────────────────────────
_IMM  = 0    # Immediate:          #$nn
_ZP   = 1    # Zero Page:          $nn
_ZPX  = 2    # Zero Page,X:        $nn,X
_ZPY  = 3    # Zero Page,Y:        $nn,Y
_ABS  = 4    # Absolute:           $nnnn
_ABX  = 5    # Absolute,X:         $nnnn,X
_ABY  = 6    # Absolute,Y:         $nnnn,Y
_INX  = 7    # (Indirect,X):       ($nn,X)
_INY  = 8    # (Indirect),Y:       ($nn),Y
_IMP  = 9    # Implied
_ACC  = 10   # Accumulator
_REL  = 11   # Relative (branches)
_IND  = 12   # Absolute Indirect   (JMP only)

# ── Opcode table ─────────────────────────────────────────────────────────────
# Each entry: (mnemonic, addressing_mode)
# Opcodes not in this table raise ValueError ("illegal opcode").
# We include all 151 official NMOS 6502 opcodes.
_OPTABLE: dict[int, tuple[str, int]] = {
    # BRK / NOP
    0x00: ("BRK", _IMP),
    0xEA: ("NOP", _IMP),

    # Load A
    0xA9: ("LDA", _IMM),
    0xA5: ("LDA", _ZP),
    0xB5: ("LDA", _ZPX),
    0xAD: ("LDA", _ABS),
    0xBD: ("LDA", _ABX),
    0xB9: ("LDA", _ABY),
    0xA1: ("LDA", _INX),
    0xB1: ("LDA", _INY),

    # Load X
    0xA2: ("LDX", _IMM),
    0xA6: ("LDX", _ZP),
    0xB6: ("LDX", _ZPY),
    0xAE: ("LDX", _ABS),
    0xBE: ("LDX", _ABY),

    # Load Y
    0xA0: ("LDY", _IMM),
    0xA4: ("LDY", _ZP),
    0xB4: ("LDY", _ZPX),
    0xAC: ("LDY", _ABS),
    0xBC: ("LDY", _ABX),

    # Store A
    0x85: ("STA", _ZP),
    0x95: ("STA", _ZPX),
    0x8D: ("STA", _ABS),
    0x9D: ("STA", _ABX),
    0x99: ("STA", _ABY),
    0x81: ("STA", _INX),
    0x91: ("STA", _INY),

    # Store X
    0x86: ("STX", _ZP),
    0x96: ("STX", _ZPY),
    0x8E: ("STX", _ABS),

    # Store Y
    0x84: ("STY", _ZP),
    0x94: ("STY", _ZPX),
    0x8C: ("STY", _ABS),

    # Register transfers
    0xAA: ("TAX", _IMP),
    0xA8: ("TAY", _IMP),
    0x8A: ("TXA", _IMP),
    0x98: ("TYA", _IMP),
    0xBA: ("TSX", _IMP),
    0x9A: ("TXS", _IMP),

    # Stack
    0x48: ("PHA", _IMP),
    0x68: ("PLA", _IMP),
    0x08: ("PHP", _IMP),
    0x28: ("PLP", _IMP),

    # ADC
    0x69: ("ADC", _IMM),
    0x65: ("ADC", _ZP),
    0x75: ("ADC", _ZPX),
    0x6D: ("ADC", _ABS),
    0x7D: ("ADC", _ABX),
    0x79: ("ADC", _ABY),
    0x61: ("ADC", _INX),
    0x71: ("ADC", _INY),

    # SBC
    0xE9: ("SBC", _IMM),
    0xE5: ("SBC", _ZP),
    0xF5: ("SBC", _ZPX),
    0xED: ("SBC", _ABS),
    0xFD: ("SBC", _ABX),
    0xF9: ("SBC", _ABY),
    0xE1: ("SBC", _INX),
    0xF1: ("SBC", _INY),

    # AND
    0x29: ("AND", _IMM),
    0x25: ("AND", _ZP),
    0x35: ("AND", _ZPX),
    0x2D: ("AND", _ABS),
    0x3D: ("AND", _ABX),
    0x39: ("AND", _ABY),
    0x21: ("AND", _INX),
    0x31: ("AND", _INY),

    # ORA
    0x09: ("ORA", _IMM),
    0x05: ("ORA", _ZP),
    0x15: ("ORA", _ZPX),
    0x0D: ("ORA", _ABS),
    0x1D: ("ORA", _ABX),
    0x19: ("ORA", _ABY),
    0x01: ("ORA", _INX),
    0x11: ("ORA", _INY),

    # EOR
    0x49: ("EOR", _IMM),
    0x45: ("EOR", _ZP),
    0x55: ("EOR", _ZPX),
    0x4D: ("EOR", _ABS),
    0x5D: ("EOR", _ABX),
    0x59: ("EOR", _ABY),
    0x41: ("EOR", _INX),
    0x51: ("EOR", _INY),

    # BIT
    0x24: ("BIT", _ZP),
    0x2C: ("BIT", _ABS),

    # INC
    0xE6: ("INC", _ZP),
    0xF6: ("INC", _ZPX),
    0xEE: ("INC", _ABS),
    0xFE: ("INC", _ABX),

    # INX / INY
    0xE8: ("INX", _IMP),
    0xC8: ("INY", _IMP),

    # DEC
    0xC6: ("DEC", _ZP),
    0xD6: ("DEC", _ZPX),
    0xCE: ("DEC", _ABS),
    0xDE: ("DEC", _ABX),

    # DEX / DEY
    0xCA: ("DEX", _IMP),
    0x88: ("DEY", _IMP),

    # ASL
    0x0A: ("ASL", _ACC),
    0x06: ("ASL", _ZP),
    0x16: ("ASL", _ZPX),
    0x0E: ("ASL", _ABS),
    0x1E: ("ASL", _ABX),

    # LSR
    0x4A: ("LSR", _ACC),
    0x46: ("LSR", _ZP),
    0x56: ("LSR", _ZPX),
    0x4E: ("LSR", _ABS),
    0x5E: ("LSR", _ABX),

    # ROL
    0x2A: ("ROL", _ACC),
    0x26: ("ROL", _ZP),
    0x36: ("ROL", _ZPX),
    0x2E: ("ROL", _ABS),
    0x3E: ("ROL", _ABX),

    # ROR
    0x6A: ("ROR", _ACC),
    0x66: ("ROR", _ZP),
    0x76: ("ROR", _ZPX),
    0x6E: ("ROR", _ABS),
    0x7E: ("ROR", _ABX),

    # CMP
    0xC9: ("CMP", _IMM),
    0xC5: ("CMP", _ZP),
    0xD5: ("CMP", _ZPX),
    0xCD: ("CMP", _ABS),
    0xDD: ("CMP", _ABX),
    0xD9: ("CMP", _ABY),
    0xC1: ("CMP", _INX),
    0xD1: ("CMP", _INY),

    # CPX
    0xE0: ("CPX", _IMM),
    0xE4: ("CPX", _ZP),
    0xEC: ("CPX", _ABS),

    # CPY
    0xC0: ("CPY", _IMM),
    0xC4: ("CPY", _ZP),
    0xCC: ("CPY", _ABS),

    # Branches (all REL mode)
    0x90: ("BCC", _REL),
    0xB0: ("BCS", _REL),
    0xF0: ("BEQ", _REL),
    0xD0: ("BNE", _REL),
    0x10: ("BPL", _REL),
    0x30: ("BMI", _REL),
    0x50: ("BVC", _REL),
    0x70: ("BVS", _REL),

    # Jumps
    0x4C: ("JMP", _ABS),
    0x6C: ("JMP", _IND),
    0x20: ("JSR", _ABS),
    0x60: ("RTS", _IMP),
    0x40: ("RTI", _IMP),

    # Flag instructions
    0x18: ("CLC", _IMP),
    0x38: ("SEC", _IMP),
    0xD8: ("CLD", _IMP),
    0xF8: ("SED", _IMP),
    0x58: ("CLI", _IMP),
    0x78: ("SEI", _IMP),
    0xB8: ("CLV", _IMP),
}


class MOS6502Simulator(Simulator[MOS6502State]):
    """Behavioral simulator for the MOS 6502 (NMOS) microprocessor.

    Implements the full SIM00 Simulator[MOS6502State] protocol:
    reset(), load(), step(), execute(), get_state().

    Memory-mapped I/O:
      Reads from 0xFF00–0xFFEF → input_ports[port]  (port = addr - 0xFF00)
      Writes to  0xFF00–0xFFEF → output_ports[port]

    Example::

        sim = MOS6502Simulator()
        result = sim.execute(bytes([
            0xA9, 0x0A,  # LDA #10
            0x69, 0x05,  # ADC #5
            0x00,        # BRK
        ]))
        assert result.final_state.a == 15
    """

    def __init__(self) -> None:
        self._memory = bytearray(65536)
        self._a = 0
        self._x = 0
        self._y = 0
        self._s = _RESET_S
        self._pc = 0
        # Flags
        self._flag_n = False
        self._flag_v = False
        self._flag_b = False
        self._flag_d = False
        self._flag_i = True    # I=1 at power-on
        self._flag_z = False
        self._flag_c = False
        self._halted = False
        # I/O
        self._input_ports: list[int] = [0] * _NUM_PORTS
        self._output_ports: list[int] = [0] * _NUM_PORTS

    # ── SIM00 protocol ───────────────────────────────────────────────────────

    def reset(self) -> None:
        """Reset the CPU to power-on state.

        Registers: A=X=Y=0, S=0xFD, PC=0x0000
        Flags:     I=1, all others False; bit5 always 1 (implicit)
        Memory:    cleared to all zeros
        """
        self._memory = bytearray(65536)
        self._a = 0
        self._x = 0
        self._y = 0
        self._s = _RESET_S
        self._pc = 0
        self._flag_n = False
        self._flag_v = False
        self._flag_b = False
        self._flag_d = False
        self._flag_i = True
        self._flag_z = False
        self._flag_c = False
        self._halted = False

    def load(self, program: bytes, origin: int = 0x0000) -> None:
        """Write program bytes into memory at origin and set PC.

        Args:
            program: Machine code bytes to load.
            origin:  Start address (default 0x0000).
        """
        if not (0 <= origin <= 0xFFFF):
            raise ValueError(f"origin {origin:#06x} out of range 0x0000–0xFFFF")
        for i, byte in enumerate(program):
            addr = (origin + i) & 0xFFFF
            self._memory[addr] = byte & 0xFF
        self._pc = origin
        self._halted = False

    def step(self) -> StepTrace:
        """Execute one instruction and return a StepTrace.

        Raises:
            RuntimeError: If the CPU is halted (BRK was executed).
        """
        if self._halted:
            raise RuntimeError("CPU is halted — call reset() or load() first")

        pc_before = self._pc
        opcode = self._read_pc()

        if opcode not in _OPTABLE:
            raise ValueError(
                f"Illegal opcode {opcode:#04x} at PC={pc_before:#06x}"
            )

        mnemonic, mode = _OPTABLE[opcode]
        desc = self._execute(opcode, mnemonic, mode)
        pc_after = self._pc

        return StepTrace(
            pc_before=pc_before,
            pc_after=pc_after,
            mnemonic=mnemonic,
            description=desc,
        )

    def execute(
        self,
        program: bytes,
        origin: int = 0x0000,
        max_steps: int = 100_000,
    ) -> ExecutionResult[MOS6502State]:
        """Load and run until BRK or max_steps.

        I/O port values set before calling execute() are preserved.
        The reset() clears memory but not the port arrays.

        Args:
            program:   Machine code bytes.
            origin:    Load address (default 0x0000).
            max_steps: Safety limit to prevent infinite loops.

        Returns:
            ExecutionResult with final state, step count, and trace list.
        """
        saved_input = list(self._input_ports)
        saved_output = list(self._output_ports)

        self.reset()
        self._input_ports = saved_input
        self._output_ports = saved_output
        self.load(program, origin)

        traces: list[StepTrace] = []
        steps = 0

        while not self._halted and steps < max_steps:
            trace = self.step()
            traces.append(trace)
            steps += 1

        return ExecutionResult(
            halted=self._halted,
            steps=steps,
            final_state=self.get_state(),
            error=None,
            traces=traces,
        )

    def get_state(self) -> MOS6502State:
        """Return an immutable snapshot of the current CPU state."""
        return MOS6502State(
            a=self._a,
            x=self._x,
            y=self._y,
            s=self._s,
            pc=self._pc,
            flag_n=self._flag_n,
            flag_v=self._flag_v,
            flag_b=self._flag_b,
            flag_d=self._flag_d,
            flag_i=self._flag_i,
            flag_z=self._flag_z,
            flag_c=self._flag_c,
            halted=self._halted,
            memory=tuple(self._memory),
        )

    def set_input_port(self, port: int, value: int) -> None:
        """Set the value that will be returned when reading port `port`.

        Ports 0–239 map to memory addresses 0xFF00–0xFFEF.

        Args:
            port:  Port number 0–239.
            value: Byte value 0–255.

        Raises:
            ValueError: If port or value is out of range.
        """
        if not (0 <= port < _NUM_PORTS):
            raise ValueError(f"port {port} out of range 0–{_NUM_PORTS - 1}")
        if not (0 <= value <= 255):
            raise ValueError(f"value {value} out of range 0–255")
        self._input_ports[port] = value

    def get_output_port(self, port: int) -> int:
        """Return the last value written to output port `port`.

        Args:
            port: Port number 0–239.

        Raises:
            ValueError: If port is out of range.
        """
        if not (0 <= port < _NUM_PORTS):
            raise ValueError(f"port {port} out of range 0–{_NUM_PORTS - 1}")
        return self._output_ports[port]

    # ── Internal helpers ─────────────────────────────────────────────────────

    def _read_pc(self) -> int:
        """Fetch the byte at PC and advance PC by 1."""
        value = self._memory[self._pc]
        self._pc = (self._pc + 1) & 0xFFFF
        return value

    def _read_pc16(self) -> int:
        """Fetch a 16-bit little-endian word at PC and advance PC by 2."""
        lo = self._read_pc()
        hi = self._read_pc()
        return (hi << 8) | lo

    def _read_mem(self, addr: int) -> int:
        """Read a byte from memory, intercepting memory-mapped I/O."""
        addr &= 0xFFFF
        if _IO_BASE <= addr <= _IO_END:
            return self._input_ports[addr - _IO_BASE]
        return self._memory[addr]

    def _write_mem(self, addr: int, value: int) -> None:
        """Write a byte to memory, intercepting memory-mapped I/O."""
        addr &= 0xFFFF
        value &= 0xFF
        if _IO_BASE <= addr <= _IO_END:
            self._output_ports[addr - _IO_BASE] = value
        else:
            self._memory[addr] = value

    def _push(self, value: int) -> None:
        """Push a byte onto the stack (0x0100 + S page)."""
        self._memory[0x0100 | self._s] = value & 0xFF
        self._s = (self._s - 1) & 0xFF

    def _pull(self) -> int:
        """Pull a byte from the stack."""
        self._s = (self._s + 1) & 0xFF
        return self._memory[0x0100 | self._s]

    def _set_nz(self, value: int) -> None:
        """Update N and Z flags from an 8-bit value."""
        self._flag_n, self._flag_z = compute_nz(value)

    def _resolve_address(self, mode: int) -> int | None:
        """Decode the effective address for an addressing mode.

        Returns the effective *memory address* (int), or None for
        modes that don't produce a memory address (IMP, ACC, REL).
        The PC is advanced past the operand byte(s).

        Special case: IMM returns the *address of the immediate byte*
        (i.e., the current PC before the byte is read), so the caller
        can read from memory to get the value.
        """
        if mode in (_IMP, _ACC):
            return None

        if mode == _IMM:
            addr = self._pc
            self._pc = (self._pc + 1) & 0xFFFF
            return addr

        if mode == _ZP:
            return self._read_pc()

        if mode == _ZPX:
            return (self._read_pc() + self._x) & 0xFF

        if mode == _ZPY:
            return (self._read_pc() + self._y) & 0xFF

        if mode == _ABS:
            return self._read_pc16()

        if mode == _ABX:
            return (self._read_pc16() + self._x) & 0xFFFF

        if mode == _ABY:
            return (self._read_pc16() + self._y) & 0xFFFF

        if mode == _INX:
            zp = (self._read_pc() + self._x) & 0xFF
            lo = self._memory[zp]
            hi = self._memory[(zp + 1) & 0xFF]
            return (hi << 8) | lo

        if mode == _INY:
            zp = self._read_pc()
            lo = self._memory[zp]
            hi = self._memory[(zp + 1) & 0xFF]
            return ((hi << 8) | lo) + self._y & 0xFFFF

        if mode == _IND:
            # Absolute Indirect — JMP only
            # The 6502 bug: if low byte of pointer is 0xFF, high byte wraps
            # within the same page instead of crossing to the next page.
            ptr = self._read_pc16()
            lo = self._memory[ptr]
            # Bug: (ptr & 0xFF00) | ((ptr + 1) & 0xFF) wraps in page
            hi_addr = (ptr & 0xFF00) | ((ptr + 1) & 0xFF)
            hi = self._memory[hi_addr]
            return (hi << 8) | lo

        if mode == _REL:
            # Branch: read signed offset, return *target* PC
            offset = self._read_pc()
            if offset >= 0x80:
                offset -= 0x100     # sign-extend 8-bit to Python int
            return (self._pc + offset) & 0xFFFF

        msg = f"Unknown addressing mode {mode}"
        raise ValueError(msg)

    # ── Instruction execution ────────────────────────────────────────────────

    def _execute(self, opcode: int, mnemonic: str, mode: int) -> str:
        """Dispatch opcode to its handler. Returns a description string."""

        # ── BRK ─────────────────────────────────────────────────────────────
        if mnemonic == "BRK":
            # Push PC+1 (the byte after BRK's operand byte)
            ret = (self._pc + 1) & 0xFFFF
            self._push((ret >> 8) & 0xFF)
            self._push(ret & 0xFF)
            # Push P with B=1
            p = (
                (int(self._flag_n) << 7)
                | (int(self._flag_v) << 6)
                | 0x30                # bits 5 and 4 set
                | (int(self._flag_d) << 3)
                | (int(self._flag_i) << 2)
                | (int(self._flag_z) << 1)
                | int(self._flag_c)
            )
            self._push(p)
            self._flag_i = True
            self._flag_b = True
            self._halted = True
            return "BRK — software interrupt / halt"

        # ── NOP ─────────────────────────────────────────────────────────────
        if mnemonic == "NOP":
            return "NOP — no operation"

        # ── Load ────────────────────────────────────────────────────────────
        if mnemonic == "LDA":
            addr = self._resolve_address(mode)
            self._a = self._read_mem(addr)  # type: ignore[arg-type]
            self._set_nz(self._a)
            return f"LDA — A ← mem[{addr:#06x}] = {self._a:#04x}"

        if mnemonic == "LDX":
            addr = self._resolve_address(mode)
            self._x = self._read_mem(addr)  # type: ignore[arg-type]
            self._set_nz(self._x)
            return f"LDX — X ← {self._x:#04x}"

        if mnemonic == "LDY":
            addr = self._resolve_address(mode)
            self._y = self._read_mem(addr)  # type: ignore[arg-type]
            self._set_nz(self._y)
            return f"LDY — Y ← {self._y:#04x}"

        # ── Store ────────────────────────────────────────────────────────────
        if mnemonic == "STA":
            addr = self._resolve_address(mode)
            self._write_mem(addr, self._a)  # type: ignore[arg-type]
            return f"STA — mem[{addr:#06x}] ← A={self._a:#04x}"

        if mnemonic == "STX":
            addr = self._resolve_address(mode)
            self._write_mem(addr, self._x)  # type: ignore[arg-type]
            return f"STX — mem[{addr:#06x}] ← X={self._x:#04x}"

        if mnemonic == "STY":
            addr = self._resolve_address(mode)
            self._write_mem(addr, self._y)  # type: ignore[arg-type]
            return f"STY — mem[{addr:#06x}] ← Y={self._y:#04x}"

        # ── Transfers ────────────────────────────────────────────────────────
        if mnemonic == "TAX":
            self._x = self._a
            self._set_nz(self._x)
            return f"TAX — X ← A={self._a:#04x}"

        if mnemonic == "TAY":
            self._y = self._a
            self._set_nz(self._y)
            return f"TAY — Y ← A={self._a:#04x}"

        if mnemonic == "TXA":
            self._a = self._x
            self._set_nz(self._a)
            return f"TXA — A ← X={self._x:#04x}"

        if mnemonic == "TYA":
            self._a = self._y
            self._set_nz(self._a)
            return f"TYA — A ← Y={self._y:#04x}"

        if mnemonic == "TSX":
            self._x = self._s
            self._set_nz(self._x)
            return f"TSX — X ← S={self._s:#04x}"

        if mnemonic == "TXS":
            self._s = self._x   # TXS does NOT set flags
            return f"TXS — S ← X={self._x:#04x}"

        # ── Stack ────────────────────────────────────────────────────────────
        if mnemonic == "PHA":
            self._push(self._a)
            return f"PHA — push A={self._a:#04x}"

        if mnemonic == "PLA":
            self._a = self._pull()
            self._set_nz(self._a)
            return f"PLA — pop A={self._a:#04x}"

        if mnemonic == "PHP":
            p = (
                (int(self._flag_n) << 7)
                | (int(self._flag_v) << 6)
                | 0x30                # bits 5 and 4 always set when pushed
                | (int(self._flag_d) << 3)
                | (int(self._flag_i) << 2)
                | (int(self._flag_z) << 1)
                | int(self._flag_c)
            )
            self._push(p)
            return f"PHP — push P={p:#04x}"

        if mnemonic == "PLP":
            p = self._pull()
            n, v, b, d, i, z, c = unpack_p(p)
            self._flag_n = n
            self._flag_v = v
            self._flag_b = b
            self._flag_d = d
            self._flag_i = i
            self._flag_z = z
            self._flag_c = c
            return f"PLP — pop P={p:#04x}"

        # ── ADC ──────────────────────────────────────────────────────────────
        if mnemonic == "ADC":
            addr = self._resolve_address(mode)
            m = self._read_mem(addr)  # type: ignore[arg-type]
            a = self._a
            if self._flag_d:
                result, c_out = bcd_add(a, m, self._flag_c)
                # NMOS: N, V, Z reflect the binary result before BCD adjust
                bin_result = (a + m + int(self._flag_c)) & 0xFF
                self._flag_n, self._flag_z = compute_nz(bin_result)
                self._flag_v = compute_overflow_add(a, m, bin_result)
                self._flag_c = c_out
                self._a = result
            else:
                total = a + m + int(self._flag_c)
                result = total & 0xFF
                self._flag_n, self._flag_z = compute_nz(result)
                self._flag_v = compute_overflow_add(a, m, result)
                self._flag_c = total > 0xFF
                self._a = result
            return f"ADC — A ← {a:#04x} + {m:#04x} + C = {self._a:#04x}"

        # ── SBC ──────────────────────────────────────────────────────────────
        if mnemonic == "SBC":
            addr = self._resolve_address(mode)
            m = self._read_mem(addr)  # type: ignore[arg-type]
            a = self._a
            if self._flag_d:
                result, c_out = bcd_sub(a, m, self._flag_c)
                bin_result = (a - m - int(not self._flag_c)) & 0xFF
                self._flag_n, self._flag_z = compute_nz(bin_result)
                self._flag_v = compute_overflow_sub(a, m, bin_result)
                self._flag_c = c_out
                self._a = result
            else:
                # SBC = ADC with inverted operand
                m_inv = (~m) & 0xFF
                total = a + m_inv + int(self._flag_c)
                result = total & 0xFF
                self._flag_n, self._flag_z = compute_nz(result)
                self._flag_v = compute_overflow_add(a, m_inv, result)
                self._flag_c = total > 0xFF
                self._a = result
            return f"SBC — A ← {a:#04x} - {m:#04x} = {self._a:#04x}"

        # ── AND ──────────────────────────────────────────────────────────────
        if mnemonic == "AND":
            addr = self._resolve_address(mode)
            m = self._read_mem(addr)  # type: ignore[arg-type]
            self._a &= m
            self._set_nz(self._a)
            return f"AND — A ← A & {m:#04x} = {self._a:#04x}"

        # ── ORA ──────────────────────────────────────────────────────────────
        if mnemonic == "ORA":
            addr = self._resolve_address(mode)
            m = self._read_mem(addr)  # type: ignore[arg-type]
            self._a |= m
            self._set_nz(self._a)
            return f"ORA — A ← A | {m:#04x} = {self._a:#04x}"

        # ── EOR ──────────────────────────────────────────────────────────────
        if mnemonic == "EOR":
            addr = self._resolve_address(mode)
            m = self._read_mem(addr)  # type: ignore[arg-type]
            self._a ^= m
            self._set_nz(self._a)
            return f"EOR — A ← A ^ {m:#04x} = {self._a:#04x}"

        # ── BIT ──────────────────────────────────────────────────────────────
        if mnemonic == "BIT":
            addr = self._resolve_address(mode)
            m = self._read_mem(addr)  # type: ignore[arg-type]
            self._flag_n = bool(m & 0x80)
            self._flag_v = bool(m & 0x40)
            self._flag_z = (self._a & m) == 0
            return f"BIT — N={int(self._flag_n)} V={int(self._flag_v)} Z={int(self._flag_z)}"

        # ── Shift/Rotate ─────────────────────────────────────────────────────
        if mnemonic == "ASL":
            if mode == _ACC:
                c = bool(self._a & 0x80)
                self._a = (self._a << 1) & 0xFF
                self._flag_c = c
                self._set_nz(self._a)
                return f"ASL A — A={self._a:#04x} C={int(c)}"
            addr = self._resolve_address(mode)
            v = self._read_mem(addr)  # type: ignore[arg-type]
            c = bool(v & 0x80)
            result = (v << 1) & 0xFF
            self._write_mem(addr, result)  # type: ignore[arg-type]
            self._flag_c = c
            self._set_nz(result)
            return f"ASL ${addr:#06x} — {result:#04x}"

        if mnemonic == "LSR":
            if mode == _ACC:
                c = bool(self._a & 0x01)
                self._a = self._a >> 1
                self._flag_c = c
                self._set_nz(self._a)
                return f"LSR A — A={self._a:#04x} C={int(c)}"
            addr = self._resolve_address(mode)
            v = self._read_mem(addr)  # type: ignore[arg-type]
            c = bool(v & 0x01)
            result = v >> 1
            self._write_mem(addr, result)  # type: ignore[arg-type]
            self._flag_c = c
            self._set_nz(result)
            return f"LSR ${addr:#06x} — {result:#04x}"

        if mnemonic == "ROL":
            cin = int(self._flag_c)
            if mode == _ACC:
                c = bool(self._a & 0x80)
                self._a = ((self._a << 1) | cin) & 0xFF
                self._flag_c = c
                self._set_nz(self._a)
                return f"ROL A — A={self._a:#04x}"
            addr = self._resolve_address(mode)
            v = self._read_mem(addr)  # type: ignore[arg-type]
            c = bool(v & 0x80)
            result = ((v << 1) | cin) & 0xFF
            self._write_mem(addr, result)  # type: ignore[arg-type]
            self._flag_c = c
            self._set_nz(result)
            return f"ROL ${addr:#06x} — {result:#04x}"

        if mnemonic == "ROR":
            cin = int(self._flag_c)
            if mode == _ACC:
                c = bool(self._a & 0x01)
                self._a = (self._a >> 1) | (cin << 7)
                self._flag_c = c
                self._set_nz(self._a)
                return f"ROR A — A={self._a:#04x}"
            addr = self._resolve_address(mode)
            v = self._read_mem(addr)  # type: ignore[arg-type]
            c = bool(v & 0x01)
            result = (v >> 1) | (cin << 7)
            self._write_mem(addr, result)  # type: ignore[arg-type]
            self._flag_c = c
            self._set_nz(result)
            return f"ROR ${addr:#06x} — {result:#04x}"

        # ── INC/DEC (memory) ─────────────────────────────────────────────────
        if mnemonic == "INC":
            addr = self._resolve_address(mode)
            v = (self._read_mem(addr) + 1) & 0xFF  # type: ignore[arg-type]
            self._write_mem(addr, v)  # type: ignore[arg-type]
            self._set_nz(v)
            return f"INC ${addr:#06x} — {v:#04x}"

        if mnemonic == "DEC":
            addr = self._resolve_address(mode)
            v = (self._read_mem(addr) - 1) & 0xFF  # type: ignore[arg-type]
            self._write_mem(addr, v)  # type: ignore[arg-type]
            self._set_nz(v)
            return f"DEC ${addr:#06x} — {v:#04x}"

        # ── INX/INY/DEX/DEY ──────────────────────────────────────────────────
        if mnemonic == "INX":
            self._x = (self._x + 1) & 0xFF
            self._set_nz(self._x)
            return f"INX — X={self._x:#04x}"

        if mnemonic == "INY":
            self._y = (self._y + 1) & 0xFF
            self._set_nz(self._y)
            return f"INY — Y={self._y:#04x}"

        if mnemonic == "DEX":
            self._x = (self._x - 1) & 0xFF
            self._set_nz(self._x)
            return f"DEX — X={self._x:#04x}"

        if mnemonic == "DEY":
            self._y = (self._y - 1) & 0xFF
            self._set_nz(self._y)
            return f"DEY — Y={self._y:#04x}"

        # ── Compare ──────────────────────────────────────────────────────────
        if mnemonic in ("CMP", "CPX", "CPY"):
            addr = self._resolve_address(mode)
            m = self._read_mem(addr)  # type: ignore[arg-type]
            reg = self._a if mnemonic == "CMP" else (self._x if mnemonic == "CPX" else self._y)
            diff = (reg - m) & 0xFF
            self._flag_n, self._flag_z = compute_nz(diff)
            self._flag_c = reg >= m
            return f"{mnemonic} — {reg:#04x} vs {m:#04x}: N={int(self._flag_n)} Z={int(self._flag_z)} C={int(self._flag_c)}"

        # ── Branches ─────────────────────────────────────────────────────────
        if mnemonic in ("BCC", "BCS", "BEQ", "BNE", "BPL", "BMI", "BVC", "BVS"):
            target = self._resolve_address(_REL)
            condition = {
                "BCC": not self._flag_c,
                "BCS": self._flag_c,
                "BEQ": self._flag_z,
                "BNE": not self._flag_z,
                "BPL": not self._flag_n,
                "BMI": self._flag_n,
                "BVC": not self._flag_v,
                "BVS": self._flag_v,
            }[mnemonic]
            if condition:
                self._pc = target  # type: ignore[assignment]
                return f"{mnemonic} — branch taken to {target:#06x}"
            return f"{mnemonic} — not taken"

        # ── JMP ──────────────────────────────────────────────────────────────
        if mnemonic == "JMP":
            target = self._resolve_address(mode)
            self._pc = target  # type: ignore[assignment]
            return f"JMP → {target:#06x}"

        # ── JSR ──────────────────────────────────────────────────────────────
        if mnemonic == "JSR":
            target = self._read_pc16()
            ret = (self._pc - 1) & 0xFFFF   # JSR pushes PC-1
            self._push((ret >> 8) & 0xFF)
            self._push(ret & 0xFF)
            self._pc = target
            return f"JSR → {target:#06x} (push ret={ret:#06x})"

        # ── RTS ──────────────────────────────────────────────────────────────
        if mnemonic == "RTS":
            lo = self._pull()
            hi = self._pull()
            self._pc = ((hi << 8) | lo) + 1
            return f"RTS → {self._pc:#06x}"

        # ── RTI ──────────────────────────────────────────────────────────────
        if mnemonic == "RTI":
            p = self._pull()
            n, v, b, d, i, z, c = unpack_p(p)
            self._flag_n = n
            self._flag_v = v
            self._flag_b = b
            self._flag_d = d
            self._flag_i = i
            self._flag_z = z
            self._flag_c = c
            lo = self._pull()
            hi = self._pull()
            self._pc = (hi << 8) | lo     # RTI does NOT add 1
            return f"RTI → P={p:#04x} PC={self._pc:#06x}"

        # ── Flag instructions ─────────────────────────────────────────────────
        if mnemonic == "CLC":
            self._flag_c = False
            return "CLC — C=0"
        if mnemonic == "SEC":
            self._flag_c = True
            return "SEC — C=1"
        if mnemonic == "CLD":
            self._flag_d = False
            return "CLD — D=0"
        if mnemonic == "SED":
            self._flag_d = True
            return "SED — D=1"
        if mnemonic == "CLI":
            self._flag_i = False
            return "CLI — I=0"
        if mnemonic == "SEI":
            self._flag_i = True
            return "SEI — I=1"
        if mnemonic == "CLV":
            self._flag_v = False
            return "CLV — V=0"

        raise ValueError(f"Unhandled mnemonic {mnemonic!r}")
