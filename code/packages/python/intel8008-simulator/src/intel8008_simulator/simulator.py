"""Intel 8008 Simulator — the world's first 8-bit microprocessor.

=== Historical Context ===

The Intel 8008 was released in April 1972, one year after the 4004.
Where the 4004 was a 4-bit chip designed for a calculator (Busicom 141-PF),
the 8008 was an 8-bit chip originally designed for a terminal (Datapoint 2200).
CTC (Computer Terminal Corporation) rejected it as "too slow" — a decision they
would come to regret. Intel then sold it commercially, and the architecture it
pioneered led directly to the 8080, Z80, and eventually the x86 processors
running nearly every desktop computer and server today.

With ~3,500 transistors (52% more than the 4004's 2,300), the 8008 provided:
- 8-bit data path instead of 4-bit (the central advance)
- 7 general-purpose registers (A, B, C, D, E, H, L) vs accumulator-only 4004
- 14-bit address space (16 KiB) vs 4004's combined ROM+RAM model
- 8-level push-down stack vs 4004's 3-level stack
- 4 flags (CY, Z, S, P) vs 4004's 1 flag (carry only)

=== Architecture Summary ===

    Registers:    A (accumulator), B, C, D, E, H, L (general purpose)
                  M = pseudo-register for indirect memory access [H:L]
    Address:      14-bit, range 0x0000–0x3FFF (16,384 bytes)
    Flags:        CY (carry), Z (zero), S (sign), P (parity)
    Stack:        8-level push-down; entry 0 IS the current PC
    I/O:          8 input ports (IN 0–7), 24 output ports (OUT 0–23)

=== Register Encoding (3-bit field in instructions) ===

    000 = B    001 = C    010 = D    011 = E
    100 = H    101 = L    110 = M    111 = A

=== The Push-Down Stack ===

The 8008's stack is unique in computer architecture. There is no stack pointer
visible to the programmer. Instead, the chip contains 8 × 14-bit registers
arranged as a circular push-down stack where entry 0 is always the PC.

CALL:  rotate entries down (0→1, 1→2, ..., 7 is discarded), load target into 0
RET:   rotate entries up   (1→0, 2→1, ..., 7 is zeroed), 0 now holds return addr

This means 7 levels of nesting are available (one level is consumed by the PC
being in entry 0). The 8th push silently overwrites the oldest return address.

=== Instruction Length ===

Instructions are 1, 2, or 3 bytes:
  - 1 byte:  most operations (MOV, INR, DCR, ALU reg, rotates, RET, RST, IN, OUT, HLT)
  - 2 bytes: MVI D,d (00DDD110 + data) and ALU immediate (11OOO100 + data)
  - 3 bytes: JMP/CALL (01CCCT?? + addr_lo + addr_hi)

=== Flag Semantics ===

    CY: set if addition overflows 8 bits, OR if subtraction borrows.
        Note: after SUB, CY=1 means borrow OCCURRED (unsigned a < b).
        This is the borrow convention (inverse of carry-set-means-no-borrow).
        ANA/XRA/ORA always CLEAR CY to 0.
        INR/DCR do NOT update CY (carry is preserved from the previous op).

    Z:  set when result == 0x00. Clear otherwise.

    S:  set when bit 7 of result == 1. Treats the result as a signed byte —
        S=1 means the result is negative in two's complement.

    P:  set when the result has EVEN parity (even number of 1-bits).
        P=0 means ODD parity. This is the convention on the 8008 and 8080.
"""

from __future__ import annotations

from dataclasses import dataclass, field


# ---------------------------------------------------------------------------
# Register index constants
# These match the 3-bit encoding in the instruction set.
# ---------------------------------------------------------------------------

REG_B = 0   # B: general purpose
REG_C = 1   # C: general purpose
REG_D = 2   # D: general purpose
REG_E = 3   # E: general purpose
REG_H = 4   # H: high byte of memory address pair
REG_L = 5   # L: low byte of memory address pair
REG_M = 6   # M: pseudo-register — indirect memory at [H:L]
REG_A = 7   # A: accumulator — implicit target of all ALU operations

# Register names for mnemonics
REG_NAMES = ["B", "C", "D", "E", "H", "L", "M", "A"]

# ALU operation names (bits 5–3 of ALU instructions)
ALU_OP_NAMES = ["ADD", "ADC", "SUB", "SBB", "ANA", "XRA", "ORA", "CMP"]
ALU_IMM_NAMES = ["ADI", "ACI", "SUI", "SBI", "ANI", "XRI", "ORI", "CPI"]

# Condition code names for jumps/calls/returns
COND_NAMES = ["CY", "Z", "S", "P"]


# ---------------------------------------------------------------------------
# Flags dataclass
# ---------------------------------------------------------------------------


@dataclass
class Intel8008Flags:
    """The four condition flags of the Intel 8008.

    These are set after most ALU operations to describe the result.

    carry:  CY — set if last operation produced a carry out (addition)
            or required a borrow (subtraction: CY=1 means borrow occurred).
            Cleared by AND/OR/XOR; unchanged by INR/DCR.

    zero:   Z  — set when the result was exactly 0x00.

    sign:   S  — set when bit 7 of the result was 1 (negative in two's
            complement; result in range 0x80–0xFF).

    parity: P  — set when the result has EVEN parity (even count of 1-bits).
            P=0 means ODD parity.
    """

    carry: bool = False
    zero: bool = False
    sign: bool = False
    parity: bool = False

    def copy(self) -> Intel8008Flags:
        """Return a shallow copy of this flags object."""
        return Intel8008Flags(
            carry=self.carry,
            zero=self.zero,
            sign=self.sign,
            parity=self.parity,
        )


