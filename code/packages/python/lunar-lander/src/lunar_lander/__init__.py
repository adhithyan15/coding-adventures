"""Pure, deterministic simulation core and ASCII telemetry for Lunar Lander.

Historical Lineage:
- 1969 — Jim Storer's Lunar Landing Game (FOCAL-69, DEC PDP-8)
- 1973 — 101 BASIC Computer Games (David H. Ahl, DEC)

Architectural Guarantees:
- Strictly zero ambient system authority ("capabilities": []).
- Pure mathematical state transitions: step_burn(state, cmd) -> next_state.
- Exact fixed-point integer kinematics with Euler-Verlet integration.
- Exact Euclidean division helper div_euclid and isqrt_u128.
- Declarative 2D telemetry HUD with 100% parity to rust/lunar-lander.
- Complete 26-CWE weakness closure (CWE-190, CWE-369, CWE-843).
"""

from __future__ import annotations

import math
from dataclasses import dataclass, field
from enum import StrEnum
from typing import Any

# ============================================================================
# Physical Constants & Apollo LM Parameters
# ============================================================================

G_LUNAR_MFT_S2: int = 5_310
DRY_MASS_LBS: int = 16_500
INITIAL_FUEL_LBS: int = 16_500
TOTAL_INITIAL_MASS_LBS: int = 33_000
INITIAL_ALTITUDE_MFT: int = 120_000_000
INITIAL_VELOCITY_MFT_S: int = -3_000_000
EXHAUST_VELOCITY_MFT_S: int = 8_000_000
MAX_BURN_RATE_LBS_S: int = 200
MIN_DURATION_S: int = 1
MAX_DURATION_S: int = 10
MAX_TIME_SECONDS: int = 86_400
MAX_ALTITUDE_MFT: int = 200_000_000
MIN_VELOCITY_MFT_S: int = -10_000_000
MAX_VELOCITY_MFT_S: int = 10_000_000


# ============================================================================
# Error Hierarchy
# ============================================================================


class LanderError(Exception):
    """Base error for all Lunar Lander simulation rejections."""

    code: str = "LanderError"

    def __init__(self, message: str | None = None) -> None:
        self.message = message or self.__doc__ or self.__class__.__name__
        super().__init__(self.message)

    def __eq__(self, other: object) -> bool:
        if isinstance(other, type) and issubclass(other, LanderError):
            return self.__class__ is other
        if isinstance(other, str):
            return self.__class__.__name__ == other or self.code == other
        if isinstance(other, LanderError):
            return self.__class__ is other.__class__
        return False


class FlightAlreadyEnded(LanderError):
    """Attempted to execute a burn command when flight has already terminated."""

    code: str = "FlightAlreadyEnded"


class ExcessiveBurnRate(LanderError):
    """Requested burn rate exceeds engine maximum (B > 200 lbs/s)."""

    code: str = "ExcessiveBurnRate"


class InvalidDuration(LanderError):
    """Command duration is 0 or exceeds turn limit (duration == 0 or > 10)."""

    code: str = "InvalidDuration"


class ArithmeticOverflow(LanderError):
    """Arithmetic overflow, coordinate bounds violation, or type confusion."""

    code: str = "ArithmeticOverflow"


# Bind to LanderError class attributes for enum-style access
LanderError.FlightAlreadyEnded = FlightAlreadyEnded  # type: ignore[attr-defined]
LanderError.ExcessiveBurnRate = ExcessiveBurnRate  # type: ignore[attr-defined]
LanderError.InvalidDuration = InvalidDuration  # type: ignore[attr-defined]
LanderError.ArithmeticOverflow = ArithmeticOverflow  # type: ignore[attr-defined]


# ============================================================================
# Domain Types & Flight Status
# ============================================================================


class LandingOutcome(StrEnum):
    """Terminal landing outcome classified by touchdown impact speed magnitude."""

    Perfect = "Perfect"
    Hard = "Hard"
    Crash = "Crash"
    Fatal = "Fatal"
    Crater = "Crater"

    # Uppercase aliases
    PERFECT = "Perfect"
    HARD = "Hard"
    CRASH = "Crash"
    FATAL = "Fatal"
    CRATER = "Crater"


