"""Tests for property-based testing and shrinking."""

from itertools import islice

import pytest

import coding_adventures_test_bootstrap.pbt as pbt
from coding_adventures_test_bootstrap.pbt import (
    MAX_CASES,
    MAX_INPUT_BYTES,
    MAX_LIST_LEN,
    MAX_SHRINK_STEPS,
    for_all,
    gen_bool,
    gen_bytes,
    gen_choice,
    gen_float,
    gen_int,
    gen_list,
    gen_one_of,
    gen_text,
    gen_u8,
    gen_u64,
    shrink_bytes,
    shrink_int,
    shrink_list,
    shrink_text,
    shrink_value,
)
from coding_adventures_test_bootstrap.prng import SplitMix64


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


def test_shrinking_primitives() -> None:
    # int
    shrunk_i = list(shrink_int(100, 0))
    assert 0 in shrunk_i
    assert 50 in shrunk_i
    assert list(shrink_int(0, 0)) == []

    with pytest.raises(TypeError):
        list(shrink_int(True, 0))  # type: ignore[arg-type]
    with pytest.raises(TypeError):
        list(shrink_int(0, False))  # type: ignore[arg-type]

    # Bound the negative regression so the old floor-division loop fails
    # promptly instead of hanging the test process.
    assert list(islice(shrink_int(-8), 5)) == [0, -4, -6, -7]
    assert list(shrink_int(8, 3)) == [3, 6, 7]
    assert list(shrink_int(-8, -3)) == [-3, -6, -7]
    assert list(shrink_int(8, 8)) == []

    # Binary bisection emits the target and each halved distance once, even
    # for translated targets and integers far beyond machine word sizes.
    for val in range(-16, 17):
        for target in range(-16, 17):
            proposals = list(shrink_int(val, target))
            assert len(proposals) == len(set(proposals))
            assert all(abs(item - target) < abs(val - target) for item in proposals)
            if val != target:
                assert proposals[0] == target
                assert len(proposals) == abs(val - target).bit_length()
    huge_val = (1 << 4096) + 123
    huge_target = -(1 << 2048) + 17
    huge_proposals = list(shrink_int(huge_val, huge_target))
    assert len(huge_proposals) == abs(huge_val - huge_target).bit_length()
    assert len(huge_proposals) == len(set(huge_proposals))
    assert all(
        abs(item - huge_target) < abs(huge_val - huge_target)
        for item in huge_proposals
    )

    # bytes
    shrunk_b = list(shrink_bytes(b"hello world"))
    assert b"" in shrunk_b

    assert list(shrink_bytes(b"")) == []
    # zeroing check
    zeroed = list(shrink_bytes(b"\x01"))
    assert b"\x00" in zeroed

    # text
    shrunk_t = list(shrink_text("testing"))
    assert "" in shrunk_t
    assert list(shrink_text("")) == []
    simpler = list(shrink_text("z"))
    assert "a" in simpler

    # list
    shrunk_l = list(shrink_list([10, 20, 30]))
    assert [] in shrunk_l
    assert list(shrink_list([])) == []

    # shrink_value
    assert list(shrink_value(True)) == [False]
    assert list(shrink_value(False)) == []
    assert len(list(shrink_value((1, 2)))) > 0


def test_for_all_success() -> None:
    # Commutative addition
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


def test_seeded_integer_shrink_properties() -> None:
    limit = (1 << 63) - 1

    def shrink_invariants(value: int, target: int) -> bool:
        proposals = list(shrink_int(value, target))
        return (
            (not proposals if value == target else proposals[0] == target)
            and len(proposals) == len(set(proposals))
            and all(
                abs(candidate - target) < abs(value - target)
                for candidate in proposals
            )
            and len(proposals) == abs(value - target).bit_length()
        )

    result = for_all(
        shrink_invariants,
        gen_int(-limit, limit),
        gen_int(-limit, limit),
        count=256,
        seed=0x5A17,
    )
    assert result.passed
    assert result.cases_run == 256


def test_for_all_failure_counterexample_and_shrinking() -> None:
    # Property: all integers < 10 (will fail for >= 10)
    res = for_all(
        lambda x: x < 10,
        gen_int(0, 100),
        count=50,
        seed=42,
    )
    assert not res.passed
    assert res.counterexample is not None
    assert res.counterexample[0] >= 10
    assert res.shrunken is not None
    assert res.shrunken[0] >= 10
    assert res.error == "Property returned False"


def test_for_all_shrinks_negative_only_witness() -> None:
    fixed_negative = pbt.Gen(lambda rng: -8)
    res = for_all(
        lambda x: x >= 0,
        fixed_negative,
        count=1,
        max_shrink_steps=100,
    )
    assert not res.passed
    assert res.counterexample == (-8,)
    assert res.shrunken == (-1,)
    assert res.shrink_steps <= 100


@pytest.mark.parametrize("witness", [64, -8])
def test_for_all_consumes_only_the_shrink_step_budget(witness: int) -> None:
    calls = 0

    def fails_only_at_witness(value: int) -> bool:
        nonlocal calls
        calls += 1
        return value != witness

    fixed = pbt.Gen(lambda rng: witness)
    result = for_all(
        fails_only_at_witness,
        fixed,
        count=1,
        max_shrink_steps=1,
    )
    assert not result.passed
    assert result.counterexample == (witness,)
    assert result.shrink_steps == 1
    assert calls == 2  # one generated value and one consumed proposal


def test_for_all_zero_shrink_budget_does_not_consume_proposals() -> None:
    calls = 0

    def always_fails(value: int) -> bool:
        nonlocal calls
        calls += 1
        return False

    fixed = pbt.Gen(lambda rng: 9)
    result = for_all(always_fails, fixed, count=1, max_shrink_steps=0)
    assert result.counterexample == (9,)
    assert result.shrunken == (9,)
    assert result.shrink_steps == 0
    assert calls == 1


def test_for_all_duplicate_shrinker_proposals_still_use_budget(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    calls = 0
    yielded = 0

    def always_fails(value: int) -> bool:
        nonlocal calls
        calls += 1
        return False

    def repeating_shrinker(value: int):
        nonlocal yielded
        while True:
            yielded += 1
            yield value

    monkeypatch.setattr(pbt, "shrink_value", repeating_shrinker)
    result = for_all(
        always_fails,
        pbt.Gen(lambda rng: 9),
        count=1,
        max_shrink_steps=3,
    )
    assert result.shrunken == (9,)
    assert result.shrink_steps == 3
    assert yielded == 3
    assert calls == 1  # duplicate proposals are counted, not rechecked


def test_for_all_exception_handling() -> None:
    def buggy_fn(x: int) -> bool:
        if x > 20:
            raise ArithmeticError("Value too large")
        return True

    res = for_all(
        buggy_fn,
        gen_int(0, 50),
        count=50,
        seed=99,
    )
    assert not res.passed
    assert "ArithmeticError" in (res.error or "")


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
