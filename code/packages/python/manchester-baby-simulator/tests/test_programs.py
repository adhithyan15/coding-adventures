"""Test suite: multi-instruction programs for the Manchester Baby.

Tests complete programs that exercise several instructions working together:
  - Negate a value (LDN + STO)
  - Absolute value (LDN + CMP + SUB)
  - Compute sum using LDN/SUB idiom (the Baby has no ADD instruction)
  - Countdown loop (LDN + SUB + CMP + JRP)
  - Kilburn-style modulo: repeated subtraction to find N mod D
  - BabyState helper properties (acc_signed, present_instruction)

LAYOUT CONVENTION: instructions at low lines (0, 1, 2, ...), data at
high lines (25–31).  A data word like 0xFFFFFFFB (= −5) has bits 13–15 = 111
which decodes as STP if executed; keeping it at a high line prevents
accidental execution.
"""

from __future__ import annotations

from manchester_baby_simulator import BabySimulator, BabyState

# ── Helpers ───────────────────────────────────────────────────────────────────

def w(value: int) -> bytes:
    """Encode a 32-bit word as 4 little-endian bytes."""
    return (value & 0xFFFFFFFF).to_bytes(4, "little")


def build(store: list[int]) -> bytes:
    """Pad store to 32 words and encode as 128 bytes."""
    padded = (store + [0] * 32)[:32]
    return b"".join(w(v) for v in padded)


def jmp(s: int) -> int:
    return (0b000 << 13) | (s & 0x1F)


def jrp(s: int) -> int:
    return (0b001 << 13) | (s & 0x1F)


def ldn(s: int) -> int:
    return (0b010 << 13) | (s & 0x1F)


def sto(s: int) -> int:
    return (0b011 << 13) | (s & 0x1F)


def sub(s: int) -> int:
    return (0b100 << 13) | (s & 0x1F)


def cmp_i() -> int:
    return 0b110 << 13


STP = 0b111 << 13


# ── Program 1: Negate ─────────────────────────────────────────────────────────

class TestNegateProgram:
    """Negate the value stored at line 28 and store at line 29.

    Program (instructions at lines 0-2, data at lines 28-29):
        line 28: data = X
        line 0:  LDN 28  → A = −X
        line 1:  STO 29  → Store[29] = A = −X
        line 2:  STP
    """

    def test_negate_42(self):
        store = [0] * 32
        store[28] = 42
        store[0] = ldn(28)   # A = -42
        store[1] = sto(29)   # Store[29] = -42
        store[2] = STP
        sim = BabySimulator()
        result = sim.execute(build(store))
        assert result.ok
        assert result.final_state.acc_signed == -42
        assert result.final_state.store[29] == ((-42) & 0xFFFFFFFF)

    def test_negate_zero(self):
        store = [0] * 32
        store[28] = 0
        store[0] = ldn(28)
        store[1] = sto(29)
        store[2] = STP
        sim = BabySimulator()
        result = sim.execute(build(store))
        assert result.ok
        assert result.final_state.store[29] == 0

    def test_negate_one(self):
        store = [0] * 32
        store[28] = 1
        store[0] = ldn(28)
        store[1] = sto(29)
        store[2] = STP
        sim = BabySimulator()
        result = sim.execute(build(store))
        assert result.ok
        assert result.final_state.acc_signed == -1

    def test_negate_negative_value(self):
        # Negating a negative gives positive: −(−5) = 5
        store = [0] * 32
        store[28] = 0xFFFFFFFB   # −5 (data; placed at line 28 not line 0)
        store[0] = ldn(28)       # A = 5
        store[1] = sto(29)
        store[2] = STP
        sim = BabySimulator()
        result = sim.execute(build(store))
        assert result.ok
        assert result.final_state.store[29] == 5

    def test_steps_count(self):
        # 3 instructions: LDN, STO, STP
        store = [0] * 32
        store[28] = 42
        store[0] = ldn(28)
        store[1] = sto(29)
        store[2] = STP
        sim = BabySimulator()
        result = sim.execute(build(store))
        assert result.steps == 3   # LDN, STO, STP


# ── Program 2: Absolute value ─────────────────────────────────────────────────

