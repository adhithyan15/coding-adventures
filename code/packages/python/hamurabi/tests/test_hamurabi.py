"""Unit and adversarial security tests for Hamurabi simulation core.

Covers:
- Initial state verification (new_game)
- All 8 error arms and refusal precedence
- Integer overflow and wraparound guards (CWE-190)
- Python type-confusion hardening (CWE-843 / CWE-20)
- Even rng_inc rejection and LCG collapse prevention (CWE-400 / CWE-835)
- State invariant checks (VER04m state_inv)
- Immutability and freshness (VER04m fresh)
- Teletype report formatting across all statuses and ratings
- Static capability and AST audit (CAP001, VER08a)
- 1000-case randomized property invariance tests
"""

from __future__ import annotations

import ast
import random
from dataclasses import FrozenInstanceError, replace
from pathlib import Path

import pytest

from hamurabi import (
    GameAlreadyEnded,
    GameOverExtinction,
    HamurabiError,
    HamurabiState,
    Impeached,
    InsufficientAcresToPlant,
    InsufficientAcresToSell,
    InsufficientBushelsForLand,
    InsufficientBushelsToFeed,
    InsufficientGrainForSeed,
    InsufficientPopulationToPlant,
    InvalidOrderBounds,
    Playing,
    Rating,
    TurnOrder,
    Won,
    format_report,
    new_game,
    parse_status,
    step_year,
    validate_order,
)


def test_initial_state() -> None:
    """Verifies that new_game initializes canonical initial conditions."""
    state = new_game(1337)
    assert isinstance(state, HamurabiState)
    assert state.year == 1
    assert state.population == 100
    assert state.bushels == 2800
    assert state.acres == 1000
    assert 17 <= state.land_price <= 26
    assert state.harvest_per_acre == 3
    assert state.rats_eaten == 200
    assert state.starved_last_year == 0
    assert state.starved_total == 0
    assert state.percent_starved_sum == 0
    assert state.immigrants_last_year == 5
    assert not state.plague_struck
    assert state.status == Playing()
    assert (state.rng_inc & 1) == 1


def test_all_eight_error_arms() -> None:
    """Validates refusal precedence and error conditions for all 8 error arms."""
    state = new_game(1337)
    normal_order = TurnOrder(acres_delta=0, bushels_to_feed=2000, acres_to_plant=100)

    # Arm 1: GameAlreadyEnded
    ended_state = replace(state, status=Won(rating=Rating.CHARLEMAGNE))
    with pytest.raises(GameAlreadyEnded):
        validate_order(ended_state, normal_order)
    with pytest.raises(GameAlreadyEnded):
        step_year(ended_state, normal_order)

    # Arm 2: InvalidOrderBounds (corrupted land price or arithmetic overflow on trade)
    bad_state = replace(state, land_price=10)  # valid range is 17..=26
    with pytest.raises(InvalidOrderBounds):
        validate_order(bad_state, normal_order)

    # Also test overflow in bushels_after_trade:
    rich_state = replace(state, acres=2_000_000_000, bushels=4_000_000_000)
    overflow_sell = TurnOrder(
        acres_delta=-1_000_000_000, bushels_to_feed=0, acres_to_plant=0
    )
    with pytest.raises(InvalidOrderBounds):
        validate_order(rich_state, overflow_sell)

    # Arm 3: InsufficientBushelsForLand (trying to buy more than bushels allow)
    buy_too_much = TurnOrder(acres_delta=200, bushels_to_feed=0, acres_to_plant=0)
    with pytest.raises(InsufficientBushelsForLand):
        validate_order(state, buy_too_much)

    # Arm 4: InsufficientAcresToSell (selling more acres than owned)
    sell_too_much = TurnOrder(acres_delta=-1001, bushels_to_feed=0, acres_to_plant=0)
    with pytest.raises(InsufficientAcresToSell):
        validate_order(state, sell_too_much)

    # Arm 5: InsufficientBushelsToFeed (feeding more bushels than available)
    feed_too_much = TurnOrder(acres_delta=0, bushels_to_feed=2801, acres_to_plant=0)
    with pytest.raises(InsufficientBushelsToFeed):
        validate_order(state, feed_too_much)

    # Arm 6: InsufficientAcresToPlant (planting more acres than owned)
    plant_too_much = TurnOrder(acres_delta=0, bushels_to_feed=1000, acres_to_plant=1001)
    with pytest.raises(InsufficientAcresToPlant):
        validate_order(state, plant_too_much)

    # Arm 7: InsufficientPopulationToPlant (1 person tends 10 acres)
    small_pop_state = replace(state, population=50)  # can only tend 500
    plant_too_much_pop = TurnOrder(
        acres_delta=0, bushels_to_feed=1000, acres_to_plant=501
    )
    with pytest.raises(InsufficientPopulationToPlant):
        validate_order(small_pop_state, plant_too_much_pop)

    # Arm 8: InsufficientGrainForSeed (1 bushel seeds 2 acres)
    # Bushels left after feed: 2800 - 2500 = 300. 800 acres needs 400 bushels seed.
    seed_starved = TurnOrder(
        acres_delta=0, bushels_to_feed=2500, acres_to_plant=800
    )
    with pytest.raises(InsufficientGrainForSeed):
        validate_order(state, seed_starved)


