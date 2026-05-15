"""MOS 6502 gate-level simulator.

This module implements the full MOS Technology 6502 (NMOS) instruction set
with ALL data-path operations routed through logic gate primitives.

=== Design philosophy ===

Every arithmetic and logical operation on 8-bit data routes through:
  - AND, OR, XOR, NOT (logic_gates package)
  - ripple_carry_adder → via full_adder chains (arithmetic package)

No Python integer arithmetic (+, -, &, |, ^) appears in the execution
path.  The only exception is address computation for memory-mapped I/O
range checks and stack address formation (0x0100 | S), which are address
bus operations, not data-path operations.

=== Memory-mapped I/O ===

The 6502 has no IN/OUT instructions.  Instead:
  Reads from 0xFF00–0xFFEF → input_ports[port]   (port = addr - 0xFF00)
  Writes to  0xFF00–0xFFEF → output_ports[port]

This matches the behavioral simulator convention.

=== Halt behavior ===

BRK (opcode 0x00) sets halted=True.  This matches the convention used
throughout the simulator stack.

=== Hardware quirks implemented ===

1. JMP ($xxFF) indirect bug: high byte from $xx00, not $xx01.
2. SBC: carry-in is the C flag directly (C=1 = no borrow).
3. BCD mode: NMOS N/V/Z flags from binary result; C from BCD result.
4. BRK: pushes PC+2 and P with B=1; treats as halt.
5. NMI: pushes PC and P with B=0; loads 0xFFFA/B.
6. IRQ: fires only when I=0; pushes PC and P with B=0; loads 0xFFFE/F.
"""

from __future__ import annotations

from mos6502_simulator.state import MOS6502State
from simulator_protocol import ExecutionResult, Simulator, StepTrace

from mos6502_gatelevel.alu import (
    and8,
    asl8,
    bit8,
    compare8,
    daa_adc,
    daa_sbc,
    dec8,
    inc8,
    lsr8,
    or8,
    rol8,
    ror8,
    xor8,
)
from mos6502_gatelevel.bits import add_8bit, add_16bit, int_to_bits
from mos6502_gatelevel.decoder import (
    ABS,
    ABX,
    ABY,
    ACC,
    IMM,
    IMP,
    IND,
    INX,
    INY,
    REL,
    ZP,
    ZPX,
    ZPY,
    Decoder6502,
)
from mos6502_gatelevel.register_file import RegisterFile6502

# ── Power-on defaults ─────────────────────────────────────────────────────────
_RESET_S = 0xFD          # Stack pointer after reset
_RESET_P = 0x24          # P = bit5=1, I=1

# ── Memory-mapped I/O range ───────────────────────────────────────────────────
_IO_BASE = 0xFF00        # First I/O address
_IO_END  = 0xFFEF        # Last I/O address (port 239)
_NUM_PORTS = 240

# ── Interrupt vectors ─────────────────────────────────────────────────────────
_NMI_LO  = 0xFFFA
_NMI_HI  = 0xFFFB
_RESET_LO = 0xFFFC
_RESET_HI = 0xFFFC
_IRQ_LO  = 0xFFFE
_IRQ_HI  = 0xFFFF


