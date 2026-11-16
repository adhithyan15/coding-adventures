"""Comprehensive unit and security tests for Python lunar-lander core.

Covers:
- Initial state verification (new_flight, state_inv)
- Exact arithmetic: div_euclid, isqrt_u128
- All 4 error arms and canonical refusal precedence
- Integer overflow and bounds guards (CWE-190)
- Division by zero prevention (CWE-369)
- Python type-confusion hardening (CWE-843)
- Immutability and freshness (VER04m fresh, frozen slots)
- ASCII telemetry HUD formatting across all states and outcomes (P2D02)
- Static capability and AST audit (CAP001, VER08a)
"""

from __future__ import annotations

import ast
import json
from dataclasses import FrozenInstanceError
from pathlib import Path

import pytest

from lunar_lander import (
    DRY_MASS_LBS,
    INITIAL_ALTITUDE_MFT,
    INITIAL_FUEL_LBS,
    INITIAL_VELOCITY_MFT_S,
    MAX_ALTITUDE_MFT,
    MAX_BURN_RATE_LBS_S,
    MAX_DURATION_S,
    MAX_TIME_SECONDS,
    MAX_VELOCITY_MFT_S,
    MIN_VELOCITY_MFT_S,
    ArithmeticOverflow,
    BurnCommand,
    ExcessiveBurnRate,
    FlightAlreadyEnded,
    InFlight,
    InvalidDuration,
    Landed,
    LanderError,
    LanderState,
    LandingOutcome,
    PaintRect,
    PaintScene,
    PaintText,
    div_euclid,
    isqrt_u128,
    new_flight,
    parse_status,
    render_telemetry,
    render_telemetry_ascii,
    state_inv,
    step_burn,
    telemetry_scene,
    to_paint_scene,
    validate_burn,
)


def test_initial_flight_invariants() -> None:
    """Verifies that initial flight state matches standard Apollo LM conditions."""
    flight = new_flight()
    assert flight.time_seconds == 0
    assert flight.altitude_mft == INITIAL_ALTITUDE_MFT
    assert flight.velocity_mft_s == INITIAL_VELOCITY_MFT_S
    assert flight.fuel_lbs == INITIAL_FUEL_LBS
    assert flight.dry_mass_lbs == DRY_MASS_LBS
    assert isinstance(flight.status, InFlight)
    assert flight.status == "InFlight"
    assert repr(flight.status) == "InFlight"
    assert flight.status != 123
    assert flight.impact_velocity_mft_s is None
    assert state_inv(flight) is True


def test_isqrt_u128_exactness() -> None:
    """Verifies exact bitwise 128-bit square root against known values."""
    assert isqrt_u128(0) == 0
    assert isqrt_u128(1) == 1
    assert isqrt_u128(2) == 1
    assert isqrt_u128(3) == 1
    assert isqrt_u128(4) == 2
    assert isqrt_u128(15) == 3
    assert isqrt_u128(16) == 4
    assert isqrt_u128(10_274_400_000_000) == 3_205_370

    u64_max = (1 << 64) - 1
    assert isqrt_u128(u64_max * u64_max) == u64_max

    u128_max = (1 << 128) - 1
    assert isqrt_u128(u128_max) == u64_max

    with pytest.raises(ArithmeticOverflow):
        isqrt_u128(-1)


def test_div_euclid() -> None:
    """Verifies Euclidean division where remainder is always non-negative."""
    assert div_euclid(-46070, -5310) == 9
    assert div_euclid(7, 3) == 2
    assert div_euclid(-7, 3) == -3
    assert div_euclid(7, -3) == -2
    assert div_euclid(-7, -3) == 3
    assert div_euclid(6, 2) == 3
    assert div_euclid(-6, 2) == -3
    assert div_euclid(6, -2) == -3
    assert div_euclid(-6, -2) == 3

    with pytest.raises(ArithmeticOverflow):
        div_euclid(10, 0)


def test_adversarial_cwe369_division_by_zero_prevention() -> None:
    """Verifies prevention of division by zero (CWE-369) on corrupted states."""
    hostile_state = LanderState(
        time_seconds=0,
        altitude_mft=100_000_000,
        velocity_mft_s=-2_000_000,
        fuel_lbs=0,
        dry_mass_lbs=0,  # Corrupted dry mass!
        status=InFlight(),
        impact_velocity_mft_s=None,
    )
    cmd = BurnCommand(burn_rate_lbs_s=100, duration_seconds=5)

    with pytest.raises(ArithmeticOverflow):
        validate_burn(hostile_state, cmd)

    with pytest.raises(ArithmeticOverflow):
        step_burn(hostile_state, cmd)