def test_cwe_190_overflow_guard() -> None:
    """Verifies integer overflow and wraparound defenses."""
    state = new_game(42)

    # Huge acres_delta to buy: checked mul must not overflow
    huge_buy = TurnOrder(
        acres_delta=2147483647, bushels_to_feed=0, acres_to_plant=0
    )
    with pytest.raises(InsufficientBushelsForLand):
        validate_order(state, huge_buy)

    # Negative minimum i32
    min_sell = TurnOrder(
        acres_delta=-2147483648, bushels_to_feed=0, acres_to_plant=0
    )
    with pytest.raises(InsufficientAcresToSell):
        validate_order(state, min_sell)

    # Huge acres_to_plant
    huge_plant = TurnOrder(
        acres_delta=0, bushels_to_feed=0, acres_to_plant=0xFFFFFFFF
    )
    with pytest.raises(InsufficientAcresToPlant):
        validate_order(state, huge_plant)

    # Harvest overflow guard: 900M acres * 5 = 4.5B > 2^32-1
    rich_state = replace(
        state,
        population=100_000_000,
        acres=1_000_000_000,
        bushels=2_000_000_000,
    )
    overflow_harvest_order = TurnOrder(
        acres_delta=0, bushels_to_feed=0, acres_to_plant=900_000_000
    )
    with pytest.raises(InvalidOrderBounds):
        validate_order(rich_state, overflow_harvest_order)
    with pytest.raises(InvalidOrderBounds):
        step_year(rich_state, overflow_harvest_order)

    # Acres overflow after buy
    max_acres_state = replace(state, acres=0xFFFFFFFF, bushels=0xFFFFFFFF)
    buy_one_overflow = TurnOrder(acres_delta=1, bushels_to_feed=0, acres_to_plant=0)
    with pytest.raises(InvalidOrderBounds):
        validate_order(max_acres_state, buy_one_overflow)

    # Max labor capacity overflow
    huge_pop_state = replace(state, population=500_000_000)
    order = TurnOrder(acres_delta=0, bushels_to_feed=0, acres_to_plant=100)
    with pytest.raises(InvalidOrderBounds):
        validate_order(huge_pop_state, order)

    # Saturating addition for starved_total and percent_starved_sum
    saturated_state = replace(
        state, starved_total=0xFFFFFFFF, percent_starved_sum=0xFFFFFFFF - 10
    )
    order = TurnOrder(acres_delta=0, bushels_to_feed=1000, acres_to_plant=0)
    next_s = step_year(saturated_state, order)
    assert next_s.starved_total == 0xFFFFFFFF
    assert next_s.percent_starved_sum == 0xFFFFFFFF


