"""Test suite: Manchester Baby instruction set.

Tests each of the 7 opcodes in isolation with edge cases:
  - JMP S  (F=000): absolute jump; CI ← Store[S]
  - JRP S  (F=001): relative jump; CI ← CI + Store[S]
  - LDN S  (F=010): load negative; A ← −Store[S]
  - STO S  (F=011): store; Store[S] ← A
  - SUB S  (F=100): subtract; A ← A − Store[S]
  - SUB S  (F=101): subtract (alternate encoding); same as F=100
  - CMP    (F=110): conditional skip; if A < 0: CI += 1
  - STP    (F=111): halt

Also covers:
  - Two's-complement overflow wrap
  - CMP skip vs no-skip
  - LDN of zero and negative numbers
  - SUB producing zero, positive, and negative results

LAYOUT CONVENTION: instructions at low lines (0, 1, 2, ...), data at
high lines (28–31).  A data word like 0xFFFFFFFF would decode as STP
(F=7) if placed at line 0; keeping data at the high end of the store
prevents accidental execution.
"""

from __future__ import annotations

from manchester_baby_simulator import BabySimulator

# ── Helpers ───────────────────────────────────────────────────────────────────

def w(value: int) -> bytes:
    """Encode a 32-bit word as 4 little-endian bytes."""
    return (value & 0xFFFFFFFF).to_bytes(4, "little")


def build(store: list[int]) -> bytes:
    """Pad to 32 words and encode as 128 bytes."""
    padded = (store + [0] * 32)[:32]
    return b"".join(w(v) for v in padded)


# Instruction constructors
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


def sub2(s: int) -> int:
    """Alternate SUB encoding (F=101); must behave identically to F=100."""
    return (0b101 << 13) | (s & 0x1F)


def cmp_instr() -> int:
    return 0b110 << 13


STP = 0b111 << 13


def run(store: list[int], max_steps: int = 100) -> BabySimulator:
    """Execute a full 32-word store image; return the simulator after halt."""
    sim = BabySimulator()
    sim.execute(build(store), max_steps=max_steps)
    return sim


# ── STP ───────────────────────────────────────────────────────────────────────

class TestSTP:
    """STP (F=111) must halt the machine immediately."""

    def test_stp_sets_halted(self):
        sim = BabySimulator()
        result = sim.execute(w(STP))
        assert result.final_state.halted is True

    def test_stp_steps_count_is_1(self):
        sim = BabySimulator()
        result = sim.execute(w(STP))
        assert result.steps == 1

    def test_stp_does_not_change_accumulator(self):
        # Accumulator is 0 on reset; STP should not modify it
        sim = BabySimulator()
        result = sim.execute(w(STP))
        assert result.final_state.accumulator == 0

    def test_stp_at_line_3(self):
        # Reach STP at line 3 by executing three harmless LDN 31 first
        store = [0] * 32
        store[0] = ldn(31)   # A = −Store[31] = 0 (harmless)
        store[1] = ldn(31)
        store[2] = ldn(31)
        store[3] = STP
        sim = BabySimulator()
        result = sim.execute(build(store))
        assert result.final_state.halted is True
        assert result.final_state.ci == 3

    def test_stp_mnemonic(self):
        sim = BabySimulator()
        result = sim.execute(w(STP))
        assert result.traces[-1].mnemonic == "STP"


# ── LDN ───────────────────────────────────────────────────────────────────────