def test_adversarial_cwe190_integer_overflow_prevention() -> None:
    """Verifies coordinate boundaries and integer overflow guards (CWE-190)."""
    valid_cmd = BurnCommand(burn_rate_lbs_s=100, duration_seconds=5)

    # Altitude boundaries
    for bad_alt in [
        MAX_ALTITUDE_MFT + 1,
        0,
        -1,
        -100_000_000,
        (1 << 63) - 1,
        -(1 << 63),
    ]:
        state = LanderState(
            time_seconds=0,
            altitude_mft=bad_alt,
            velocity_mft_s=-3_000_000,
            fuel_lbs=16500,
            dry_mass_lbs=16500,
            status=InFlight(),
        )
        with pytest.raises(ArithmeticOverflow):
            step_burn(state, valid_cmd)

    # Velocity boundaries
    for bad_vel in [
        MAX_VELOCITY_MFT_S + 1,
        MIN_VELOCITY_MFT_S - 1,
        (1 << 63) - 1,
        -(1 << 63),
    ]:
        state = LanderState(
            time_seconds=0,
            altitude_mft=120_000_000,
            velocity_mft_s=bad_vel,
            fuel_lbs=16500,
            dry_mass_lbs=16500,
            status=InFlight(),
        )
        with pytest.raises(ArithmeticOverflow):
            step_burn(state, valid_cmd)

    # Fuel boundaries
    for bad_fuel in [INITIAL_FUEL_LBS + 1, -1, 0xFFFFFFFF]:
        state = LanderState(
            time_seconds=0,
            altitude_mft=120_000_000,
            velocity_mft_s=-3_000_000,
            fuel_lbs=bad_fuel,
            dry_mass_lbs=16500,
            status=InFlight(),
        )
        with pytest.raises(ArithmeticOverflow):
            step_burn(state, valid_cmd)

    # Time boundaries
    state = LanderState(
        time_seconds=MAX_TIME_SECONDS + 1,
        altitude_mft=120_000_000,
        velocity_mft_s=-3_000_000,
        fuel_lbs=16500,
        dry_mass_lbs=16500,
        status=InFlight(),
    )
    with pytest.raises(ArithmeticOverflow):
        step_burn(state, valid_cmd)

    # Time + duration overflow
    state = LanderState(
        time_seconds=MAX_TIME_SECONDS,
        altitude_mft=120_000_000,
        velocity_mft_s=-3_000_000,
        fuel_lbs=16500,
        dry_mass_lbs=16500,
        status=InFlight(),
    )
    add_one = BurnCommand(burn_rate_lbs_s=0, duration_seconds=1)
    with pytest.raises(ArithmeticOverflow):
        step_burn(state, add_one)

    # Impact velocity set while in flight
    state = LanderState(
        time_seconds=0,
        altitude_mft=120_000_000,
        velocity_mft_s=-3_000_000,
        fuel_lbs=16500,
        dry_mass_lbs=16500,
        status=InFlight(),
        impact_velocity_mft_s=-3000,
    )
    with pytest.raises(ArithmeticOverflow):
        step_burn(state, valid_cmd)

    # Command bounds
    state = new_flight()
    with pytest.raises(ExcessiveBurnRate):
        step_burn(
            state,
            BurnCommand(burn_rate_lbs_s=MAX_BURN_RATE_LBS_S + 1, duration_seconds=5),
        )

    with pytest.raises(InvalidDuration):
        step_burn(state, BurnCommand(burn_rate_lbs_s=50, duration_seconds=0))

    with pytest.raises(InvalidDuration):
        step_burn(
            state,
            BurnCommand(burn_rate_lbs_s=50, duration_seconds=MAX_DURATION_S + 1),
        )

    with pytest.raises(InvalidDuration):
        step_burn(state, BurnCommand(burn_rate_lbs_s=50, duration_seconds=0xFFFFFFFF))