def test_cwe_843_type_confusion_hardening() -> None:
    """Verifies strict type enforcement (rejecting bool, float, str, None)."""
    # TurnOrder type confusion
    for invalid in [True, False, 1.5, "100", None, complex(1, 2)]:
        with pytest.raises(InvalidOrderBounds):
            TurnOrder(acres_delta=invalid, bushels_to_feed=2000, acres_to_plant=100)  # type: ignore[arg-type]
        with pytest.raises(InvalidOrderBounds):
            TurnOrder(acres_delta=0, bushels_to_feed=invalid, acres_to_plant=100)  # type: ignore[arg-type]
        with pytest.raises(InvalidOrderBounds):
            TurnOrder(acres_delta=0, bushels_to_feed=2000, acres_to_plant=invalid)  # type: ignore[arg-type]

    # TurnOrder integer range checks
    with pytest.raises(InvalidOrderBounds):
        TurnOrder(acres_delta=2147483648, bushels_to_feed=0, acres_to_plant=0)
    with pytest.raises(InvalidOrderBounds):
        TurnOrder(acres_delta=-2147483649, bushels_to_feed=0, acres_to_plant=0)
    with pytest.raises(InvalidOrderBounds):
        TurnOrder(acres_delta=0, bushels_to_feed=0x1_0000_0000, acres_to_plant=0)
    with pytest.raises(InvalidOrderBounds):
        TurnOrder(acres_delta=0, bushels_to_feed=0, acres_to_plant=0x1_0000_0000)

    # new_game seed type confusion & bounds
    for invalid_seed in [True, False, 1.5, "42", None, -1, 0x1_0000_0000_0000_0000]:
        with pytest.raises(InvalidOrderBounds):
            new_game(invalid_seed)  # type: ignore[arg-type]

    # HamurabiState field type confusion
    state = new_game(1)
    with pytest.raises(InvalidOrderBounds):
        replace(state, year=True)  # bool rejected for int
    with pytest.raises(InvalidOrderBounds):
        replace(state, population=100.5)  # float rejected
    with pytest.raises(InvalidOrderBounds):
        replace(state, plague_struck=1)  # int rejected for bool
    with pytest.raises(InvalidOrderBounds):
        replace(state, rng_state=0x1_0000_0000_0000_0000)  # u64 overflow
    with pytest.raises(InvalidOrderBounds):
        replace(state, rng_inc=0x1_0000_0000_0000_0000)  # u64 overflow
    with pytest.raises(InvalidOrderBounds):
        replace(state, status=123)  # type: ignore[arg-type]

    # validate_order parameter type checks
    with pytest.raises(InvalidOrderBounds):
        validate_order("invalid_state", TurnOrder(0, 0, 0))  # type: ignore[arg-type]
    with pytest.raises(InvalidOrderBounds):
        validate_order(state, "invalid_order")  # type: ignore[arg-type]


def test_cwe_400_even_rng_inc_refusal() -> None:
    """Verifies that even rng_inc is rejected to prevent LCG period collapse."""
    state = new_game(1)
    order = TurnOrder(acres_delta=0, bushels_to_feed=2000, acres_to_plant=1000)

    for even_inc in [0, 2, 4, 108]:
        bad_inc_state = replace(state, rng_inc=even_inc)
        with pytest.raises(InvalidOrderBounds):
            validate_order(bad_inc_state, order)
        with pytest.raises(InvalidOrderBounds):
            step_year(bad_inc_state, order)


def test_adversarial_state_inv_bounds() -> None:
    """Verifies domain boundary checks (year, land_price, harvest_per_acre)."""
    state = new_game(1)
    order = TurnOrder(acres_delta=0, bushels_to_feed=2000, acres_to_plant=1000)

    # Invalid years (valid: 1..=10 for active play)
    for bad_year in [0, 11, 255]:
        bad_state = replace(state, year=bad_year)
        with pytest.raises(InvalidOrderBounds):
            validate_order(bad_state, order)

    # Invalid land prices (valid: 17..=26)
    for bad_price in [0, 16, 27, 100]:
        bad_state = replace(state, land_price=bad_price)
        with pytest.raises(InvalidOrderBounds):
            validate_order(bad_state, order)

    # Invalid harvest per acre (valid: 1..=5)
    for bad_harvest in [0, 6, 99]:
        bad_state = replace(state, harvest_per_acre=bad_harvest)
        with pytest.raises(InvalidOrderBounds):
            validate_order(bad_state, order)


def test_cwe_502_zero_deserialization_and_injection() -> None:
    """Verifies zero eval/exec/pickle surface in hamurabi package."""
    src_dir = Path(__file__).parents[1] / "src" / "hamurabi"
    for py_file in src_dir.glob("*.py"):
        code = py_file.read_text(encoding="utf-8")
        assert "eval(" not in code, f"eval found in {py_file}"
        assert "exec(" not in code, f"exec found in {py_file}"
        assert "pickle" not in code, f"pickle found in {py_file}"


def test_immutability_and_ver04m_freshness() -> None:
    """Verifies that records are frozen and step_year produces fresh instances."""
    state = new_game(10)
    order = TurnOrder(acres_delta=0, bushels_to_feed=2000, acres_to_plant=800)

    with pytest.raises(FrozenInstanceError):
        state.year = 2  # type: ignore[misc]

    with pytest.raises(FrozenInstanceError):
        order.acres_delta = 10  # type: ignore[misc]

    next_state = step_year(state, order)
    assert next_state is not state
    assert next_state.year == 2
    assert state.year == 1  # original state untouched