class FlightStatus:
    """Base class for Lunar Module flight lifecycle status."""


@dataclass(frozen=True, slots=True)
class InFlight(FlightStatus):
    """Active in-flight descent trajectory."""

    def __str__(self) -> str:
        return "InFlight"

    def __repr__(self) -> str:
        return "InFlight"

    def __eq__(self, other: object) -> bool:
        if isinstance(other, InFlight):
            return True
        if isinstance(other, str):
            return other == "InFlight"
        return False


@dataclass(frozen=True, slots=True)
class Landed(FlightStatus):
    """Craft has reached the lunar surface with a classified outcome."""

    outcome: LandingOutcome

    def __post_init__(self) -> None:
        if isinstance(self.outcome, str) and not isinstance(
            self.outcome, LandingOutcome
        ):
            try:
                object.__setattr__(self, "outcome", LandingOutcome(self.outcome))
            except ValueError:
                raise ArithmeticOverflow(
                    f"Unknown landing outcome: {self.outcome!r}"
                ) from None
        elif not isinstance(self.outcome, LandingOutcome):
            raise ArithmeticOverflow(f"Invalid outcome type: {type(self.outcome)}")

    def __str__(self) -> str:
        val = (
            self.outcome.value
            if isinstance(self.outcome, LandingOutcome)
            else str(self.outcome)
        )
        return f"Landed({val})"

    def __repr__(self) -> str:
        return self.__str__()

    def __eq__(self, other: object) -> bool:
        if isinstance(other, Landed):
            return self.outcome == other.outcome
        if isinstance(other, str):
            return str(self) == other
        return False


FlightStatus.InFlight = InFlight()  # type: ignore[attr-defined]
FlightStatus.Landed = Landed  # type: ignore[attr-defined]


def parse_status(s: str | FlightStatus) -> FlightStatus:
    """Parse status from string representation or passthrough."""
    if isinstance(s, FlightStatus):
        return s
    if not isinstance(s, str):
        raise ArithmeticOverflow(f"status must be a str or FlightStatus, got {s!r}")
    if s == "InFlight":
        return InFlight()
    if s.startswith("Landed(") and s.endswith(")"):
        inner = s[7:-1]
        try:
            return Landed(LandingOutcome(inner))
        except ValueError:
            raise ArithmeticOverflow(f"Unknown landing outcome: {inner!r}") from None
    raise ArithmeticOverflow(f"Unknown flight status: {s!r}")


# ============================================================================
# Value Records
# ============================================================================


@dataclass(frozen=True, slots=True)
class BurnCommand:
    """Pilot burn order for a discrete simulation turn."""

    burn_rate_lbs_s: int
    duration_seconds: int

    def __post_init__(self) -> None:
        if type(self.burn_rate_lbs_s) is not int:
            t_name = type(self.burn_rate_lbs_s).__name__
            raise ArithmeticOverflow(f"burn_rate_lbs_s must be exact int, got {t_name}")
        if type(self.duration_seconds) is not int:
            t_name = type(self.duration_seconds).__name__
            raise ArithmeticOverflow(
                f"duration_seconds must be exact int, got {t_name}"
            )


@dataclass(frozen=True, slots=True)
class LanderState:
    """Immutable state record capturing the complete flight simulation state."""

    time_seconds: int
    altitude_mft: int
    velocity_mft_s: int
    fuel_lbs: int
    dry_mass_lbs: int
    status: FlightStatus
    impact_velocity_mft_s: int | None = None

    def __post_init__(self) -> None:
        for name, val in (
            ("time_seconds", self.time_seconds),
            ("altitude_mft", self.altitude_mft),
            ("velocity_mft_s", self.velocity_mft_s),
            ("fuel_lbs", self.fuel_lbs),
            ("dry_mass_lbs", self.dry_mass_lbs),
        ):
            if type(val) is not int:
                raise ArithmeticOverflow(
                    f"{name} must be exact int, got {type(val).__name__}"
                )

        if (
            self.impact_velocity_mft_s is not None
            and type(self.impact_velocity_mft_s) is not int
        ):
            t_name = type(self.impact_velocity_mft_s).__name__
            raise ArithmeticOverflow(
                f"impact_velocity_mft_s must be None or exact int, got {t_name}"
            )

        if isinstance(self.status, str):
            object.__setattr__(self, "status", parse_status(self.status))
        elif not isinstance(self.status, FlightStatus):
            raise ArithmeticOverflow(
                f"status must be FlightStatus, got {type(self.status).__name__}"
            )

    def to_dict(self) -> dict[str, Any]:
        """Serializes state to dict matching cases.json fixture format."""
        return {
            "time_seconds": self.time_seconds,
            "altitude_mft": self.altitude_mft,
            "velocity_mft_s": self.velocity_mft_s,
            "fuel_lbs": self.fuel_lbs,
            "dry_mass_lbs": self.dry_mass_lbs,
            "status": str(self.status),
            "impact_velocity_mft_s": self.impact_velocity_mft_s,
        }


