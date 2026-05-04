"""Manchester Baby (SSEM, 1948) behavioral simulator.

──────────────────────────────────────────────────────────────────────────────
BACKGROUND
──────────────────────────────────────────────────────────────────────────────

On 21 June 1948, a 32-word program stored on a Williams tube CRT ran to
completion at the University of Manchester.  It was the world's first
*stored-program* execution.  The program, written by Tom Kilburn, found the
highest proper divisor of 2¹⁸ = 262 144 by testing every potential factor via
repeated subtraction.  It ran for 52 minutes and executed 3.5 million
operations on a machine with just 7 instructions and 32 words of memory.

This simulator faithfully reproduces that machine's behavior.

──────────────────────────────────────────────────────────────────────────────
INSTRUCTION ENCODING
──────────────────────────────────────────────────────────────────────────────

Every instruction is a 32-bit word.  Only two fields matter:

    Bit:   31  …  16  15  14  13  12  …  5   4   3   2   1   0
                       F2  F1  F0             S4  S3  S2  S1  S0

    S (bits 0–4):   The operand — which line of the store to use.
    F (bits 13–15): The function code — which instruction to execute.
    All other bits: Ignored (conventionally zero).

Extraction:
    s = word & 0x1F           # 5-bit operand
    f = (word >> 13) & 0x7    # 3-bit function code

Function codes:
    000  JMP S  — CI ← Store[S]              (absolute jump)
    001  JRP S  — CI ← CI + Store[S]         (relative jump)
    010  LDN S  — A ← −Store[S]             (load negated)
    011  STO S  — Store[S] ← A              (store)
    100  SUB S  — A ← A − Store[S]          (subtract)
    101  SUB S  — same as 100 (alternate encoding)
    110  CMP    — if A < 0: CI ← CI + 1     (conditional skip)
    111  STP    — halt execution

──────────────────────────────────────────────────────────────────────────────
THE FETCH-DECODE-EXECUTE CYCLE
──────────────────────────────────────────────────────────────────────────────

The Baby's hardware cycle is slightly unusual: it *pre-increments* CI before
fetching.  CI is initialised to 31 (= −1 mod 32) at reset.

    Step 1:  CI ← (CI + 1) mod 32     — increment BEFORE fetch
    Step 2:  PI ← Store[CI]            — fetch the instruction
    Step 3:  Decode: S = PI[0:4], F = PI[13:15]
    Step 4:  Execute instruction

Why does this matter?

  • JMP S:  Sets CI ← Store[S].  The *next* fetch will be from
            (Store[S] + 1) mod 32, so to jump to line N you must store N−1.
            Real Baby programmers often had to remember this −1 offset!

  • JRP S:  Sets CI ← CI + Store[S].  CI here is already the incremented
            value.  The next fetch is CI_current + Store[S] + 1.

  • CMP:    If A < 0, increments CI by an *extra* 1 after the normal
            increment.  The skip advances from "next line" to "line after
            next" — effectively skipping the next instruction entirely.

──────────────────────────────────────────────────────────────────────────────
SIGNED ARITHMETIC
──────────────────────────────────────────────────────────────────────────────

All arithmetic is 32-bit two's complement with *silent* overflow (no flags,
no exceptions — overflow just wraps modulo 2³²).

    LDN S:  A ← (−Store[S]) & 0xFFFFFFFF
    SUB S:  A ← (A − Store[S]) & 0xFFFFFFFF

The "negative" test in CMP checks bit 31:
    A is negative ⟺ A & 0x80000000 ≠ 0  ⟺ A ≥ 0x80000000

──────────────────────────────────────────────────────────────────────────────
NO I/O
──────────────────────────────────────────────────────────────────────────────

The SSEM had *no* I/O instructions whatsoever.  The programmer read output
by physically examining the Williams tube phosphorescence under UV light.
This simulator provides no set_input_port / get_output_port methods.

──────────────────────────────────────────────────────────────────────────────
"""

from __future__ import annotations