def test_impeachment_over_45_percent_starvation() -> None:
    """Verifies immediate impeachment when > 45% starvation occurs."""
    state = new_game(100)
    # Feed 0 bushels to 100 people -> 100 starve (100% > 45%)
    starve_order = TurnOrder(acres_delta=0, bushels_to_feed=0, acres_to_plant=0)
    next_state = step_year(state, starve_order)

    assert isinstance(next_state.status, Impeached)
    assert next_state.status.starved_this_turn == 100
    assert next_state.status.percent == 100

    report = format_report(next_state)
    assert "YOU STARVED 100 PEOPLE IN ONE YEAR!!!" in report
    assert "IMPEACHED AND THROWN OUT OF OFFICE" in report


def test_extinction_when_all_survivors_zero() -> None:
    """Verifies GameOverExtinction condition."""
    state = new_game(1)
    # Force state where population is 0
    extinct_state = replace(state, population=0)
    order = TurnOrder(acres_delta=0, bushels_to_feed=0, acres_to_plant=0)
    next_state = step_year(extinct_state, order)
    assert isinstance(next_state.status, GameOverExtinction)
    assert "EXTINCT!" in format_report(next_state)


def test_all_four_ratings_format_report() -> None:
    """Verifies teletype report generation for all 4 rating tiers."""
    base = replace(new_game(1), year=11)

    # 1. Charlemagne
    won_c = replace(base, status=Won(rating=Rating.CHARLEMAGNE))
    assert "CHARLEMAGNE, DISRAELI, AND JEFFERSON" in format_report(won_c)

    # 2. Acre
    won_a = replace(base, status=Won(rating=Rating.ACRE))
    assert "REASONABLE ACREAGE" in format_report(won_a)

    # 3. Nero
    won_n = replace(base, status=Won(rating=Rating.NERO))
    assert "NERO WOULD HAVE BEEN PROUD OF YOU" in format_report(won_n)

    # 4. Disaster
    won_d = replace(base, status=Won(rating=Rating.DISASTER))
    assert "OFFICIAL NATIONAL DISASTER" in format_report(won_d)


