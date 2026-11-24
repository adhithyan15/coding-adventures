"""Tests for property-based testing and shrinking."""

import pytest

import coding_adventures_test_bootstrap.pbt as pbt
from coding_adventures_test_bootstrap.pbt import (
    MAX_CASES,
    MAX_INPUT_BYTES,
    MAX_LIST_LEN,
    MAX_SHRINK_STEPS,
    for_all,
    frequency,
    gen_bool,
    gen_bytes,
    gen_choice,
    gen_constant,
    gen_float,
    gen_i128,
    gen_int,
    gen_list,
    gen_one_of,
    gen_text,
    gen_tuple,
    gen_u8,
    gen_u64,
    shrink_bytes,
    shrink_int,
    shrink_list,
    simulate_replay,
)
from coding_adventures_test_bootstrap.prng import SplitMix64, fnv1a64


def test_gen_primitives() -> None:
    rng = SplitMix64(42)

    b = gen_bool().sample(rng)
    assert isinstance(b, bool)

    i = gen_int(-100, 100).sample(rng)
    assert -100 <= i <= 100

    u8 = gen_u8().sample(rng)
    assert 0 <= u8 <= 255

    u64 = gen_u64().sample(rng)
    assert 0 <= u64 < (1 << 64)

    flt = gen_float(-10.0, 10.0).sample(rng)
    assert -10.0 <= flt <= 10.0

    byt = gen_bytes(2, 5).sample(rng)
    assert 2 <= len(byt) <= 5

    txt = gen_text(3, 6, alphabet="abc").sample(rng)
    assert 3 <= len(txt) <= 6
    assert all(c in "abc" for c in txt)

    ch = gen_choice([10, 20, 30]).sample(rng)
    assert ch in (10, 20, 30)

    lst = gen_list(gen_int(0, 5), 1, 3).sample(rng)
    assert 1 <= len(lst) <= 3
    assert all(0 <= x <= 5 for x in lst)

    one = gen_one_of(gen_bool(), gen_u8()).sample(rng)
    assert isinstance(one, (bool, int))


def test_gen_validation_errors() -> None:
    with pytest.raises(ValueError):
        gen_int(10, 5)
    with pytest.raises(ValueError):
        gen_float(10.0, 5.0)
    with pytest.raises(ValueError):
        gen_bytes(10, 5)
    with pytest.raises(ValueError):
        gen_bytes(-1, 5)
    with pytest.raises(ValueError):
        gen_bytes(0, MAX_INPUT_BYTES + 1)
    with pytest.raises(ValueError):
        gen_text(10, 5)
    with pytest.raises(ValueError):
        gen_text(0, 5, alphabet="")
    with pytest.raises(ValueError):
        gen_text(0, MAX_INPUT_BYTES + 1)
    with pytest.raises(ValueError):
        gen_choice([])
    with pytest.raises(ValueError):
        gen_list(gen_bool(), 5, 2)
    with pytest.raises(ValueError):
        gen_list(gen_bool(), -1, 2)
    with pytest.raises(ValueError, match="MAX_LIST_LEN"):
        gen_list(gen_bool(), 0, MAX_LIST_LEN + 1)
    with pytest.raises(ValueError):
        gen_one_of()


def test_gen_combinators() -> None:
    rng = SplitMix64(123)

    # map
    g_str = gen_int(1, 10).map(lambda x: str(x * 2))
    val_str = g_str.sample(rng)
    assert isinstance(val_str, str)
    assert int(val_str) % 2 == 0

    # filter
    g_even = gen_int(1, 100).filter(lambda x: x % 2 == 0)
    for _ in range(10):
        assert g_even.sample(rng) % 2 == 0

    # filter retry exhaustion
    g_impossible = gen_int(1, 10).filter(lambda x: x > 100, max_retries=5)
    with pytest.raises(RuntimeError, match="exceeded max_retries"):
        g_impossible.sample(rng)

    # flat_map
    g_pair = gen_int(1, 4).flat_map(lambda n: gen_list(gen_bool(), n, n))
    sampled_list = g_pair.sample(rng)
    assert 1 <= len(sampled_list) <= 4