class TestAbsoluteValue:
    """Compute |X| using the LDN idiom.

    abs(positive X): X is already positive; LDN of -X at line 28 gives X.
    abs(negative X): LDN of X (which stores unsigned two's-complement) gives -X = |X|.
    """

    def test_abs_of_positive_via_cmp_skip(self):
        # X = 5 (positive).  For a program that checks sign and branches,
        # use CMP to detect negativity and skip the undo step.
        # Layout:
        #   line 28: 5     data: X=5
        #   line 0:  LDN 28  A = −5 (negative)
        #   line 1:  STO 29  temp[29] = −5
        #   line 2:  LDN 29  A = −(−5) = 5  (double negation = absolute value)
        #   line 3:  STO 30  result[30] = 5
        #   line 4:  STP
        store = [0] * 32
        store[28] = 5
        store[0] = ldn(28)    # A = -5
        store[1] = sto(29)    # temp = -5
        store[2] = ldn(29)    # A = -(-5) = +5
        store[3] = sto(30)    # result = 5
        store[4] = STP

        sim = BabySimulator()
        result = sim.execute(build(store))
        assert result.ok
        assert result.final_state.store[30] == 5

    def test_abs_of_negative_via_ldn(self):
        # abs(−7) = 7.  Store −7 at line 28; LDN 28 → A = −(−7) = 7.
        store = [0] * 32
        store[28] = 0xFFFFFFF9   # −7 (data; would be STP at line 0)
        store[0] = ldn(28)       # A = 7
        store[1] = sto(29)
        store[2] = STP

        sim = BabySimulator()
        result = sim.execute(build(store))
        assert result.ok
        assert result.final_state.store[29] == 7


# ── Program 3: Sum via LDN/SUB idiom ─────────────────────────────────────────

class TestSumProgram:
    """Compute X + Y using LDN/SUB (the Baby has no ADD instruction).

    To compute X + Y:
      1. Store −Y at line 28 (unsigned: if Y=5, store 0xFFFFFFFB).
      2. LDN 28 → A = −(−Y) = Y.
      3. Store −X at line 29 (unsigned: if X=3, store 0xFFFFFFFD).
      4. SUB 29 → A = Y − (−X) = Y + X. ✓

    Data placed at lines 28-29 to avoid accidental instruction execution.
    """

    def test_add_3_plus_5(self):
        # Store[28] = −3; LDN 28 → A=3; Store[29] = −5; SUB 29 → A=3−(−5)=8.
        store = [0] * 32
        store[28] = 0xFFFFFFFD   # −3
        store[29] = 0xFFFFFFFB   # −5
        store[0] = ldn(28)       # A = 3
        store[1] = sub(29)       # A = 3 − (−5) = 8
        store[2] = STP
        sim = BabySimulator()
        result = sim.execute(build(store))
        assert result.ok
        assert result.final_state.acc_signed == 8

    def test_add_zero_to_value(self):
        # A = 50 + 0 = 50
        store = [0] * 32
        store[28] = 0xFFFFFFCE   # −50 (data; STP if at line 0)
        store[29] = 0            # 0
        store[0] = ldn(28)       # A = 50
        store[1] = sub(29)       # A = 50 − 0 = 50
        store[2] = STP
        sim = BabySimulator()
        result = sim.execute(build(store))
        assert result.ok
        assert result.final_state.acc_signed == 50

    def test_add_negative_numbers(self):
        # (−3) + (−5) = −8.
        # LDN 28 (A=−3) then SUB 29 (subtract 5) → A = −3 − 5 = −8.
        store = [0] * 32
        store[28] = 3    # LDN 28 → A = −3
        store[29] = 5    # SUB 29 → A = −3 − 5 = −8
        store[0] = ldn(28)
        store[1] = sub(29)
        store[2] = STP
        sim = BabySimulator()
        result = sim.execute(build(store))
        assert result.ok
        assert result.final_state.acc_signed == -8


# ── Program 4: Countdown loop ─────────────────────────────────────────────────

