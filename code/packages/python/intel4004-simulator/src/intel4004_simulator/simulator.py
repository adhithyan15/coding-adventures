"""Intel 4004 Simulator — the world's first commercial microprocessor.

=== What is the Intel 4004? ===

The Intel 4004 was the world's first commercial single-chip microprocessor,
released by Intel in 1971. It was designed by Federico Faggin, Ted Hoff, and
Stanley Mazor for the Busicom 141-PF calculator — a Japanese desktop printing
calculator. Intel negotiated to retain the rights to the chip design, which
turned out to be one of the most consequential business decisions in history.

The entire processor contained just 2,300 transistors. For perspective, a
modern CPU has billions. The 4004 ran at 740 kHz — about a million times
slower than today's processors. Yet it proved that a general-purpose processor
could be built on a single chip, launching the microprocessor revolution.

=== Architecture ===

The 4004 is a 4-bit processor with an accumulator architecture:

    Data width:      4 bits (values 0–15)
    Instruction:     8 bits (some instructions are 2 bytes)
    Registers:       16 × 4-bit (R0–R15), organized as 8 pairs
    Accumulator:     4-bit (A) — most arithmetic goes through here
    Carry flag:      1 bit — set on overflow/borrow
    Program counter: 12 bits (addresses 4096 bytes of ROM)
    Stack:           3-level hardware stack (12-bit return addresses)
    ROM:             4096 × 8-bit (program storage)
    RAM:             4 banks × 4 registers × (16 main + 4 status) nibbles

=== Execution Engine ===

This simulator uses GenericVM from the virtual-machine package as its
execution engine. GenericVM provides the fetch-decode-execute loop, PC
management, step/run infrastructure, and execution tracing. We register
all 46 Intel 4004 opcodes as handler functions and store 4004-specific
state (accumulator, registers, carry, RAM, stack) on the simulator instance.

The GenericVM's stack is unused — the 4004 is accumulator-based, not
stack-based. But the GenericVM chassis works perfectly: its opcode dispatch,
PC management, and tracing are exactly what we need.

=== Complete Instruction Set (46 instructions) ===

The 4004 has 46 instructions organized by encoding:

    0x00       NOP          No operation
    0x01       HLT          Halt (simulator-only)
    0x1_       JCN c,a  *   Conditional jump (c=condition nibble)
    0x2_ even  FIM Pp,d *   Fetch immediate to register pair
    0x2_ odd   SRC Pp       Send register control (pair as address)
    0x3_ even  FIN Pp       Fetch indirect from ROM via P0
    0x3_ odd   JIN Pp       Jump indirect via register pair
    0x4_       JUN a    *   Unconditional jump (12-bit address)
    0x5_       JMS a    *   Jump to subroutine
    0x6_       INC Rn       Increment register
    0x7_       ISZ Rn,a *   Increment and skip if zero
    0x8_       ADD Rn       Add register to accumulator
    0x9_       SUB Rn       Subtract register from accumulator
    0xA_       LD Rn        Load register into accumulator
    0xB_       XCH Rn       Exchange accumulator and register
    0xC_       BBL n        Branch back and load
    0xD_       LDM n        Load immediate into accumulator
    0xE0–0xEF  I/O ops      RAM/ROM read/write operations
    0xF0–0xFD  Accum ops    Accumulator manipulation

    * = 2-byte instruction (second byte is data or address)
"""

from __future__ import annotations

from dataclasses import dataclass

from virtual_machine import CodeObject, GenericVM, Instruction

from intel4004_simulator.state import Intel4004State

# ---------------------------------------------------------------------------
# Trace — what happened during one instruction
# ---------------------------------------------------------------------------


@dataclass
class Intel4004Trace:
    """Record of a single instruction execution.

    Fields:
        address:            PC where this instruction was fetched from.
        raw:                The raw first byte (0x00–0xFF).
        raw2:               The raw second byte for 2-byte instructions, else None.
        mnemonic:           Human-readable instruction (e.g., "LDM 1", "ADD R0").
        accumulator_before: Value of A before execution.
        accumulator_after:  Value of A after execution.
        carry_before:       Carry flag before execution.
        carry_after:        Carry flag after execution.
    """

    address: int
    raw: int
    raw2: int | None
    mnemonic: str
    accumulator_before: int
    accumulator_after: int
    carry_before: bool
    carry_after: bool


# ---------------------------------------------------------------------------
# Opcode constants
# ---------------------------------------------------------------------------
# We map the upper nibble (or full byte for 0xE_/0xF_ range) to a
# "family" opcode that GenericVM dispatches on. The lower nibble is
# stored as the Instruction.operand.
#
# For instructions with variable lower nibbles (ADD R0–R15, LD R0–R15, etc.),
# the opcode is the upper nibble shifted left by 4 (e.g., ADD = 0x80).
# For fixed instructions (0xE0–0xEF, 0xF0–0xFD), the opcode is the full byte.

# Family opcodes (upper nibble determines instruction)
OP_NOP = 0x00
OP_HLT = 0x01
OP_JCN = 0x10  # 2-byte: conditional jump
OP_FIM = 0x20  # 2-byte: fetch immediate to pair (even lower nibble)
OP_SRC = 0x21  # send register control (odd lower nibble)
OP_FIN = 0x30  # fetch indirect (even lower nibble)
OP_JIN = 0x31  # jump indirect (odd lower nibble)
OP_JUN = 0x40  # 2-byte: unconditional jump
OP_JMS = 0x50  # 2-byte: jump to subroutine
OP_INC = 0x60  # increment register
OP_ISZ = 0x70  # 2-byte: increment and skip if zero
OP_ADD = 0x80  # add register to accumulator
OP_SUB = 0x90  # subtract register from accumulator
OP_LD = 0xA0   # load register into accumulator
OP_XCH = 0xB0  # exchange accumulator and register
OP_BBL = 0xC0  # branch back and load
OP_LDM = 0xD0  # load immediate

# I/O opcodes (full byte)
OP_WRM = 0xE0  # write RAM main character
OP_WMP = 0xE1  # write RAM output port
OP_WRR = 0xE2  # write ROM I/O port
OP_WPM = 0xE3  # write program RAM (not simulated)
OP_WR0 = 0xE4  # write RAM status character 0
OP_WR1 = 0xE5  # write RAM status character 1
OP_WR2 = 0xE6  # write RAM status character 2
OP_WR3 = 0xE7  # write RAM status character 3
OP_SBM = 0xE8  # subtract RAM from accumulator
OP_RDM = 0xE9  # read RAM main character
OP_RDR = 0xEA  # read ROM I/O port
OP_ADM = 0xEB  # add RAM to accumulator
OP_RD0 = 0xEC  # read RAM status character 0
OP_RD1 = 0xED  # read RAM status character 1
OP_RD2 = 0xEE  # read RAM status character 2
OP_RD3 = 0xEF  # read RAM status character 3