def test_shrink_int_basic_and_bounds() -> None:
    # Target probe: 0 itself fails, so one step.
    assert shrink_int(100, 0, 0, 200, lambda x: x < 0) == (0, 1)
    # Bisection then unit steps down to the threshold.
    assert shrink_int(54321, 0, 0, 100000, lambda x: x <= 100) == (101, 15)
    # Negative side, never crossing the bound (-10 holds, -11 fails).
    assert shrink_int(-80, 0, -200, 200, lambda x: -10 <= x <= 50) == (-11, 11)
    # A budget of zero returns the start untouched.
    assert shrink_int(500, 0, 0, 1000, lambda x: x <= 100, max_steps=0) == (500, 0)
    # A budget of three stops mid-walk but still returns a falsifying value.
    value, steps = shrink_int(500, 0, 0, 1000, lambda x: x <= 100, max_steps=3)
    assert steps == 3 and value > 100
    # Already at the target and failing: nothing to do.
    assert shrink_int(7, 7, 0, 10, lambda x: x != 7) == (7, 0)
    # The target is outside [min, max]: no probe; walk stays inside the bounds.
    value, _ = shrink_int(90, 0, 50, 100, lambda x: x < 50)
    assert value == 90 or 50 <= value <= 100
    assert shrink_int(90, 0, 50, 100, lambda x: x < 60)[0] == 60


def test_shrink_int_extremes_do_not_overflow() -> None:
    lo, hi = -(1 << 127), (1 << 127) - 1
    # Opposite-extreme start/target: fixed-width lanes need saturating maths.
    assert shrink_int(hi, lo, lo, hi, lambda x: False)[0] == lo
    value, steps = shrink_int(hi, 0, lo, hi, lambda x: x < 1000)
    assert value == 1000 and steps > 100


def test_shrink_int_rejects_bad_calls() -> None:
    with pytest.raises(ValueError, match="must falsify"):
        shrink_int(5, 0, 0, 10, lambda x: True)
    with pytest.raises(ValueError, match="within"):
        shrink_int(50, 0, 0, 10, lambda x: False)
    with pytest.raises(ValueError, match="exceed"):
        shrink_int(5, 0, 10, 0, lambda x: False)
    with pytest.raises(TypeError):
        shrink_int(True, 0, 0, 10, lambda x: False)  # type: ignore[arg-type]
    with pytest.raises(ValueError, match="negative"):
        shrink_int(5, 0, 0, 10, lambda x: False, max_steps=-1)
    with pytest.raises(ValueError, match="MAX_SHRINK_STEPS"):
        shrink_int(5, 0, 0, 10, lambda x: False, max_steps=MAX_SHRINK_STEPS + 1)
    with pytest.raises(TypeError):
        shrink_int(5, 0, 0, 10, lambda x: False, max_steps="3")  # type: ignore[arg-type]


def test_shrink_list_and_bytes() -> None:
    assert shrink_list(
        [1, 2, 3, 3, 4, 5],
        0,
        lambda xs: all(a != b for a, b in zip(xs, xs[1:], strict=False)),
    )[0] == [3, 3]
    # Empty probe wins in one step.
    assert shrink_list([1, 2, 3], 0, lambda xs: False) == ([], 1)
    assert shrink_list([10, 5, 8, 4], 0, lambda xs: sum(xs) <= 20) == ([8, 5, 8], 3)
    assert shrink_list([9, 9], 0, lambda xs: False, max_steps=0) == ([9, 9], 0)
    assert shrink_bytes(bytes(range(1, 9)), lambda b: len(b) < 5) == (b"\x00" * 5, 16)
    assert shrink_bytes(b"\x07", lambda b: False, max_steps=0) == (b"\x07", 0)
    with pytest.raises(ValueError, match="must falsify"):
        shrink_list([1], 0, lambda xs: True)
    with pytest.raises(ValueError, match="must falsify"):
        shrink_bytes(b"a", lambda b: True)
    with pytest.raises(TypeError):
        shrink_list([1.5], 0, lambda xs: False)  # type: ignore[list-item]
    with pytest.raises(TypeError):
        shrink_list([1], True, lambda xs: False)  # type: ignore[arg-type]