class TestLDN:
    """LDN S (F=010) — Load Negative: A ← (−Store[S]) & 0xFFFFFFFF."""

    def test_ldn_positive_value(self):
        # Store[28] = 42 → A = −42 = 0xFFFFFFD6
        store = [0] * 32
        store[28] = 42
        store[0] = ldn(28)
        store[1] = STP
        sim = run(store)
        assert sim.get_state().acc_signed == -42

    def test_ldn_result_unsigned(self):
        store = [0] * 32
        store[28] = 42
        store[0] = ldn(28)
        store[1] = STP
        sim = run(store)
        assert sim.get_state().accumulator == ((-42) & 0xFFFFFFFF)

    def test_ldn_zero(self):
        # −0 = 0
        store = [0] * 32
        store[28] = 0
        store[0] = ldn(28)
        store[1] = STP
        sim = run(store)
        assert sim.get_state().accumulator == 0

    def test_ldn_one(self):
        store = [0] * 32
        store[28] = 1
        store[0] = ldn(28)
        store[1] = STP
        sim = run(store)
        assert sim.get_state().acc_signed == -1

    def test_ldn_min_int(self):
        # −(−2³¹) overflows to −2³¹ in two's complement
        # Store[28] = 0x80000000; LDN: A = (−0x80000000) & 0xFFFFFFFF = 0x80000000
        store = [0] * 32
        store[28] = 0x80000000
        store[0] = ldn(28)
        store[1] = STP
        sim = run(store)
        assert sim.get_state().accumulator == 0x80000000

    def test_ldn_max_positive(self):
        # Store[28] = 0x7FFFFFFF → A = −0x7FFFFFFF = 0x80000001
        store = [0] * 32
        store[28] = 0x7FFFFFFF   # data at a high line — won't execute as instruction
        store[0] = ldn(28)
        store[1] = STP
        sim = run(store)
        assert sim.get_state().accumulator == ((-0x7FFFFFFF) & 0xFFFFFFFF)

    def test_ldn_negative_value_in_store(self):
        # Store[28] = 0xFFFFFFFF (= −1 signed) → LDN: A = −(−1) = 1
        # (Data placed at line 28; if placed at line 0 it decodes as STP.)
        store = [0] * 32
        store[28] = 0xFFFFFFFF   # −1 as data
        store[0] = ldn(28)
        store[1] = STP
        sim = run(store)
        assert sim.get_state().accumulator == 1

    def test_ldn_mnemonic(self):
        store = [0] * 32
        store[28] = 42
        store[0] = ldn(28)
        store[1] = STP
        sim = BabySimulator()
        result = sim.execute(build(store))
        assert any(t.mnemonic == "LDN 28" for t in result.traces)

    def test_ldn_from_line_5(self):
        # Data at line 5; instructions at lines 0-1
        store = [0] * 32
        store[5] = 100         # data
        store[0] = ldn(5)      # A = −100
        store[1] = STP
        sim = BabySimulator()
        result = sim.execute(build(store))
        assert result.final_state.acc_signed == -100


# ── STO ───────────────────────────────────────────────────────────────────────

class TestSTO:
    """STO S (F=011) — Store: Store[S] ← A."""

    def test_sto_writes_accumulator_to_store(self):
        # LDN 28 (A←−42), STO 29 (Store[29]←A), STP
        store = [0] * 32
        store[28] = 42
        store[0] = ldn(28)
        store[1] = sto(29)
        store[2] = STP
        sim = run(store)
        assert sim.get_state().store[29] == ((-42) & 0xFFFFFFFF)

    def test_sto_does_not_change_accumulator(self):
        store = [0] * 32
        store[28] = 42
        store[0] = ldn(28)
        store[1] = sto(29)
        store[2] = STP
        sim = run(store)
        assert sim.get_state().acc_signed == -42

    def test_sto_zero(self):
        # A=0 (from reset, no LDN), STO 29 → Store[29] = 0
        store = [0] * 32
        store[0] = sto(29)
        store[1] = STP
        sim = run(store)
        assert sim.get_state().store[29] == 0

    def test_sto_overwrites_existing(self):
        # Store[29] starts at 99; STO overwrites it with A
        store = [0] * 32
        store[28] = 42
        store[29] = 99         # pre-existing value
        store[0] = ldn(28)     # A = −42
        store[1] = sto(29)     # Store[29] ← A = −42
        store[2] = STP
        sim = run(store)
        assert sim.get_state().store[29] == ((-42) & 0xFFFFFFFF)

    def test_sto_self_modifying(self):
        # STO can write to any store address, including high data lines.
        # Store[28] holds the STP word; we negate it and write it back.
        store = [0] * 32
        store[28] = STP          # data: the STP instruction word (0xE000)
        store[0] = ldn(28)       # A ← −STP
        store[1] = sto(28)       # Store[28] ← A (overwrites data)
        store[2] = STP           # halt
        sim = run(store)
        expected = (-STP) & 0xFFFFFFFF
        assert sim.get_state().store[28] == expected

    def test_sto_mnemonic(self):
        store = [0] * 32
        store[28] = 42
        store[0] = ldn(28)
        store[1] = sto(29)
        store[2] = STP
        sim = BabySimulator()
        result = sim.execute(build(store))
        assert any(t.mnemonic == "STO 29" for t in result.traces)