def test_full_10_year_deterministic_game() -> None:
    """Simulates a complete 10-year term to completion and victory."""
    state = new_game(4242)

    for expected_year in range(1, 11):
        assert state.year == expected_year
        assert isinstance(state.status, Playing)

        feed_needed = state.population * 20
        # If low on bushels, sell land; if surplus, buy land
        if state.bushels < feed_needed + 500:
            deficit = (feed_needed + 500) - state.bushels
            acres_to_sell = min(
                (deficit + state.land_price - 1) // state.land_price,
                state.acres // 3,
            )
            acres_delta = -acres_to_sell
        elif state.bushels > feed_needed + 2000:
            surplus = state.bushels - (feed_needed + 1000)
            acres_delta = min(surplus // state.land_price, 50)
        else:
            acres_delta = 0

        if acres_delta >= 0:
            acres_after_trade = state.acres + acres_delta
            bushels_after_trade = state.bushels - (acres_delta * state.land_price)
        else:
            acres_after_trade = state.acres - (-acres_delta)
            bushels_after_trade = state.bushels + ((-acres_delta) * state.land_price)

        bushels_to_feed = min(feed_needed, bushels_after_trade)
        bushels_remaining = bushels_after_trade - bushels_to_feed

        max_plantable = min(acres_after_trade, state.population * 10)
        max_from_seed = bushels_remaining * 2
        acres_to_plant = min(max_plantable, max_from_seed)

        order = TurnOrder(
            acres_delta=acres_delta,
            bushels_to_feed=bushels_to_feed,
            acres_to_plant=acres_to_plant,
        )

        state = step_year(state, order)

    assert state.year == 11
    assert isinstance(state.status, Won)
    assert state.status.rating in [
        Rating.CHARLEMAGNE,
        Rating.ACRE,
        Rating.NERO,
        Rating.DISASTER,
    ]
    report = format_report(state)
    assert "IN YOUR 10-YEAR TERM OF OFFICE" in report


def test_plague_population_reduction() -> None:
    """Verifies that plague strikes and halves population."""
    found_plague = False
    order = TurnOrder(acres_delta=0, bushels_to_feed=2000, acres_to_plant=0)
    for seed in range(1000):
        state = new_game(seed)
        next_s = step_year(state, order)
        if next_s.plague_struck:
            found_plague = True
            assert next_s.population >= 50
            report = format_report(next_s)
            assert "A HORRIBLE PLAGUE STRUCK! HALF THE PEOPLE DIED." in report
            break
    assert found_plague, "Expected plague occurrence in 1000 random seeds"


def test_status_and_error_representations() -> None:
    """Tests string representation and equality for status variants and errors."""
    # Playing
    p = Playing()
    assert str(p) == "Playing"
    assert p == "Playing"
    assert p == Playing()
    assert p != 123

    # Won
    w = Won(Rating.CHARLEMAGNE)
    assert str(w) == "Won(Charlemagne)"
    assert w == "Won(Charlemagne)"
    assert w == Won(Rating.CHARLEMAGNE)
    assert w != Won(Rating.ACRE)
    assert w != 123

    # Impeached
    imp = Impeached(50, 50)
    assert str(imp) == "Impeached(starved=50, percent=50)"
    assert imp == "Impeached(starved=50, percent=50)"
    assert imp == Impeached(50, 50)
    assert imp != Impeached(50, 40)
    assert imp != 123

    # GameOverExtinction
    ext = GameOverExtinction()
    assert str(ext) == "GameOverExtinction"
    assert ext == "GameOverExtinction"
    assert ext == GameOverExtinction()
    assert ext != 123

    # parse_status
    assert parse_status(p) is p
    assert parse_status("Playing") == Playing()
    assert parse_status("GameOverExtinction") == GameOverExtinction()
    assert parse_status("Won(Charlemagne)") == Won(Rating.CHARLEMAGNE)
    str_status_state = replace(new_game(1), status="Won(Charlemagne)")
    assert str_status_state.status == Won(Rating.CHARLEMAGNE)
    with pytest.raises(InvalidOrderBounds):
        parse_status(123)  # type: ignore[arg-type]
    with pytest.raises(InvalidOrderBounds):
        parse_status("InvalidUnknownStatus")

    # HamurabiError
    err = HamurabiError("HamurabiError")
    assert str(err) == "HamurabiError"
    assert err == HamurabiError
    assert err == "HamurabiError"
    assert err == HamurabiError("HamurabiError")
    assert err != 123
    default_err = HamurabiError()
    assert str(default_err) == HamurabiError.__doc__


def test_static_capability_audit_ver08a() -> None:
    """Scans src AST to verify zero ambient capability imports (CAP001, VER08a)."""
    src_file = Path(__file__).parents[1] / "src" / "hamurabi" / "__init__.py"
    tree = ast.parse(src_file.read_text(encoding="utf-8"))

    forbidden_modules = {
        "os",
        "sys",
        "subprocess",
        "socket",
        "http",
        "urllib",
        "requests",
        "time",
        "threading",
        "multiprocessing",
        "asyncio",
        "pickle",
        "shutil",
        "ctypes",
    }

    for node in ast.walk(tree):
        if isinstance(node, ast.Import):
            for alias in node.names:
                root_mod = alias.name.split(".")[0]
                assert (
                    root_mod not in forbidden_modules
                ), f"Forbidden import: {alias.name}"
        elif isinstance(node, ast.ImportFrom) and node.module:
            root_mod = node.module.split(".")[0]
            assert (
                root_mod not in forbidden_modules
            ), f"Forbidden import from: {node.module}"


def test_property_based_invariants() -> None:
    """Executes 1,000 randomized state transitions asserting mathematical invariants."""
    rng = random.Random(42)

    for _ in range(1000):
        seed = rng.randint(0, 0xFFFFFFFF)
        state = new_game(seed)

        # Generate random orders (mix of valid and hostile bounds)
        acres_delta = rng.randint(-1500, 1500)
        bushels_to_feed = rng.randint(0, 5000)
        acres_to_plant = rng.randint(0, 2000)

        order = TurnOrder(
            acres_delta=acres_delta,
            bushels_to_feed=bushels_to_feed,
            acres_to_plant=acres_to_plant,
        )

        try:
            validate_order(state, order)
            valid = True
        except HamurabiError:
            valid = False

        if valid:
            next_state = step_year(state, order)
            # Conservation & domain invariants
            assert 1 <= next_state.year <= 11
            assert 17 <= next_state.land_price <= 26
            assert 1 <= next_state.harvest_per_acre <= 5
            assert (next_state.rng_inc & 1) == 1
            assert 0 <= next_state.population <= 0xFFFFFFFF
            assert 0 <= next_state.bushels <= 0xFFFFFFFF
            assert 0 <= next_state.acres <= 0xFFFFFFFF
            assert next_state.starved_total >= state.starved_total
            assert next_state.percent_starved_sum >= state.percent_starved_sum
        else:
            with pytest.raises(HamurabiError):
                step_year(state, order)