def test_shrink_sequence_respects_min_len_and_budget() -> None:
    gen = gen_list(gen_int(0, 100), min_len=2, max_len=10)
    # fails whenever the sum is over 3; min_len 2 must survive shrinking.
    value, steps = gen.shrink([50, 60, 70, 80], lambda xs: sum(xs) > 3, 1000)
    assert len(value) == 2 and sum(value) > 3 and steps > 0
    assert value == [0, 4] or sum(value) == 4
    # prefix probe with min_len>0 already minimal-length and elements shrink.
    value, _ = gen.shrink([5, 5], lambda xs: True, 1000)
    assert value == [0, 0]
    # a tight budget is respected
    _, steps = gen.shrink([50, 60, 70, 80], lambda xs: sum(xs) > 3, 2)
    assert steps <= 2
    # bisection that would cross min_len is skipped
    four = gen_list(gen_int(0, 9), min_len=4, max_len=4)
    value, _ = four.shrink([9, 9, 9, 9], lambda xs: sum(xs) > 0, 100)
    assert len(value) == 4 and sum(value) == 1


def test_gen_shrinkers_stay_inside_their_bounds() -> None:
    g = gen_int(5, 10)
    assert g.shrink(10, lambda v: True, 100) == (5, 1)  # target = nearest 0 = 5
    assert gen_int(-10, -3).shrink(-9, lambda v: v < -4, 100)[0] == -5
    assert gen_u8().shrink(200, lambda v: v > 10, 100)[0] == 11
    assert gen_u64().shrink(10**18, lambda v: True, 10)[0] == 0
    assert gen_i128().shrink(-(10**30), lambda v: True, 10)[0] == 0
    assert gen_bool().shrink(True, lambda v: True, 5) == (False, 1)
    assert gen_bool().shrink(True, lambda v: v, 5) == (True, 0)
    assert gen_bool().shrink(False, lambda v: True, 5) == (False, 0)
    assert gen_bool().shrink(True, lambda v: True, 0) == (True, 0)
    # text: shorter, then toward alphabet[0]
    assert gen_text(0, 8, "abc").shrink("cbcb", lambda s: len(s) >= 2, 100)[0] == "aa"
    assert gen_text(1, 8, "abc").shrink("cb", lambda s: True, 100)[0] == "a"
    # bytes respects min_len
    assert (
        gen_bytes(2, 8).shrink(b"\x05\x06\x07", lambda b: True, 100)[0] == b"\x00\x00"
    )
    # un-shrinkable generators hand the value back
    assert gen_float(0, 1).shrink(0.5, lambda v: True, 5) == (0.5, 0)
    assert gen_choice([1, 2]).shrink(2, lambda v: True, 5) == (2, 0)
    assert gen_int(1, 5).map(str).shrink("3", lambda v: True, 5) == ("3", 0)
    # filter keeps the predicate: only even values are valid candidates
    even = gen_int(0, 100).filter(lambda x: x % 2 == 0)
    shrunk, _ = even.shrink(64, lambda v: v >= 10, 100)
    assert shrunk == 16  # odd candidates (15) are invalid inputs: local minimum


def test_gen_tuple_samples_in_order_and_shrinks_each_slot() -> None:
    g = gen_tuple(gen_int(0, 9), gen_bool(), gen_constant("k"))
    rng_a, rng_b = SplitMix64(7), SplitMix64(7)
    a = g.sample(rng_a)
    assert a == (gen_int(0, 9).sample(rng_b), gen_bool().sample(rng_b), "k")
    value, steps = g.shrink((9, True, "k"), lambda t: t[0] + t[1] >= 1, 100)
    assert value == (0, True, "k") or value == (1, False, "k")
    assert steps >= 1


def test_frequency_and_one_of() -> None:
    rng = SplitMix64(3)
    never = gen_constant("never")
    g = frequency((0, never), (1, gen_constant("a")), (3, gen_constant("b")))
    draws = [g.sample(rng) for _ in range(2000)]
    assert "never" not in draws
    assert 0.20 < draws.count("a") / 2000 < 0.30  # ~1/4
    assert set(draws) == {"a", "b"}
    with pytest.raises(ValueError):
        frequency()
    with pytest.raises(ValueError, match="> 0"):
        frequency((0, never))
    with pytest.raises(ValueError, match="negative"):
        frequency((-1, never), (2, never))
    with pytest.raises(ValueError, match="exceeds"):
        frequency((1 << 32, never), (1, never))
    with pytest.raises(TypeError):
        frequency((1.5, never))  # type: ignore[arg-type]
    assert pbt.one_of is gen_one_of
    # single-arm choices still consume exactly one stream word
    r1, r2 = SplitMix64(11), SplitMix64(11)
    gen_one_of(never).sample(r1)
    r2.next_u64()
    assert r1.state == r2.state
    r3, r4 = SplitMix64(11), SplitMix64(11)
    gen_int(5, 5).sample(r3)
    r4.next_u64()
    assert r3.state == r4.state