# ── SUB ───────────────────────────────────────────────────────────────────────

class TestSUB:
    """SUB S (F=100/101) — Subtract: A ← (A − Store[S]) & 0xFFFFFFFF."""

    def test_sub_basic(self):
        # A=0 (reset), Store[28]=10, SUB 28 → A = 0−10 = −10
        store = [0] * 32
        store[28] = 10
        store[0] = sub(28)
        store[1] = STP
        sim = run(store)
        assert sim.get_state().acc_signed == -10

    def test_sub_produces_zero(self):
        # A=0 (reset default), Store[28]=0, SUB 28 → A = 0−0 = 0
        store = [0] * 32
        store[28] = 0
        store[0] = sub(28)
        store[1] = STP
        sim = run(store)
        assert sim.get_state().accumulator == 0

    def test_sub_produces_positive(self):
        # LDN 28 (A = −(−5) = 5), SUB 29 (A = 5−3 = 2)
        store = [0] * 32
        store[28] = 0xFFFFFFFB   # −5 (data at high line; would be STP if at line 0)
        store[29] = 3
        store[0] = ldn(28)       # A = 5
        store[1] = sub(29)       # A = 5 − 3 = 2
        store[2] = STP
        sim = run(store)
        assert sim.get_state().acc_signed == 2

    def test_sub_overflow_wraps(self):
        # A=0, Store[28]=1 → A = 0−1 = −1 = 0xFFFFFFFF
        store = [0] * 32
        store[28] = 1
        store[0] = sub(28)
        store[1] = STP
        sim = run(store)
        assert sim.get_state().accumulator == 0xFFFFFFFF

    def test_sub_alternate_encoding_f101(self):
        # F=101 must behave identically to F=100
        store = [0] * 32
        store[28] = 10
        store[0] = sub2(28)
        store[1] = STP
        sim = run(store)
        assert sim.get_state().acc_signed == -10

    def test_sub_both_encodings_identical(self):
        store1 = [0] * 32
        store1[28] = 7
        store1[0] = sub(28)
        store1[1] = STP

        store2 = [0] * 32
        store2[28] = 7
        store2[0] = sub2(28)
        store2[1] = STP

        sim1 = run(store1)
        sim2 = run(store2)
        assert sim1.get_state().accumulator == sim2.get_state().accumulator

    def test_sub_mnemonic_f100(self):
        store = [0] * 32
        store[28] = 1
        store[0] = sub(28)
        store[1] = STP
        sim = BabySimulator()
        result = sim.execute(build(store))
        assert any(t.mnemonic == "SUB 28" for t in result.traces)

    def test_sub_mnemonic_f101(self):
        store = [0] * 32
        store[28] = 1
        store[0] = sub2(28)
        store[1] = STP
        sim = BabySimulator()
        result = sim.execute(build(store))
        assert any(t.mnemonic == "SUB 28" for t in result.traces)

    def test_sub_from_nonzero_address(self):
        store = [0] * 32
        store[7] = 20
        store[0] = sub(7)
        store[1] = STP
        sim = run(store)
        assert sim.get_state().acc_signed == -20


# ── CMP ───────────────────────────────────────────────────────────────────────