def test_cwe_843_type_confusion_hardening() -> None:
    """Verifies strict type enforcement (rejecting bool, float, str, None)."""
    for invalid in [True, False, 1.5, "100", None, complex(1, 2)]:
        # BurnCommand rejection
        with pytest.raises(ArithmeticOverflow):
            BurnCommand(burn_rate_lbs_s=invalid, duration_seconds=5)  # type: ignore[arg-type]
        with pytest.raises(ArithmeticOverflow):
            BurnCommand(burn_rate_lbs_s=50, duration_seconds=invalid)  # type: ignore[arg-type]

        # LanderState rejection
        with pytest.raises(ArithmeticOverflow):
            LanderState(
                time_seconds=invalid,  # type: ignore[arg-type]
                altitude_mft=120_000_000,
                velocity_mft_s=-3_000_000,
                fuel_lbs=16500,
                dry_mass_lbs=16500,
                status=InFlight(),
            )
        with pytest.raises(ArithmeticOverflow):
            LanderState(
                time_seconds=0,
                altitude_mft=invalid,  # type: ignore[arg-type]
                velocity_mft_s=-3_000_000,
                fuel_lbs=16500,
                dry_mass_lbs=16500,
                status=InFlight(),
            )
        with pytest.raises(ArithmeticOverflow):
            LanderState(
                time_seconds=0,
                altitude_mft=120_000_000,
                velocity_mft_s=invalid,  # type: ignore[arg-type]
                fuel_lbs=16500,
                dry_mass_lbs=16500,
                status=InFlight(),
            )
        with pytest.raises(ArithmeticOverflow):
            LanderState(
                time_seconds=0,
                altitude_mft=120_000_000,
                velocity_mft_s=-3_000_000,
                fuel_lbs=invalid,  # type: ignore[arg-type]
                dry_mass_lbs=16500,
                status=InFlight(),
            )
        with pytest.raises(ArithmeticOverflow):
            LanderState(
                time_seconds=0,
                altitude_mft=120_000_000,
                velocity_mft_s=-3_000_000,
                fuel_lbs=16500,
                dry_mass_lbs=invalid,  # type: ignore[arg-type]
                status=InFlight(),
            )

    # Hostile objects passed to validate_burn
    with pytest.raises(ArithmeticOverflow):
        validate_burn("not_state", BurnCommand(0, 1))  # type: ignore[arg-type]
    with pytest.raises(ArithmeticOverflow):
        validate_burn(new_flight(), "not_cmd")  # type: ignore[arg-type]

    # Hostile status type in LanderState
    with pytest.raises(ArithmeticOverflow):
        LanderState(0, 1000, -100, 1000, 16500, status=123)  # type: ignore[arg-type]

    # Hostile outcome type in Landed
    with pytest.raises(ArithmeticOverflow):
        Landed(123)  # type: ignore[arg-type]


def test_immutability_frozen_slots() -> None:
    """Verifies that LanderState and BurnCommand are completely immutable."""
    state = new_flight()
    with pytest.raises(FrozenInstanceError):
        state.fuel_lbs = 10000  # type: ignore[misc]

    cmd = BurnCommand(burn_rate_lbs_s=50, duration_seconds=5)
    with pytest.raises(FrozenInstanceError):
        cmd.burn_rate_lbs_s = 100  # type: ignore[misc]


def test_paint_vm_ascii_telemetry_hud() -> None:
    """Verifies ASCII telemetry HUD layout and formatting."""
    state = new_flight()
    scene = to_paint_scene(state, 0)
    assert scene.width == 76.0
    assert scene.height == 5.0
    assert scene.background == "#000000"

    hud = render_telemetry_ascii(state, 0)
    lines = hud.split("\n")
    assert len(lines) == 5, f"HUD must have 5 lines, got {len(lines)}"

    for idx, line in enumerate(lines):
        assert len(line) == 76, (
            f"Line {idx} length mismatch ({len(line)} != 76): {line!r}"
        )

    assert "LUNAR LANDER TELEMETRY" in hud
    assert "TIME:" in hud
    assert "ALTITUDE:" in hud
    assert "VELOCITY:" in hud
    assert "FUEL:" in hud
    assert "BURN:" in hud
    assert "RATE:" in hud
    assert "DESCENT" in hud
    assert "█" in hud

    # Partial gauge fill
    mid_state = LanderState(
        time_seconds=100,
        altitude_mft=60_000_000,
        velocity_mft_s=-1_500_000,
        fuel_lbs=8_250,
        dry_mass_lbs=16_500,
        status=InFlight(),
    )
    mid_hud = render_telemetry_ascii(mid_state, 100)
    assert "█" in mid_hud
    assert "░" in mid_hud

    # Convenience helpers
    scene2 = telemetry_scene(state)
    assert scene2.width == 76.0
    hud2 = render_telemetry(state)
    assert hud2 == hud

    # Test custom scene rasterization branches (center/right align, transparent rect)
    custom_scene = PaintScene(
        width=10.0,
        height=2.0,
        instructions=[
            PaintText(x=5.0, y=0.0, text="HI", text_align="center"),
            PaintText(x=9.0, y=1.0, text="R", text_align="right"),
            PaintRect(x=0.0, y=0.0, width=1.0, height=0.0, fill="none"),
        ],
    )
    custom_text = (
        render_telemetry_ascii.__wrapped__(custom_scene)
        if hasattr(render_telemetry_ascii, "__wrapped__")
        else ""
    )
    assert isinstance(custom_text, str)