from simulator_protocol import ExecutionResult, StepTrace

from manchester_baby_simulator.state import BabyState

# ── Hardware constants ────────────────────────────────────────────────────────
_STORE_SIZE = 32          # 32 lines of Williams-tube store
_WORD_MASK = 0xFFFFFFFF   # 32-bit unsigned mask — applied after arithmetic
_CI_MASK = 0x1F           # CI is 5 bits (0–31)
_CI_INIT = 0x1F           # CI starts at 31 so first increment → 0

# ── Function codes (F field, bits 13–15 of each instruction word) ─────────────
#
# The original SSEM documentation used decimal line numbers and wrote function
# codes in binary.  We name them here as Python constants for readability.
#
_F_JMP = 0b000   # CI ← Store[S]             absolute jump
_F_JRP = 0b001   # CI ← CI + Store[S]        relative jump
_F_LDN = 0b010   # A ← −Store[S]            load negated
_F_STO = 0b011   # Store[S] ← A             store
_F_SUB = 0b100   # A ← A − Store[S]         subtract  (primary encoding)
_F_SUB2 = 0b101  # A ← A − Store[S]         subtract  (alternate encoding)
_F_CMP = 0b110   # if A < 0: CI += 1         conditional skip
_F_STP = 0b111   # halt


class BabySimulator:
    """Behavioral simulator for the Manchester Baby (SSEM, 1948).

    Implements the ``Simulator[BabyState]`` protocol from
    ``simulator_protocol`` (SIM00).

    Usage
    -----
    The simplest workflow is ``execute()``:

        sim = BabySimulator()
        result = sim.execute(program_bytes)
        if result.ok:
            print(result.final_state.store[1])

    For step-by-step debugging use ``load()`` + ``step()`` in a loop:

        sim = BabySimulator()
        sim.reset()
        sim.load(program_bytes)
        while not sim._halted:
            trace = sim.step()
            print(trace.mnemonic, "→", trace.description)

    Memory layout
    -------------
    ``load(program, origin=0)`` interprets ``program`` as consecutive 4-byte
    little-endian words and writes them to the store starting at word
    ``origin``.  A complete 32-word image is 128 bytes.

    The ``origin`` parameter is in *word* units, not bytes.

    Reset state
    -----------
    After ``reset()``:
      - All 32 store words are 0.
      - Accumulator A = 0.
      - CI = 31 (so first increment → 0, first instruction is line 0).
      - halted = False.
    """

    def __init__(self) -> None:
        self._store: list[int] = [0] * _STORE_SIZE   # 32 unsigned 32-bit words
        self._acc: int = 0                            # accumulator (unsigned)
        self._ci: int = _CI_INIT                     # control instruction (5-bit)
        self._halted: bool = False

    # ------------------------------------------------------------------
    # SIM00 protocol — public methods
    # ------------------------------------------------------------------

    def reset(self) -> None:
        """Reset the simulator to its power-on state.

        Post-conditions:
          - All store words are 0.
          - A = 0.
          - CI = 31 (first step() will increment to 0).
          - halted = False.
        """
        self._store = [0] * _STORE_SIZE
        self._acc = 0
        self._ci = _CI_INIT
        self._halted = False

    def load(self, program: bytes, origin: int = 0) -> None:
        """Decode ``program`` bytes as 32-bit little-endian words into the store.

        Parameters
        ----------
        program :
            Raw bytes.  Must be a multiple of 4 in length (each word is
            exactly 4 bytes).  Trailing incomplete words are silently ignored.
        origin :
            Starting store line (word index, 0–31) where the first word will
            be written.  Defaults to 0.

        The conversion is simple little-endian word assembly:

            word_N = (byte[4N+0])
                   | (byte[4N+1] << 8)
                   | (byte[4N+2] << 16)
                   | (byte[4N+3] << 24)

        Writing beyond line 31 (origin + word count > 32) silently stops at
        line 31, matching hardware behaviour (the store wraps, but we just
        stop to avoid index errors).

        Examples
        --------
        Load the literal value 42 into line 0:

            sim.load(b'\\x2a\\x00\\x00\\x00')    # 0x0000002A = 42
            assert sim._store[0] == 42

        Load an LDN 0 instruction (F=010, S=0) at line 2:

            LDN_0 = (0b010 << 13)               # = 0x00004000
            sim.load(LDN_0.to_bytes(4, 'little'), origin=2)
        """
        n_words = len(program) // 4
        for i in range(n_words):
            word_idx = origin + i
            if word_idx >= _STORE_SIZE:
                break
            b0 = program[4 * i]
            b1 = program[4 * i + 1]
            b2 = program[4 * i + 2]
            b3 = program[4 * i + 3]
            self._store[word_idx] = b0 | (b1 << 8) | (b2 << 16) | (b3 << 24)

    def step(self) -> StepTrace:
        """Execute one full fetch-decode-execute cycle.

        The Baby's cycle always *pre-increments* CI before fetching:

            1.  CI ← (CI + 1) mod 32     — increment FIRST
            2.  PI ← Store[CI]            — fetch instruction
            3.  Decode S and F from PI
            4.  Execute instruction

        Returns
        -------
        StepTrace :
            - ``pc_before``: CI value *before* the increment (previous instr)
            - ``pc_after``:  CI value *after* execution (next instr to run
                             will be at (pc_after + 1) mod 32)
            - ``mnemonic``:  e.g. ``"LDN 5"``, ``"SUB 3"``, ``"STP"``
            - ``description``: e.g. ``"LDN 5 @ line 2"``

        Raises
        ------
        RuntimeError :
            If the simulator is already halted.
        """
        if self._halted:
            raise RuntimeError("BabySimulator is halted; call reset() to restart")

        # ── Step 1: pre-increment CI ──────────────────────────────────────
        ci_before = self._ci
        self._ci = (self._ci + 1) & _CI_MASK

        # ── Step 2: fetch instruction from store ──────────────────────────
        word = self._store[self._ci]

        # ── Step 3: decode S and F ────────────────────────────────────────
        s = word & 0x1F           # bits 0–4: operand (store line)
        f = (word >> 13) & 0x7   # bits 13–15: function code

        # ── Step 4: execute ───────────────────────────────────────────────
        mnemonic = self._execute(f, s)

        ci_after = self._ci
        description = f"{mnemonic} @ line {self._ci}"

        return StepTrace(
            pc_before=ci_before,
            pc_after=ci_after,
            mnemonic=mnemonic,
            description=description,
        )

    def execute(
        self, program: bytes, max_steps: int = 10_000
    ) -> ExecutionResult[BabyState]:
        """Load ``program``, run to STP or ``max_steps``, return full result.

        This is the primary entry point for end-to-end testing.

        Parameters
        ----------
        program :
            Raw machine-code bytes (little-endian 32-bit words).
        max_steps :
            Safety ceiling.  The Baby's famous first program used 3.5 million
            steps on real hardware; the default 10 000 is fine for unit tests.

        Returns
        -------
        ExecutionResult[BabyState] :
            - ``halted``: True if STP was reached, False if max_steps hit.
            - ``steps``:  Number of instructions executed.
            - ``final_state``: Frozen BabyState snapshot.
            - ``error``:  None on clean halt; error string otherwise.
            - ``traces``: Per-instruction StepTrace list.

        The program is loaded at origin 0 (the conventional start).
        """
        self.reset()
        self.load(program)

        traces: list[StepTrace] = []
        steps = 0

        while not self._halted and steps < max_steps:
            trace = self.step()
            traces.append(trace)
            steps += 1

        error = None if self._halted else f"max_steps ({max_steps}) exceeded"

        return ExecutionResult(
            halted=self._halted,
            steps=steps,
            final_state=self.get_state(),
            error=error,
            traces=traces,
        )

    def get_state(self) -> BabyState:
        """Return an immutable snapshot of the current machine state.

        The returned ``BabyState`` is frozen — the simulator can continue
        executing without affecting previously captured snapshots.  The store
        is converted from a mutable list to an immutable tuple.
        """
        return BabyState(
            store=tuple(self._store),
            accumulator=self._acc,
            ci=self._ci,
            halted=self._halted,
        )

    # ------------------------------------------------------------------
    # Internal execution helpers
    # ------------------------------------------------------------------

    def _execute(self, f: int, s: int) -> str:
        """Execute the decoded instruction and return its mnemonic string.

        Parameters
        ----------
        f : int
            3-bit function code (0–7).
        s : int
            5-bit operand / line number (0–31).

        Returns
        -------
        str
            Human-readable mnemonic such as ``"LDN 5"`` or ``"STP"``.

        Notes on each instruction
        -------------------------

        **JMP S** (F=000):
            CI ← Store[S]
            Absolute jump.  Because CI is pre-incremented *before* fetch,
            the next instruction executed is at (Store[S] + 1) mod 32.
            To jump to line N, store N−1 at address S.

        **JRP S** (F=001):
            CI ← CI + Store[S]
            Relative jump.  CI here is *already incremented* (the current
            line number).  The store value is treated as a signed displacement.
            Next fetch: (CI + Store[S] + 1) mod 32.

            The displacement is interpreted as a *signed* 32-bit value so that
            negative displacements (backwards jumps) work correctly.

        **LDN S** (F=010):
            A ← (−Store[S]) & 0xFFFFFFFF
            Load Negative.  The SSEM has no ADD instruction, only SUB.  To
            add a value X, load its negative with LDN then subtract with SUB:
                A ← A − (−X) = A + X

        **STO S** (F=011):
            Store[S] ← A
            Store the accumulator's current value into line S.

        **SUB S** (F=100 or F=101):
            A ← (A − Store[S]) & 0xFFFFFFFF
            Subtract.  Both encodings 100 and 101 perform the same operation.
            All arithmetic is mod 2³² (silent overflow).

        **CMP** (F=110):
            if A < 0: CI ← CI + 1
            Compare / conditional skip.  "Negative" means bit 31 is set
            (A ≥ 0x80000000 in unsigned terms).  The extra +1 comes *after*
            the normal pre-increment, so the total skip is 2 from the last
            instruction — the next instruction is skipped entirely.

        **STP** (F=111):
            halted ← True
            Stop.  The machine halts immediately; no more instructions execute.
        """
        if f == _F_JMP:
            # Absolute jump: CI ← Store[S]
            # The pre-increment in step() will add 1 more, so next fetch is
            # (Store[S] + 1) mod 32.
            self._ci = self._store[s] & _CI_MASK
            return f"JMP {s}"

        if f == _F_JRP:
            # Relative jump: CI ← CI + Store[S]
            # The store displacement is treated as signed 32-bit for backwards
            # jumps, then masked to 5 bits.
            displacement = self._store[s]
            if displacement >= 0x80000000:
                displacement -= 0x100000000   # interpret as signed
            self._ci = (self._ci + displacement) & _CI_MASK
            return f"JRP {s}"

        if f == _F_LDN:
            # Load Negative: A ← (−Store[S]) mod 2³²
            self._acc = (-self._store[s]) & _WORD_MASK
            return f"LDN {s}"

        if f == _F_STO:
            # Store: Store[S] ← A
            self._store[s] = self._acc
            return f"STO {s}"

        if f in (_F_SUB, _F_SUB2):
            # Subtract: A ← (A − Store[S]) mod 2³²
            self._acc = (self._acc - self._store[s]) & _WORD_MASK
            return f"SUB {s}"

        if f == _F_CMP:
            # Conditional skip: if bit 31 of A is set (A is negative), skip
            # the next instruction by adding 1 more to CI.
            if self._acc & 0x80000000:
                self._ci = (self._ci + 1) & _CI_MASK
            return "CMP"

        # f == _F_STP
        self._halted = True
        return "STP"
