"""IBM 704 behavioral simulator.

Executes 36-bit IBM 704 instruction words against a 32K-word core memory and
the 704's register set (AC, MQ, three index registers). Conforms to the
``Simulator[IBM704State]`` protocol from ``simulator-protocol`` so the same
end-to-end test harness used for every other ISA in this repo works here too.

Architecture overview
---------------------
* **AC (38 bits)** — sign + Q + P + 35-bit magnitude. The Q and P bits are
  overflow indicators set by arithmetic instructions; if P is set, the
  ``overflow_trigger`` is also set so software can detect overflow with TOV.
* **MQ (36 bits)** — multiplier-quotient register; participates in MPY, DVP,
  DVH, and the loads/stores LDQ/STQ.
* **Index registers (3 × 15 bits)** — IRA, IRB, IRC. Selected by the tag
  field (bits 15-17 of any instruction). Tag 1 → IRA, 2 → IRB, 4 → IRC; if
  multiple bits are set, the OR of the selected registers' contents is
  used (per the 1955 *Reference Manual*).
* **Memory** — 32,768 words of magnetic core, addressed by a 15-bit address.
* **PC** — 15 bits.

Instruction dispatch
--------------------
Every word can be decoded two ways:

* **Type A** (TXI, TIX, TXH, TXL): the top 3 bits are a *prefix* opcode in
  ``{1, 2, 3, 5}``. The remaining bits encode a 15-bit decrement, a 3-bit
  tag, and a 15-bit address.
* **Type B** (everything else): the top 12 bits are the full opcode, then
  3 unused bits, 3-bit tag, 15-bit address.

The decoder tests the prefix first; if it matches a Type A pattern, the
Type A handler is invoked; otherwise the Type B opcode table is consulted.

Effective address
-----------------
For any instruction with an address Y and tag T::

    eff_addr = (Y - C(T)) & 0x7FFF

where ``C(T)`` is the OR of the contents of the index registers selected by
the bits of T. Tag = 0 means no indexing. The mask ``& 0x7FFF`` ensures
underflow wraps cleanly within the 15-bit address space.

What's not implemented (deferred to v2)
---------------------------------------
* I/O instructions (RDS, WRS, BSR, BSF, REW, RTB, RCD, etc.)
* Sense lights and switches (SLT, SLN, SWT, SLF)
* The full shift family (LRS, LLS, ARS, ALS, LGR, LGL, RQL)
* BCD character manipulation (CVR, CRQ, ORA, ORS, ANA, ANS, ERA)
* Sign-manipulation beyond ADM (SSP, SSM, CHS, CLS)
* Round and floating-point reciprocal (RND, FRN, UFA, UFS, UFM, UFDP)
* Programmed interrupts and the trap mechanism

These are in scope for follow-up work but not required for hosting FORTRAN-
style numeric programs or LISP-style cons-cell manipulation.
"""

from __future__ import annotations

from collections.abc import Callable

from simulator_protocol import ExecutionResult, StepTrace

from ibm704_simulator.state import IBM704State
from ibm704_simulator.word import (
    ADDRESS_MASK,
    FP_CHAR_MASK,
    MAGNITUDE_MASK,
    SIGN_BIT,
    TAG_MASK,
    WORD_BYTES,
    WORD_MASK,
    add_sign_magnitude,
    fp_to_float,
    float_to_fp,
    make_word,
    pack_word,
    unpack_program,
    word_magnitude,
    word_sign,
)

# Memory size — the maximum 704 configuration was 32K words.
MEMORY_WORDS = 32768
PC_MASK = ADDRESS_MASK  # PC is 15 bits, same as address field

# ---------------------------------------------------------------------------
# Opcode constants
# ---------------------------------------------------------------------------
# Type B opcodes occupy the top 12 bits of the 36-bit instruction word.
# IBM's *Reference Manual* presents these as octal numbers with a sign prefix:
# +0500 means S=0 plus 11-bit core 0o500 = 0x140. -0500 means S=1 plus the
# same core, so the full 12-bit opcode = (1 << 11) | 0x140 = 0x940.
#
# We define each opcode as a plain Python int — the 12-bit value with sign
# folded in. The dispatcher reads bits 24-35 of the instruction word and
# looks up the handler.

OP_HTR = 0x000  # Halt and Transfer
OP_HPR = 0x110  # Halt and Proceed
OP_NOP = 0x1F1  # No Operation
OP_CLA = 0x140  # Clear and Add
OP_CAL = 0x940  # Clear and Add Logical (S=1 of CLA)
OP_ADD = 0x100  # Add
OP_SUB = 0x102  # Subtract
OP_ADM = 0x101  # Add Magnitude
OP_STO = 0x181  # Store
OP_STZ = 0x180  # Store Zero
OP_STQ = 0x980  # Store MQ (S=1 of STZ)
OP_LDQ = 0x170  # Load MQ
OP_XCA = 0x059  # Exchange AC and MQ
OP_MPY = 0x080  # Multiply
OP_DVP = 0x091  # Divide or Proceed
OP_DVH = 0x090  # Divide or Halt
OP_TRA = 0x010  # Transfer
OP_TZE = 0x040  # Transfer on Zero
OP_TNZ = 0x840  # Transfer on Non-Zero
OP_TPL = 0x050  # Transfer on Plus
OP_TMI = 0x850  # Transfer on Minus
OP_TOV = 0x060  # Transfer on Overflow
OP_TNO = 0x860  # Transfer on No Overflow
OP_TQO = 0x071  # Transfer on MQ Overflow
OP_TQP = 0x072  # Transfer on MQ Plus
OP_LXA = 0x15C  # Load Index from Address
OP_LXD = 0x95C  # Load Index from Decrement
OP_SXA = 0x19C  # Store Index in Address
OP_SXD = 0x99C  # Store Index in Decrement
OP_PAX = 0x1DC  # Place Address in Index
OP_PDX = 0x9DC  # Place Decrement in Index
OP_PXA = 0x1EC  # Place Index in Address
OP_FAD = 0x0C0  # Floating Add
OP_FSB = 0x0C2  # Floating Subtract
OP_FMP = 0x0B0  # Floating Multiply
OP_FDP = 0x0A0  # Floating Divide or Proceed