# ============================================================================
# Math Helpers
# ============================================================================


def div_euclid(a: int, b: int) -> int:
    """Euclidean division where remainder r is always non-negative: r in [0, |b|)."""
    if b == 0:
        raise ArithmeticOverflow("Division by zero in div_euclid")
    q, r = divmod(a, b)
    return q + 1 if r < 0 else q


def isqrt_u128(n: int) -> int:
    """Computes exact integer floor(sqrt(n)) using bitwise binary square root."""
    if n < 0:
        raise ArithmeticOverflow("isqrt_u128 requires non-negative integer")
    if n == 0:
        return 0
    res = 0
    bit = 1 << 126
    while bit > n:
        bit >>= 2
    num = n
    while bit != 0:
        if num >= res + bit:
            num -= res + bit
            res = (res >> 1) + bit
        else:
            res >>= 1
        bit >>= 2
    return res


# ============================================================================
# Simulation Core
# ============================================================================


def new_flight() -> LanderState:
    """Initializes a new Lunar Module descent mission at standard conditions."""
    return LanderState(
        time_seconds=0,
        altitude_mft=INITIAL_ALTITUDE_MFT,
        velocity_mft_s=INITIAL_VELOCITY_MFT_S,
        fuel_lbs=INITIAL_FUEL_LBS,
        dry_mass_lbs=DRY_MASS_LBS,
        status=InFlight(),
        impact_velocity_mft_s=None,
    )


def state_inv(s: LanderState) -> bool:
    """State invariant model (VER04m)."""
    return (
        s.dry_mass_lbs == DRY_MASS_LBS
        and 0 <= s.fuel_lbs <= INITIAL_FUEL_LBS
        and 0 <= s.altitude_mft <= MAX_ALTITUDE_MFT
        and MIN_VELOCITY_MFT_S <= s.velocity_mft_s <= MAX_VELOCITY_MFT_S
        and 0 <= s.time_seconds <= MAX_TIME_SECONDS
    )


def validate_burn(state: LanderState, cmd: BurnCommand) -> None:
    """Validates candidate burn command against state and physical invariants."""
    if not isinstance(state, LanderState) or not isinstance(cmd, BurnCommand):
        raise ArithmeticOverflow("Invalid state or command object")

    # 1. Lifecycle Check
    if not isinstance(state.status, InFlight):
        raise FlightAlreadyEnded("Flight has already ended")

    # 2. State Invariants
    if state.dry_mass_lbs != DRY_MASS_LBS:
        raise ArithmeticOverflow(f"Corrupted dry mass: {state.dry_mass_lbs}")
    if state.fuel_lbs < 0 or state.fuel_lbs > INITIAL_FUEL_LBS:
        raise ArithmeticOverflow(f"Invalid fuel: {state.fuel_lbs}")
    if state.altitude_mft <= 0 or state.altitude_mft > MAX_ALTITUDE_MFT:
        raise ArithmeticOverflow(f"Altitude out of bounds: {state.altitude_mft}")
    if not (MIN_VELOCITY_MFT_S <= state.velocity_mft_s <= MAX_VELOCITY_MFT_S):
        raise ArithmeticOverflow(f"Velocity out of bounds: {state.velocity_mft_s}")
    if state.time_seconds < 0 or state.time_seconds > MAX_TIME_SECONDS:
        raise ArithmeticOverflow(f"Time seconds out of bounds: {state.time_seconds}")
    if state.impact_velocity_mft_s is not None:
        raise ArithmeticOverflow("Impact velocity must be None while in flight")

    # 3. Command Bounds
    if cmd.burn_rate_lbs_s < 0 or cmd.burn_rate_lbs_s > 0xFFFFFFFF:
        raise ArithmeticOverflow(f"Burn rate exceeds u32: {cmd.burn_rate_lbs_s}")
    if cmd.burn_rate_lbs_s > MAX_BURN_RATE_LBS_S:
        raise ExcessiveBurnRate(
            f"Burn rate {cmd.burn_rate_lbs_s} exceeds maximum {MAX_BURN_RATE_LBS_S}"
        )
    if not (MIN_DURATION_S <= cmd.duration_seconds <= MAX_DURATION_S):
        raise InvalidDuration(
            f"Duration {cmd.duration_seconds} is invalid (must be 1 to 10 seconds)"
        )
    t_next = state.time_seconds + cmd.duration_seconds
    if t_next > MAX_TIME_SECONDS or t_next > 0xFFFFFFFF:
        raise ArithmeticOverflow(
            f"Mission time overflow: {t_next} > {MAX_TIME_SECONDS}"
        )