def test_filter_exhaustion_is_typed_and_validates() -> None:
    with pytest.raises(pbt.ExhaustedError):
        gen_int(1, 10).filter(lambda x: False, max_retries=3).sample(SplitMix64(1))
    with pytest.raises(ValueError):
        gen_int(1, 10).filter(lambda x: True, max_retries=0)
    with pytest.raises(TypeError):
        gen_int(1, 10).filter(lambda x: True, max_retries=True)  # type: ignore[arg-type]


def test_wide_bounded_int_is_uniform_and_in_range() -> None:
    lo, hi = -(1 << 127), (1 << 127) - 1  # the full i128 range: span == 2^128
    rng = SplitMix64(5)
    values = [pbt.gen_bounded_int(rng, lo, hi) for _ in range(50)]
    assert all(lo <= v <= hi for v in values)
    assert any(v < 0 for v in values) and any(v > 0 for v in values)
    # span just above 2^64 uses k-word rejection and never leaves the range
    values = [pbt.gen_bounded_int(rng, 0, (1 << 64) + 5) for _ in range(200)]
    assert all(0 <= v <= (1 << 64) + 5 for v in values)
    # a span of 3 * 2^126: each third of the range is drawn about 1/3 of the time
    span = 3 << 126
    thirds = [0, 0, 0]
    for _ in range(3000):
        thirds[pbt.gen_bounded_int(rng, 0, span - 1) // (1 << 126)] += 1
    assert all(850 < t < 1150 for t in thirds), thirds
    # wider than 128 bits still works
    assert 0 <= pbt.gen_bounded_int(rng, 0, 1 << 200) <= 1 << 200
    # span == 2^64 is one raw draw
    r1, r2 = SplitMix64(9), SplitMix64(9)
    assert pbt.gen_bounded_int(r1, 0, (1 << 64) - 1) == r2.next_u64()
    with pytest.raises(ValueError):
        pbt.gen_bounded_int(rng, 3, 2)
    with pytest.raises(TypeError):
        pbt.gen_bounded_int(rng, 0, 1.5)  # type: ignore[arg-type]


def test_corpus_entry_points_validate() -> None:
    with pytest.raises(ValueError, match="unknown primitive"):
        pbt.gen_primitive(0, "f32", 1)
    assert pbt.gen_primitive(0, "u64", 1) == [16_294_208_416_658_607_535]
    assert pbt.gen_bounded(7, 10, 20, 2) == [12, 10]
    assert pbt.gen_bytes_corpus(1, 0, 0, 2) == [b"", b""]
    with pytest.raises(ValueError):
        pbt.gen_bounded(0, 5, 1, 1)


def test_sub_seed_matches_the_spec_formula() -> None:
    # VER05 2.3: SplitMix64(run_seed ^ fnv1a64(id)).next_u64()
    for run_seed, name in ((0, ""), (7, "prop/a"), (1 << 70, "x"), (42, "ünï")):
        expected = SplitMix64((run_seed & ((1 << 64) - 1)) ^ fnv1a64(name)).next_u64()
        assert pbt.sub_seed(run_seed, name) == expected
    # Pinned vectors from the independent reference (pbt_conformance.py).
    assert pbt.sub_seed(0, "") == 0xC381_7C01_6BA4_FF30
    assert pbt.sub_seed(7, "prop/a") == 0x2319_916F_3176_AD0D
    assert pbt.sub_seed(42, "a") == 0x3260_8AEF_48BB_7618
    assert pbt.sub_seed(42, "a") != pbt.sub_seed(42, "b")
    assert pbt.sub_seed(1, "a") != 1 ^ fnv1a64("a")  # scrambled, not a bare xor
    with pytest.raises(TypeError):
        pbt.sub_seed(True, "a")  # type: ignore[arg-type]
    with pytest.raises(TypeError):
        pbt.sub_seed(1, b"a")  # type: ignore[arg-type]


def test_simulate_replay() -> None:
    assert simulate_replay(42, lambda x: x <= 100) == (413, 101)
    with pytest.raises(ValueError, match="not falsified"):
        simulate_replay(1, lambda x: True)


# --- for_all -----------------------------------------------------------------


def test_for_all_success() -> None:
    res = for_all(
        lambda a, b: a + b == b + a,
        gen_int(-1000, 1000),
        gen_int(-1000, 1000),
        count=50,
        seed=101,
    )
    assert res.passed
    assert res.cases_run == 50
    assert res.counterexample is None
    assert res.error is None


def test_for_all_failure_shrinks_to_the_threshold() -> None:
    res = for_all(lambda x: x < 10, gen_int(0, 100), count=50, seed=42)
    assert not res.passed
    assert res.counterexample is not None and res.counterexample[0] >= 10
    assert res.shrunken == (10,)
    assert res.error == "Property returned False"
    assert res.shrink_steps >= 1


def test_for_all_shrinking_never_leaves_generator_bounds() -> None:
    # The old engine shrank toward 0 whatever the generator said.
    res = for_all(lambda x: False, gen_int(5, 10), count=3, seed=1)
    assert res.shrunken == (5,)
    res = for_all(lambda x, y: False, gen_int(-9, -2), gen_int(100, 200), count=3)
    assert res.shrunken == (-2, 100)


def test_for_all_unshrinkable_generator_keeps_its_value() -> None:
    fixed_negative = pbt.Gen(lambda rng: -8)
    res = for_all(lambda x: x >= 0, fixed_negative, count=1, max_shrink_steps=100)
    assert not res.passed
    assert res.counterexample == (-8,)
    assert res.shrunken == (-8,)
    assert res.shrink_steps == 0


def test_for_all_budget_counts_accepted_moves() -> None:
    calls: list[int] = []

    def prop(value: int) -> bool:
        calls.append(value)
        return value <= 100

    res = for_all(prop, gen_int(0, 100_000), count=200, seed=3, max_shrink_steps=2)
    assert res.shrink_steps == 2
    assert res.shrunken is not None and res.shrunken[0] > 100

    zero = for_all(lambda v: False, gen_int(50, 60), count=1, max_shrink_steps=0)
    assert zero.shrunken == zero.counterexample
    assert zero.shrink_steps == 0


def test_for_all_multi_argument_is_deterministic_and_replayable() -> None:
    def prop(flag: bool, value: int) -> bool:
        return value < 64

    kwargs = {"count": 200, "seed": 0xAF04C0, "max_shrink_steps": 100}
    first = for_all(prop, gen_bool(), gen_int(0, 1000), **kwargs)
    again = for_all(prop, gen_bool(), gen_int(0, 1000), **kwargs)
    assert first == again
    assert first.shrunken == (False, 64)


def test_for_all_exception_handling() -> None:
    def buggy_fn(x: int) -> bool:
        if x > 20:
            raise ArithmeticError("Value too large")
        return True

    res = for_all(buggy_fn, gen_int(0, 50), count=50, seed=99)
    assert not res.passed
    assert "ArithmeticError" in (res.error or "")
    assert res.shrunken == (21,)  # a raising property still counts as failing


def test_for_all_validation() -> None:
    with pytest.raises(TypeError):
        for_all(lambda x: True, gen_int(), count="50")  # type: ignore[arg-type]
    with pytest.raises(ValueError, match="positive"):
        for_all(lambda x: True, gen_int(), count=0)
    with pytest.raises(ValueError, match="MAX_CASES"):
        for_all(lambda x: True, gen_int(), count=MAX_CASES + 1)
    with pytest.raises(TypeError):
        for_all(lambda x: True, gen_int(), max_shrink_steps="5")  # type: ignore[arg-type]
    with pytest.raises(ValueError, match="negative"):
        for_all(lambda x: True, gen_int(), max_shrink_steps=-1)
    with pytest.raises(ValueError, match="MAX_SHRINK_STEPS"):
        for_all(lambda x: True, gen_int(), max_shrink_steps=MAX_SHRINK_STEPS + 1)
    with pytest.raises(ValueError, match="at least one generator"):
        for_all(lambda: True)


# --- check_property: verdict honesty -----------------------------------------

LOCAL = pbt.PbtOptions(num_tests=50, ci=False)
REDACT = pbt.PbtOptions(num_tests=50, ci=True)


def test_check_property_passes_only_when_every_case_ran_and_held() -> None:
    v = pbt.check_property(1, gen_int(0, 9), lambda x: 0 <= x <= 9, LOCAL)
    assert v.passed and not v.failed and v.cases_run == 50 and v.failure_id is None
    # returning None (assertion style) is a pass
    assert pbt.check_property(1, gen_int(0, 9), lambda x: None, LOCAL).passed


def test_zero_cases_is_not_a_pass() -> None:
    v = pbt.check_property(
        1, gen_int(0, 9), lambda x: True, pbt.PbtOptions(num_tests=0)
    )
    assert v.failed and v.failure_id == pbt.FID_EXHAUSTED and v.cases_run == 0


def test_falsifiable_property_is_never_a_pass() -> None:
    for seed in range(20):
        v = pbt.check_property(
            seed,
            gen_int(0, 1000),
            lambda x: x != 500,
            pbt.PbtOptions(num_tests=5000, ci=False),
        )
        assert v.failed and v.failure_id == pbt.FID_COUNTEREXAMPLE
        assert v.minimal == 500 and v.counterexample == 500
    v = pbt.check_property(2, gen_int(0, 100), lambda x: x < 10, LOCAL)
    assert v.failed and v.minimal == 10 and v.counterexample >= 10
    assert v.shrink_steps >= 1 and v.seed == 2 and v.iteration == v.cases_run
    assert "seed=2" in v.message and "minimal=10" in v.message


def test_property_panic_and_generator_errors() -> None:
    def boom(x: int) -> bool:
        raise KeyError("secret-input-detail")

    v = pbt.check_property(1, gen_int(0, 9), boom, LOCAL)
    assert v.failure_id == pbt.FID_PANIC and "secret-input-detail" in v.message
    v = pbt.check_property(1, gen_int(0, 9), boom, REDACT)
    assert v.failure_id == pbt.FID_PANIC and "secret-input-detail" not in v.message
    assert v.seed is None and "KeyError" in v.message

    bad_gen = pbt.Gen(lambda rng: 1 // 0)
    v = pbt.check_property(1, bad_gen, lambda x: True, LOCAL)
    assert v.failure_id == pbt.FID_EXHAUSTED and "ZeroDivisionError" in v.message
    v = pbt.check_property(1, bad_gen, lambda x: True, REDACT)
    assert v.failure_id == pbt.FID_EXHAUSTED and "division" not in v.message

    impossible = gen_int(0, 9).filter(lambda x: False, max_retries=7)
    v = pbt.check_property(1, impossible, lambda x: True, LOCAL)
    assert v.failure_id == pbt.FID_EXHAUSTED and v.cases_run == 0 and v.iteration == 1
    assert "filter precondition" in v.message


@pytest.mark.parametrize(
    "lying_shrinker",
    [
        lambda value, falsifies, budget: (-12345, 3),  # returns a passing value
        lambda value, falsifies, budget: (value + 0, 10**9),  # absurd step count
        lambda value, falsifies, budget: (value, True),  # bool step count
        lambda value, falsifies, budget: 1 / 0,  # raises
    ],
)
def test_a_lying_shrinker_cannot_change_the_verdict(lying_shrinker) -> None:
    v = pbt.check_property(
        3, gen_int(0, 1000), lambda x: x < 500, LOCAL, shrink=lying_shrinker
    )
    assert v.failure_id == pbt.FID_COUNTEREXAMPLE
    assert v.minimal == v.counterexample and v.counterexample >= 500
    assert v.shrink_steps == 0


def test_shrinker_value_the_property_panics_on_is_not_reported() -> None:
    def prop(x: int) -> bool:
        if x == 7:
            raise RuntimeError("panics on the shrunk value")
        return x < 500

    v = pbt.check_property(
        3, gen_int(0, 1000), prop, LOCAL, shrink=lambda value, f, b: (7, 1)
    )
    assert v.failure_id == pbt.FID_COUNTEREXAMPLE
    assert v.minimal == v.counterexample and v.shrink_steps == 0


def test_candidates_the_property_panics_on_are_not_accepted_as_smaller() -> None:
    def prop(x: int) -> bool:
        if x < 300:
            raise RuntimeError("different failure below 300")
        return x < 500

    v = pbt.check_property(3, gen_int(300, 1000), prop, LOCAL)
    assert v.failure_id == pbt.FID_COUNTEREXAMPLE and v.minimal >= 500


def test_ci_redaction_hides_witness_and_seed() -> None:
    v = pbt.check_property(2, gen_int(0, 100), lambda x: x < 10, REDACT)
    assert v.failure_id == pbt.FID_COUNTEREXAMPLE
    assert v.seed is None and v.counterexample is None and v.minimal is None
    assert v.digest is not None and len(v.digest) == 16
    assert "repro-fnv: " + v.digest in v.message
    assert "seed" not in v.message and "initial=" not in v.message
    assert "minimal=" not in v.message
    # default options redact (fail closed)
    d = pbt.check_property(2, gen_int(0, 100), lambda x: x < 10)
    assert d.seed is None and d.minimal is None and d.counterexample is None
    # the digest is deterministic
    again = pbt.check_property(2, gen_int(0, 100), lambda x: x < 10, REDACT)
    assert again.digest == v.digest


def test_ci_from_environ_is_pure() -> None:
    assert pbt.ci_from_environ({"CI": "true"})
    assert pbt.ci_from_environ({"CI": "1"})
    assert not pbt.ci_from_environ({})
    assert not pbt.ci_from_environ({"CI": ""})
    assert not pbt.ci_from_environ({"CI": "0"})
    assert not pbt.ci_from_environ({"CI": " False "})


def test_check_property_validation() -> None:
    with pytest.raises(TypeError):
        pbt.check_property(True, gen_int(), lambda x: True)  # type: ignore[arg-type]
    with pytest.raises(ValueError, match="negative"):
        pbt.check_property(1, gen_int(), lambda x: True, pbt.PbtOptions(num_tests=-1))
    with pytest.raises(ValueError, match="MAX_CASES"):
        pbt.check_property(
            1, gen_int(), lambda x: True, pbt.PbtOptions(num_tests=MAX_CASES + 1)
        )
    with pytest.raises(ValueError, match="MAX_SHRINK_STEPS"):
        pbt.check_property(
            1,
            gen_int(),
            lambda x: True,
            pbt.PbtOptions(max_shrink_steps=MAX_SHRINK_STEPS + 1),
        )
    with pytest.raises(TypeError):
        pbt.check_property(1, gen_int(), lambda x: True, pbt.PbtOptions(num_tests="1"))  # type: ignore[arg-type]


def test_assert_property() -> None:
    pbt.assert_property(gen_int(0, 5), lambda x: x <= 5, LOCAL)
    with pytest.raises(pbt.PbtFailure) as info:
        pbt.assert_property(gen_int(0, 100), lambda x: x < 10, LOCAL, seed=9)
    assert info.value.failure_id == pbt.FID_COUNTEREXAMPLE
    assert "property-counterexample" in str(info.value)


def test_canonical_witness_text() -> None:
    assert pbt._canon(True) == "true"
    assert pbt._canon(-5) == "-5"
    assert pbt._canon(b"\x01\xff") == "0x01ff"
    assert pbt._canon((1, [2, False], b"")) == "[1,[2,false],0x]"
    assert pbt._canon("s") == "'s'"
    v = pbt.check_property(1, gen_bytes(1, 4), lambda b: False, LOCAL)
    assert "minimal=0x00" in v.message


def test_failure_ids_are_closed() -> None:
    assert pbt.PBT_FAILURE_IDS == (
        "property-counterexample",
        "property-exhausted",
        "property-timeout",
        "property-panic",
    )
    assert all(pbt.is_pbt_failure_id(i) for i in pbt.PBT_FAILURE_IDS)
    for bad in ("unregistered-id", "", "Property-Panic", None, 3, b"property-panic"):
        assert not pbt.is_pbt_failure_id(bad)


def test_tuple_shrink_budget_spans_slots_and_bisection_keeps_the_right_half() -> None:
    g = gen_tuple(gen_int(0, 100), gen_int(0, 100), gen_int(0, 100))
    value, steps = g.shrink((50, 50, 50), lambda t: True, 2)
    assert steps == 2 and value == (0, 0, 50)  # budget exhausted before slot 3
    # only the right half still fails (sum >= 18): bisection must keep it
    out, steps = shrink_list([1, 2, 9, 9], 0, lambda xs: sum(xs) < 18)
    assert out == [9, 9] and steps >= 1