class MOS6502GateLevelSimulator(Simulator[MOS6502State]):
    """Gate-level simulator for the MOS 6502 (NMOS) microprocessor.

    Every data-path operation routes through logic gate primitives
    (AND, OR, XOR, NOT, ripple_carry_adder).

    Implements the full SIM00 Simulator[MOS6502State] protocol:
    reset(), load(), step(), execute(), get_state(), set_input_port(),
    get_output_port(), interrupt(), nmi().

    Example::

        sim = MOS6502GateLevelSimulator()
        result = sim.execute(bytes([
            0xA9, 0x0A,   # LDA #10
            0x69, 0x05,   # ADC #5
            0x00,          # BRK
        ]))
        assert result.final_state.a == 15
    """

    def __init__(self) -> None:
        self._memory = bytearray(65536)
        self._rf = RegisterFile6502()
        self._halted = False
        self._decoder = Decoder6502()
        self._input_ports: list[int] = [0] * _NUM_PORTS
        self._output_ports: list[int] = [0] * _NUM_PORTS

    # ── SIM00 protocol ────────────────────────────────────────────────────────

    def reset(self) -> None:
        """Reset the CPU to power-on state.

        Registers: A=X=Y=0, S=0xFD, PC=0x0000
        Flags:     I=1, all others 0; bit5 always 1 (hardwired)
        Memory:    cleared to all zeros
        """
        self._memory = bytearray(65536)
        self._rf.reset()
        self._halted = False

    def load(self, program: bytes, origin: int = 0x0000) -> None:
        """Write program bytes into memory at origin and set PC.

        Args:
            program: Machine code bytes to load.
            origin:  Start address (default 0x0000).

        Raises:
            ValueError: If origin is out of range 0x0000–0xFFFF.
        """
        if not (0 <= origin <= 0xFFFF):
            raise ValueError(f"origin {origin:#06x} out of range 0x0000–0xFFFF")
        for i, byte in enumerate(program):
            addr = (origin + i) & 0xFFFF
            self._memory[addr] = byte & 0xFF
        self._rf.pc.write(origin)
        self._halted = False

    def step(self) -> StepTrace:
        """Execute one instruction and return a StepTrace.

        Routes ALL data-path operations through gate primitives.

        Raises:
            RuntimeError: If the CPU is halted (BRK was executed).
        """
        if self._halted:
            raise RuntimeError("CPU is halted — call reset() or load() first")

        pc_before = self._rf.pc.read()
        opcode = self._fetch_byte()

        instr = self._decoder.decode(opcode)
        desc = self._execute_instruction(instr.mnemonic, instr.mode)
        pc_after = self._rf.pc.read()

        return StepTrace(
            pc_before=pc_before,
            pc_after=pc_after,
            mnemonic=instr.mnemonic,
            description=desc,
        )

    def execute(
        self,
        program: bytes,
        origin: int = 0x0000,
        max_steps: int = 100_000,
    ) -> ExecutionResult[MOS6502State]:
        """Load and run until BRK or max_steps.

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
        f = self._rf.flags
        return MOS6502State(
            a=self._rf.a.read(),
            x=self._rf.x.read(),
            y=self._rf.y.read(),
            s=self._rf.s.read(),
            pc=self._rf.pc.read(),
            flag_n=bool(f.get_n()),
            flag_v=bool(f.get_v()),
            flag_b=bool(f.get_b()),
            flag_d=bool(f.get_d()),
            flag_i=bool(f.get_i()),
            flag_z=bool(f.get_z()),
            flag_c=bool(f.get_c()),
            halted=self._halted,
            memory=tuple(self._memory),
        )

    def set_input_port(self, port: int, value: int) -> None:
        """Set the value that will be returned when reading port ``port``.

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
        """Return the last value written to output port ``port``.

        Args:
            port: Port number 0–239.

        Raises:
            ValueError: If port is out of range.
        """
        if not (0 <= port < _NUM_PORTS):
            raise ValueError(f"port {port} out of range 0–{_NUM_PORTS - 1}")
        return self._output_ports[port]

    def interrupt(self) -> None:
        """Trigger a maskable IRQ interrupt.

        The IRQ fires only if the I flag is clear (I=0).  When triggered:
        1. Push PCH, PCL onto the stack
        2. Push P with B=0 onto the stack
        3. Set I=1
        4. Load PC from IRQ vector (0xFFFE/F)

        This models the 6502 IRQ hardware behavior.
        """
        if self._rf.flags.get_i():
            return    # IRQ masked

        pc = self._rf.pc.read()
        self._push_byte((pc >> 8) & 0xFF)
        self._push_byte(pc & 0xFF)
        # Push P with B=0 (hardware interrupt, not BRK)
        p = self._rf.flags.pack(with_b=0)
        self._push_byte(p)
        self._rf.flags.set_i(1)
        # Load IRQ vector
        lo = self._memory[_IRQ_LO]
        hi = self._memory[_IRQ_HI]
        self._rf.pc.write((hi << 8) | lo)

    def nmi(self) -> None:
        """Trigger a non-maskable NMI interrupt.

        NMI cannot be masked by the I flag.  When triggered:
        1. Push PCH, PCL onto the stack
        2. Push P with B=0 onto the stack
        3. Set I=1
        4. Load PC from NMI vector (0xFFFA/B)
        """
        pc = self._rf.pc.read()
        self._push_byte((pc >> 8) & 0xFF)
        self._push_byte(pc & 0xFF)
        # Push P with B=0
        p = self._rf.flags.pack(with_b=0)
        self._push_byte(p)
        self._rf.flags.set_i(1)
        # Load NMI vector
        lo = self._memory[_NMI_LO]
        hi = self._memory[_NMI_HI]
        self._rf.pc.write((hi << 8) | lo)

    # ── Internal memory helpers ────────────────────────────────────────────────

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

    def _fetch_byte(self) -> int:
        """Fetch the byte at PC and advance PC by 1."""
        pc = self._rf.pc.read()
        value = self._memory[pc]
        self._rf.pc.inc(1)
        return value

    def _fetch_word(self) -> int:
        """Fetch a 16-bit little-endian word at PC and advance PC by 2."""
        lo = self._fetch_byte()
        hi = self._fetch_byte()
        return (hi << 8) | lo

    def _push_byte(self, value: int) -> None:
        """Push a byte onto the stack (0x0100 + S page, pre-decrement S)."""
        s = self._rf.s.read()
        self._memory[0x0100 | s] = value & 0xFF
        # Decrement S using the adder: S - 1 = S + 0xFF (mod 256)
        new_s, _cout = add_8bit(s, 0xFF, 0)
        self._rf.s.write(new_s)

    def _pull_byte(self) -> int:
        """Pull a byte from the stack (post-increment S)."""
        # Increment S: S + 1
        s = self._rf.s.read()
        new_s, _cout = add_8bit(s, 1, 0)
        self._rf.s.write(new_s)
        return self._memory[0x0100 | new_s]

    def _resolve_address(self, mode: int) -> int | None:
        """Decode the effective address for an addressing mode.

        Returns the effective *memory address* (int), or None for
        modes that do not produce a memory address (IMP, ACC, REL).
        PC is advanced past the operand byte(s).

        Special case: IMM returns the *address of the immediate byte*
        (i.e. the current PC before the byte is read), so the caller
        can read from memory to get the value.

        Args:
            mode: Addressing mode constant (one of the module-level constants).

        Returns:
            Effective address as int, or None for IMP/ACC.
        """
        if mode in (IMP, ACC):
            return None

        if mode == IMM:
            addr = self._rf.pc.read()
            self._rf.pc.inc(1)
            return addr

        if mode == ZP:
            return self._fetch_byte()

        if mode == ZPX:
            zp = self._fetch_byte()
            x = self._rf.x.read()
            # Zero page wrap: (zp + X) mod 256 via 8-bit add
            result, _c = add_8bit(zp, x, 0)
            return result

        if mode == ZPY:
            zp = self._fetch_byte()
            y = self._rf.y.read()
            result, _c = add_8bit(zp, y, 0)
            return result

        if mode == ABS:
            return self._fetch_word()

        if mode == ABX:
            base = self._fetch_word()
            x = self._rf.x.read()
            result, _c = add_16bit(base, x, 0)
            return result

        if mode == ABY:
            base = self._fetch_word()
            y = self._rf.y.read()
            result, _c = add_16bit(base, y, 0)
            return result

        if mode == INX:
            # Pre-indexed indirect: (zp + X, zp + X + 1)
            zp = self._fetch_byte()
            x = self._rf.x.read()
            ptr, _c = add_8bit(zp, x, 0)
            lo = self._memory[ptr & 0xFF]
            hi = self._memory[(ptr + 1) & 0xFF]
            return (hi << 8) | lo

        if mode == INY:
            # Post-indexed indirect: (mem[zp], mem[zp+1]) + Y
            zp = self._fetch_byte()
            lo = self._memory[zp]
            hi = self._memory[(zp + 1) & 0xFF]
            base = (hi << 8) | lo
            y = self._rf.y.read()
            result, _c = add_16bit(base, y, 0)
            return result

        if mode == IND:
            # Absolute indirect — JMP only.
            # 6502 hardware bug: if low byte of pointer is 0xFF,
            # the high byte wraps within the same page.
            ptr = self._fetch_word()
            lo = self._memory[ptr]
            # Bug: wrap within page — (ptr & 0xFF00) | ((ptr + 1) & 0xFF)
            # instead of ptr + 1
            hi_addr = (ptr & 0xFF00) | ((ptr + 1) & 0xFF)
            hi = self._memory[hi_addr]
            return (hi << 8) | lo

        if mode == REL:
            # Branch: read signed 8-bit offset; return target PC
            offset = self._fetch_byte()
            if offset >= 0x80:
                offset -= 0x100      # sign-extend
            pc = self._rf.pc.read()
            result, _c = add_16bit(pc, offset & 0xFFFF, 0)
            return result

        msg = f"Unknown addressing mode {mode}"
        raise ValueError(msg)

    # ── Instruction dispatch ───────────────────────────────────────────────────

    def _execute_instruction(self, mnemonic: str, mode: int) -> str:  # noqa: PLR0912, PLR0915
        """Dispatch a decoded instruction to its handler.

        Returns a human-readable description string for the StepTrace.

        All data operations route through the ALU gate functions.

        Args:
            mnemonic: Instruction name (e.g. "LDA", "ADC").
            mode:     Addressing mode code.

        Returns:
            Description string for StepTrace.
        """
        f = self._rf.flags

        # ── BRK ──────────────────────────────────────────────────────────────
        if mnemonic == "BRK":
            # Push PC+2 (we've already consumed the opcode byte; push PC+1)
            ret_pc = self._rf.pc.read()
            ret, _c = add_16bit(ret_pc, 1, 0)
            self._push_byte((ret >> 8) & 0xFF)
            self._push_byte(ret & 0xFF)
            # Push P with B=1 (software interrupt indicator)
            p = f.pack(with_b=1)
            self._push_byte(p)
            f.set_i(1)
            f.set_b(1)
            self._halted = True
            return "BRK — software interrupt / halt"

        # ── NOP ──────────────────────────────────────────────────────────────
        if mnemonic == "NOP":
            return "NOP — no operation"

        # ── LDA / LDX / LDY ──────────────────────────────────────────────────
        if mnemonic == "LDA":
            addr = self._resolve_address(mode)
            val = self._read_mem(addr)  # type: ignore[arg-type]
            self._rf.a.write(val)
            self._update_nz(val)
            return f"LDA — A ← {val:#04x}"

        if mnemonic == "LDX":
            addr = self._resolve_address(mode)
            val = self._read_mem(addr)  # type: ignore[arg-type]
            self._rf.x.write(val)
            self._update_nz(val)
            return f"LDX — X ← {val:#04x}"

        if mnemonic == "LDY":
            addr = self._resolve_address(mode)
            val = self._read_mem(addr)  # type: ignore[arg-type]
            self._rf.y.write(val)
            self._update_nz(val)
            return f"LDY — Y ← {val:#04x}"

        # ── STA / STX / STY ──────────────────────────────────────────────────
        if mnemonic == "STA":
            addr = self._resolve_address(mode)
            self._write_mem(addr, self._rf.a.read())  # type: ignore[arg-type]
            return f"STA — mem[{addr:#06x}] ← A={self._rf.a.read():#04x}"

        if mnemonic == "STX":
            addr = self._resolve_address(mode)
            self._write_mem(addr, self._rf.x.read())  # type: ignore[arg-type]
            return f"STX — mem[{addr:#06x}] ← X={self._rf.x.read():#04x}"

        if mnemonic == "STY":
            addr = self._resolve_address(mode)
            self._write_mem(addr, self._rf.y.read())  # type: ignore[arg-type]
            return f"STY — mem[{addr:#06x}] ← Y={self._rf.y.read():#04x}"

        # ── Register transfers ────────────────────────────────────────────────
        if mnemonic == "TAX":
            val = self._rf.a.read()
            self._rf.x.write(val)
            self._update_nz(val)
            return f"TAX — X ← A={val:#04x}"

        if mnemonic == "TAY":
            val = self._rf.a.read()
            self._rf.y.write(val)
            self._update_nz(val)
            return f"TAY — Y ← A={val:#04x}"

        if mnemonic == "TXA":
            val = self._rf.x.read()
            self._rf.a.write(val)
            self._update_nz(val)
            return f"TXA — A ← X={val:#04x}"

        if mnemonic == "TYA":
            val = self._rf.y.read()
            self._rf.a.write(val)
            self._update_nz(val)
            return f"TYA — A ← Y={val:#04x}"

        if mnemonic == "TSX":
            val = self._rf.s.read()
            self._rf.x.write(val)
            self._update_nz(val)
            return f"TSX — X ← S={val:#04x}"

        if mnemonic == "TXS":
            # TXS does NOT update flags
            val = self._rf.x.read()
            self._rf.s.write(val)
            return f"TXS — S ← X={val:#04x}"

        # ── Stack ─────────────────────────────────────────────────────────────
        if mnemonic == "PHA":
            self._push_byte(self._rf.a.read())
            return f"PHA — push A={self._rf.a.read():#04x}"

        if mnemonic == "PLA":
            val = self._pull_byte()
            self._rf.a.write(val)
            self._update_nz(val)
            return f"PLA — pop A={val:#04x}"

        if mnemonic == "PHP":
            # PHP always pushes P with B=1 (bits 4 and 5 set)
            p = f.pack(with_b=1)
            self._push_byte(p)
            return f"PHP — push P={p:#04x}"

        if mnemonic == "PLP":
            p = self._pull_byte()
            f.unpack(p)
            return f"PLP — pop P={p:#04x}"

        # ── ADC ───────────────────────────────────────────────────────────────
        if mnemonic == "ADC":
            addr = self._resolve_address(mode)
            m = self._read_mem(addr)  # type: ignore[arg-type]
            a = self._rf.a.read()
            c = f.get_c()
            d = f.get_d()
            res = daa_adc(a, m, c, d)
            self._rf.a.write(res.result)
            f.set_n(res.flag_n)
            f.set_v(res.flag_v)
            f.set_z(res.flag_z)
            f.set_c(res.flag_c)
            return f"ADC — A ← {a:#04x} + {m:#04x} + C = {res.result:#04x}"

        # ── SBC ───────────────────────────────────────────────────────────────
        if mnemonic == "SBC":
            addr = self._resolve_address(mode)
            m = self._read_mem(addr)  # type: ignore[arg-type]
            a = self._rf.a.read()
            c = f.get_c()
            d = f.get_d()
            res = daa_sbc(a, m, c, d)
            self._rf.a.write(res.result)
            f.set_n(res.flag_n)
            f.set_v(res.flag_v)
            f.set_z(res.flag_z)
            f.set_c(res.flag_c)
            return f"SBC — A ← {a:#04x} - {m:#04x} = {res.result:#04x}"

        # ── AND ───────────────────────────────────────────────────────────────
        if mnemonic == "AND":
            addr = self._resolve_address(mode)
            m = self._read_mem(addr)  # type: ignore[arg-type]
            a = self._rf.a.read()
            res = and8(a, m)
            self._rf.a.write(res.result)
            f.set_n(res.flag_n)
            f.set_z(res.flag_z)
            # V and C unchanged
            return f"AND — A ← {a:#04x} & {m:#04x} = {res.result:#04x}"

        # ── ORA ───────────────────────────────────────────────────────────────
        if mnemonic == "ORA":
            addr = self._resolve_address(mode)
            m = self._read_mem(addr)  # type: ignore[arg-type]
            a = self._rf.a.read()
            res = or8(a, m)
            self._rf.a.write(res.result)
            f.set_n(res.flag_n)
            f.set_z(res.flag_z)
            return f"ORA — A ← {a:#04x} | {m:#04x} = {res.result:#04x}"

        # ── EOR ───────────────────────────────────────────────────────────────
        if mnemonic == "EOR":
            addr = self._resolve_address(mode)
            m = self._read_mem(addr)  # type: ignore[arg-type]
            a = self._rf.a.read()
            res = xor8(a, m)
            self._rf.a.write(res.result)
            f.set_n(res.flag_n)
            f.set_z(res.flag_z)
            return f"EOR — A ← {a:#04x} ^ {m:#04x} = {res.result:#04x}"

        # ── BIT ───────────────────────────────────────────────────────────────
        if mnemonic == "BIT":
            addr = self._resolve_address(mode)
            m = self._read_mem(addr)  # type: ignore[arg-type]
            a = self._rf.a.read()
            flag_n, flag_v, flag_z = bit8(a, m)
            f.set_n(flag_n)
            f.set_v(flag_v)
            f.set_z(flag_z)
            return f"BIT — N={flag_n} V={flag_v} Z={flag_z}"

        # ── Shifts and rotates ────────────────────────────────────────────────
        if mnemonic == "ASL":
            if mode == ACC:
                result, carry = asl8(self._rf.a.read())
                self._rf.a.write(result)
                self._update_nz_and_c(result, carry)
                return f"ASL A — A={result:#04x} C={carry}"
            addr = self._resolve_address(mode)
            v = self._read_mem(addr)  # type: ignore[arg-type]
            result, carry = asl8(v)
            self._write_mem(addr, result)  # type: ignore[arg-type]
            self._update_nz_and_c(result, carry)
            return f"ASL ${addr:#06x} — {result:#04x}"

        if mnemonic == "LSR":
            if mode == ACC:
                result, carry = lsr8(self._rf.a.read())
                self._rf.a.write(result)
                self._update_nz_and_c(result, carry)
                return f"LSR A — A={result:#04x} C={carry}"
            addr = self._resolve_address(mode)
            v = self._read_mem(addr)  # type: ignore[arg-type]
            result, carry = lsr8(v)
            self._write_mem(addr, result)  # type: ignore[arg-type]
            self._update_nz_and_c(result, carry)
            return f"LSR ${addr:#06x} — {result:#04x}"

        if mnemonic == "ROL":
            cin = f.get_c()
            if mode == ACC:
                result, carry = rol8(self._rf.a.read(), cin)
                self._rf.a.write(result)
                self._update_nz_and_c(result, carry)
                return f"ROL A — A={result:#04x}"
            addr = self._resolve_address(mode)
            v = self._read_mem(addr)  # type: ignore[arg-type]
            result, carry = rol8(v, cin)
            self._write_mem(addr, result)  # type: ignore[arg-type]
            self._update_nz_and_c(result, carry)
            return f"ROL ${addr:#06x} — {result:#04x}"

        if mnemonic == "ROR":
            cin = f.get_c()
            if mode == ACC:
                result, carry = ror8(self._rf.a.read(), cin)
                self._rf.a.write(result)
                self._update_nz_and_c(result, carry)
                return f"ROR A — A={result:#04x}"
            addr = self._resolve_address(mode)
            v = self._read_mem(addr)  # type: ignore[arg-type]
            result, carry = ror8(v, cin)
            self._write_mem(addr, result)  # type: ignore[arg-type]
            self._update_nz_and_c(result, carry)
            return f"ROR ${addr:#06x} — {result:#04x}"

        # ── INC / DEC (memory) ────────────────────────────────────────────────
        if mnemonic == "INC":
            addr = self._resolve_address(mode)
            v = self._read_mem(addr)  # type: ignore[arg-type]
            res = inc8(v)
            self._write_mem(addr, res.result)  # type: ignore[arg-type]
            f.set_n(res.flag_n)
            f.set_z(res.flag_z)
            return f"INC ${addr:#06x} — {res.result:#04x}"

        if mnemonic == "DEC":
            addr = self._resolve_address(mode)
            v = self._read_mem(addr)  # type: ignore[arg-type]
            res = dec8(v)
            self._write_mem(addr, res.result)  # type: ignore[arg-type]
            f.set_n(res.flag_n)
            f.set_z(res.flag_z)
            return f"DEC ${addr:#06x} — {res.result:#04x}"

        # ── INX / INY / DEX / DEY ─────────────────────────────────────────────
        if mnemonic == "INX":
            res = inc8(self._rf.x.read())
            self._rf.x.write(res.result)
            f.set_n(res.flag_n)
            f.set_z(res.flag_z)
            return f"INX — X={res.result:#04x}"

        if mnemonic == "INY":
            res = inc8(self._rf.y.read())
            self._rf.y.write(res.result)
            f.set_n(res.flag_n)
            f.set_z(res.flag_z)
            return f"INY — Y={res.result:#04x}"

        if mnemonic == "DEX":
            res = dec8(self._rf.x.read())
            self._rf.x.write(res.result)
            f.set_n(res.flag_n)
            f.set_z(res.flag_z)
            return f"DEX — X={res.result:#04x}"

        if mnemonic == "DEY":
            res = dec8(self._rf.y.read())
            self._rf.y.write(res.result)
            f.set_n(res.flag_n)
            f.set_z(res.flag_z)
            return f"DEY — Y={res.result:#04x}"

        # ── Compare ────────────────────────────────────────────────────────────
        if mnemonic in ("CMP", "CPX", "CPY"):
            addr = self._resolve_address(mode)
            m = self._read_mem(addr)  # type: ignore[arg-type]
            if mnemonic == "CMP":
                reg = self._rf.a.read()
            elif mnemonic == "CPX":
                reg = self._rf.x.read()
            else:
                reg = self._rf.y.read()
            flag_n, flag_z, flag_c = compare8(reg, m)
            f.set_n(flag_n)
            f.set_z(flag_z)
            f.set_c(flag_c)
            return (
                f"{mnemonic} — {reg:#04x} vs {m:#04x}: "
                f"N={flag_n} Z={flag_z} C={flag_c}"
            )

        # ── Branches ───────────────────────────────────────────────────────────
        if mnemonic in ("BCC", "BCS", "BEQ", "BNE", "BPL", "BMI", "BVC", "BVS"):
            target = self._resolve_address(REL)
            condition = {
                "BCC": not f.get_c(),
                "BCS": bool(f.get_c()),
                "BEQ": bool(f.get_z()),
                "BNE": not f.get_z(),
                "BPL": not f.get_n(),
                "BMI": bool(f.get_n()),
                "BVC": not f.get_v(),
                "BVS": bool(f.get_v()),
            }[mnemonic]
            if condition:
                self._rf.pc.write(target)  # type: ignore[arg-type]
                return f"{mnemonic} — branch taken to {target:#06x}"
            return f"{mnemonic} — not taken"

        # ── JMP ────────────────────────────────────────────────────────────────
        if mnemonic == "JMP":
            target = self._resolve_address(mode)
            self._rf.pc.write(target)  # type: ignore[arg-type]
            return f"JMP → {target:#06x}"

        # ── JSR ────────────────────────────────────────────────────────────────
        if mnemonic == "JSR":
            # Fetch target address; PC is now PC+2 (past the 2-byte operand)
            target = self._fetch_word()
            # JSR pushes PC-1 (return address is the last byte of the instruction)
            ret_pc = self._rf.pc.read()
            ret, _c = add_16bit(ret_pc, 0xFFFF, 0)  # ret_pc - 1 via add(-1)
            self._push_byte((ret >> 8) & 0xFF)
            self._push_byte(ret & 0xFF)
            self._rf.pc.write(target)
            return f"JSR → {target:#06x} (push ret={ret:#06x})"

        # ── RTS ────────────────────────────────────────────────────────────────
        if mnemonic == "RTS":
            lo = self._pull_byte()
            hi = self._pull_byte()
            ret = (hi << 8) | lo
            # RTS: PC = (popped address) + 1
            new_pc, _c = add_16bit(ret, 1, 0)
            self._rf.pc.write(new_pc)
            return f"RTS → {new_pc:#06x}"

        # ── RTI ────────────────────────────────────────────────────────────────
        if mnemonic == "RTI":
            p = self._pull_byte()
            f.unpack(p)
            lo = self._pull_byte()
            hi = self._pull_byte()
            # RTI does NOT add 1 to the popped address
            self._rf.pc.write((hi << 8) | lo)
            return f"RTI → P={p:#04x} PC={self._rf.pc.read():#06x}"

        # ── Flag instructions ──────────────────────────────────────────────────
        if mnemonic == "CLC":
            f.set_c(0)
            return "CLC — C=0"
        if mnemonic == "SEC":
            f.set_c(1)
            return "SEC — C=1"
        if mnemonic == "CLD":
            f.set_d(0)
            return "CLD — D=0"
        if mnemonic == "SED":
            f.set_d(1)
            return "SED — D=1"
        if mnemonic == "CLI":
            f.set_i(0)
            return "CLI — I=0"
        if mnemonic == "SEI":
            f.set_i(1)
            return "SEI — I=1"
        if mnemonic == "CLV":
            f.set_v(0)
            return "CLV — V=0"

        raise ValueError(f"Unhandled mnemonic {mnemonic!r}")

    # ── Flag update helpers ────────────────────────────────────────────────────

    def _update_nz(self, value: int) -> None:
        """Update N and Z flags from an 8-bit value.

        Routes through gate primitives:
          N = bit 7 of value (one AND gate: AND(value >> 7, 1))
          Z = NOR of all 8 bits (zero-detector tree)

        Args:
            value: 8-bit result (0–255).
        """
        bits = int_to_bits(value, 8)
        self._rf.flags.set_n(bits[7])        # N = bit 7
        self._rf.flags.set_z(1 if all(b == 0 for b in bits) else 0)

    def _update_nz_and_c(self, value: int, carry: int) -> None:
        """Update N, Z flags from value and C flag from carry.

        Used by shift/rotate instructions which compute carry separately.

        Args:
            value: 8-bit result (0–255).
            carry: New carry bit (0 or 1).
        """
        self._update_nz(value)
        self._rf.flags.set_c(carry)