# Type A prefix codes (top 3 bits of the 36-bit word)
PREFIX_TXI = 0b001  # 1 — Transfer with Index Incremented
PREFIX_TIX = 0b010  # 2 — Transfer on Index
PREFIX_TXH = 0b011  # 3 — Transfer on Index High
PREFIX_TXL = 0b101  # 5 — Transfer on Index Low or Equal

TYPE_A_PREFIXES = {PREFIX_TXI, PREFIX_TIX, PREFIX_TXH, PREFIX_TXL}


# ---------------------------------------------------------------------------
# IBM 704 simulator
# ---------------------------------------------------------------------------


class IBM704Simulator:
    """Behavioral simulator for the IBM 704 mainframe (1954).

    Conforms to ``Simulator[IBM704State]``. See the module-level docstring
    for an architecture overview and the ``simulator-protocol`` package for
    the protocol contract.

    Parameters
    ----------
    memory_words:
        Number of 36-bit words of core memory. Defaults to the maximum
        704 configuration of 32,768. Smaller values are useful for tests
        that want to verify out-of-range address handling.

    Examples
    --------
    >>> sim = IBM704Simulator()
    >>> # Smallest possible program: HTR 0 (halt at address 0)
    >>> from ibm704_simulator.word import pack_word
    >>> result = sim.execute(pack_word(0x000_0_0_0000))
    >>> result.ok
    True
    >>> result.steps
    1
    """

    def __init__(self, memory_words: int = MEMORY_WORDS) -> None:
        if memory_words <= 0 or memory_words > MEMORY_WORDS:
            raise ValueError(
                f"memory_words must be in [1, {MEMORY_WORDS}], "
                f"got {memory_words}"
            )
        self._memory_size = memory_words
        # Dispatch table built once per instance.
        self._type_b_handlers: dict[
            int, Callable[[int, int, int], tuple[int, str]]
        ] = {
            OP_HTR: self._op_htr,
            OP_HPR: self._op_hpr,
            OP_NOP: self._op_nop,
            OP_CLA: self._op_cla,
            OP_CAL: self._op_cal,
            OP_ADD: self._op_add,
            OP_SUB: self._op_sub,
            OP_ADM: self._op_adm,
            OP_STO: self._op_sto,
            OP_STZ: self._op_stz,
            OP_STQ: self._op_stq,
            OP_LDQ: self._op_ldq,
            OP_XCA: self._op_xca,
            OP_MPY: self._op_mpy,
            OP_DVP: self._op_dvp,
            OP_DVH: self._op_dvh,
            OP_TRA: self._op_tra,
            OP_TZE: self._op_tze,
            OP_TNZ: self._op_tnz,
            OP_TPL: self._op_tpl,
            OP_TMI: self._op_tmi,
            OP_TOV: self._op_tov,
            OP_TNO: self._op_tno,
            OP_TQO: self._op_tqo,
            OP_TQP: self._op_tqp,
            OP_LXA: self._op_lxa,
            OP_LXD: self._op_lxd,
            OP_SXA: self._op_sxa,
            OP_SXD: self._op_sxd,
            OP_PAX: self._op_pax,
            OP_PDX: self._op_pdx,
            OP_PXA: self._op_pxa,
            OP_FAD: self._op_fad,
            OP_FSB: self._op_fsb,
            OP_FMP: self._op_fmp,
            OP_FDP: self._op_fdp,
        }
        self.reset()

    # ------------------------------------------------------------------
    # Protocol API
    # ------------------------------------------------------------------

    def reset(self) -> None:
        """Reset all CPU and memory state to power-on defaults."""
        self._ac_sign: int = 0
        self._ac_p: int = 0
        self._ac_q: int = 0
        self._ac_magnitude: int = 0
        self._mq: int = 0
        self._index_a: int = 0
        self._index_b: int = 0
        self._index_c: int = 0
        self._pc: int = 0
        self._halted: bool = False
        self._overflow_trigger: bool = False
        self._divide_check_trigger: bool = False
        self._mq_overflow: bool = False
        self._memory: list[int] = [0] * self._memory_size

    def load(self, program: bytes) -> None:
        """Load a packed-word byte stream into core memory starting at word 0.

        See ``ibm704_simulator.word.pack_program`` for the encoding (5 bytes
        per 36-bit word, big-endian, top 4 bits zero).

        Raises ``ValueError`` if the byte stream is malformed or too long.
        """
        words = unpack_program(program)
        if len(words) > self._memory_size:
            raise ValueError(
                f"program is {len(words)} words; memory holds "
                f"{self._memory_size}"
            )
        for i, word in enumerate(words):
            self._memory[i] = word

    def step(self) -> StepTrace:
        """Execute exactly one instruction and return a trace.

        Raises ``RuntimeError`` if the machine is halted.
        """
        if self._halted:
            raise RuntimeError("cannot step: machine is halted")
        return self._step_once()

    def execute(
        self, program: bytes, max_steps: int = 100_000
    ) -> ExecutionResult[IBM704State]:
        """Reset, load, run to halt or ``max_steps``, return full result."""
        self.reset()
        self.load(program)
        traces: list[StepTrace] = []
        error: str | None = None
        steps = 0
        while steps < max_steps:
            if self._halted:
                break
            try:
                trace = self._step_once()
            except RuntimeError as exc:  # invalid opcode etc.
                error = str(exc)
                self._halted = True
                break
            traces.append(trace)
            steps += 1
        else:
            error = f"max_steps ({max_steps}) exceeded"
        return ExecutionResult(
            halted=self._halted,
            steps=len(traces),
            final_state=self.get_state(),
            error=error,
            traces=traces,
        )

    def get_state(self) -> IBM704State:
        """Return a frozen snapshot of all CPU and memory state."""
        return IBM704State(
            accumulator_sign=bool(self._ac_sign),
            accumulator_p=bool(self._ac_p),
            accumulator_q=bool(self._ac_q),
            accumulator_magnitude=self._ac_magnitude,
            mq=self._mq,
            mq_sign=bool(self._mq & SIGN_BIT),
            mq_magnitude=self._mq & MAGNITUDE_MASK,
            index_a=self._index_a,
            index_b=self._index_b,
            index_c=self._index_c,
            pc=self._pc,
            halted=self._halted,
            overflow_trigger=self._overflow_trigger,
            divide_check_trigger=self._divide_check_trigger,
            memory=tuple(self._memory),
        )

    # ------------------------------------------------------------------
    # Internals: fetch / decode / dispatch
    # ------------------------------------------------------------------

    def _step_once(self) -> StepTrace:
        pc_before = self._pc
        if pc_before >= self._memory_size:
            raise RuntimeError(
                f"PC {pc_before:#06x} outside memory "
                f"(size {self._memory_size})"
            )
        word = self._memory[pc_before]
        prefix = (word >> 33) & 0x7
        if prefix in TYPE_A_PREFIXES:
            mnemonic, description = self._dispatch_type_a(prefix, word)
        else:
            opcode = (word >> 24) & 0xFFF
            tag = (word >> 15) & TAG_MASK
            address = word & ADDRESS_MASK
            mnemonic, description = self._dispatch_type_b(
                opcode, tag, address
            )
        return StepTrace(
            pc_before=pc_before,
            pc_after=self._pc,
            mnemonic=mnemonic,
            description=description,
        )

    def _dispatch_type_b(
        self, opcode: int, tag: int, address: int
    ) -> tuple[str, str]:
        handler = self._type_b_handlers.get(opcode)
        if handler is None:
            raise RuntimeError(
                f"unknown opcode {opcode:#05x} at PC={self._pc:#06x}"
            )
        return handler(opcode, tag, address)

    def _dispatch_type_a(self, prefix: int, word: int) -> tuple[str, str]:
        decrement = (word >> 18) & ADDRESS_MASK
        tag = (word >> 15) & TAG_MASK
        address = word & ADDRESS_MASK
        if prefix == PREFIX_TXI:
            return self._op_txi(decrement, tag, address)
        if prefix == PREFIX_TIX:
            return self._op_tix(decrement, tag, address)
        if prefix == PREFIX_TXH:
            return self._op_txh(decrement, tag, address)
        if prefix == PREFIX_TXL:
            return self._op_txl(decrement, tag, address)
        raise RuntimeError(f"unreachable Type A prefix {prefix}")

    # ------------------------------------------------------------------
    # Helpers: index registers, effective address, memory access
    # ------------------------------------------------------------------

    def _index_combined(self, tag: int) -> int:
        """Return the bitwise OR of the index registers selected by ``tag``.

        Tag bit 0 (value 1) selects IRA, bit 1 (value 2) IRB, bit 2 (value 4)
        IRC. With multiple bits set the registers' contents are OR'd
        together — this matches the 1955 *Reference Manual* and is observable
        because the OR can produce a different value than the sum.
        """
        val = 0
        if tag & 1:
            val |= self._index_a
        if tag & 2:
            val |= self._index_b
        if tag & 4:
            val |= self._index_c
        return val

    def _effective_address(self, address: int, tag: int) -> int:
        """Compute the 15-bit effective address: ``(Y - C(T)) & 0x7FFF``."""
        return (address - self._index_combined(tag)) & ADDRESS_MASK

    def _index_register(self, tag: int) -> int:
        """Return the value of the *single* index register named by ``tag``.

        Used by LXA/SXA/PAX-style instructions where the tag picks exactly
        one register. If multiple bits are set we still OR them — matching
        the same register-combination rule as the address calculation.
        """
        return self._index_combined(tag)

    def _set_index_register(self, tag: int, value: int) -> None:
        """Set the index register(s) named by ``tag`` to a 15-bit value."""
        value &= ADDRESS_MASK
        if tag & 1:
            self._index_a = value
        if tag & 2:
            self._index_b = value
        if tag & 4:
            self._index_c = value

    def _check_address(self, address: int) -> None:
        if address >= self._memory_size:
            raise RuntimeError(
                f"address {address:#06x} outside memory "
                f"(size {self._memory_size})"
            )

    def _read_word(self, address: int) -> int:
        self._check_address(address)
        return self._memory[address]

    def _write_word(self, address: int, word: int) -> None:
        self._check_address(address)
        self._memory[address] = word & WORD_MASK

    def _ac_word(self) -> int:
        """Return the AC's bits S,1-35 packed as a 36-bit word.

        The Q and P overflow bits are *not* part of the stored word — they
        are part of the live AC only. STO, for instance, writes ``S | mag``
        and discards Q and P, per the *Reference Manual*.
        """
        return make_word(self._ac_sign, self._ac_magnitude)

    def _set_ac_from_word(self, word: int) -> None:
        """Load the AC's S, magnitude from a 36-bit word; clear Q and P."""
        self._ac_sign = word_sign(word)
        self._ac_magnitude = word_magnitude(word)
        self._ac_q = 0
        self._ac_p = 0

    def _record_overflow(self, overflow: bool) -> None:
        """Set the P bit and overflow trigger if an arithmetic op overflowed."""
        if overflow:
            self._ac_p = 1
            self._overflow_trigger = True

    def _advance_pc(self) -> None:
        self._pc = (self._pc + 1) & PC_MASK

    # ------------------------------------------------------------------
    # Type B handlers
    # ------------------------------------------------------------------
    # Each handler returns ``(mnemonic, description)`` for the StepTrace.
    # By convention the address operand is shown in IBM-style decimal even
    # though it is 15 bits — the simulator never disassembles a tag of 0
    # to keep listings concise.

    def _format_addr(self, address: int, tag: int) -> str:
        if tag:
            return f"{address},{tag}"
        return f"{address}"

    # ----- Halts -----

    def _op_htr(
        self, _opcode: int, tag: int, address: int
    ) -> tuple[str, str]:
        eff = self._effective_address(address, tag)
        self._pc = eff
        self._halted = True
        return f"HTR {self._format_addr(address, tag)}", (
            f"HTR @ pc={self._pc:#06x} — halted, PC = {eff}"
        )

    def _op_hpr(
        self, _opcode: int, tag: int, address: int
    ) -> tuple[str, str]:
        eff = self._effective_address(address, tag)
        self._pc = eff
        self._halted = True
        return f"HPR {self._format_addr(address, tag)}", (
            f"HPR @ pc={self._pc:#06x} — halt-and-proceed, PC = {eff}"
        )

    def _op_nop(
        self, _opcode: int, _tag: int, _address: int
    ) -> tuple[str, str]:
        self._advance_pc()
        return "NOP", "NOP — no operation"

    # ----- Loads / stores -----

    def _op_cla(
        self, _opcode: int, tag: int, address: int
    ) -> tuple[str, str]:
        eff = self._effective_address(address, tag)
        self._set_ac_from_word(self._read_word(eff))
        self._advance_pc()
        return f"CLA {self._format_addr(address, tag)}", (
            f"CLA Y={eff} → AC sign={self._ac_sign} "
            f"mag={self._ac_magnitude}"
        )

    def _op_cal(
        self, _opcode: int, tag: int, address: int
    ) -> tuple[str, str]:
        eff = self._effective_address(address, tag)
        word = self._read_word(eff)
        # CAL is "logical" — bit S of memory becomes magnitude bit, sign = 0.
        # In practical terms the AC ends up with sign=0 and the full 36-bit
        # word loaded into its magnitude (the high bit of mag is the old S).
        self._ac_sign = 0
        self._ac_magnitude = word & WORD_MASK
        # CAL can produce a magnitude wider than 35 bits (it's logical),
        # but our magnitude field is 35 bits — the high bit lands in Q
        # to preserve the value. This matches the manual.
        if self._ac_magnitude > MAGNITUDE_MASK:
            self._ac_q = 1
            self._ac_magnitude &= MAGNITUDE_MASK
        else:
            self._ac_q = 0
        self._ac_p = 0
        self._advance_pc()
        return f"CAL {self._format_addr(address, tag)}", (
            f"CAL Y={eff} → AC logical, mag={self._ac_magnitude}"
        )

    def _op_sto(
        self, _opcode: int, tag: int, address: int
    ) -> tuple[str, str]:
        eff = self._effective_address(address, tag)
        self._write_word(eff, self._ac_word())
        self._advance_pc()
        return f"STO {self._format_addr(address, tag)}", (
            f"STO Y={eff} ← AC ({self._ac_word():#012o})"
        )

    def _op_stz(
        self, _opcode: int, tag: int, address: int
    ) -> tuple[str, str]:
        eff = self._effective_address(address, tag)
        self._write_word(eff, 0)
        self._advance_pc()
        return f"STZ {self._format_addr(address, tag)}", (
            f"STZ Y={eff} ← +0"
        )

    def _op_stq(
        self, _opcode: int, tag: int, address: int
    ) -> tuple[str, str]:
        eff = self._effective_address(address, tag)
        self._write_word(eff, self._mq)
        self._advance_pc()
        return f"STQ {self._format_addr(address, tag)}", (
            f"STQ Y={eff} ← MQ ({self._mq:#012o})"
        )

    def _op_ldq(
        self, _opcode: int, tag: int, address: int
    ) -> tuple[str, str]:
        eff = self._effective_address(address, tag)
        self._mq = self._read_word(eff)
        self._advance_pc()
        return f"LDQ {self._format_addr(address, tag)}", (
            f"LDQ Y={eff} → MQ ({self._mq:#012o})"
        )

    def _op_xca(
        self, _opcode: int, _tag: int, _address: int
    ) -> tuple[str, str]:
        ac_word = self._ac_word()
        self._set_ac_from_word(self._mq)
        self._mq = ac_word
        self._advance_pc()
        return "XCA", "XCA — exchanged AC and MQ"

    # ----- Integer arithmetic -----

    def _op_add(
        self, _opcode: int, tag: int, address: int
    ) -> tuple[str, str]:
        eff = self._effective_address(address, tag)
        m = self._read_word(eff)
        sign, mag, overflow = add_sign_magnitude(
            self._ac_sign, self._ac_magnitude,
            word_sign(m), word_magnitude(m),
        )
        self._ac_sign = sign
        self._ac_magnitude = mag
        self._record_overflow(overflow)
        self._advance_pc()
        return f"ADD {self._format_addr(address, tag)}", (
            f"ADD Y={eff} → AC sign={sign} mag={mag} ovf={overflow}"
        )

    def _op_sub(
        self, _opcode: int, tag: int, address: int
    ) -> tuple[str, str]:
        eff = self._effective_address(address, tag)
        m = self._read_word(eff)
        # SUB = AC + (negated M). Negation is just sign-flip on 704.
        sign, mag, overflow = add_sign_magnitude(
            self._ac_sign, self._ac_magnitude,
            1 - word_sign(m), word_magnitude(m),
        )
        self._ac_sign = sign
        self._ac_magnitude = mag
        self._record_overflow(overflow)
        self._advance_pc()
        return f"SUB {self._format_addr(address, tag)}", (
            f"SUB Y={eff} → AC sign={sign} mag={mag} ovf={overflow}"
        )

    def _op_adm(
        self, _opcode: int, tag: int, address: int
    ) -> tuple[str, str]:
        eff = self._effective_address(address, tag)
        m = self._read_word(eff)
        # Add magnitude — treat M as positive regardless of its sign bit.
        sign, mag, overflow = add_sign_magnitude(
            self._ac_sign, self._ac_magnitude,
            0, word_magnitude(m),
        )
        self._ac_sign = sign
        self._ac_magnitude = mag
        self._record_overflow(overflow)
        self._advance_pc()
        return f"ADM {self._format_addr(address, tag)}", (
            f"ADM Y={eff} → AC sign={sign} mag={mag} ovf={overflow}"
        )

    def _op_mpy(
        self, _opcode: int, tag: int, address: int
    ) -> tuple[str, str]:
        eff = self._effective_address(address, tag)
        m = self._read_word(eff)
        a_mag = self._mq & MAGNITUDE_MASK
        b_mag = word_magnitude(m)
        a_sign = (self._mq >> 35) & 1
        b_sign = word_sign(m)
        product_mag = a_mag * b_mag
        result_sign = a_sign ^ b_sign
        # Distribute the 70-bit product: AC gets bits 35-69, MQ gets bits 0-34.
        ac_mag = (product_mag >> 35) & MAGNITUDE_MASK
        mq_mag = product_mag & MAGNITUDE_MASK
        if result_sign and product_mag == 0:
            result_sign = 0  # canonicalize -0 → +0
        self._ac_sign = result_sign
        self._ac_magnitude = ac_mag
        self._ac_p = 0
        self._ac_q = 0
        self._mq = make_word(result_sign, mq_mag)
        self._advance_pc()
        return f"MPY {self._format_addr(address, tag)}", (
            f"MPY Y={eff} → AC,MQ = {result_sign}|{ac_mag}|{mq_mag}"
        )

    def _op_dvp(
        self, _opcode: int, tag: int, address: int
    ) -> tuple[str, str]:
        return self._divide(tag, address, halt_on_check=False, mnem="DVP")

    def _op_dvh(
        self, _opcode: int, tag: int, address: int
    ) -> tuple[str, str]:
        return self._divide(tag, address, halt_on_check=True, mnem="DVH")

    def _divide(
        self, tag: int, address: int, *, halt_on_check: bool, mnem: str
    ) -> tuple[str, str]:
        eff = self._effective_address(address, tag)
        m = self._read_word(eff)
        divisor_mag = word_magnitude(m)
        divisor_sign = word_sign(m)
        # Dividend is the 70-bit (AC magnitude << 35) | MQ magnitude with
        # the sign of AC.
        dividend_mag = (self._ac_magnitude << 35) | (
            self._mq & MAGNITUDE_MASK
        )
        dividend_sign = self._ac_sign
        if divisor_mag == 0 or divisor_mag <= self._ac_magnitude:
            # Divide check — quotient won't fit in 35 bits, or division by zero.
            self._divide_check_trigger = True
            if halt_on_check:
                self._halted = True
                self._pc = eff  # halt at the offending instruction's target
                return f"{mnem} {self._format_addr(address, tag)}", (
                    f"{mnem} Y={eff} — divide-check halt"
                )
            self._advance_pc()
            return f"{mnem} {self._format_addr(address, tag)}", (
                f"{mnem} Y={eff} — divide-check (proceeding)"
            )
        quotient_mag, remainder_mag = divmod(dividend_mag, divisor_mag)
        # Quotient sign: XOR of AC and divisor signs. Remainder sign = AC sign.
        q_sign = dividend_sign ^ divisor_sign
        if quotient_mag == 0:
            q_sign = 0
        if remainder_mag == 0:
            r_sign = 0
        else:
            r_sign = dividend_sign
        self._mq = make_word(q_sign, quotient_mag & MAGNITUDE_MASK)
        self._ac_sign = r_sign
        self._ac_magnitude = remainder_mag & MAGNITUDE_MASK
        self._ac_q = 0
        self._ac_p = 0
        self._advance_pc()
        return f"{mnem} {self._format_addr(address, tag)}", (
            f"{mnem} Y={eff} → MQ=quot({q_sign}|{quotient_mag}) "
            f"AC=rem({r_sign}|{remainder_mag})"
        )

    # ----- Transfers -----

    def _op_tra(
        self, _opcode: int, tag: int, address: int
    ) -> tuple[str, str]:
        eff = self._effective_address(address, tag)
        self._pc = eff
        return f"TRA {self._format_addr(address, tag)}", (
            f"TRA → PC = {eff}"
        )

    def _branch_if(
        self,
        condition: bool,
        tag: int,
        address: int,
        mnem: str,
    ) -> tuple[str, str]:
        eff = self._effective_address(address, tag)
        if condition:
            self._pc = eff
            outcome = f"taken → PC = {eff}"
        else:
            self._advance_pc()
            outcome = "not taken"
        return f"{mnem} {self._format_addr(address, tag)}", (
            f"{mnem} Y={eff} — {outcome}"
        )

    def _op_tze(
        self, _opcode: int, tag: int, address: int
    ) -> tuple[str, str]:
        return self._branch_if(
            self._ac_magnitude == 0, tag, address, "TZE"
        )

    def _op_tnz(
        self, _opcode: int, tag: int, address: int
    ) -> tuple[str, str]:
        return self._branch_if(
            self._ac_magnitude != 0, tag, address, "TNZ"
        )

    def _op_tpl(
        self, _opcode: int, tag: int, address: int
    ) -> tuple[str, str]:
        return self._branch_if(self._ac_sign == 0, tag, address, "TPL")

    def _op_tmi(
        self, _opcode: int, tag: int, address: int
    ) -> tuple[str, str]:
        return self._branch_if(self._ac_sign == 1, tag, address, "TMI")

    def _op_tov(
        self, _opcode: int, tag: int, address: int
    ) -> tuple[str, str]:
        # TOV branches on overflow trigger, then *clears* it.
        triggered = self._overflow_trigger
        if triggered:
            self._overflow_trigger = False
        return self._branch_if(triggered, tag, address, "TOV")

    def _op_tno(
        self, _opcode: int, tag: int, address: int
    ) -> tuple[str, str]:
        # TNO branches when overflow trigger is *not* set; does not clear.
        return self._branch_if(
            not self._overflow_trigger, tag, address, "TNO"
        )

    def _op_tqo(
        self, _opcode: int, tag: int, address: int
    ) -> tuple[str, str]:
        triggered = self._mq_overflow
        if triggered:
            self._mq_overflow = False
        return self._branch_if(triggered, tag, address, "TQO")

    def _op_tqp(
        self, _opcode: int, tag: int, address: int
    ) -> tuple[str, str]:
        return self._branch_if(
            (self._mq & SIGN_BIT) == 0, tag, address, "TQP"
        )

    # ----- Index registers -----

    # NOTE on the index-register family (LXA/LXD/SXA/SXD/PAX/PDX/PXA):
    # For these instructions the tag is interpreted *only* as the register
    # selector — the address field is used directly, NOT subtracted by the
    # contents of the tagged index register. (This is what makes "store this
    # index register at this address" useful: otherwise the store target
    # would shift around as the register's value changed.)

    def _op_lxa(
        self, _opcode: int, tag: int, address: int
    ) -> tuple[str, str]:
        word = self._read_word(address)
        self._set_index_register(tag, word & ADDRESS_MASK)
        self._advance_pc()
        return f"LXA {self._format_addr(address, tag)}", (
            f"LXA Y={address},{tag} → IR{tag} = {word & ADDRESS_MASK}"
        )

    def _op_lxd(
        self, _opcode: int, tag: int, address: int
    ) -> tuple[str, str]:
        word = self._read_word(address)
        decrement = (word >> 18) & ADDRESS_MASK
        self._set_index_register(tag, decrement)
        self._advance_pc()
        return f"LXD {self._format_addr(address, tag)}", (
            f"LXD Y={address},{tag} → IR{tag} = {decrement}"
        )

    def _op_sxa(
        self, _opcode: int, tag: int, address: int
    ) -> tuple[str, str]:
        existing = self._read_word(address)
        new_word = (existing & ~ADDRESS_MASK) | self._index_register(tag)
        self._write_word(address, new_word)
        self._advance_pc()
        return f"SXA {self._format_addr(address, tag)}", (
            f"SXA Y={address},{tag} ← IR{tag} = {self._index_register(tag)}"
        )

    def _op_sxd(
        self, _opcode: int, tag: int, address: int
    ) -> tuple[str, str]:
        existing = self._read_word(address)
        decrement_field_mask = ADDRESS_MASK << 18
        new_word = (existing & ~decrement_field_mask) | (
            self._index_register(tag) << 18
        )
        self._write_word(address, new_word)
        self._advance_pc()
        return f"SXD {self._format_addr(address, tag)}", (
            f"SXD Y={address},{tag} ← IR{tag}"
        )

    def _op_pax(
        self, _opcode: int, tag: int, _address: int
    ) -> tuple[str, str]:
        # AC's address field is bits 0-14 of the AC word.
        ac_word = self._ac_word()
        self._set_index_register(tag, ac_word & ADDRESS_MASK)
        self._advance_pc()
        return f"PAX 0,{tag}", (
            f"PAX → IR{tag} = {ac_word & ADDRESS_MASK} "
            f"(from AC.address)"
        )

    def _op_pdx(
        self, _opcode: int, tag: int, _address: int
    ) -> tuple[str, str]:
        ac_word = self._ac_word()
        decrement = (ac_word >> 18) & ADDRESS_MASK
        self._set_index_register(tag, decrement)
        self._advance_pc()
        return f"PDX 0,{tag}", (
            f"PDX → IR{tag} = {decrement} (from AC.decrement)"
        )

    def _op_pxa(
        self, _opcode: int, tag: int, _address: int
    ) -> tuple[str, str]:
        value = self._index_register(tag)
        self._ac_sign = 0
        self._ac_magnitude = value
        self._ac_p = 0
        self._ac_q = 0
        self._advance_pc()
        return f"PXA 0,{tag}", (
            f"PXA → AC = +{value} (from IR{tag})"
        )

    # ----- Floating-point -----

    def _store_fp_result(self, value: float) -> None:
        """Store a Python-float result back into AC and MQ as 704 FP."""
        word = float_to_fp(value)
        self._set_ac_from_word(word)
        self._mq = word

    def _op_fad(
        self, _opcode: int, tag: int, address: int
    ) -> tuple[str, str]:
        eff = self._effective_address(address, tag)
        m = self._read_word(eff)
        result = fp_to_float(self._ac_word()) + fp_to_float(m)
        self._store_fp_result(result)
        self._advance_pc()
        return f"FAD {self._format_addr(address, tag)}", (
            f"FAD Y={eff} → AC = {result}"
        )

    def _op_fsb(
        self, _opcode: int, tag: int, address: int
    ) -> tuple[str, str]:
        eff = self._effective_address(address, tag)
        m = self._read_word(eff)
        result = fp_to_float(self._ac_word()) - fp_to_float(m)
        self._store_fp_result(result)
        self._advance_pc()
        return f"FSB {self._format_addr(address, tag)}", (
            f"FSB Y={eff} → AC = {result}"
        )

    def _op_fmp(
        self, _opcode: int, tag: int, address: int
    ) -> tuple[str, str]:
        eff = self._effective_address(address, tag)
        m = self._read_word(eff)
        result = fp_to_float(self._mq) * fp_to_float(m)
        self._store_fp_result(result)
        self._advance_pc()
        return f"FMP {self._format_addr(address, tag)}", (
            f"FMP Y={eff} → AC,MQ = {result}"
        )

    def _op_fdp(
        self, _opcode: int, tag: int, address: int
    ) -> tuple[str, str]:
        eff = self._effective_address(address, tag)
        m = self._read_word(eff)
        divisor = fp_to_float(m)
        if divisor == 0.0:
            self._divide_check_trigger = True
            self._advance_pc()
            return f"FDP {self._format_addr(address, tag)}", (
                f"FDP Y={eff} — divide-check (divisor = 0)"
            )
        dividend = fp_to_float(self._ac_word())
        quotient = dividend / divisor
        remainder = dividend - quotient * divisor
        self._mq = float_to_fp(quotient)
        self._set_ac_from_word(float_to_fp(remainder))
        self._advance_pc()
        return f"FDP {self._format_addr(address, tag)}", (
            f"FDP Y={eff} → MQ = {quotient}, AC = {remainder}"
        )

    # ------------------------------------------------------------------
    # Type A handlers
    # ------------------------------------------------------------------

    def _op_txi(
        self, decrement: int, tag: int, address: int
    ) -> tuple[str, str]:
        # IR(T) = IR(T) + D, then PC = address. Always transfers.
        new_value = (self._index_register(tag) + decrement) & ADDRESS_MASK
        self._set_index_register(tag, new_value)
        self._pc = address
        return f"TXI {address},{tag},{decrement}", (
            f"TXI IR{tag} += {decrement} → {new_value}; PC = {address}"
        )

    def _op_tix(
        self, decrement: int, tag: int, address: int
    ) -> tuple[str, str]:
        # If IR(T) > D, IR(T) -= D and transfer; else fall through.
        ir = self._index_register(tag)
        if ir > decrement:
            new_value = (ir - decrement) & ADDRESS_MASK
            self._set_index_register(tag, new_value)
            self._pc = address
            return f"TIX {address},{tag},{decrement}", (
                f"TIX IR{tag}={ir} > {decrement}: -= {decrement} "
                f"→ {new_value}; PC = {address}"
            )
        self._advance_pc()
        return f"TIX {address},{tag},{decrement}", (
            f"TIX IR{tag}={ir} <= {decrement}: fall through"
        )

    def _op_txh(
        self, decrement: int, tag: int, address: int
    ) -> tuple[str, str]:
        # If IR(T) > D, transfer (no decrement of IR).
        ir = self._index_register(tag)
        if ir > decrement:
            self._pc = address
            return f"TXH {address},{tag},{decrement}", (
                f"TXH IR{tag}={ir} > {decrement}: PC = {address}"
            )
        self._advance_pc()
        return f"TXH {address},{tag},{decrement}", (
            f"TXH IR{tag}={ir} <= {decrement}: fall through"
        )

    def _op_txl(
        self, decrement: int, tag: int, address: int
    ) -> tuple[str, str]:
        # If IR(T) <= D, transfer.
        ir = self._index_register(tag)
        if ir <= decrement:
            self._pc = address
            return f"TXL {address},{tag},{decrement}", (
                f"TXL IR{tag}={ir} <= {decrement}: PC = {address}"
            )
        self._advance_pc()
        return f"TXL {address},{tag},{decrement}", (
            f"TXL IR{tag}={ir} > {decrement}: fall through"
        )