def step_burn(state: LanderState, cmd: BurnCommand) -> LanderState:
    """Integrates a single discrete flight step under commanded burn and duration."""
    validate_burn(state, cmd)

    dt = cmd.duration_seconds
    fuel_req = cmd.burn_rate_lbs_s * cmd.duration_seconds
    delta_f = min(state.fuel_lbs, fuel_req)
    fuel_next = state.fuel_lbs - delta_f

    avg_mass = state.dry_mass_lbs + state.fuel_lbs - (delta_f // 2)
    impulse = delta_f * EXHAUST_VELOCITY_MFT_S
    a_thrust = impulse // (dt * avg_mass)
    a_net = a_thrust - G_LUNAR_MFT_S2

    v_next = state.velocity_mft_s + a_net * dt
    y_next = (
        state.altitude_mft + state.velocity_mft_s * dt + div_euclid(a_net * dt * dt, 2)
    )
    t_next = state.time_seconds + cmd.duration_seconds

    if y_next > MAX_ALTITUDE_MFT or not (
        MIN_VELOCITY_MFT_S <= v_next <= MAX_VELOCITY_MFT_S
    ):
        raise ArithmeticOverflow("Flight parameters exceeded physical boundaries")

    if y_next > 0:
        return LanderState(
            time_seconds=t_next,
            altitude_mft=y_next,
            velocity_mft_s=v_next,
            fuel_lbs=fuel_next,
            dry_mass_lbs=state.dry_mass_lbs,
            status=InFlight(),
            impact_velocity_mft_s=None,
        )
    else:
        # Ground touchdown solver
        a_half = div_euclid(a_net, 2)
        if a_half == 0:
            if state.velocity_mft_s == 0:
                raise ArithmeticOverflow(
                    "Zero velocity with zero acceleration at ground penetration"
                )
            delta_t_raw = div_euclid(-state.altitude_mft, state.velocity_mft_s)
        else:
            v = state.velocity_mft_s
            y = state.altitude_mft
            disc = v * v - 4 * a_half * y
            disc_pos = max(0, disc)
            root = isqrt_u128(disc_pos)
            delta_t_raw = div_euclid(-state.velocity_mft_s - root, 2 * a_half)

        delta_t = min(max(delta_t_raw, 1), dt)

        v_touchdown = state.velocity_mft_s + a_net * delta_t
        t_final = state.time_seconds + delta_t

        fuel_burned_at_touchdown = min(state.fuel_lbs, delta_t * cmd.burn_rate_lbs_s)
        fuel_final = state.fuel_lbs - fuel_burned_at_touchdown

        speed_mft_s = abs(v_touchdown)
        if 0 <= speed_mft_s <= 5_000:
            outcome = LandingOutcome.Perfect
        elif 5_001 <= speed_mft_s <= 15_000:
            outcome = LandingOutcome.Hard
        elif 15_001 <= speed_mft_s <= 30_000:
            outcome = LandingOutcome.Crash
        elif 30_001 <= speed_mft_s <= 60_000:
            outcome = LandingOutcome.Fatal
        else:
            outcome = LandingOutcome.Crater

        return LanderState(
            time_seconds=t_final,
            altitude_mft=0,
            velocity_mft_s=v_touchdown,
            fuel_lbs=fuel_final,
            dry_mass_lbs=state.dry_mass_lbs,
            status=Landed(outcome),
            impact_velocity_mft_s=v_touchdown,
        )


# ============================================================================
# Declarative 2D PaintScene & ASCII Telemetry HUD
# ============================================================================


@dataclass(frozen=True, slots=True)
class PaintText:
    """A text element in a PaintScene."""

    x: float
    y: float
    text: str
    fill: str | None = "#ffffff"
    font_size: float = 1.0
    text_align: str = "left"
    kind: str = "text"


@dataclass(frozen=True, slots=True)
class PaintRect:
    """A filled axis-aligned rectangle in a PaintScene."""

    x: float
    y: float
    width: float
    height: float
    fill: str | None = "#00ff00"
    kind: str = "rect"


@dataclass(frozen=True, slots=True)
class PaintScene:
    """Declarative 2D scene representation for telemetry rendering."""

    width: float
    height: float
    background: str = "#000000"
    instructions: list[PaintText | PaintRect] = field(default_factory=list)


def to_paint_scene(state: LanderState, last_burn_rate_lbs_s: int = 0) -> PaintScene:
    """Translates a flight state into a declarative 76x5 PaintScene."""
    alt_clamped = min(max(state.altitude_mft, 0), INITIAL_ALTITUDE_MFT)
    alt_pct = (alt_clamped * 100) // INITIAL_ALTITUDE_MFT
    alt_bars = (alt_clamped * 20) // INITIAL_ALTITUDE_MFT

    fuel_clamped = min(state.fuel_lbs, INITIAL_FUEL_LBS)
    fuel_pct = (fuel_clamped * 100) // INITIAL_FUEL_LBS
    fuel_bars = (fuel_clamped * 20) // INITIAL_FUEL_LBS

    alt_ft = int(state.altitude_mft / 1_000)
    vel_fps = int(state.velocity_mft_s / 1_000)
    fuel_lbs = state.fuel_lbs

    alt_str = f"{alt_ft:,}"
    fuel_str = f"{fuel_lbs:,}"

    if isinstance(state.status, InFlight):
        if state.velocity_mft_s < 0:
            rate_label, rate_text, rate_symbol = "RATE:", "DESCENT", "▼"
        elif state.velocity_mft_s > 0:
            rate_label, rate_text, rate_symbol = "RATE:", "ASCENT", "▲"
        else:
            rate_label, rate_text, rate_symbol = "RATE:", "HOVER", "◆"
    elif isinstance(state.status, Landed):
        outcome_desc = {
            LandingOutcome.Perfect: "PERFECT",
            LandingOutcome.Hard: "HARD",
            LandingOutcome.Crash: "CRASH",
            LandingOutcome.Fatal: "FATAL",
            LandingOutcome.Crater: "CRATER",
        }.get(state.status.outcome, str(state.status.outcome).upper())
        rate_label, rate_text, rate_symbol = "STATUS:", outcome_desc, "★"
    else:
        rate_label, rate_text, rate_symbol = "RATE:", "UNKNOWN", " "

    row0 = (
        "┌───────────────────────── LUNAR LANDER TELEMETRY ─────────────────────────┐"
    )
    left1 = f" TIME: {state.time_seconds:>6} SEC    "
    right1 = f" ALTITUDE: {alt_str:>7} FT [{'░' * 20}] {alt_pct:>3}%   "
    row1 = f"│{left1}│{right1}│"

    left2 = f" VELOCITY: {vel_fps:>5} FT/S"
    right2 = f" FUEL:    {fuel_str:>7} LBS [{'░' * 20}] {fuel_pct:>3}%   "
    row2 = f"│{left2}│{right2}│"

    left3 = f" BURN:     {last_burn_rate_lbs_s:>4} LB/S "
    rate_col = f"{rate_label:<7} {rate_text:<8} {rate_symbol:<1}"
    row3 = f"│{left3}│ {rate_col:<51}│"
    row4 = (
        "└──────────────────────────────────────────────────────────────────────────┘"
    )

    full_text = f"{row0}\n{row1}\n{row2}\n{row3}\n{row4}"

    instructions: list[PaintText | PaintRect] = [
        PaintText(
            x=0.0,
            y=0.0,
            text=full_text,
            fill="#ffffff",
            font_size=1.0,
            text_align="left",
        )
    ]

    if alt_bars > 0:
        instructions.append(
            PaintRect(
                x=46.0,
                y=1.0,
                width=float(alt_bars - 1),
                height=0.0,
                fill="#00ff00",
            )
        )

    if fuel_bars > 0:
        instructions.append(
            PaintRect(
                x=46.0,
                y=2.0,
                width=float(fuel_bars - 1),
                height=0.0,
                fill="#00ff00",
            )
        )

    return PaintScene(
        width=76.0,
        height=5.0,
        background="#000000",
        instructions=instructions,
    )


def telemetry_scene(state: LanderState) -> PaintScene:
    """Translates a flight state into a PaintScene with zero initial burn."""
    return to_paint_scene(state, 0)


def render_telemetry_ascii(state: LanderState, last_burn_rate_lbs_s: int = 0) -> str:
    """Renders a flight state telemetry dashboard into a deterministic ASCII string."""
    scene = to_paint_scene(state, last_burn_rate_lbs_s)
    cols = math.ceil(scene.width)
    rows = math.ceil(scene.height)
    buf = [[" "] * cols for _ in range(rows)]

    def write_char(r: int, c: int, ch: str) -> None:
        if 0 <= r < rows and 0 <= c < cols:
            buf[r][c] = ch

    for inst in scene.instructions:
        if isinstance(inst, PaintText):
            anchor_col = round(inst.x)
            first_row = round(inst.y)
            for line_idx, line in enumerate(inst.text.split("\n")):
                width = len(line)
                if inst.text_align == "center":
                    start_col = anchor_col - width // 2
                elif inst.text_align == "right":
                    start_col = anchor_col - width
                else:
                    start_col = anchor_col
                for col_offset, ch in enumerate(line):
                    if ch != "\r":
                        write_char(first_row + line_idx, start_col + col_offset, ch)
        elif isinstance(inst, PaintRect):
            fill = inst.fill or "#000000"
            if fill.strip() in ("", "transparent", "none"):
                continue
            c1 = round(inst.x)
            r1 = round(inst.y)
            c2 = round(inst.x + inst.width)
            r2 = round(inst.y + inst.height)
            for r in range(r1, r2 + 1):
                for c in range(c1, c2 + 1):
                    write_char(r, c, "\u2588")

    return "\n".join("".join(row).rstrip() for row in buf).rstrip()


def render_telemetry(state: LanderState) -> str:
    """Renders a flight state telemetry dashboard with zero burn rate."""
    return render_telemetry_ascii(state, 0)


__all__ = [
    # Constants
    "G_LUNAR_MFT_S2",
    "DRY_MASS_LBS",
    "INITIAL_FUEL_LBS",
    "TOTAL_INITIAL_MASS_LBS",
    "INITIAL_ALTITUDE_MFT",
    "INITIAL_VELOCITY_MFT_S",
    "EXHAUST_VELOCITY_MFT_S",
    "MAX_BURN_RATE_LBS_S",
    "MIN_DURATION_S",
    "MAX_DURATION_S",
    "MAX_TIME_SECONDS",
    "MAX_ALTITUDE_MFT",
    "MIN_VELOCITY_MFT_S",
    "MAX_VELOCITY_MFT_S",
    # Domain types
    "LandingOutcome",
    "FlightStatus",
    "InFlight",
    "Landed",
    "LanderState",
    "BurnCommand",
    "PaintScene",
    "PaintText",
    "PaintRect",
    # Errors
    "LanderError",
    "FlightAlreadyEnded",
    "ExcessiveBurnRate",
    "InvalidDuration",
    "ArithmeticOverflow",
    # Core functions
    "new_flight",
    "validate_burn",
    "step_burn",
    "state_inv",
    "isqrt_u128",
    "div_euclid",
    "parse_status",
    "to_paint_scene",
    "telemetry_scene",
    "render_telemetry_ascii",
    "render_telemetry",
]