class TestCountdownLoop:
    """Count down from N to 0 using SUB + CMP loop.

    Algorithm:
        A ← −N           (LDN from data slot at line 28)
        loop:
          A ← A − (−1)   (SUB line 29, which holds −1; subtracting −1 adds 1)
          CMP             (if A < 0: skip STP → continue loop)
          STP             (A ≥ 0 means counter reached zero)
          JRP             (loop back to SUB)

    Layout (instructions at lines 0-4, data at lines 28-30):
        line 28: N (positive)
        line 29: 0xFFFFFFFF (= −1; SUB 29 adds 1 to A)
        line 30: displacement for JRP (−4 = 0xFFFFFFFC)
        line 0:  LDN 28    A ← −N
        line 1:  SUB 29    A ← A + 1   (loop body)
        line 2:  CMP       if A<0: skip STP → continue
        line 3:  STP       halt when A≥0
        line 4:  JRP 30    CI ← 4 + (−4) = 0 → next CI = 1 (loop body)

    After N iterations: A increments from −N to 0. CMP does not skip; STP halts.
    """

    def _countdown_program(self, n: int) -> bytes:
        """Build a countdown-from-N program with data at high lines."""
        store = [0] * 32
        store[28] = n           # N (positive)
        store[29] = 0xFFFFFFFF  # −1 (SUB 29 adds 1)
        store[30] = 0xFFFFFFFC  # displacement −4 for JRP
        store[0] = ldn(28)      # A ← −N
        store[1] = sub(29)      # A ← A − (−1) = A + 1  (loop body)
        store[2] = cmp_i()      # if A<0: skip STP → continue
        store[3] = STP          # halt when A≥0
        store[4] = jrp(30)      # CI ← 4 + (−4) = 0 → next CI = 1
        return build(store)

    def test_countdown_3(self):
        # −3, −2, −1, 0 → 3 iterations before STP
        prog = self._countdown_program(3)
        sim = BabySimulator()
        result = sim.execute(prog, max_steps=200)
        assert result.ok
        assert result.final_state.accumulator == 0

    def test_countdown_5(self):
        prog = self._countdown_program(5)
        sim = BabySimulator()
        result = sim.execute(prog, max_steps=200)
        assert result.ok
        assert result.final_state.accumulator == 0

    def test_countdown_1(self):
        # Single iteration: A = −1 → +0 → STP
        prog = self._countdown_program(1)
        sim = BabySimulator()
        result = sim.execute(prog, max_steps=50)
        assert result.ok
        assert result.final_state.accumulator == 0

    def test_countdown_terminates(self):
        # Verify the loop doesn't run forever for N=10
        prog = self._countdown_program(10)
        sim = BabySimulator()
        result = sim.execute(prog, max_steps=500)
        assert result.ok
        assert result.halted is True


# ── Program 5: Kilburn-style N mod D via repeated subtraction ─────────────────