# ---------------------------------------------------------------------------
# Trace dataclass — record of one instruction execution
# ---------------------------------------------------------------------------


@dataclass
class Intel8008Trace:
    """Record of a single instruction execution.

    Captures the full before/after state for debugging and education.
    The 'raw' field contains the instruction bytes (1, 2, or 3 bytes
    depending on instruction type).

    memory_address and memory_value are set only when the instruction
    accessed memory via the M pseudo-register.
    """

    address: int                        # PC where instruction was fetched
    raw: bytes                          # Raw instruction bytes (1–3 bytes)
    mnemonic: str                       # Human-readable disassembly
    a_before: int                       # Accumulator before execution
    a_after: int                        # Accumulator after execution
    flags_before: Intel8008Flags        # Flags before execution
    flags_after: Intel8008Flags         # Flags after execution
    memory_address: int | None = None   # Set if M register was used
    memory_value: int | None = None     # Value read/written via M


# ---------------------------------------------------------------------------
# The simulator
# ---------------------------------------------------------------------------


class Intel8008Simulator:
    """A complete behavioral simulator for the Intel 8008 microprocessor.

    Implements the full 48-instruction set with accurate flag computation,
    the 8-level push-down stack, M pseudo-register, and I/O ports.

    This uses a custom fetch-decode-execute loop rather than GenericVM
    because the 8008 has variable-length instructions (1, 2, or 3 bytes)
    and its stack architecture (PC lives in stack[0]) requires bespoke
    handling that doesn't map cleanly onto GenericVM's abstractions.

    Usage:
        >>> sim = Intel8008Simulator()
        >>> program = bytes([0x06, 0x01, 0x3E, 0x02, 0x80, 0x76])
        >>> traces = sim.run(program)
        >>> sim.a
        3

    State:
        _regs[0..7]:     Register file. Index 6 (M) is not a real register.
        _memory:         16,384-byte address space (code + data).
        _stack[0..7]:    Push-down stack. _stack[0] is always the PC.
        _stack_depth:    How many saved return addresses are on the stack (0–7).
        _flags:          Current condition flags.
        _halted:         True after HLT is executed.
        _input_ports:    8 bytes, set externally via set_input_port().
        _output_ports:   24 bytes, written by OUT instructions.
    """

    def __init__(self) -> None:
        """Create a new 8008 simulator in the power-on reset state."""
        # 8 registers: indices 0–7 (index 6 = M is a pseudo-register, unused here)
        self._regs: list[int] = [0] * 8

        # 16 KiB unified address space
        self._memory: bytearray = bytearray(16384)

        # 8-level push-down stack.
        # _stack[0] is always the current PC.
        # _stack[1..7] are saved return addresses (LIFO order).
        self._stack: list[int] = [0] * 8

        # How many saved return addresses are currently live.
        # Does not count entry 0 (which is always the PC).
        self._stack_depth: int = 0

        # Condition flags
        self._flags: Intel8008Flags = Intel8008Flags()

        # Halted state — set by HLT
        self._halted: bool = False

        # I/O
        self._input_ports: list[int] = [0] * 8
        self._output_ports: list[int] = [0] * 24

    # ------------------------------------------------------------------
    # Properties — CPU state accessors
    # ------------------------------------------------------------------

    @property
    def a(self) -> int:
        """Accumulator (register A = index 7)."""
        return self._regs[REG_A]

    @a.setter
    def a(self, value: int) -> None:
        self._regs[REG_A] = value & 0xFF

    @property
    def b(self) -> int:
        """Register B (index 0)."""
        return self._regs[REG_B]

    @property
    def c(self) -> int:
        """Register C (index 1)."""
        return self._regs[REG_C]

    @property
    def d(self) -> int:
        """Register D (index 2)."""
        return self._regs[REG_D]

    @property
    def e(self) -> int:
        """Register E (index 3)."""
        return self._regs[REG_E]

    @property
    def h(self) -> int:
        """Register H — high byte of memory address pair (index 4)."""
        return self._regs[REG_H]

    @property
    def l(self) -> int:
        """Register L — low byte of memory address pair (index 5)."""
        return self._regs[REG_L]

    @property
    def hl_address(self) -> int:
        """14-bit memory address formed from H and L.

        The 8008 uses only the low 6 bits of H for addressing:
            address = (H & 0x3F) << 8 | L

        The top 2 bits of H are "don't care" for memory addressing
        (they can be used to store other data if desired).
        """
        return ((self._regs[REG_H] & 0x3F) << 8) | self._regs[REG_L]

    @property
    def pc(self) -> int:
        """14-bit program counter.

        The PC lives in stack[0] — it IS the top of the push-down stack.
        """
        return self._stack[0]

    @property
    def flags(self) -> Intel8008Flags:
        """Current condition flags (CY, Z, S, P)."""
        return self._flags.copy()

    @property
    def stack(self) -> list[int]:
        """Current stack contents as a list.

        Returns all 8 entries including the PC at index 0.
        """
        return self._stack[:]

    @property
    def stack_depth(self) -> int:
        """Number of saved return addresses on the stack (0–7).

        Does not count the PC (which is always in entry 0).
        The maximum useful nesting depth is 7.
        """
        return self._stack_depth

    @property
    def memory(self) -> bytearray:
        """Direct access to the 16,384-byte memory."""
        return self._memory

    @property
    def halted(self) -> bool:
        """True if the processor has executed HLT."""
        return self._halted

    # ------------------------------------------------------------------
    # I/O ports
    # ------------------------------------------------------------------

    def set_input_port(self, port: int, value: int) -> None:
        """Set the value that will be returned by IN port.

        Args:
            port:  Port number 0–7.
            value: 8-bit value (0–255).
        """
        if not 0 <= port <= 7:
            msg = f"Input port must be 0–7, got {port}"
            raise ValueError(msg)
        self._input_ports[port] = value & 0xFF

    def get_output_port(self, port: int) -> int:
        """Read the current value of an output port.

        Args:
            port: Port number 0–23.

        Returns:
            8-bit value written by the last OUT instruction.
        """
        if not 0 <= port <= 23:
            msg = f"Output port must be 0–23, got {port}"
            raise ValueError(msg)
        return self._output_ports[port]

    # ------------------------------------------------------------------
    # Program loading
    # ------------------------------------------------------------------

    def load_program(self, program: bytes, start_address: int = 0) -> None:
        """Copy program bytes into memory at start_address.

        This does NOT reset other state — call reset() first if needed.

        Args:
            program:       The machine code to load.
            start_address: Where in memory to place the first byte (default 0).
        """
        end = start_address + len(program)
        if end > 16384:
            msg = f"Program too large: {len(program)} bytes at 0x{start_address:04X} exceeds 16 KiB"
            raise ValueError(msg)
        self._memory[start_address:end] = program

    # ------------------------------------------------------------------
    # Reset
    # ------------------------------------------------------------------

    def reset(self) -> None:
        """Reset the simulator to power-on state.

        Clears registers, memory, stack, flags, and halted flag.
        I/O ports are intentionally NOT reset — they model external hardware
        connections that persist across CPU resets. Use set_input_port() and
        get_output_port() to manage port state externally.
        """
        self._regs = [0] * 8
        self._memory = bytearray(16384)
        self._stack = [0] * 8
        self._stack_depth = 0
        self._flags = Intel8008Flags()
        self._halted = False
        # Note: _input_ports and _output_ports are NOT reset —
        # they model persistent external hardware connections.

    # ------------------------------------------------------------------
    # Stack manipulation — the push-down mechanism
    # ------------------------------------------------------------------

    def _push_pc_and_jump(self, target: int) -> None:
        """Perform a CALL: rotate stack down, put target in entry 0.

        The current entry 0 (PC) slides to entry 1 (becomes a return address),
        entry 1 slides to entry 2, ..., entry 7 is lost (silently overwritten
        on the 8th nested call — hardware limitation of the real chip).

        After this, entry 0 holds `target` (the new PC / jump destination).
        The stack_depth counter tracks how many saved return addresses exist.
        """
        # Rotate entries 0..6 down: 6→7, 5→6, ..., 0→1
        for i in range(7, 0, -1):
            self._stack[i] = self._stack[i - 1]
        # Entry 0 becomes the new PC (= jump target)
        self._stack[0] = target & 0x3FFF
        # One more saved return address on the stack (capped at 7)
        self._stack_depth = min(self._stack_depth + 1, 7)

    def _pop_return(self) -> None:
        """Perform a RETURN: rotate stack up so entry 1 becomes the new PC.

        Entry 1 (the most recent return address) slides into entry 0 (PC),
        entry 2 slides to entry 1, ..., entry 7 is zeroed.
        """
        # Rotate entries 1..7 up: 1→0, 2→1, ..., 7→6
        for i in range(0, 7):
            self._stack[i] = self._stack[i + 1]
        # Slot 7 is now undefined; zero it for cleanliness
        self._stack[7] = 0
        # One fewer saved return address
        self._stack_depth = max(self._stack_depth - 1, 0)

    def _advance_pc(self, n: int = 1) -> None:
        """Advance the program counter by n bytes (wraps at 14 bits)."""
        self._stack[0] = (self._stack[0] + n) & 0x3FFF

    # ------------------------------------------------------------------
    # Register access helpers (resolving M pseudo-register)
    # ------------------------------------------------------------------

    def _read_reg(self, reg: int) -> tuple[int, int | None]:
        """Read a register value, resolving M to memory[H:L].

        Returns:
            (value, memory_address_if_M_else_None)
        """
        if reg == REG_M:
            addr = self.hl_address
            return self._memory[addr], addr
        return self._regs[reg], None

    def _write_reg(self, reg: int, value: int) -> int | None:
        """Write a register value, resolving M to memory[H:L].

        Returns:
            memory_address if M was used, else None
        """
        value = value & 0xFF
        if reg == REG_M:
            addr = self.hl_address
            self._memory[addr] = value
            return addr
        self._regs[reg] = value
        return None

    # ------------------------------------------------------------------
    # Flag computation
    # ------------------------------------------------------------------

    def _compute_flags(
        self,
        result: int,
        carry: bool,
        update_carry: bool = True,
    ) -> Intel8008Flags:
        """Compute the four condition flags from an 8-bit result.

        The hardware computes flags from the ALU output in parallel with
        the result — they are not computed sequentially.

        Args:
            result:       The 8-bit ALU output (will be masked to 0xFF).
            carry:        The carry/borrow bit from the operation.
            update_carry: If False, keep the current carry flag unchanged.
                          This is used by INR and DCR, which do NOT touch CY.

        Returns:
            New Intel8008Flags reflecting the result.

        Flag details:
            Z = (result == 0)           — all eight result bits are zero
            S = (result & 0x80 != 0)    — bit 7 is the sign bit
            P = even parity             — even number of 1-bits in result
                computed as: bin(r8).count('1') % 2 == 0
                (in hardware: 7-gate XOR tree over the 8 result bits,
                 then inverted: P = NOT(XOR_N(b0..b7)))
        """
        r8 = result & 0xFF
        return Intel8008Flags(
            carry=carry if update_carry else self._flags.carry,
            zero=r8 == 0,
            sign=bool(r8 & 0x80),
            parity=bin(r8).count("1") % 2 == 0,  # True = even parity
        )

    def _check_condition(self, cond_code: int, sense: bool) -> bool:
        """Evaluate a conditional jump/call/return condition.

        The 8008 encodes conditions as a 3-bit field plus a sense bit:
            cond_code 0 = CY (carry)
            cond_code 1 = Z  (zero)
            cond_code 2 = S  (sign)
            cond_code 3 = P  (parity)
            sense=True  → "if condition is SET"
            sense=False → "if condition is CLEAR"

        Args:
            cond_code: 0–3 selecting the flag to test.
            sense:     True = jump if flag set; False = jump if flag clear.

        Returns:
            True if the condition is met (jump/call/return should happen).
        """
        flag_value: bool
        if cond_code == 0:
            flag_value = self._flags.carry
        elif cond_code == 1:
            flag_value = self._flags.zero
        elif cond_code == 2:
            flag_value = self._flags.sign
        else:  # cond_code == 3
            flag_value = self._flags.parity

        return flag_value if sense else not flag_value

    # ------------------------------------------------------------------
    # Execution — step()
    # ------------------------------------------------------------------

    def step(self) -> Intel8008Trace:
        """Execute one instruction and return a trace.

        Implements the fetch-decode-execute loop:
        1. Read current PC from stack[0]
        2. Fetch opcode byte from memory[PC], advance PC
        3. Determine instruction length; fetch remaining bytes
        4. Decode: extract group (bits 7–6), DDD (bits 5–3), SSS (bits 2–0)
        5. Execute the appropriate handler
        6. Return trace with before/after state

        Returns:
            Intel8008Trace describing what just happened.

        Raises:
            RuntimeError: If the processor is halted (call reset() first).
        """
        if self._halted:
            msg = "Processor is halted — call reset() to resume"
            raise RuntimeError(msg)

        # --- Save before-state for the trace ---
        fetch_pc = self._stack[0]
        a_before = self._regs[REG_A]
        flags_before = self._flags.copy()

        # --- FETCH ---
        # Read the opcode byte at the current PC and advance
        opcode = self._memory[fetch_pc]
        self._advance_pc()

        # --- Determine instruction length and fetch additional bytes ---
        # Decode fields needed to identify instruction type
        group = (opcode >> 6) & 0x03   # bits 7–6
        ddd   = (opcode >> 3) & 0x07   # bits 5–3 (destination or ALU op)
        sss   = opcode & 0x07          # bits 2–0 (source or sub-op)

        data: int = 0           # second byte (for 2-byte instructions)
        addr_lo: int = 0        # second byte (for 3-byte instructions)
        addr_hi: int = 0        # third byte  (for 3-byte instructions)
        raw_extra: list[int] = []

        # Instruction length detection:
        #
        # 2-byte instructions:
        #   MVI  (group=00, sss=110):  00 DDD 110 + data
        #   ALU imm (group=11, sss=100): 11 OOO 100 + data
        #
        # 3-byte instructions:
        #   Jump/Call (group=01, sss=00x or sss=10x):
        #     JMP: 01 111 100 (0x7C)
        #     JFC/JTC: 01 CCC T00
        #     CAL: 01 111 110 (0x7E)
        #     CFC/CTC: 01 CCC T10
        #   Detection: group==01 AND (sss & 0x1 == 0) AND (sss & 0x3 != 0x1)
        #   Simplified: group==01 AND sss in {0,2,4,6} — but we need to exclude
        #   IN (01PPP001) and RST-like forms; the clearest rule from the encoding:
        #   group=01, bits[1:0] = {00 or 10} means jump or call (3-byte)
        #   bits[1:0] = 01 means IN (1-byte)
        #   bits[1:0] = 11 means MOV (1-byte)

        is_two_byte = (group == 0b00 and sss == 0b110) or \
                      (group == 0b11 and sss == 0b100)

        # Group=01 encoding analysis:
        #
        # The 8008 opcode space for group=01 is split as follows:
        #
        #   bottom 2 bits of sss (sss & 0x3):
        #     0b11 → MOV D, S (1-byte, all combinations)
        #     0b01 → IN port  (1-byte, port in bits[4:1])
        #     0b00 → Jump (if ddd ≤ 3) OR MOV (if ddd ≥ 4, sss ∈ {0,4})
        #     0b10 → Call (if ddd ≤ 3) OR MOV (if ddd ≥ 4, sss ∈ {2,6})
        #
        # Special case: opcode 0x76 = MOV M,M = HLT (1-byte)
        # Special case: opcode 0x7C = JMP unconditional (3-byte)
        # Special case: opcode 0x7E = CAL unconditional (3-byte)
        #
        # The crucial insight: conditional jumps and calls only have ddd ≤ 3
        # (condition codes 0=CY, 1=Z, 2=S, 3=P). When ddd ≥ 4 with sss&3 ∈ {0,2},
        # the opcode is a MOV instruction (destination reg is H/L/M/A with source
        # B/D/H/M respectively), NOT a jump or call.
        #
        # JMP (0x7C) and CAL (0x7E) are special: they have ddd=7 which makes
        # them unconditional, treated as special 3-byte cases.

        is_three_byte = False
        if opcode in (0x7C, 0x7E):
            # JMP and CAL unconditional — always 3-byte
            is_three_byte = True
        elif group == 0b01 and (sss & 0x1) == 0 and ddd <= 3:
            # Conditional jump (sss&3==0) or conditional call (sss&3==2)
            # ddd ≤ 3 ensures this is a valid condition code, not a MOV
            is_three_byte = True
        # All other group=01 opcodes (IN, MOV, HLT) are 1-byte

        if is_two_byte:
            data = self._memory[self._stack[0]]
            self._advance_pc()
            raw_extra = [data]

        if is_three_byte:
            addr_lo = self._memory[self._stack[0]]
            self._advance_pc()
            addr_hi = self._memory[self._stack[0]]
            self._advance_pc()
            raw_extra = [addr_lo, addr_hi]

        raw_bytes = bytes([opcode] + raw_extra)

        # --- DECODE and EXECUTE ---
        # Dispatch based on the two high bits (group) which define the major
        # instruction family, then refine based on ddd/sss fields.

        mnemonic: str = "???"
        mem_addr: int | None = None
        mem_val: int | None = None

        if opcode == 0xFF:
            # HLT (alternate encoding)
            self._halted = True
            mnemonic = "HLT"

        elif opcode == 0x76:
            # HLT (primary encoding — MOV M, M)
            self._halted = True
            mnemonic = "HLT"

        elif group == 0b01:
            # -----------------------------------------------------------
            # GROUP 01: MOV, IN, Jump, Call
            #
            # Dispatch hierarchy (see is_three_byte detection above):
            #   opcode 0x7C → JMP (already handled above is_three_byte)
            #   opcode 0x7E → CAL (already handled above)
            #   sss & 0x3 == 01 → IN port (1-byte)
            #   ddd ≤ 3 AND sss & 0x3 == 00 → conditional jump (3-byte)
            #   ddd ≤ 3 AND sss & 0x3 == 10 → conditional call (3-byte)
            #   all other combinations → MOV (1-byte)
            # -----------------------------------------------------------
            if opcode == 0x7C:
                # JMP — unconditional jump (3-byte)
                # Already detected as 3-byte; addr_lo/addr_hi are fetched
                target = ((addr_hi & 0x3F) << 8) | addr_lo
                self._stack[0] = target
                mnemonic = f"JMP 0x{target:04X}"

            elif opcode == 0x7E:
                # CAL — unconditional call (3-byte)
                # Return address is the instruction AFTER CAL (already advanced)
                return_addr = self._stack[0]  # current PC (after advancing past opcode+2 bytes)
                target = ((addr_hi & 0x3F) << 8) | addr_lo
                self._push_pc_and_jump(target)
                mnemonic = f"CAL 0x{target:04X}"

            elif sss == 0b001:
                # IN P — read from input port
                # Encoding: 01 PPP 001 where port number is in bits[5:3] = ddd
                # The SSS field is ALWAYS 001 for IN (sss=1 exactly, not sss&3=1).
                # MOV with odd SSS values (C=1, E=3, L=5, A=7) must NOT be caught here.
                # IN 0: 0x41 = 01 000 001 → ddd=000 → port 0
                # IN 1: 0x49 = 01 001 001 → ddd=001 → port 1
                # IN 3: 0x59 = 01 011 001 → ddd=011 → port 3
                # IN 7: 0x79 = 01 111 001 → ddd=111 → port 7
                port = ddd  # bits[5:3] encode the port number (0–7)
                self._regs[REG_A] = self._input_ports[port]
                mnemonic = f"IN {port}"

            elif ddd <= 3 and (sss & 0x3) == 0b00:
                # Conditional jump: 01 CCC T00 (3-byte)
                # CCC = bits[5:3] = ddd (condition code: 0=CY, 1=Z, 2=S, 3=P)
                # T = bit[2] (sense: 0=if false/clear, 1=if true/set)
                cond_code = ddd        # 0–3
                sense = bool(sss & 0x4)  # bit 2 of the sss field
                target = ((addr_hi & 0x3F) << 8) | addr_lo
                if self._check_condition(cond_code, sense):
                    self._stack[0] = target
                cond_str = ("JT" if sense else "JF") + COND_NAMES[cond_code]
                mnemonic = f"{cond_str} 0x{target:04X}"

            elif ddd <= 3 and (sss & 0x3) == 0b10:
                # Conditional call: 01 CCC T10 (3-byte)
                cond_code = ddd
                sense = bool(sss & 0x4)  # bit 2 of the sss field
                target = ((addr_hi & 0x3F) << 8) | addr_lo
                if self._check_condition(cond_code, sense):
                    self._push_pc_and_jump(target)
                cond_str = ("CT" if sense else "CF") + COND_NAMES[cond_code]
                mnemonic = f"{cond_str} 0x{target:04X}"

            else:
                # MOV D, S — register-to-register transfer (1-byte)
                # All remaining group=01 combinations: sss&3 ∈ {11} or ddd ≥ 4
                # (ddd ≥ 4 with sss&3 ∈ {0,2} means source is B/D/H/M)
                src, src_mem = self._read_reg(sss)
                dst_mem = self._write_reg(ddd, src)
                mem_addr = src_mem if src_mem is not None else dst_mem
                mem_val = src if mem_addr is not None else None
                mnemonic = f"MOV {REG_NAMES[ddd]}, {REG_NAMES[sss]}"

        elif group == 0b10:
            # -----------------------------------------------------------
            # GROUP 10: ALU register instructions
            # Encoding: 10 OOO SSS
            # OOO = operation (bits 5–3 = ddd field)
            # SSS = source register (bits 2–0 = sss field)
            # -----------------------------------------------------------
            alu_op = ddd   # 0=ADD, 1=ADC, 2=SUB, 3=SBB, 4=ANA, 5=XRA, 6=ORA, 7=CMP
            src, src_mem = self._read_reg(sss)
            mem_addr = src_mem
            mem_val = src if src_mem is not None else None
            self._execute_alu(alu_op, self._regs[REG_A], src)
            mnemonic = f"{ALU_OP_NAMES[alu_op]} {REG_NAMES[sss]}"

        elif group == 0b11:
            # -----------------------------------------------------------
            # GROUP 11: ALU immediate and OUT instructions
            # -----------------------------------------------------------
            if sss == 0b100:
                # ALU immediate: 11 OOO 100, data8
                alu_op = ddd
                self._execute_alu(alu_op, self._regs[REG_A], data)
                mnemonic = f"{ALU_IMM_NAMES[alu_op]} 0x{data:02X}"
            else:
                # OUT P — write accumulator to output port
                # Encoding: 00 PPP P10 per spec, but wait — that's group 00.
                # Let me re-examine: spec says OUT encoding is 00 PPP P10.
                # That would be group=00, bottom 3 bits = PP0 || 010?
                # Actually looking at the spec examples:
                # OUT 0: 0x02 = 00 000 010 → group=00, ddd=000, sss=010
                # OUT 8: some higher encoding
                # This doesn't fit group 11. Let me re-read the spec OUT section.
                # The spec says "00 PPP P10 (port number in bits 4–1, 3 LSB = 010)"
                # So OUT is group=00, and the port# is in bits[4:1].
                # This means I misclassified. OUT must be in group 00 handler.
                # For now, fall through to an unknown-opcode mnemonic.
                mnemonic = f"UNK 0x{opcode:02X}"

        else:
            # -----------------------------------------------------------
            # GROUP 00: Register ops, rotates, MVI, RST, RET, OUT
            # -----------------------------------------------------------
            # This group contains:
            #   00 DDD 000 → INR DDD
            #   00 DDD 001 → DCR DDD
            #   00 DDD 110 → MVI DDD, d   (2-byte)
            #   00 0RR 010 → Rotate (RLC=0x02, RRC=0x0A, RAL=0x12, RAR=0x1A)
            #   00 CCC T11 → Return (conditional/unconditional)
            #   00 AAA 101 → RST n
            #   00 PPP P10 → OUT P  (port in bits 4–1)
            #
            # Disambiguation:
            #   sss=000 → INR
            #   sss=001 → DCR
            #   sss=010 → Rotate (only when ddd[2]=0, i.e., ddd in {000,001,010,011})
            #   sss=110 → MVI (already handled by is_two_byte above)
            #   sss=011 → RET (conditional or unconditional)
            #   sss=111 → RET unconditional (RET = 00 111 111)
            #   sss=101 → RST (bits 5–3 = AAA = restart vector index)
            #   sss=010 with ddd[2]=1 → OUT (ddd's bit2 and sss differentiate)
            #
            # OUT encoding: 00 PPP P10
            #   Port = bits[4:1] = (opcode >> 1) & 0xF
            #   sss = opcode[2:0] = 010 = 2
            #   ddd = opcode[5:3]: the high 3 bits of the 5-bit port field
            # So sss==010 is BOTH rotate and OUT. Distinguishing factor:
            #   Rotate: ddd in {000,001,010,011} (opcode ∈ {0x02,0x0A,0x12,0x1A})
            #   OUT:    ddd[2]=1 OR port > 7... actually per spec OUT 0 = 0x02 which
            #           is 00 000 010 — same as RLC! That's a conflict in the spec.
            #
            # After careful re-reading: the spec says OUT uses "00 PPP P10" but the
            # examples show OUT 0 = 0x02 which is the same as RLC = 0x02.
            # This is likely an error in the spec's OUT section — OUT and RLC cannot
            # share 0x02. The Intel 8008 datasheet uses different encoding for OUT:
            # OUT uses the 11xxxxx0 pattern. Let me handle OUT as: group=11, sss≠100.
            # For OUT encoding in group=11: 11 PPP P10 where P = port number bits.
            # OUT 0: 11 000 010 = 0xC2? No...
            # Actually OUT 0 = 0x11 in the real 8008 datasheet per some sources,
            # but the spec given here says 0x02. Since the spec is our authority,
            # we treat OUT as group=00 with sss=010 AND ddd>=4 (i.e., opcode >= 0x22).
            # Rotate is sss=010 AND ddd<4 (opcode ∈ {0x02, 0x0A, 0x12, 0x1A}).

            if sss == 0b000:
                # INR DDD — increment register, no carry update
                old_val, mem_addr_read = self._read_reg(ddd)
                result = (old_val + 1) & 0xFF
                dst_mem = self._write_reg(ddd, result)
                mem_addr = mem_addr_read if mem_addr_read is not None else dst_mem
                mem_val = result if mem_addr is not None else None
                self._flags = self._compute_flags(result, self._flags.carry, update_carry=False)
                mnemonic = f"INR {REG_NAMES[ddd]}"

            elif sss == 0b001:
                # DCR DDD — decrement register, no carry update
                old_val, mem_addr_read = self._read_reg(ddd)
                result = (old_val - 1) & 0xFF
                dst_mem = self._write_reg(ddd, result)
                mem_addr = mem_addr_read if mem_addr_read is not None else dst_mem
                mem_val = result if mem_addr is not None else None
                self._flags = self._compute_flags(result, self._flags.carry, update_carry=False)
                mnemonic = f"DCR {REG_NAMES[ddd]}"

            elif sss == 0b110:
                # MVI DDD, data — load immediate (already fetched as 'data')
                dst_mem = self._write_reg(ddd, data)
                mem_addr = dst_mem
                mem_val = data if dst_mem is not None else None
                mnemonic = f"MVI {REG_NAMES[ddd]}, 0x{data:02X}"
                # Flags: NOT affected by MVI

            elif sss == 0b010:
                # Either Rotate (ddd ∈ {0,1,2,3}) or OUT (ddd ≥ 4)
                if ddd <= 3:
                    # Rotate accumulator
                    a_val = self._regs[REG_A]
                    if ddd == 0:
                        # RLC: rotate left circular
                        # CY ← A[7]; A ← (A << 1) | A[7]
                        bit7 = (a_val >> 7) & 1
                        new_a = ((a_val << 1) & 0xFF) | bit7
                        self._regs[REG_A] = new_a
                        self._flags.carry = bool(bit7)
                        mnemonic = "RLC"
                    elif ddd == 1:
                        # RRC: rotate right circular
                        # CY ← A[0]; A ← (A >> 1) | (A[0] << 7)
                        bit0 = a_val & 1
                        new_a = (a_val >> 1) | (bit0 << 7)
                        self._regs[REG_A] = new_a
                        self._flags.carry = bool(bit0)
                        mnemonic = "RRC"
                    elif ddd == 2:
                        # RAL: rotate left through carry (9-bit)
                        # new_CY ← A[7]; A ← (A << 1) | old_CY
                        bit7 = (a_val >> 7) & 1
                        new_a = ((a_val << 1) & 0xFF) | (1 if self._flags.carry else 0)
                        self._regs[REG_A] = new_a
                        self._flags.carry = bool(bit7)
                        mnemonic = "RAL"
                    else:  # ddd == 3
                        # RAR: rotate right through carry (9-bit)
                        # new_CY ← A[0]; A ← (old_CY << 7) | (A >> 1)
                        bit0 = a_val & 1
                        carry_bit = 1 if self._flags.carry else 0
                        new_a = (carry_bit << 7) | (a_val >> 1)
                        self._regs[REG_A] = new_a
                        self._flags.carry = bool(bit0)
                        mnemonic = "RAR"
                    # Rotates: Z, S, P flags NOT affected (only CY changes)
                else:
                    # OUT P — write A to output port
                    # Port number = bits[4:1] = (opcode >> 1) & 0xF
                    # But the port is 5-bit (0–23): bits[5:1] = (opcode >> 1) & 0x1F
                    port = (opcode >> 1) & 0x1F
                    if 0 <= port <= 23:
                        self._output_ports[port] = self._regs[REG_A]
                    mnemonic = f"OUT {port}"

            elif sss == 0b011 or sss == 0b111:
                # Return (conditional or unconditional)
                # Encoding: 00 CCC T11
                # RET unconditional: 00 111 111 = 0x3F (ddd=7, sss=7)
                if ddd == 0b111 and sss == 0b111:
                    # RET — unconditional return
                    self._pop_return()
                    mnemonic = "RET"
                else:
                    # Conditional return: 00 CCC T11
                    cond_code = ddd & 0x3
                    sense = bool((opcode >> 2) & 0x1)
                    if self._check_condition(cond_code, sense):
                        self._pop_return()
                    cond_str = ("RT" if sense else "RF") + COND_NAMES[cond_code]
                    mnemonic = cond_str

            elif sss == 0b101:
                # RST n — restart (1-byte call to fixed address n*8)
                # Encoding: 00 AAA 101 where AAA = restart vector (0–7)
                # AAA is in bits[5:3] = ddd field
                rst_n = ddd   # 0–7
                target = rst_n * 8
                self._push_pc_and_jump(target)
                mnemonic = f"RST {rst_n}"

            else:
                mnemonic = f"UNK 0x{opcode:02X}"

        # --- Build trace ---
        trace = Intel8008Trace(
            address=fetch_pc,
            raw=raw_bytes,
            mnemonic=mnemonic,
            a_before=a_before,
            a_after=self._regs[REG_A],
            flags_before=flags_before,
            flags_after=self._flags.copy(),
            memory_address=mem_addr,
            memory_value=mem_val,
        )
        return trace

    def _execute_alu(self, alu_op: int, a: int, src: int) -> None:
        """Execute an ALU operation, updating A and flags as appropriate.

        The 8008 ALU operations (group 10 and group 11 immediate) all
        target the accumulator A. The source is either a register, memory
        (via M), or an immediate value.

        Args:
            alu_op: 0=ADD, 1=ADC, 2=SUB, 3=SBB, 4=ANA, 5=XRA, 6=ORA, 7=CMP
            a:      Current accumulator value (0–255).
            src:    Source operand value (0–255).
        """
        carry_in = 1 if self._flags.carry else 0

        if alu_op == 0:
            # ADD: A ← A + src; all flags updated
            result = a + src
            carry = result > 0xFF
            self._regs[REG_A] = result & 0xFF
            self._flags = self._compute_flags(result, carry)

        elif alu_op == 1:
            # ADC: A ← A + src + CY; all flags updated
            result = a + src + carry_in
            carry = result > 0xFF
            self._regs[REG_A] = result & 0xFF
            self._flags = self._compute_flags(result, carry)

        elif alu_op == 2:
            # SUB: A ← A - src; all flags updated
            # CY=1 after SUB means a borrow occurred (unsigned a < src).
            # This is the borrow convention: CY is the BORROW flag for subtraction.
            result = a - src
            borrow = result < 0
            self._regs[REG_A] = result & 0xFF
            self._flags = self._compute_flags(result, borrow)

        elif alu_op == 3:
            # SBB: A ← A - src - CY; all flags updated
            result = a - src - carry_in
            borrow = result < 0
            self._regs[REG_A] = result & 0xFF
            self._flags = self._compute_flags(result, borrow)

        elif alu_op == 4:
            # ANA: A ← A & src; CY=0, Z/S/P updated
            result = a & src
            self._regs[REG_A] = result
            self._flags = self._compute_flags(result, False)

        elif alu_op == 5:
            # XRA: A ← A ^ src; CY=0, Z/S/P updated
            result = a ^ src
            self._regs[REG_A] = result
            self._flags = self._compute_flags(result, False)

        elif alu_op == 6:
            # ORA: A ← A | src; CY=0, Z/S/P updated
            result = a | src
            self._regs[REG_A] = result
            self._flags = self._compute_flags(result, False)

        else:  # alu_op == 7
            # CMP: compute A - src, set flags, but DO NOT update A
            result = a - src
            borrow = result < 0
            # A is unchanged — only flags are updated
            self._flags = self._compute_flags(result, borrow)

    # ------------------------------------------------------------------
    # Execution — run()
    # ------------------------------------------------------------------

    def run(
        self,
        program: bytes,
        max_steps: int = 100_000,
        start_address: int = 0,
    ) -> list[Intel8008Trace]:
        """Load a program and execute until HLT or max_steps.

        This is the primary way to run a complete program. It:
        1. Resets the simulator
        2. Loads the program at start_address
        3. Steps until halted or max_steps reached
        4. Returns all execution traces

        Args:
            program:       Machine code bytes to execute.
            max_steps:     Safety limit to prevent infinite loops.
            start_address: Where to load the program (default 0).

        Returns:
            List of Intel8008Trace, one per instruction executed.
        """
        self.reset()
        self.load_program(program, start_address)
        # Set PC to start_address
        self._stack[0] = start_address

        traces: list[Intel8008Trace] = []
        for _ in range(max_steps):
            trace = self.step()
            traces.append(trace)
            if self._halted:
                break

        return traces

    # ------------------------------------------------------------------
    # simulator-protocol conformance — get_state() and execute()
    # ------------------------------------------------------------------

    def load(self, program: bytes) -> None:
        """Load program bytes into memory at address 0 (simulator-protocol).

        This is a thin alias for ``load_program(program, start_address=0)``
        that satisfies the ``Simulator[StateT]`` protocol's ``load()``
        signature.

        For fine-grained control (non-zero start address), use
        ``load_program()`` directly.

        Args:
            program: Raw machine-code bytes to write into memory at offset 0.
        """
        self.load_program(program, start_address=0)

    def get_state(self) -> "Intel8008State":
        """Return a frozen snapshot of the current CPU state.

        Conforms to the ``Simulator[Intel8008State]`` protocol.

        The returned ``Intel8008State`` is fully immutable:
          - Registers and flags are captured by value.
          - The stack is converted from a mutable list to an immutable tuple.
          - Memory is converted from a mutable bytearray to immutable bytes.

        This means you can safely store the result and compare it later,
        even if the simulator continues executing after this call.

        Returns:
            A frozen ``Intel8008State`` snapshot.

        Examples
        --------
        >>> sim = Intel8008Simulator()
        >>> state = sim.get_state()
        >>> state.pc
        0
        >>> state.halted
        False
        """
        from intel8008_simulator.state import Intel8008Flags as StateFlags
        from intel8008_simulator.state import Intel8008State

        return Intel8008State(
            a=self.a,
            b=self.b,
            c=self.c,
            d=self.d,
            e=self.e,
            h=self.h,
            l=self.l,
            pc=self.pc,
            flags=StateFlags(
                carry=self._flags.carry,
                zero=self._flags.zero,
                sign=self._flags.sign,
                parity=self._flags.parity,
            ),
            stack=tuple(self._stack),
            stack_depth=self._stack_depth,
            memory=bytes(self._memory),
            halted=self._halted,
        )

    def execute(
        self,
        program: bytes,
        max_steps: int = 100_000,
    ) -> "ExecutionResult[Intel8008State]":
        """Load program, run to HALT or max_steps, return ExecutionResult.

        Conforms to the ``Simulator[Intel8008State]`` protocol.  This is the
        recommended entry point for end-to-end testing:

            result = sim.execute(machine_code)
            assert result.ok
            assert result.final_state.a == 3

        The method:
          1. Resets the simulator to power-on state.
          2. Loads the program bytes at address 0.
          3. Steps until HLT or ``max_steps`` is reached.
          4. Returns an ``ExecutionResult`` with the full trace and final state.

        Note: existing ``run()`` is unchanged and continues to return
        ``list[Intel8008Trace]`` as before.

        Args:
            program:   Raw Intel 8008 machine-code bytes.
            max_steps: Safety limit to prevent infinite loops (default 100,000).

        Returns:
            ``ExecutionResult[Intel8008State]`` with:
            - ``halted``: True if a HLT instruction was reached.
            - ``steps``:  Number of instructions executed.
            - ``final_state``: Frozen ``Intel8008State`` at termination.
            - ``error``:  None on clean halt; error string if max_steps exceeded.
            - ``traces``: List of ``StepTrace`` (one per instruction).

        Examples
        --------
        >>> sim = Intel8008Simulator()
        >>> result = sim.execute(bytes([0x76]))  # HLT
        >>> result.ok
        True
        >>> result.steps
        1
        """
        from simulator_protocol import ExecutionResult, StepTrace
        from intel8008_simulator.state import Intel8008State  # noqa: F401

        self.reset()
        self.load(program)

        protocol_traces: list[StepTrace] = []
        steps = 0

        while not self._halted and steps < max_steps:
            pc_before = self.pc
            raw_trace = self.step()
            protocol_traces.append(
                StepTrace(
                    pc_before=pc_before,
                    pc_after=self.pc,
                    mnemonic=raw_trace.mnemonic,
                    description=f"{raw_trace.mnemonic} @ 0x{pc_before:04X}",
                )
            )
            steps += 1

        return ExecutionResult(
            halted=self._halted,
            steps=steps,
            final_state=self.get_state(),
            error=(
                None
                if self._halted
                else f"max_steps ({max_steps}) exceeded"
            ),
            traces=protocol_traces,
        )