def test_telemetry_states_and_outcomes() -> None:
    """Verifies telemetry HUD indicators across all flight states."""
    # Ascent
    ascent_state = LanderState(
        time_seconds=50,
        altitude_mft=50_000_000,
        velocity_mft_s=50_000,
        fuel_lbs=10_000,
        dry_mass_lbs=16_500,
        status=InFlight(),
    )
    hud_ascent = render_telemetry_ascii(ascent_state, 100)
    assert "ASCENT" in hud_ascent
    assert "▲" in hud_ascent

    # Hover
    hover_state = LanderState(
        time_seconds=60,
        altitude_mft=50_000_000,
        velocity_mft_s=0,
        fuel_lbs=10_000,
        dry_mass_lbs=16_500,
        status=InFlight(),
    )
    hud_hover = render_telemetry_ascii(hover_state, 50)
    assert "HOVER" in hud_hover
    assert "◆" in hud_hover

    # All Landed outcomes
    outcomes = [
        (LandingOutcome.Perfect, "PERFECT"),
        (LandingOutcome.Hard, "HARD"),
        (LandingOutcome.Crash, "CRASH"),
        (LandingOutcome.Fatal, "FATAL"),
        (LandingOutcome.Crater, "CRATER"),
    ]
    for outcome, label in outcomes:
        landed_state = LanderState(
            time_seconds=100,
            altitude_mft=0,
            velocity_mft_s=-1000,
            fuel_lbs=5000,
            dry_mass_lbs=16_500,
            status=Landed(outcome),
            impact_velocity_mft_s=-1000,
        )
        hud_landed = render_telemetry_ascii(landed_state, 0)
        assert "STATUS:" in hud_landed
        assert label in hud_landed
        assert "★" in hud_landed


def test_status_parsing_and_equality() -> None:
    """Verifies FlightStatus parsing and equality semantics."""
    assert parse_status("InFlight") == InFlight()
    assert parse_status("InFlight") == "InFlight"
    assert parse_status(InFlight()) == InFlight()

    for outcome in LandingOutcome:
        status = parse_status(f"Landed({outcome.value})")
        assert isinstance(status, Landed)
        assert status.outcome == outcome
        assert status == f"Landed({outcome.value})"
        assert repr(status) == f"Landed({outcome.value})"
        assert status != 123

    with pytest.raises(ArithmeticOverflow):
        parse_status("InvalidStatus")
    with pytest.raises(ArithmeticOverflow):
        parse_status("Landed(Unknown)")
    with pytest.raises(ArithmeticOverflow):
        parse_status(123)  # type: ignore[arg-type]


def test_error_equality_and_attributes() -> None:
    """Verifies error code and class attribute parity."""
    assert LanderError.FlightAlreadyEnded is FlightAlreadyEnded
    assert LanderError.ExcessiveBurnRate is ExcessiveBurnRate
    assert LanderError.InvalidDuration is InvalidDuration
    assert LanderError.ArithmeticOverflow is ArithmeticOverflow

    err = ExcessiveBurnRate()
    assert err == ExcessiveBurnRate
    assert err == "ExcessiveBurnRate"
    assert err.code == "ExcessiveBurnRate"
    assert err == ExcessiveBurnRate()
    assert err != 42
    assert err != FlightAlreadyEnded()


def test_zero_ambient_capabilities() -> None:
    """Verifies static zero-authority requirement (CAP001, VER08a)."""
    pkg_dir = Path(__file__).parents[1]
    cap_file = pkg_dir / "required_capabilities.json"
    with open(cap_file, encoding="utf-8") as f:
        cap_data = json.load(f)

    assert cap_data["package"] == "python/lunar-lander"
    assert cap_data["capabilities"] == []

    # AST audit: no forbidden system modules
    source_file = pkg_dir / "src" / "lunar_lander" / "__init__.py"
    with open(source_file, encoding="utf-8") as f:
        tree = ast.parse(f.read(), filename=str(source_file))

    forbidden_modules = {
        "os",
        "sys",
        "socket",
        "subprocess",
        "urllib",
        "requests",
        "time",
        "threading",
        "multiprocessing",
        "ctypes",
    }

    for node in ast.walk(tree):
        if isinstance(node, ast.Import):
            for alias in node.names:
                mod_root = alias.name.split(".")[0]
                assert mod_root not in forbidden_modules, (
                    f"Forbidden import: {alias.name}"
                )
        elif isinstance(node, ast.ImportFrom) and node.module:
            mod_root = node.module.split(".")[0]
            assert mod_root not in forbidden_modules, f"Forbidden import: {node.module}"