class TestDivisorSearch:
    """Compute N mod D using the subtraction loop technique.

    This is the same approach used in Kilburn's 1948 first program, which
    found the highest proper divisor of 2¹⁸ by repeated subtraction.

    Algorithm:
        A ← N                (via LDN of −N stored at line 25)
        loop:
          A ← A − D          (SUB 26, D stored at line 26)
          CMP                 (if A<0: skip JRP → exit loop)
          JRP 31              (loop back to SUB; displacement at line 31)
        A ← A + D            (SUB 27, where Store[27]=−D; undoes overshoot)
        STP                   (A = N mod D)

    Layout:
        line 25: −N (LDN 25 gives N)
        line 26: D
        line 27: −D (SUB 27 adds D back, undoing one over-subtraction)
        line 31: displacement −3 (JRP: CI=3 → CI+disp=0 → next CI=1)
        line 0:  LDN 25    A ← N
        line 1:  SUB 26    A ← A − D       (loop body)
        line 2:  CMP       if A<0: skip JRP (exit)
        line 3:  JRP 31    loop back to line 1
        line 4:  SUB 27    A ← A + D       (undo overshoot)
        line 5:  STP

    Example — 7 mod 3:
        A=7, SUB 3: A=4 (≥0 → JRP), SUB 3: A=1 (≥0 → JRP),
        SUB 3: A=−2 (<0 → skip → SUB −3 → A=1), STP. A=1. ✓
    """

    def _remainder_program(self, n: int, d: int) -> bytes:
        """Build a program that computes N mod D and leaves result in A."""
        neg_n = (-n) & 0xFFFFFFFF
        neg_d = (-d) & 0xFFFFFFFF

        store = [0] * 32
        store[25] = neg_n       # −N (LDN 25 → A = N)
        store[26] = d           # D
        store[27] = neg_d       # −D (SUB 27 → A += D)
        store[31] = 0xFFFFFFFD  # displacement −3 (JRP at CI=3: CI←3+(−3)=0→next=1)

        store[0] = ldn(25)      # A ← N
        store[1] = sub(26)      # A ← A − D         (loop body)
        store[2] = cmp_i()      # if A<0: skip JRP
        store[3] = jrp(31)      # CI ← 3 + (−3) = 0 → next CI = 1
        store[4] = sub(27)      # A ← A + D         (undo overshoot)
        store[5] = STP

        return build(store)

    def test_12_mod_4_is_zero(self):
        prog = self._remainder_program(12, 4)
        sim = BabySimulator()
        result = sim.execute(prog, max_steps=500)
        assert result.ok
        assert result.final_state.accumulator == 0

    def test_12_mod_3_is_zero(self):
        prog = self._remainder_program(12, 3)
        sim = BabySimulator()
        result = sim.execute(prog, max_steps=500)
        assert result.ok
        assert result.final_state.accumulator == 0

    def test_7_mod_3_is_1(self):
        prog = self._remainder_program(7, 3)
        sim = BabySimulator()
        result = sim.execute(prog, max_steps=500)
        assert result.ok
        assert result.final_state.acc_signed == 1

    def test_10_mod_7_is_3(self):
        prog = self._remainder_program(10, 7)
        sim = BabySimulator()
        result = sim.execute(prog, max_steps=500)
        assert result.ok
        assert result.final_state.acc_signed == 3

    def test_1_mod_1_is_0(self):
        prog = self._remainder_program(1, 1)
        sim = BabySimulator()
        result = sim.execute(prog, max_steps=100)
        assert result.ok
        assert result.final_state.accumulator == 0


# ── BabyState helper properties ───────────────────────────────────────────────

class TestBabyStateHelpers:
    """Tests for BabyState.acc_signed and BabyState.present_instruction."""

    def test_acc_signed_zero(self):
        state = BabyState(store=tuple([0] * 32), accumulator=0, ci=0, halted=False)
        assert state.acc_signed == 0

    def test_acc_signed_positive(self):
        state = BabyState(store=tuple([0] * 32), accumulator=42, ci=0, halted=False)
        assert state.acc_signed == 42

    def test_acc_signed_minus_one(self):
        state = BabyState(
            store=tuple([0] * 32), accumulator=0xFFFFFFFF, ci=0, halted=False
        )
        assert state.acc_signed == -1

    def test_acc_signed_minus_42(self):
        state = BabyState(
            store=tuple([0] * 32),
            accumulator=0xFFFFFFD6,
            ci=0,
            halted=False,
        )
        assert state.acc_signed == -42

    def test_acc_signed_max_positive(self):
        state = BabyState(
            store=tuple([0] * 32), accumulator=0x7FFFFFFF, ci=0, halted=False
        )
        assert state.acc_signed == 2147483647

    def test_acc_signed_min_negative(self):
        state = BabyState(
            store=tuple([0] * 32), accumulator=0x80000000, ci=0, halted=False
        )
        assert state.acc_signed == -2147483648

    def test_present_instruction_at_ci(self):
        store_data = [0] * 32
        store_data[5] = 0xABCD1234
        state = BabyState(
            store=tuple(store_data), accumulator=0, ci=5, halted=False
        )
        assert state.present_instruction == 0xABCD1234

    def test_present_instruction_at_ci_0(self):
        store_data = [0x1234] + [0] * 31
        state = BabyState(
            store=tuple(store_data), accumulator=0, ci=0, halted=False
        )
        assert state.present_instruction == 0x1234

    def test_present_instruction_from_execute(self):
        # After executing a 3-instruction program, CI should be at the STP line (2).
        # Instructions at lines 0-2; data at line 28.
        store = [0] * 32
        store[28] = 42         # data
        store[0] = ldn(28)     # line 0
        store[1] = sto(29)     # line 1
        store[2] = STP         # line 2
        sim = BabySimulator()
        result = sim.execute(build(store))
        assert result.final_state.ci == 2
        assert result.final_state.present_instruction == STP