# Accumulator opcodes (full byte)
OP_CLB = 0xF0  # clear both (A=0, carry=0)
OP_CLC = 0xF1  # clear carry
OP_IAC = 0xF2  # increment accumulator
OP_CMC = 0xF3  # complement carry
OP_CMA = 0xF4  # complement accumulator
OP_RAL = 0xF5  # rotate left through carry
OP_RAR = 0xF6  # rotate right through carry
OP_TCC = 0xF7  # transfer carry to accumulator
OP_DAC = 0xF8  # decrement accumulator
OP_TCS = 0xF9  # transfer carry subtract
OP_STC = 0xFA  # set carry
OP_DAA = 0xFB  # decimal adjust accumulator
OP_KBP = 0xFC  # keyboard process
OP_DCL = 0xFD  # designate command line


# ---------------------------------------------------------------------------
# Helper: detect 2-byte instructions
# ---------------------------------------------------------------------------

def _is_two_byte(raw: int) -> bool:
    """Return True if the raw byte starts a 2-byte instruction.

    The 4004 has five 2-byte instruction families:
        0x1_ JCN  — conditional jump
        0x2_ FIM  — fetch immediate (even lower nibble only)
        0x4_ JUN  — unconditional jump
        0x5_ JMS  — jump to subroutine
        0x7_ ISZ  — increment and skip if zero
    """
    upper = (raw >> 4) & 0xF
    if upper in (0x1, 0x4, 0x5, 0x7):
        return True
    # FIM is 0x2_ with even lower nibble
    return bool(upper == 0x2 and (raw & 0x1) == 0)


# ---------------------------------------------------------------------------
# Opcode mapping helpers
# ---------------------------------------------------------------------------


def _family_opcode(raw: int) -> int:
    """Map a raw 4004 byte to its family opcode for GenericVM dispatch."""
    upper = (raw >> 4) & 0xF

    # Special cases: NOP and HLT
    if raw == 0x00:
        return OP_NOP
    if raw == 0x01:
        return OP_HLT

    # 0xE_ and 0xF_ range: full byte is the opcode
    if upper in (0xE, 0xF):
        return raw

    # 0x2_ and 0x3_ depend on even/odd lower nibble
    if upper == 0x2:
        return OP_FIM if (raw & 0x1) == 0 else OP_SRC
    if upper == 0x3:
        return OP_FIN if (raw & 0x1) == 0 else OP_JIN

    # All others: upper nibble shifted left by 4
    return upper << 4


def _decode_operand(raw: int, raw2: int | None, pc: int) -> dict:
    """Build an operand dict from raw instruction bytes.

    The operand dict contains all the information a handler needs:
      - raw: first byte
      - raw2: second byte (or None)
      - reg: register number (lower nibble, for register ops)
      - pair: register pair index (lower nibble >> 1, for pair ops)
      - imm: immediate value (lower nibble, for LDM/BBL)
      - cond: condition code (lower nibble, for JCN)
      - addr: full address (for JUN/JMS) or page-relative (for JCN/ISZ)
      - target_addr: resolved ROM address for jumps
      - pc: the PC at which this instruction was fetched
    """
    upper = (raw >> 4) & 0xF
    lower = raw & 0xF
    operand: dict = {"raw": raw, "raw2": raw2, "pc": pc}

    # Register index (for ADD, SUB, LD, XCH, INC, ISZ)
    operand["reg"] = lower

    # Register pair index (for FIM, SRC, FIN, JIN)
    # Pair index = lower nibble >> 1 (since pairs use even register numbers)
    operand["pair"] = lower >> 1

    # Immediate value (for LDM, BBL)
    operand["imm"] = lower

    # Condition code (for JCN)
    operand["cond"] = lower

    if raw2 is not None:
        if upper == 0x1:
            # JCN: page-relative jump
            # Target is within the same 256-byte page
            page = (pc + 2) & 0xF00
            operand["addr"] = raw2
            operand["target_addr"] = page | raw2
        elif upper == 0x2:
            # FIM: 8-bit immediate data
            operand["data"] = raw2
        elif upper == 0x4:
            # JUN: 12-bit absolute address
            addr12 = (lower << 8) | raw2
            operand["addr"] = addr12
            operand["target_addr"] = addr12
        elif upper == 0x5:
            # JMS: 12-bit absolute address
            addr12 = (lower << 8) | raw2
            operand["addr"] = addr12
            operand["target_addr"] = addr12
        elif upper == 0x7:
            # ISZ: page-relative jump
            page = (pc + 2) & 0xF00
            operand["addr"] = raw2
            operand["target_addr"] = page | raw2

    return operand


# ---------------------------------------------------------------------------
# The simulator
# ---------------------------------------------------------------------------