class TestCMP:
    """CMP (F=110) — Skip if accumulator is negative (bit 31 = 1)."""

    def test_cmp_skips_when_negative(self):
        # A < 0 → skip the next instruction (STO 31 sentinel)
        store = [0] * 32
        store[28] = 1
        store[0] = ldn(28)     # A = −1 (negative)
        store[1] = cmp_instr() # A<0 → skip STO 31
        store[2] = sto(31)     # SKIPPED (would corrupt store[31])
        store[3] = STP
        sim = run(store)
        assert sim.get_state().store[31] == 0   # skipped
        assert sim.get_state().halted is True

    def test_cmp_no_skip_when_zero(self):
        # A = 0 → no skip; STP at line 3 executes (not line 4)
        store = [0] * 32
        store[28] = 0
        store[0] = ldn(28)     # A = −0 = 0
        store[1] = cmp_instr() # A=0: no skip
        store[2] = STP         # executes
        store[3] = sto(31)     # never reached (we halted at line 2)
        sim = run(store)
        assert sim.get_state().ci == 2   # halted at STP on line 2

    def test_cmp_no_skip_when_positive(self):
        # A > 0 → no skip; STO 31 sentinel IS executed
        store = [0] * 32
        store[28] = 0xFFFFFFFF   # −1 as data (would be STP if at line 0)
        store[0] = ldn(28)       # A = −(−1) = 1 (positive)
        store[1] = cmp_instr()   # A>0: no skip
        store[2] = sto(31)       # executed → Store[31] = 1
        store[3] = STP
        sim = run(store)
        assert sim.get_state().store[31] == 1   # STO was NOT skipped

    def test_cmp_skip_advances_by_one_extra(self):
        # CMP at line 1 with A<0 skips line 2; halts at line 3
        store = [0] * 32
        store[28] = 1
        store[0] = ldn(28)       # A = −1
        store[1] = cmp_instr()   # skips line 2
        store[2] = sto(31)       # SKIPPED
        store[3] = STP
        sim = run(store)
        assert sim.get_state().ci == 3   # halted at line 3

    def test_cmp_negative_boundary(self):
        # 0x80000000 is the most-negative 32-bit value (−2³¹); bit 31 = 1 → skip
        store = [0] * 32
        store[5] = 0x80000000   # data: will become A after LDN of LDN(5's neg)
        # To get A = 0x80000000 directly: LDN of −0x80000000 = 0x80000000 (overflow)
        store[28] = 0x80000000   # Store[28] = 0x80000000; LDN 28 → A = 0x80000000
        store[0] = ldn(28)
        store[1] = cmp_instr()
        store[2] = sto(31)       # skipped (A is negative)
        store[3] = STP
        sim = run(store)
        assert sim.get_state().store[31] == 0   # skipped

    def test_cmp_mnemonic(self):
        store = [0] * 32
        store[0] = cmp_instr()
        store[1] = STP
        sim = BabySimulator()
        result = sim.execute(build(store))
        assert any(t.mnemonic == "CMP" for t in result.traces)


# ── JMP ───────────────────────────────────────────────────────────────────────

class TestJMP:
    """JMP S (F=000) — Absolute jump: CI ← Store[S].

    Because CI is pre-incremented before each fetch, JMP loads Store[S] into CI,
    then the next step increments CI by 1.  So "jump to line N" requires
    storing N−1 at address S.
    """

    def test_jmp_basic(self):
        # Jump from line 0 to line 3 (skipping 1 and 2).
        # JMP reads Store[10] = 2 (= target 3 minus 1) → CI=2 → next CI=3.
        store = [0] * 32
        store[0] = jmp(10)     # JMP → CI ← Store[10] = 2; next fetch: CI=3
        store[10] = 2          # target - 1
        store[1] = sto(31)     # skipped
        store[2] = sto(30)     # skipped
        store[3] = STP
        sim = run(store)
        state = sim.get_state()
        assert state.ci == 3
        assert state.store[31] == 0   # skipped
        assert state.store[30] == 0   # skipped

    def test_jmp_mnemonic(self):
        store = [0] * 32
        store[0] = jmp(5)
        store[5] = 3           # CI←3; next CI=4
        store[4] = STP
        sim = BabySimulator()
        result = sim.execute(build(store))
        assert any(t.mnemonic == "JMP 5" for t in result.traces)

    def test_jmp_wrap_around(self):
        # Tight loop: JMP 10 at line 0; Store[10]=31 → CI=31 → next CI=0.
        # Line 0 = JMP 10 again. Loops forever.
        store = [0] * 32
        store[0] = jmp(10)     # CI ← Store[10] = 31; next CI = 0
        store[10] = 31
        sim = BabySimulator()
        result = sim.execute(build(store), max_steps=4)
        assert result.halted is False
        assert result.steps == 4


# ── JRP ───────────────────────────────────────────────────────────────────────