# Module-level conveniences --------------------------------------------------

__all__ = [
    "IBM704Simulator",
    "MEMORY_WORDS",
    "OP_HTR",
    "OP_HPR",
    "OP_NOP",
    "OP_CLA",
    "OP_CAL",
    "OP_ADD",
    "OP_SUB",
    "OP_ADM",
    "OP_STO",
    "OP_STZ",
    "OP_STQ",
    "OP_LDQ",
    "OP_XCA",
    "OP_MPY",
    "OP_DVP",
    "OP_DVH",
    "OP_TRA",
    "OP_TZE",
    "OP_TNZ",
    "OP_TPL",
    "OP_TMI",
    "OP_TOV",
    "OP_TNO",
    "OP_TQO",
    "OP_TQP",
    "OP_LXA",
    "OP_LXD",
    "OP_SXA",
    "OP_SXD",
    "OP_PAX",
    "OP_PDX",
    "OP_PXA",
    "OP_FAD",
    "OP_FSB",
    "OP_FMP",
    "OP_FDP",
    "PREFIX_TXI",
    "PREFIX_TIX",
    "PREFIX_TXH",
    "PREFIX_TXL",
    "encode_type_b",
    "encode_type_a",
]


def encode_type_b(opcode: int, tag: int = 0, address: int = 0) -> int:
    """Assemble a Type B instruction word from its three fields.

    ``opcode`` is the 12-bit value (bits 24-35). ``tag`` is 3 bits.
    ``address`` is 15 bits.

    Examples
    --------
    >>> from ibm704_simulator.simulator import encode_type_b, OP_CLA
    >>> hex(encode_type_b(OP_CLA, 0, 100))
    '0x140000064'
    """
    if not 0 <= opcode <= 0xFFF:
        raise ValueError(f"opcode must fit in 12 bits, got {opcode}")
    if not 0 <= tag <= 7:
        raise ValueError(f"tag must fit in 3 bits, got {tag}")
    if not 0 <= address <= ADDRESS_MASK:
        raise ValueError(f"address must fit in 15 bits, got {address}")
    return (opcode << 24) | (tag << 15) | address


def encode_type_a(
    prefix: int, decrement: int = 0, tag: int = 0, address: int = 0
) -> int:
    """Assemble a Type A instruction word (TXI/TIX/TXH/TXL).

    ``prefix`` must be one of ``PREFIX_TXI`` (1), ``PREFIX_TIX`` (2),
    ``PREFIX_TXH`` (3), or ``PREFIX_TXL`` (5).
    """
    if prefix not in TYPE_A_PREFIXES:
        raise ValueError(
            f"prefix must be one of {sorted(TYPE_A_PREFIXES)}, got {prefix}"
        )
    if not 0 <= decrement <= ADDRESS_MASK:
        raise ValueError(
            f"decrement must fit in 15 bits, got {decrement}"
        )
    if not 0 <= tag <= 7:
        raise ValueError(f"tag must fit in 3 bits, got {tag}")
    if not 0 <= address <= ADDRESS_MASK:
        raise ValueError(f"address must fit in 15 bits, got {address}")
    return (prefix << 33) | (decrement << 18) | (tag << 15) | address


# Re-export the byte-transport helpers and FP_CHAR_MASK for tests/assemblers.
__all__.extend([
    "WORD_BYTES",
    "pack_word",
    "FP_CHAR_MASK",
])