class Intel4004Simulator:
    """A simulator for the complete Intel 4004 microprocessor instruction set.

    This simulator implements all 46 real 4004 instructions plus HLT (a
    simulator-only halt instruction). It uses GenericVM from the virtual-machine
    package as the execution engine — each opcode is registered as a handler.

    Usage:
        >>> sim = Intel4004Simulator()
        >>> program = bytes([0xD1, 0xB0, 0xD2, 0x80, 0xB1, 0x01])
        >>> traces = sim.run(program)
        >>> sim.registers[1]   # R1 = 3
        3

    State:
        accumulator:  4-bit accumulator (0–15). The heart of computation.
        registers:    16 general-purpose 4-bit registers (R0–R15).
        carry:        Carry/borrow flag from the last arithmetic operation.
        pc:           Program counter (0–4095).
        halted:       True after HLT is executed.
        hw_stack:     3-level hardware call stack (12-bit addresses).
        ram:          4 banks × 4 registers × 20 nibbles.
        rom:          4096 bytes of program memory.
    """

    def __init__(self, memory_size: int = 4096) -> None:
        # --- CPU Registers ---
        self.accumulator: int = 0
        self.registers: list[int] = [0] * 16
        self.carry: bool = False

        # --- Memory ---
        self.memory_size = memory_size
        self.rom: bytearray = bytearray(memory_size)

        # --- RAM: 4 banks × 4 registers × (16 main + 4 status) nibbles ---
        # ram[bank][register][character] for main characters (0–15)
        self.ram: list[list[list[int]]] = [
            [[0] * 16 for _ in range(4)] for _ in range(4)
        ]
        # status[bank][register][status_index] for status characters (0–3)
        self.ram_status: list[list[list[int]]] = [
            [[0] * 4 for _ in range(4)] for _ in range(4)
        ]
        # RAM output port (written by WMP)
        self.ram_output: list[int] = [0] * 4  # one per bank? simplified to 4

        # --- RAM addressing (set by SRC) ---
        self.ram_bank: int = 0       # selected by DCL (0–7, but only 0–3 used)
        self.ram_register: int = 0   # high nibble of SRC pair value
        self.ram_character: int = 0  # low nibble of SRC pair value

        # --- ROM I/O port ---
        self.rom_port: int = 0

        # --- Hardware call stack ---
        self.hw_stack: list[int] = [0, 0, 0]
        self.stack_pointer: int = 0

        # --- Control ---
        self.halted: bool = False
        self._code: CodeObject | None = None
        self._addr_to_idx: dict[int, int] = {}

        # --- GenericVM ---
        self._vm = GenericVM()
        self._register_opcodes()

        # --- Tracing ---
        self._current_trace_info: dict = {}

    @property
    def pc(self) -> int:
        """Current program counter value (ROM address)."""
        if self._code is None:
            return 0
        # Convert instruction index back to ROM address
        return self._current_pc_addr

    # -----------------------------------------------------------------
    # Opcode registration
    # -----------------------------------------------------------------

    def _register_opcodes(self) -> None:
        """Register all 46 Intel 4004 opcodes as GenericVM handlers."""
        handlers = {
            OP_NOP: self._handle_nop,
            OP_HLT: self._handle_hlt,
            OP_JCN: self._handle_jcn,
            OP_FIM: self._handle_fim,
            OP_SRC: self._handle_src,
            OP_FIN: self._handle_fin,
            OP_JIN: self._handle_jin,
            OP_JUN: self._handle_jun,
            OP_JMS: self._handle_jms,
            OP_INC: self._handle_inc,
            OP_ISZ: self._handle_isz,
            OP_ADD: self._handle_add,
            OP_SUB: self._handle_sub,
            OP_LD:  self._handle_ld,
            OP_XCH: self._handle_xch,
            OP_BBL: self._handle_bbl,
            OP_LDM: self._handle_ldm,
            # I/O ops
            OP_WRM: self._handle_wrm,
            OP_WMP: self._handle_wmp,
            OP_WRR: self._handle_wrr,
            OP_WPM: self._handle_wpm,
            OP_WR0: self._handle_wr0,
            OP_WR1: self._handle_wr1,
            OP_WR2: self._handle_wr2,
            OP_WR3: self._handle_wr3,
            OP_SBM: self._handle_sbm,
            OP_RDM: self._handle_rdm,
            OP_RDR: self._handle_rdr,
            OP_ADM: self._handle_adm,
            OP_RD0: self._handle_rd0,
            OP_RD1: self._handle_rd1,
            OP_RD2: self._handle_rd2,
            OP_RD3: self._handle_rd3,
            # Accumulator ops
            OP_CLB: self._handle_clb,
            OP_CLC: self._handle_clc,
            OP_IAC: self._handle_iac,
            OP_CMC: self._handle_cmc,
            OP_CMA: self._handle_cma,
            OP_RAL: self._handle_ral,
            OP_RAR: self._handle_rar,
            OP_TCC: self._handle_tcc,
            OP_DAC: self._handle_dac,
            OP_TCS: self._handle_tcs,
            OP_STC: self._handle_stc,
            OP_DAA: self._handle_daa,
            OP_KBP: self._handle_kbp,
            OP_DCL: self._handle_dcl,
        }
        for opcode, handler in handlers.items():
            self._vm.register_opcode(opcode, handler)

    # -----------------------------------------------------------------
    # Public API
    # -----------------------------------------------------------------

    def load_program(self, program: bytes) -> None:
        """Load a program into ROM starting at address 0.

        Each byte in the program is one byte of ROM. Instructions are 8 bits,
        but some (JUN, JMS, JCN, FIM, ISZ) are 2 bytes.
        """
        self.rom = bytearray(self.memory_size)
        for i, b in enumerate(program):
            if i < self.memory_size:
                self.rom[i] = b

    def step(self) -> Intel4004Trace:
        """Fetch, decode, and execute one instruction.

        Returns an Intel4004Trace with complete before/after state.
        """
        if self.halted:
            raise RuntimeError("CPU is halted — cannot step further")

        if self._code is None:
            raise RuntimeError("No program loaded — call load_program() first")

        # Snapshot state before execution
        acc_before = self.accumulator
        carry_before = self.carry
        pc_addr = self._current_pc_addr

        # Get instruction info for trace
        instr = self._code.instructions[self._vm.pc]
        operand = instr.operand
        raw = operand["raw"] if isinstance(operand, dict) else 0
        raw2 = operand.get("raw2") if isinstance(operand, dict) else None

        # Store address for PC tracking
        self._current_trace_info = {"addr": pc_addr, "raw": raw, "raw2": raw2}

        # Execute via GenericVM
        self._vm.step(self._code)

        # Build mnemonic
        mnemonic = self._current_trace_info.get("mnemonic", f"UNKNOWN(0x{raw:02X})")

        return Intel4004Trace(
            address=pc_addr,
            raw=raw,
            raw2=raw2,
            mnemonic=mnemonic,
            accumulator_before=acc_before,
            accumulator_after=self.accumulator,
            carry_before=carry_before,
            carry_after=self.carry,
        )

    def run(self, program: bytes, max_steps: int = 10000) -> list[Intel4004Trace]:
        """Load and run a program, returning a trace of every instruction.

        Execution continues until HLT or max_steps is reached.

        Example — the x = 1 + 2 program:

            >>> sim = Intel4004Simulator()
            >>> traces = sim.run(bytes([0xD1, 0xB0, 0xD2, 0x80, 0xB1, 0x01]))
            >>> for t in traces:
            ...     print(f"{t.address:03X}: {t.mnemonic:<10} A={t.accumulator_after}")
              000: LDM 1      A=1
              001: XCH R0     A=0
              002: LDM 2      A=2
              003: ADD R0     A=3
              004: XCH R1     A=0
              005: HLT        A=0
            >>> sim.registers[1]
            3
        """
        self.reset()
        self.load_program(program)
        self._prepare_execution()

        traces: list[Intel4004Trace] = []
        for _ in range(max_steps):
            if self.halted or self._vm.pc >= len(self._code.instructions):
                break
            trace = self.step()
            traces.append(trace)

        return traces

    def get_state(self) -> Intel4004State:
        """Return a frozen snapshot of the current CPU state.

        All mutable lists are converted to tuples so the result is a true
        immutable value.  The snapshot will not change even if the simulator
        continues executing after this call returns.

        This method satisfies the ``Simulator[Intel4004State]`` protocol from
        the ``simulator-protocol`` package.

        Returns
        -------
        Intel4004State:
            A frozen dataclass capturing the complete CPU state at this moment:
            accumulator, all 16 registers, carry flag, program counter,
            halted flag, full RAM contents, hardware stack, and stack pointer.

        Examples
        --------
        >>> sim = Intel4004Simulator()
        >>> sim.run(bytes([0xD5, 0x01]))  # LDM 5, HLT  (returns trace list)
        [...]
        >>> state = sim.get_state()
        >>> state.accumulator
        0
        >>> state.halted
        True
        """
        return Intel4004State(
            accumulator=self.accumulator,
            registers=tuple(self.registers),
            carry=self.carry,
            pc=self.pc,
            halted=self.halted,
            ram=tuple(
                tuple(tuple(reg) for reg in bank)
                for bank in self.ram
            ),
            hw_stack=tuple(self.hw_stack),
            stack_pointer=self.stack_pointer,
        )

    def execute(self, program: bytes, max_steps: int = 100_000) -> "ExecutionResult[Intel4004State]":
        """Load program, run to HALT or max_steps, return ExecutionResult.

        This is the protocol-conforming entry point for the
        ``Simulator[Intel4004State]`` protocol defined in the
        ``simulator-protocol`` package.  It wraps the existing ``run()``
        method but returns a richer result type that carries the final CPU
        state snapshot, the full per-instruction trace, and a clean error
        field.

        The existing ``run()`` method is unchanged — this method calls it
        internally and adapts the return value.

        Parameters
        ----------
        program:
            Raw Intel 4004 machine-code bytes.
        max_steps:
            Maximum instructions to execute before giving up (default 100,000).
            Programs that exceed this limit likely contain an infinite loop.

        Returns
        -------
        ExecutionResult[Intel4004State]:
            - ``halted``: True if HLT was reached; False if max_steps exceeded.
            - ``steps``: total instructions executed.
            - ``final_state``: frozen ``Intel4004State`` snapshot at termination.
            - ``error``: None on clean halt; error string if max_steps exceeded.
            - ``traces``: one ``StepTrace`` per instruction executed.

        Examples
        --------
        >>> sim = Intel4004Simulator()
        >>> program = bytes([0xD3, 0xB0, 0x01])  # LDM 3, XCH R0, HLT
        >>> result = sim.execute(program)
        >>> result.ok
        True
        >>> result.final_state.registers[0]
        3
        """
        from simulator_protocol import ExecutionResult, StepTrace

        self.reset()
        self.load_program(program)
        self._prepare_execution()

        step_traces: list[StepTrace] = []
        steps = 0

        while not self.halted and steps < max_steps:
            if self._code is None or self._vm.pc >= len(self._code.instructions):
                break
            pc_before = self.pc
            trace = self.step()
            step_traces.append(
                StepTrace(
                    pc_before=pc_before,
                    pc_after=self.pc,
                    mnemonic=trace.mnemonic,
                    description=f"{trace.mnemonic} @ 0x{pc_before:03X}",
                )
            )
            steps += 1

        return ExecutionResult(
            halted=self.halted,
            steps=steps,
            final_state=self.get_state(),
            error=None if self.halted else f"max_steps ({max_steps}) exceeded",
            traces=step_traces,
        )

    def reset(self) -> None:
        """Reset all CPU state to initial values."""
        self.accumulator = 0
        self.registers = [0] * 16
        self.carry = False
        self.rom = bytearray(self.memory_size)
        self.ram = [[[0] * 16 for _ in range(4)] for _ in range(4)]
        self.ram_status = [[[0] * 4 for _ in range(4)] for _ in range(4)]
        self.ram_output = [0] * 4
        self.ram_bank = 0
        self.ram_register = 0
        self.ram_character = 0
        self.rom_port = 0
        self.hw_stack = [0, 0, 0]
        self.stack_pointer = 0
        self.halted = False
        self._code = None
        self._vm.reset()
        self._idx_to_addr = {}
        self._addr_to_idx = {}
        self._current_pc_addr = 0

    def _prepare_execution(self) -> None:
        """Parse loaded ROM into a CodeObject and prepare address maps."""
        # Parse ROM bytes into instructions
        instructions: list[Instruction] = []
        self._idx_to_addr = {}
        self._addr_to_idx = {}
        pc = 0

        while pc < self.memory_size:
            raw = self.rom[pc]
            # Stop at uninitialized region (all zeros after program)
            # But NOP is 0x00, so we need to continue through NOPs in the program.
            # We'll parse the entire ROM to be safe.

            idx = len(instructions)
            self._idx_to_addr[idx] = pc
            self._addr_to_idx[pc] = idx

            if _is_two_byte(raw):
                raw2 = self.rom[pc + 1] if pc + 1 < self.memory_size else 0
                operand = _decode_operand(raw, raw2, pc)
                family_opcode = _family_opcode(raw)
                instructions.append(Instruction(opcode=family_opcode, operand=operand))
                pc += 2
            else:
                operand = _decode_operand(raw, None, pc)
                family_opcode = _family_opcode(raw)
                instructions.append(Instruction(opcode=family_opcode, operand=operand))
                pc += 1

        # Resolve jump targets to instruction indices
        for instr in instructions:
            if isinstance(instr.operand, dict) and "target_addr" in instr.operand:
                target_addr = instr.operand["target_addr"]
                instr.operand["target_idx"] = self._addr_to_idx.get(target_addr, 0)

        self._code = CodeObject(instructions=instructions)
        self._vm.reset()
        self._current_pc_addr = 0

    # -----------------------------------------------------------------
    # PC tracking
    # -----------------------------------------------------------------

    @property
    def _current_pc_addr(self) -> int:
        """Convert current GenericVM instruction index to ROM address."""
        return self._idx_to_addr.get(self._vm.pc, 0)

    @_current_pc_addr.setter
    def _current_pc_addr(self, value: int) -> None:
        """Allow setting _current_pc_addr (used during reset)."""
        # This is a no-op setter — the real PC is in self._vm.pc
        pass

    def _jump_to_addr(self, addr: int) -> None:
        """Jump to a ROM address by setting the GenericVM PC to the
        corresponding instruction index."""
        idx = self._addr_to_idx.get(addr, 0)
        self._vm.jump_to(idx)

    # -----------------------------------------------------------------
    # Register pair helpers
    # -----------------------------------------------------------------

    def _read_pair(self, pair_idx: int) -> int:
        """Read an 8-bit value from a register pair.

        Pair 0 = R0:R1, Pair 1 = R2:R3, etc.
        High nibble is the even register, low nibble is the odd register.
        """
        high_reg = pair_idx * 2
        low_reg = high_reg + 1
        return (self.registers[high_reg] << 4) | self.registers[low_reg]

    def _write_pair(self, pair_idx: int, value: int) -> None:
        """Write an 8-bit value to a register pair."""
        high_reg = pair_idx * 2
        low_reg = high_reg + 1
        self.registers[high_reg] = (value >> 4) & 0xF
        self.registers[low_reg] = value & 0xF

    # -----------------------------------------------------------------
    # Stack helpers
    # -----------------------------------------------------------------

    def _stack_push(self, address: int) -> None:
        """Push a return address onto the 3-level hardware stack.

        The real 4004 wraps silently on overflow — the 4th push overwrites
        the oldest entry. There is no stack overflow exception.
        """
        self.hw_stack[self.stack_pointer] = address & 0xFFF
        self.stack_pointer = (self.stack_pointer + 1) % 3

    def _stack_pop(self) -> int:
        """Pop a return address from the hardware stack."""
        self.stack_pointer = (self.stack_pointer - 1) % 3
        return self.hw_stack[self.stack_pointer]

    # -----------------------------------------------------------------
    # RAM helpers
    # -----------------------------------------------------------------

    def _ram_read_main(self) -> int:
        """Read the current RAM main character (set by SRC + DCL)."""
        return self.ram[self.ram_bank][self.ram_register][self.ram_character]

    def _ram_write_main(self, value: int) -> None:
        """Write to the current RAM main character."""
        self.ram[self.ram_bank][self.ram_register][self.ram_character] = value & 0xF

    def _ram_read_status(self, index: int) -> int:
        """Read a RAM status character (0–3) for the current register."""
        return self.ram_status[self.ram_bank][self.ram_register][index]

    def _ram_write_status(self, index: int, value: int) -> None:
        """Write a RAM status character (0–3)."""
        self.ram_status[self.ram_bank][self.ram_register][index] = value & 0xF

    # -----------------------------------------------------------------
    # Instruction handlers
    # -----------------------------------------------------------------
    # Each handler follows the GenericVM OpcodeHandler protocol:
    #   def handler(vm, instr, code) -> str | None
    # The 'vm' parameter is the GenericVM instance (we ignore it and use
    # self instead). We advance PC via self._vm.advance_pc() or jump.

    def _handle_nop(self, vm: GenericVM, instr: Instruction,
                    code: CodeObject) -> str | None:
        """NOP (0x00): No operation. Just advance PC."""
        self._current_trace_info["mnemonic"] = "NOP"
        self._vm.advance_pc()
        return None

    def _handle_hlt(self, vm: GenericVM, instr: Instruction,
                    code: CodeObject) -> str | None:
        """HLT (0x01): Halt execution. Simulator-only instruction."""
        self._current_trace_info["mnemonic"] = "HLT"
        self.halted = True
        self._vm.halted = True
        self._vm.advance_pc()
        return None

    # --- Immediate load ---

    def _handle_ldm(self, vm: GenericVM, instr: Instruction,
                    code: CodeObject) -> str | None:
        """LDM N (0xDN): Load immediate 4-bit value into accumulator. A = N."""
        n = instr.operand["imm"]
        self._current_trace_info["mnemonic"] = f"LDM {n}"
        self.accumulator = n & 0xF
        self._vm.advance_pc()
        return None

    # --- Register operations ---

    def _handle_ld(self, vm: GenericVM, instr: Instruction,
                   code: CodeObject) -> str | None:
        """LD Rn (0xAR): Load register into accumulator. A = Rn."""
        reg = instr.operand["reg"]
        self._current_trace_info["mnemonic"] = f"LD R{reg}"
        self.accumulator = self.registers[reg] & 0xF
        self._vm.advance_pc()
        return None

    def _handle_xch(self, vm: GenericVM, instr: Instruction,
                    code: CodeObject) -> str | None:
        """XCH Rn (0xBR): Exchange accumulator with register. Swap A and Rn."""
        reg = instr.operand["reg"]
        self._current_trace_info["mnemonic"] = f"XCH R{reg}"
        old_a = self.accumulator
        self.accumulator = self.registers[reg] & 0xF
        self.registers[reg] = old_a & 0xF
        self._vm.advance_pc()
        return None

    def _handle_inc(self, vm: GenericVM, instr: Instruction,
                    code: CodeObject) -> str | None:
        """INC Rn (0x6R): Increment register. Rn = (Rn + 1) & 0xF.

        Note: INC does NOT affect the carry flag. It's purely a register
        increment with 4-bit wrap-around.
        """
        reg = instr.operand["reg"]
        self._current_trace_info["mnemonic"] = f"INC R{reg}"
        self.registers[reg] = (self.registers[reg] + 1) & 0xF
        self._vm.advance_pc()
        return None

    # --- Arithmetic (register) ---

    def _handle_add(self, vm: GenericVM, instr: Instruction,
                    code: CodeObject) -> str | None:
        """ADD Rn (0x8R): Add register to accumulator with carry.

        A = A + Rn + carry. Carry is set if result > 15.

        The carry flag participates in the addition — this is how multi-digit
        BCD arithmetic works. After adding two BCD digits, the carry propagates
        to the next digit pair.
        """
        reg = instr.operand["reg"]
        self._current_trace_info["mnemonic"] = f"ADD R{reg}"
        result = self.accumulator + self.registers[reg] + (1 if self.carry else 0)
        self.carry = result > 0xF
        self.accumulator = result & 0xF
        self._vm.advance_pc()
        return None

    def _handle_sub(self, vm: GenericVM, instr: Instruction,
                    code: CodeObject) -> str | None:
        """SUB Rn (0x9R): Subtract register from accumulator with borrow.

        A = A + ~Rn + (1 if not carry else 0).

        The 4004 uses complement-add for subtraction. The carry flag is
        INVERTED from what you might expect:
          - carry=1 means NO borrow occurred (result >= 0)
          - carry=0 means borrow occurred (result was negative before wrap)

        This matches the MCS-4 manual's definition. The initial carry state
        acts as an inverse borrow-in.
        """
        reg = instr.operand["reg"]
        self._current_trace_info["mnemonic"] = f"SUB R{reg}"
        complement = (~self.registers[reg]) & 0xF
        borrow_in = 0 if self.carry else 1
        result = self.accumulator + complement + borrow_in
        self.carry = result > 0xF
        self.accumulator = result & 0xF
        self._vm.advance_pc()
        return None

    # --- Arithmetic (RAM) ---

    def _handle_adm(self, vm: GenericVM, instr: Instruction,
                    code: CodeObject) -> str | None:
        """ADM (0xEB): Add RAM main character to accumulator with carry."""
        self._current_trace_info["mnemonic"] = "ADM"
        ram_val = self._ram_read_main()
        result = self.accumulator + ram_val + (1 if self.carry else 0)
        self.carry = result > 0xF
        self.accumulator = result & 0xF
        self._vm.advance_pc()
        return None

    def _handle_sbm(self, vm: GenericVM, instr: Instruction,
                    code: CodeObject) -> str | None:
        """SBM (0xE8): Subtract RAM main character from accumulator."""
        self._current_trace_info["mnemonic"] = "SBM"
        ram_val = self._ram_read_main()
        complement = (~ram_val) & 0xF
        borrow_in = 0 if self.carry else 1
        result = self.accumulator + complement + borrow_in
        self.carry = result > 0xF
        self.accumulator = result & 0xF
        self._vm.advance_pc()
        return None

    # --- Accumulator operations ---

    def _handle_clb(self, vm: GenericVM, instr: Instruction,
                    code: CodeObject) -> str | None:
        """CLB (0xF0): Clear both. A = 0, carry = 0."""
        self._current_trace_info["mnemonic"] = "CLB"
        self.accumulator = 0
        self.carry = False
        self._vm.advance_pc()
        return None

    def _handle_clc(self, vm: GenericVM, instr: Instruction,
                    code: CodeObject) -> str | None:
        """CLC (0xF1): Clear carry. carry = 0."""
        self._current_trace_info["mnemonic"] = "CLC"
        self.carry = False
        self._vm.advance_pc()
        return None

    def _handle_iac(self, vm: GenericVM, instr: Instruction,
                    code: CodeObject) -> str | None:
        """IAC (0xF2): Increment accumulator. A = (A + 1) & 0xF.

        Carry is set if A was 15 (wraps to 0).
        """
        self._current_trace_info["mnemonic"] = "IAC"
        result = self.accumulator + 1
        self.carry = result > 0xF
        self.accumulator = result & 0xF
        self._vm.advance_pc()
        return None

    def _handle_cmc(self, vm: GenericVM, instr: Instruction,
                    code: CodeObject) -> str | None:
        """CMC (0xF3): Complement carry. carry = !carry."""
        self._current_trace_info["mnemonic"] = "CMC"
        self.carry = not self.carry
        self._vm.advance_pc()
        return None

    def _handle_cma(self, vm: GenericVM, instr: Instruction,
                    code: CodeObject) -> str | None:
        """CMA (0xF4): Complement accumulator. A = ~A & 0xF (4-bit NOT)."""
        self._current_trace_info["mnemonic"] = "CMA"
        self.accumulator = (~self.accumulator) & 0xF
        self._vm.advance_pc()
        return None

    def _handle_ral(self, vm: GenericVM, instr: Instruction,
                    code: CodeObject) -> str | None:
        """RAL (0xF5): Rotate accumulator left through carry.

        Before: [carry | A3 A2 A1 A0]
        After:  [A3   | A2 A1 A0 carry_old]

        The carry shifts into the lowest bit, and the highest bit shifts
        into carry. This is a 5-bit rotation through carry.
        """
        self._current_trace_info["mnemonic"] = "RAL"
        old_carry = 1 if self.carry else 0
        # A3 goes to carry
        self.carry = bool(self.accumulator & 0x8)
        # Shift left, bring old carry into bit 0
        self.accumulator = ((self.accumulator << 1) | old_carry) & 0xF
        self._vm.advance_pc()
        return None

    def _handle_rar(self, vm: GenericVM, instr: Instruction,
                    code: CodeObject) -> str | None:
        """RAR (0xF6): Rotate accumulator right through carry.

        Before: [carry | A3 A2 A1 A0]
        After:  [A0   | carry_old A3 A2 A1]

        The carry shifts into the highest bit, and the lowest bit shifts
        into carry. This is a 5-bit rotation through carry.
        """
        self._current_trace_info["mnemonic"] = "RAR"
        old_carry = 1 if self.carry else 0
        # A0 goes to carry
        self.carry = bool(self.accumulator & 0x1)
        # Shift right, bring old carry into bit 3
        self.accumulator = ((self.accumulator >> 1) | (old_carry << 3)) & 0xF
        self._vm.advance_pc()
        return None

    def _handle_tcc(self, vm: GenericVM, instr: Instruction,
                    code: CodeObject) -> str | None:
        """TCC (0xF7): Transfer carry to accumulator, clear carry.

        A = 1 if carry was set, else 0. Carry is always cleared.
        """
        self._current_trace_info["mnemonic"] = "TCC"
        self.accumulator = 1 if self.carry else 0
        self.carry = False
        self._vm.advance_pc()
        return None

    def _handle_dac(self, vm: GenericVM, instr: Instruction,
                    code: CodeObject) -> str | None:
        """DAC (0xF8): Decrement accumulator. A = (A - 1) & 0xF.

        Carry is SET if no borrow (A > 0), CLEARED if borrow (A was 0).
        """
        self._current_trace_info["mnemonic"] = "DAC"
        result = self.accumulator - 1
        self.carry = result >= 0  # No borrow if result >= 0
        self.accumulator = result & 0xF
        self._vm.advance_pc()
        return None

    def _handle_tcs(self, vm: GenericVM, instr: Instruction,
                    code: CodeObject) -> str | None:
        """TCS (0xF9): Transfer carry subtract.

        A = 10 if carry was set, else 9. Carry is always cleared.

        This is used in BCD subtraction: it provides the tens-complement
        correction factor. After complementing a BCD digit, you add TCS
        to get the correct subtraction result.
        """
        self._current_trace_info["mnemonic"] = "TCS"
        self.accumulator = 10 if self.carry else 9
        self.carry = False
        self._vm.advance_pc()
        return None

    def _handle_stc(self, vm: GenericVM, instr: Instruction,
                    code: CodeObject) -> str | None:
        """STC (0xFA): Set carry. carry = 1."""
        self._current_trace_info["mnemonic"] = "STC"
        self.carry = True
        self._vm.advance_pc()
        return None

    def _handle_daa(self, vm: GenericVM, instr: Instruction,
                    code: CodeObject) -> str | None:
        """DAA (0xFB): Decimal adjust accumulator (BCD correction).

        If A > 9 or carry is set, add 6 to A. If the addition causes
        overflow past 15, set carry.

        This instruction exists because the 4004 was built for BCD
        calculators. When you add two BCD digits (0–9 each), the result
        might be > 9 (e.g., 7 + 8 = 15). DAA corrects this by adding 6,
        wrapping to the correct BCD digit (15 + 6 = 21, keep lower nibble
        5, set carry for the tens digit).
        """
        self._current_trace_info["mnemonic"] = "DAA"
        if self.accumulator > 9 or self.carry:
            result = self.accumulator + 6
            if result > 0xF:
                self.carry = True
            self.accumulator = result & 0xF
        self._vm.advance_pc()
        return None

    def _handle_kbp(self, vm: GenericVM, instr: Instruction,
                    code: CodeObject) -> str | None:
        """KBP (0xFC): Keyboard process.

        Converts a 1-hot encoded input to a binary position number:
            0b0000 (0)  → 0  (no key pressed)
            0b0001 (1)  → 1  (key 1)
            0b0010 (2)  → 2  (key 2)
            0b0100 (4)  → 3  (key 3)
            0b1000 (8)  → 4  (key 4)
            anything else → 15 (error: multiple keys pressed)

        This was designed for the Busicom calculator's keyboard scanning.
        """
        self._current_trace_info["mnemonic"] = "KBP"
        kbp_table = {0: 0, 1: 1, 2: 2, 4: 3, 8: 4}
        self.accumulator = kbp_table.get(self.accumulator, 15)
        self._vm.advance_pc()
        return None

    def _handle_dcl(self, vm: GenericVM, instr: Instruction,
                    code: CodeObject) -> str | None:
        """DCL (0xFD): Designate command line (select RAM bank).

        The lower 3 bits of A select the RAM bank (0–7, but only 0–3
        are typically used since the 4004 has 4 RAM banks).
        """
        self._current_trace_info["mnemonic"] = "DCL"
        self.ram_bank = self.accumulator & 0x7
        # Clamp to valid bank range
        if self.ram_bank > 3:
            self.ram_bank = self.ram_bank & 0x3
        self._vm.advance_pc()
        return None

    # --- Jump instructions ---

    def _handle_jun(self, vm: GenericVM, instr: Instruction,
                    code: CodeObject) -> str | None:
        """JUN addr (0x4H 0xLL): Unconditional jump to 12-bit address."""
        addr = instr.operand["addr"]
        self._current_trace_info["mnemonic"] = f"JUN 0x{addr:03X}"
        target_idx = instr.operand.get("target_idx", 0)
        self._vm.jump_to(target_idx)
        return None

    def _handle_jcn(self, vm: GenericVM, instr: Instruction,
                    code: CodeObject) -> str | None:
        """JCN cond,addr (0x1C 0xAA): Conditional jump.

        The condition nibble C has 4 bits:
            Bit 3 (0x8): INVERT — if set, invert the final test result
            Bit 2 (0x4): TEST_ZERO — test if accumulator == 0
            Bit 1 (0x2): TEST_CARRY — test if carry == 1
            Bit 0 (0x1): TEST_PIN — test input pin (always 0 in simulator)

        Multiple test bits can be set — they are OR'd together. If the
        (possibly inverted) result is True, the jump is taken.
        """
        cond = instr.operand["cond"]
        addr = instr.operand["addr"]
        self._current_trace_info["mnemonic"] = f"JCN {cond},{addr:02X}"

        # Evaluate condition tests (OR'd together)
        test_result = False
        if cond & 0x4:  # Test accumulator == 0
            test_result = test_result or (self.accumulator == 0)
        if cond & 0x2:  # Test carry == 1
            test_result = test_result or self.carry
        if cond & 0x1:  # Test input pin (always 0 = not asserted)
            test_result = test_result or False

        # Invert if bit 3 is set
        if cond & 0x8:
            test_result = not test_result

        if test_result:
            target_idx = instr.operand.get("target_idx", 0)
            self._vm.jump_to(target_idx)
        else:
            self._vm.advance_pc()
        return None

    def _handle_isz(self, vm: GenericVM, instr: Instruction,
                    code: CodeObject) -> str | None:
        """ISZ Rn,addr (0x7R 0xAA): Increment register, skip if zero.

        Increment Rn. If Rn != 0 after increment, jump to addr.
        If Rn == 0 (wrapped from 15), continue to next instruction.

        This is the 4004's loop counter instruction. Load a register with
        a negative count (in 4-bit two's complement, e.g., -4 = 12), then
        ISZ will loop until the register wraps to 0.
        """
        reg = instr.operand["reg"]
        addr = instr.operand["addr"]
        self._current_trace_info["mnemonic"] = f"ISZ R{reg},0x{addr:02X}"

        self.registers[reg] = (self.registers[reg] + 1) & 0xF

        if self.registers[reg] != 0:
            target_idx = instr.operand.get("target_idx", 0)
            self._vm.jump_to(target_idx)
        else:
            self._vm.advance_pc()
        return None

    # --- Subroutine instructions ---

    def _handle_jms(self, vm: GenericVM, instr: Instruction,
                    code: CodeObject) -> str | None:
        """JMS addr (0x5H 0xLL): Jump to subroutine.

        Push the address of the NEXT instruction onto the hardware stack,
        then jump to the 12-bit target address.
        """
        addr = instr.operand["addr"]
        self._current_trace_info["mnemonic"] = f"JMS 0x{addr:03X}"

        # Push return address (address of instruction AFTER this 2-byte JMS)
        return_pc = instr.operand["pc"] + 2
        self._stack_push(return_pc)

        target_idx = instr.operand.get("target_idx", 0)
        self._vm.jump_to(target_idx)
        return None

    def _handle_bbl(self, vm: GenericVM, instr: Instruction,
                    code: CodeObject) -> str | None:
        """BBL N (0xCN): Branch back and load.

        Pop the top of the hardware stack, load N into the accumulator,
        and jump to the popped address.

        This is the 4004's "return from subroutine" instruction with a
        twist — it also loads an immediate value into A. This lets a
        subroutine return a simple status code.
        """
        n = instr.operand["imm"]
        self._current_trace_info["mnemonic"] = f"BBL {n}"

        self.accumulator = n & 0xF
        return_addr = self._stack_pop()
        self._jump_to_addr(return_addr)
        return None

    # --- Register pair instructions ---

    def _handle_fim(self, vm: GenericVM, instr: Instruction,
                    code: CodeObject) -> str | None:
        """FIM Pp,data (0x2P 0xDD): Fetch immediate to register pair.

        Load the 8-bit immediate data into register pair Pp.
        High nibble goes to R(2*p), low nibble goes to R(2*p+1).
        """
        pair = instr.operand["pair"]
        data = instr.operand["data"]
        self._current_trace_info["mnemonic"] = f"FIM P{pair},0x{data:02X}"
        self._write_pair(pair, data)
        self._vm.advance_pc()
        return None

    def _handle_src(self, vm: GenericVM, instr: Instruction,
                    code: CodeObject) -> str | None:
        """SRC Pp (0x2P+1): Send register control.

        Send the 8-bit value in register pair Pp as an address for
        subsequent RAM/ROM I/O operations. The high nibble selects the
        RAM register (0–3), the low nibble selects the character (0–15).
        """
        pair = instr.operand["pair"]
        self._current_trace_info["mnemonic"] = f"SRC P{pair}"
        pair_val = self._read_pair(pair)
        self.ram_register = (pair_val >> 4) & 0xF
        self.ram_character = pair_val & 0xF
        self._vm.advance_pc()
        return None

    def _handle_fin(self, vm: GenericVM, instr: Instruction,
                    code: CodeObject) -> str | None:
        """FIN Pp (0x3P): Fetch indirect from ROM.

        Read the ROM byte at the address given by register pair P0 (R0:R1),
        and store the result into register pair Pp.

        The address used is within the same page as the current PC
        (bits 11–8 of PC are preserved, bits 7–0 come from P0).
        """
        pair = instr.operand["pair"]
        self._current_trace_info["mnemonic"] = f"FIN P{pair}"

        # Address comes from P0 (R0:R1)
        p0_val = self._read_pair(0)
        # Same page as current instruction
        current_page = instr.operand["pc"] & 0xF00
        rom_addr = current_page | p0_val
        rom_byte = self.rom[rom_addr] if rom_addr < self.memory_size else 0

        self._write_pair(pair, rom_byte)
        self._vm.advance_pc()
        return None

    def _handle_jin(self, vm: GenericVM, instr: Instruction,
                    code: CodeObject) -> str | None:
        """JIN Pp (0x3P+1): Jump indirect.

        Jump to the address formed by the current page and register pair Pp.
        PC[11:8] stays the same, PC[7:0] = pair value.
        """
        pair = instr.operand["pair"]
        self._current_trace_info["mnemonic"] = f"JIN P{pair}"
        pair_val = self._read_pair(pair)
        current_page = instr.operand["pc"] & 0xF00
        target_addr = current_page | pair_val
        self._jump_to_addr(target_addr)
        return None

    # --- RAM/ROM I/O ---

    def _handle_wrm(self, vm: GenericVM, instr: Instruction,
                    code: CodeObject) -> str | None:
        """WRM (0xE0): Write accumulator to RAM main character."""
        self._current_trace_info["mnemonic"] = "WRM"
        self._ram_write_main(self.accumulator)
        self._vm.advance_pc()
        return None

    def _handle_wmp(self, vm: GenericVM, instr: Instruction,
                    code: CodeObject) -> str | None:
        """WMP (0xE1): Write accumulator to RAM output port."""
        self._current_trace_info["mnemonic"] = "WMP"
        self.ram_output[self.ram_bank] = self.accumulator & 0xF
        self._vm.advance_pc()
        return None

    def _handle_wrr(self, vm: GenericVM, instr: Instruction,
                    code: CodeObject) -> str | None:
        """WRR (0xE2): Write accumulator to ROM I/O port."""
        self._current_trace_info["mnemonic"] = "WRR"
        self.rom_port = self.accumulator & 0xF
        self._vm.advance_pc()
        return None

    def _handle_wpm(self, vm: GenericVM, instr: Instruction,
                    code: CodeObject) -> str | None:
        """WPM (0xE3): Write program RAM. Not simulated (EPROM programming)."""
        self._current_trace_info["mnemonic"] = "WPM"
        # WPM was used for EPROM programming on the 4004 — not applicable
        # in simulation. We treat it as a NOP.
        self._vm.advance_pc()
        return None

    def _handle_wr0(self, vm: GenericVM, instr: Instruction,
                    code: CodeObject) -> str | None:
        """WR0 (0xE4): Write accumulator to RAM status character 0."""
        self._current_trace_info["mnemonic"] = "WR0"
        self._ram_write_status(0, self.accumulator)
        self._vm.advance_pc()
        return None

    def _handle_wr1(self, vm: GenericVM, instr: Instruction,
                    code: CodeObject) -> str | None:
        """WR1 (0xE5): Write accumulator to RAM status character 1."""
        self._current_trace_info["mnemonic"] = "WR1"
        self._ram_write_status(1, self.accumulator)
        self._vm.advance_pc()
        return None

    def _handle_wr2(self, vm: GenericVM, instr: Instruction,
                    code: CodeObject) -> str | None:
        """WR2 (0xE6): Write accumulator to RAM status character 2."""
        self._current_trace_info["mnemonic"] = "WR2"
        self._ram_write_status(2, self.accumulator)
        self._vm.advance_pc()
        return None

    def _handle_wr3(self, vm: GenericVM, instr: Instruction,
                    code: CodeObject) -> str | None:
        """WR3 (0xE7): Write accumulator to RAM status character 3."""
        self._current_trace_info["mnemonic"] = "WR3"
        self._ram_write_status(3, self.accumulator)
        self._vm.advance_pc()
        return None

    def _handle_rdm(self, vm: GenericVM, instr: Instruction,
                    code: CodeObject) -> str | None:
        """RDM (0xE9): Read RAM main character into accumulator."""
        self._current_trace_info["mnemonic"] = "RDM"
        self.accumulator = self._ram_read_main()
        self._vm.advance_pc()
        return None

    def _handle_rdr(self, vm: GenericVM, instr: Instruction,
                    code: CodeObject) -> str | None:
        """RDR (0xEA): Read ROM I/O port into accumulator."""
        self._current_trace_info["mnemonic"] = "RDR"
        self.accumulator = self.rom_port & 0xF
        self._vm.advance_pc()
        return None

    def _handle_rd0(self, vm: GenericVM, instr: Instruction,
                    code: CodeObject) -> str | None:
        """RD0 (0xEC): Read RAM status character 0 into accumulator."""
        self._current_trace_info["mnemonic"] = "RD0"
        self.accumulator = self._ram_read_status(0)
        self._vm.advance_pc()
        return None

    def _handle_rd1(self, vm: GenericVM, instr: Instruction,
                    code: CodeObject) -> str | None:
        """RD1 (0xED): Read RAM status character 1 into accumulator."""
        self._current_trace_info["mnemonic"] = "RD1"
        self.accumulator = self._ram_read_status(1)
        self._vm.advance_pc()
        return None

    def _handle_rd2(self, vm: GenericVM, instr: Instruction,
                    code: CodeObject) -> str | None:
        """RD2 (0xEE): Read RAM status character 2 into accumulator."""
        self._current_trace_info["mnemonic"] = "RD2"
        self.accumulator = self._ram_read_status(2)
        self._vm.advance_pc()
        return None

    def _handle_rd3(self, vm: GenericVM, instr: Instruction,
                    code: CodeObject) -> str | None:
        """RD3 (0xEF): Read RAM status character 3 into accumulator."""
        self._current_trace_info["mnemonic"] = "RD3"
        self.accumulator = self._ram_read_status(3)
        self._vm.advance_pc()
        return None