class TestJRP:
    """JRP S (F=001) — Relative jump: CI ← CI + Store[S].

    Store[S] is treated as a *signed* 32-bit displacement.
    """

    def test_jrp_forward_displacement(self):
        # JRP at line 0: CI=0; displacement Store[10]=1; CI←0+1=1; next CI=2.
        # Line 1 (STO 31) is skipped; line 2 halts.
        store = [0] * 32
        store[0] = jrp(10)    # CI ← 0 + Store[10] = 1; next fetch: 2
        store[10] = 1         # displacement +1
        store[1] = sto(31)    # would write sentinel — skipped by jump
        store[2] = STP
        sim = run(store)
        assert sim.get_state().ci == 2
        assert sim.get_state().store[31] == 0   # line 1 was skipped

    def test_jrp_zero_displacement(self):
        # Displacement 0: CI stays same; next fetch is CI+1 (normal advance)
        store = [0] * 32
        store[0] = jrp(10)    # CI ← 0+0 = 0; next CI = 1
        store[10] = 0
        store[1] = STP
        sim = run(store)
        assert sim.get_state().ci == 1

    def test_jrp_negative_displacement(self):
        # Displacement −1 (0xFFFFFFFF): tight loop.
        # Line 0: LDN 31 (harmless). Line 1: LDN 31. Line 2: JRP 10 with disp −1.
        # After pre-increment: CI=2. JRP: CI←2+(−1)=1. Next CI=2. Loop.
        store = [0] * 32
        store[0] = ldn(31)      # harmless
        store[1] = ldn(31)      # harmless; CI will land here after JRP
        store[2] = jrp(10)      # CI=2; CI ← 2+(−1) = 1; next: CI=2
        store[10] = 0xFFFFFFFF  # = −1 signed
        sim = BabySimulator()
        result = sim.execute(build(store), max_steps=20)
        assert result.halted is False
        assert result.steps == 20

    def test_jrp_mnemonic(self):
        store = [0] * 32
        store[0] = jrp(5)
        store[5] = 0       # displacement 0; next fetch line 1
        store[1] = STP
        sim = BabySimulator()
        result = sim.execute(build(store))
        assert any(t.mnemonic == "JRP 5" for t in result.traces)


# ── Combined arithmetic (no ADD — only LDN+SUB) ───────────────────────────────

class TestArithmeticEdgeCases:
    """The SSEM has no ADD.  Addition = LDN then SUB the negated value.

    Layout: instructions at lines 0-2; data at lines 28-29.
    """

    def test_add_via_ldn_sub(self):
        # X + Y = 5 + 3 = 8.
        # Store[28] = −5; LDN 28 → A = 5.
        # Store[29] = −3; SUB 29 → A = 5 − (−3) = 8.
        store = [0] * 32
        store[28] = 0xFFFFFFFB   # −5
        store[29] = 0xFFFFFFFD   # −3
        store[0] = ldn(28)       # A = 5
        store[1] = sub(29)       # A = 5 − (−3) = 8
        store[2] = STP
        sim = run(store)
        assert sim.get_state().acc_signed == 8

    def test_two_complement_overflow_wraps_silently(self):
        # A = 0, Store[28] = 1 → SUB → A = −1 = 0xFFFFFFFF (no exception)
        store = [0] * 32
        store[28] = 1
        store[0] = sub(28)
        store[1] = STP
        sim = run(store)
        assert sim.get_state().accumulator == 0xFFFFFFFF

    def test_ldn_double_negation(self):
        # −(−42) = 42.
        # Store[28] = 0xFFFFFFD6 (= −42); LDN 28 → A = 42.
        # (Data at high line; would decode as STP if placed at line 0.)
        store = [0] * 32
        store[28] = 0xFFFFFFD6   # −42
        store[0] = ldn(28)
        store[1] = STP
        sim = run(store)
        assert sim.get_state().acc_signed == 42

    def test_ldn_min_int_overflow(self):
        # −(MIN_INT) overflows: −(−2³¹) = +2³¹ wraps to −2³¹
        store = [0] * 32
        store[28] = 0x80000000
        store[0] = ldn(28)
        store[1] = STP
        sim = run(store)
        assert sim.get_state().accumulator == 0x80000000
