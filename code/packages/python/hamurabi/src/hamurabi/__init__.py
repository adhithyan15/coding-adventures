"""Pure, deterministic simulation core for the 1968 mainframe resource allocation game.

Historical Lineage:
- 1964 — The Sumerian Game (Mabel Addis & William McKay, IBM 7090)
- 1968 — FOCAL Hamurabi (Richard Merrill, DEC PDP-8)
- 1973 — 101 BASIC Computer Games (David H. Ahl, DEC)

Architectural Guarantees:
- Strictly zero ambient system authority ("capabilities": []).
- Pure mathematical state transitions: step_year(state, order) -> next_state.
- Explicit PCG32 PRNG (stream_id = 54, inc = 109, Knuth multiplier).
- Bounded integer operations ensuring 100% bit-exact cross-language determinism.
- Strict CWE/CVE closure (CWE-190, CWE-843, CWE-400, CWE-502).
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import StrEnum
from typing import Any

# ── PRNG & Numerical Constants ──────────────────────────────────────────────

PCG_MULTIPLIER: int = 6364136223846793005
HAMURABI_STREAM_ID: int = 54
DEFAULT_INC: int = (HAMURABI_STREAM_ID << 1) | 1  # 109

MASK64: int = 0xFFFFFFFFFFFFFFFF
MASK32: int = 0xFFFFFFFF


# ── PRNG Primitives (PCG32 XSH RR 64/32) ────────────────────────────────────


def pcg32_step(state: int, inc: int) -> tuple[int, int]:
    """Advances the 64-bit state and returns (next_state, 32-bit output)."""
    old_state = state & MASK64
    next_state = ((old_state * PCG_MULTIPLIER) + inc) & MASK64
    xorshifted = (((old_state >> 18) ^ old_state) >> 27) & MASK32
    rot = (old_state >> 59) & 31
    output = ((xorshifted >> rot) | (xorshifted << ((-rot) & 31))) & MASK32
    return next_state, output


def pcg32_next_int_in_range(
    state: int, inc: int, min_val: int, max_val: int
) -> tuple[int, int]:
    """Draws an unbiased integer in [min_val, max_val] via Lemire rejection sampling."""
    span = (max_val - min_val) + 1
    threshold = ((1 << 32) - span) % span
    while True:
        state, r = pcg32_step(state, inc)
        if r >= threshold:
            return state, min_val + (r % span)


# ── Domain Types ────────────────────────────────────────────────────────────


class Rating(StrEnum):
    """Rating classification awarded to ruler upon completing a 10-year term."""

    CHARLEMAGNE = "Charlemagne"
    ACRE = "Acre"
    NERO = "Nero"
    DISASTER = "Disaster"


@dataclass(frozen=True, slots=True)
class Playing:
    """Active, playable year (years 1..=10)."""

    def __str__(self) -> str:
        return "Playing"

    def __eq__(self, other: object) -> bool:
        if isinstance(other, Playing):
            return True
        if isinstance(other, str):
            return other == "Playing"
        return False


@dataclass(frozen=True, slots=True)
class Won:
    """Completed 10-year term successfully (transitioned to terminal year 11)."""

    rating: Rating

    def __str__(self) -> str:
        rating_val = (
            self.rating.value if isinstance(self.rating, Rating) else str(self.rating)
        )
        return f"Won({rating_val})"

    def __eq__(self, other: object) -> bool:
        if isinstance(other, Won):
            return self.rating == other.rating
        if isinstance(other, str):
            return str(self) == other
        return False


@dataclass(frozen=True, slots=True)
class Impeached:
    """Impeached and expelled from office due to extreme starvation (> 45%)."""

    starved_this_turn: int
    percent: int

    def __str__(self) -> str:
        return f"Impeached(starved={self.starved_this_turn}, percent={self.percent})"

    def __eq__(self, other: object) -> bool:
        if isinstance(other, Impeached):
            return (
                self.starved_this_turn == other.starved_this_turn
                and self.percent == other.percent
            )
        if isinstance(other, str):
            return str(self) == other
        return False


@dataclass(frozen=True, slots=True)
class GameOverExtinction:
    """Entire population died of starvation or disaster."""

    def __str__(self) -> str:
        return "GameOverExtinction"

    def __eq__(self, other: object) -> bool:
        if isinstance(other, GameOverExtinction):
            return True
        if isinstance(other, str):
            return other == "GameOverExtinction"
        return False


GameStatus = Playing | Won | Impeached | GameOverExtinction


def parse_status(s: str | GameStatus) -> GameStatus:
    """Parses status representation from string or passes through status object."""
    if isinstance(s, (Playing, Won, Impeached, GameOverExtinction)):
        return s
    if not isinstance(s, str):
        raise InvalidOrderBounds(f"status must be a string or GameStatus, got {s!r}")

    if s == "Playing":
        return Playing()
    if s == "GameOverExtinction":
        return GameOverExtinction()
    if s.startswith("Won(") and s.endswith(")"):
        rating_name = s[4:-1]
        return Won(Rating(rating_name))
    if s.startswith("Impeached(") and s.endswith(")"):
        inner = s[10:-1]
        parts = inner.split(",")
        starved = int(parts[0].split("=")[1].strip())
        percent = int(parts[1].split("=")[1].strip())
        return Impeached(starved_this_turn=starved, percent=percent)
    raise InvalidOrderBounds(f"Unknown status string: {s}")


# ── Error Hierarchy ─────────────────────────────────────────────────────────


class HamurabiError(Exception):
    """Base error for all Hamurabi simulation rejections and state errors."""

    code: str = "HamurabiError"

    def __init__(self, message: str | None = None) -> None:
        self.message = message or self.__doc__ or self.__class__.__name__
        super().__init__(self.message)

    def __eq__(self, other: object) -> bool:
        if isinstance(other, type) and issubclass(other, HamurabiError):
            return self.__class__ is other
        if isinstance(other, str):
            return self.__class__.__name__ == other or self.code == other
        if isinstance(other, HamurabiError):
            return self.__class__ is other.__class__
        return False


class GameAlreadyEnded(HamurabiError):
    """Attempted to execute an order when status is not Playing."""

    code: str = "GameAlreadyEnded"


class InvalidOrderBounds(HamurabiError):
    """Arithmetic overflow, type error, or invalid bounds on input parameters."""

    code: str = "InvalidOrderBounds"


class InsufficientBushelsForLand(HamurabiError):
    """Player ordered to buy land but lacks sufficient bushels."""

    code: str = "InsufficientBushelsForLand"


class InsufficientAcresToSell(HamurabiError):
    """Player ordered to sell more land than the city owns."""

    code: str = "InsufficientAcresToSell"


class InsufficientBushelsToFeed(HamurabiError):
    """Player ordered to feed more bushels than available after land trade."""

    code: str = "InsufficientBushelsToFeed"


class InsufficientAcresToPlant(HamurabiError):
    """Player ordered to plant more acres than owned after land trade."""

    code: str = "InsufficientAcresToPlant"


class InsufficientPopulationToPlant(HamurabiError):
    """Player ordered to plant more acres than population can tend."""

    code: str = "InsufficientPopulationToPlant"


class InsufficientGrainForSeed(HamurabiError):
    """Player lacks grain to seed fields (1 bushel seeds 2 acres)."""

    code: str = "InsufficientGrainForSeed"


# Provide class-attribute access on base class for convenience
HamurabiError.GameAlreadyEnded = GameAlreadyEnded  # type: ignore[attr-defined]
HamurabiError.InvalidOrderBounds = InvalidOrderBounds  # type: ignore[attr-defined]
HamurabiError.InsufficientBushelsForLand = InsufficientBushelsForLand  # type: ignore[attr-defined]
HamurabiError.InsufficientAcresToSell = InsufficientAcresToSell  # type: ignore[attr-defined]
HamurabiError.InsufficientBushelsToFeed = InsufficientBushelsToFeed  # type: ignore[attr-defined]
HamurabiError.InsufficientAcresToPlant = InsufficientAcresToPlant  # type: ignore[attr-defined]
HamurabiError.InsufficientPopulationToPlant = InsufficientPopulationToPlant  # type: ignore[attr-defined]
HamurabiError.InsufficientGrainForSeed = InsufficientGrainForSeed  # type: ignore[attr-defined]


# ── State Records ───────────────────────────────────────────────────────────


@dataclass(frozen=True, slots=True)
class TurnOrder:
    """Player orders for a single year."""

    acres_delta: int
    bushels_to_feed: int
    acres_to_plant: int

    def __post_init__(self) -> None:
        if type(self.acres_delta) is not int or not (
            -2147483648 <= self.acres_delta <= 2147483647
        ):
            raise InvalidOrderBounds(
                f"acres_delta must be i32 int, got {self.acres_delta!r}"
            )
        if type(self.bushels_to_feed) is not int or not (
            0 <= self.bushels_to_feed <= 0xFFFFFFFF
        ):
            raise InvalidOrderBounds(
                f"bushels_to_feed must be u32 int, got {self.bushels_to_feed!r}"
            )
        if type(self.acres_to_plant) is not int or not (
            0 <= self.acres_to_plant <= 0xFFFFFFFF
        ):
            raise InvalidOrderBounds(
                f"acres_to_plant must be u32 int, got {self.acres_to_plant!r}"
            )


@dataclass(frozen=True, slots=True)
class HamurabiState:
    """Immutable value record capturing the complete simulation state."""

    year: int
    population: int
    bushels: int
    acres: int
    land_price: int
    harvest_per_acre: int
    rats_eaten: int
    starved_last_year: int
    starved_total: int
    percent_starved_sum: int
    immigrants_last_year: int
    plague_struck: bool
    status: GameStatus
    rng_state: int
    rng_inc: int

    def __post_init__(self) -> None:
        if type(self.year) is not int or not (0 <= self.year <= 255):
            raise InvalidOrderBounds(
                f"year must be u8 int in [0, 255], got {self.year!r}"
            )

        int_fields_u32 = (
            ("population", self.population),
            ("bushels", self.bushels),
            ("acres", self.acres),
            ("land_price", self.land_price),
            ("harvest_per_acre", self.harvest_per_acre),
            ("rats_eaten", self.rats_eaten),
            ("starved_last_year", self.starved_last_year),
            ("starved_total", self.starved_total),
            ("percent_starved_sum", self.percent_starved_sum),
            ("immigrants_last_year", self.immigrants_last_year),
        )
        for name, val in int_fields_u32:
            if type(val) is not int or not (0 <= val <= 0xFFFFFFFF):
                raise InvalidOrderBounds(
                    f"{name} must be u32 int in [0, 0xFFFFFFFF], got {val!r}"
                )

        if type(self.plague_struck) is not bool:
            raise InvalidOrderBounds(
                f"plague_struck must be bool, got {self.plague_struck!r}"
            )

        if type(self.rng_state) is not int or not (0 <= self.rng_state <= MASK64):
            raise InvalidOrderBounds(
                f"rng_state must be u64 int in [0, 2^64-1], got {self.rng_state!r}"
            )
        if type(self.rng_inc) is not int or not (0 <= self.rng_inc <= MASK64):
            raise InvalidOrderBounds(
                f"rng_inc must be u64 int in [0, 2^64-1], got {self.rng_inc!r}"
            )

        if isinstance(self.status, str):
            object.__setattr__(self, "status", parse_status(self.status))
        elif not isinstance(self.status, (Playing, Won, Impeached, GameOverExtinction)):
            raise InvalidOrderBounds(f"Invalid status: {self.status!r}")

    def to_dict(self) -> dict[str, Any]:
        """Converts state to a dictionary representation matching cases.json schema."""
        return {
            "year": self.year,
            "population": self.population,
            "bushels": self.bushels,
            "acres": self.acres,
            "land_price": self.land_price,
            "harvest_per_acre": self.harvest_per_acre,
            "rats_eaten": self.rats_eaten,
            "starved_last_year": self.starved_last_year,
            "starved_total": self.starved_total,
            "percent_starved_sum": self.percent_starved_sum,
            "immigrants_last_year": self.immigrants_last_year,
            "plague_struck": self.plague_struck,
            "status": str(self.status),
            "rng_state": self.rng_state,
            "rng_inc": self.rng_inc,
        }


# ── Simulation Engine API ───────────────────────────────────────────────────


def new_game(seed: int) -> HamurabiState:
    """Initializes a new, deterministic 10-year reign for ruler Hamurabi.

    Seeds PCG32 with (seed, stream_id = 54) and draws initial Year 1 land price.
    """
    if type(seed) is not int or not (0 <= seed <= MASK64):
        raise InvalidOrderBounds(
            f"seed must be u64 int in [0, 2^64-1], got {seed!r}"
        )

    inc = (HAMURABI_STREAM_ID << 1) | 1
    rng_state = 0
    rng_state = ((rng_state * PCG_MULTIPLIER) + inc) & MASK64
    rng_state = (rng_state + seed) & MASK64
    rng_state = ((rng_state * PCG_MULTIPLIER) + inc) & MASK64

    rng_state, land_price = pcg32_next_int_in_range(rng_state, inc, 17, 26)

    return HamurabiState(
        year=1,
        population=100,
        bushels=2800,
        acres=1000,
        land_price=land_price,
        harvest_per_acre=3,
        rats_eaten=200,
        starved_last_year=0,
        starved_total=0,
        percent_starved_sum=0,
        immigrants_last_year=5,
        plague_struck=False,
        status=Playing(),
        rng_state=rng_state,
        rng_inc=inc,
    )


def validate_order(state: HamurabiState, order: TurnOrder) -> None:
    """Validates player orders against current state constraints.

    Evaluates checks in canonical order:
    1. Status check (GameStatus is Playing)
    2. State invariants & bounds checks
    3. Land trade resource availability
    4. Food allocation feasibility
    5. Planting acreage and labor limits
    6. Seed grain availability
    7. Maximum harvest overflow guard
    """
    if not isinstance(state, HamurabiState):
        raise InvalidOrderBounds("state must be a HamurabiState instance")
    if not isinstance(order, TurnOrder):
        raise InvalidOrderBounds("order must be a TurnOrder instance")

    # 1. Status Check
    if not isinstance(state.status, Playing):
        raise GameAlreadyEnded("Game has already ended")

    # 2. State Invariants Check (VER04m state_inv & CWE-400 / CWE-835)
    # - Enforce odd rng_inc to prevent infinite loop / short period in PCG32 LCG
    if (state.rng_inc & 1) != 1:
        raise InvalidOrderBounds("rng_inc must be odd")
    # - Enforce reign year in valid turn bounds [1, 10]
    if state.year < 1 or state.year > 10:
        raise InvalidOrderBounds("year must be in range 1..=10")
    # - Enforce land price within canonical range [17, 26]
    if state.land_price < 17 or state.land_price > 26:
        raise InvalidOrderBounds("land_price must be in range 17..=26")
    # - Enforce harvest per acre within canonical range [1, 5]
    if state.harvest_per_acre < 1 or state.harvest_per_acre > 5:
        raise InvalidOrderBounds("harvest_per_acre must be in range 1..=5")

    # 3. Land Trade Bounds & Resource Availability
    if order.acres_delta > 0:
        buy_count = order.acres_delta
        cost = buy_count * state.land_price
        if cost > state.bushels:
            raise InsufficientBushelsForLand(
                "Insufficient bushels to buy specified acreage"
            )
        if state.acres + order.acres_delta > 0xFFFFFFFF:
            raise InvalidOrderBounds("acres overflow u32")
    elif order.acres_delta < 0:
        sell_count = -order.acres_delta
        if sell_count > state.acres:
            raise InsufficientAcresToSell(
                "Cannot sell more acres than the city owns"
            )
        revenue = sell_count * state.land_price
        if state.bushels + revenue > 0xFFFFFFFF:
            raise InvalidOrderBounds("bushels overflow u32")

    # Compute intermediate balances after trade
    if order.acres_delta >= 0:
        acres_after_trade = state.acres + order.acres_delta
    else:
        sell_count = -order.acres_delta
        acres_after_trade = state.acres - sell_count

    if order.acres_delta > 0:
        cost = order.acres_delta * state.land_price
        bushels_after_trade = state.bushels - cost
    elif order.acres_delta < 0:
        sell_count = -order.acres_delta
        revenue = sell_count * state.land_price
        bushels_after_trade = state.bushels + revenue
    else:
        bushels_after_trade = state.bushels

    # 4. Food Allocation Check
    if order.bushels_to_feed > bushels_after_trade:
        raise InsufficientBushelsToFeed(
            "Insufficient bushels in store to feed population"
        )
    bushels_after_feed = bushels_after_trade - order.bushels_to_feed

    # 5. Planting Acreage & Labor Checks
    if order.acres_to_plant > acres_after_trade:
        raise InsufficientAcresToPlant(
            "Cannot plant more acres than owned after land trade"
        )
    max_labor_acres = state.population * 10
    if max_labor_acres > 0xFFFFFFFF:
        raise InvalidOrderBounds("labor capacity overflow")
    if order.acres_to_plant > max_labor_acres:
        raise InsufficientPopulationToPlant(
            "Insufficient population to tend planted acreage"
        )

    # 6. Planting Seed Check: each bushel seeds 2 acres (ceil(acres / 2))
    seed_needed = (order.acres_to_plant + 1) // 2
    if seed_needed > bushels_after_feed:
        raise InsufficientGrainForSeed("Insufficient grain to seed planted acreage")
    bushels_after_seed = bushels_after_feed - seed_needed

    # 7. Maximum Harvest Overflow Guard (CWE-190)
    # Maximum harvest per acre is 5. Enforce bushels_after_seed + acres*5 <= u32::MAX.
    max_harvest = order.acres_to_plant * 5
    max_stored = bushels_after_seed + max_harvest
    if max_stored > 0xFFFFFFFF:
        raise InvalidOrderBounds("maximum harvest would overflow u32")


def step_year(state: HamurabiState, order: TurnOrder) -> HamurabiState:
    """Advances the simulation by exactly one year according to player orders.

    Executes the strict two-phase transition:
    - Phase A: Validation & constraint checking in canonical order.
    - Phase B: Strictly locked PRNG draw sequence.
    """
    validate_order(state, order)

    pcg_state = state.rng_state
    pcg_inc = state.rng_inc

    # Intermediate balances from validated trade
    if order.acres_delta >= 0:
        acres_after_trade = state.acres + order.acres_delta
    else:
        sell_count = -order.acres_delta
        acres_after_trade = state.acres - sell_count

    if order.acres_delta > 0:
        cost = order.acres_delta * state.land_price
        bushels_after_trade = state.bushels - cost
    elif order.acres_delta < 0:
        sell_count = -order.acres_delta
        revenue = sell_count * state.land_price
        bushels_after_trade = state.bushels + revenue
    else:
        bushels_after_trade = state.bushels

    bushels_after_feed = bushels_after_trade - order.bushels_to_feed
    seed_needed = (order.acres_to_plant + 1) // 2
    bushels_after_seed = bushels_after_feed - seed_needed

    # Step 1: Harvest Yield Draw
    # Draw 1: pcg.next_int_in_range(1, 5) -> harvest_per_acre (u32)
    pcg_state, harvest_per_acre = pcg32_next_int_in_range(
        pcg_state, pcg_inc, 1, 5
    )
    total_harvest = order.acres_to_plant * harvest_per_acre
    bushels_stored = bushels_after_seed + total_harvest

    # Step 2: Rat Infestation Draws
    # Draw 2: pcg.next_int_in_range(1, 100) -> rat_roll (u32)
    pcg_state, rat_roll = pcg32_next_int_in_range(pcg_state, pcg_inc, 1, 100)
    if rat_roll <= 40:
        # Draw 3: pcg.next_int_in_range(1, 5) -> rat_factor (u32)
        pcg_state, rat_factor = pcg32_next_int_in_range(pcg_state, pcg_inc, 1, 5)
        eaten = bushels_stored // rat_factor
        rats_eaten = min(eaten, 0xFFFFFFFF)
    else:
        rats_eaten = 0
    bushels_final = min(max(bushels_stored - rats_eaten, 0), 0xFFFFFFFF)

    # Step 3: Plague Draw
    # Draw 4: pcg.next_int_in_range(1, 100) -> plague_roll (u32)
    pcg_state, plague_roll = pcg32_next_int_in_range(pcg_state, pcg_inc, 1, 100)
    plague_struck = plague_roll <= 15

    # Step 4: Starvation & Impeachment Evaluation
    people_fed = order.bushels_to_feed // 20
    if people_fed >= state.population:
        starved = 0
        survivors = state.population
    else:
        starved = state.population - people_fed
        survivors = people_fed

    percent_starved = (
        min((starved * 100) // state.population, 100)
        if state.population > 0
        else 0
    )
    new_starved_total = min(state.starved_total + starved, 0xFFFFFFFF)
    new_percent_sum = min(state.percent_starved_sum + percent_starved, 0xFFFFFFFF)

    if starved * 100 > state.population * 45:
        new_status: GameStatus = Impeached(
            starved_this_turn=starved, percent=percent_starved
        )
    elif survivors == 0:
        new_status = GameOverExtinction()
    else:
        new_status = Playing()

    # Step 5: Plague Resolution
    if plague_struck and isinstance(new_status, Playing):
        survivors //= 2

    # Step 6: Immigration Draw
    # Draw 5: pcg.next_int_in_range(1, 5) -> imm_factor (u32)
    pcg_state, imm_factor = pcg32_next_int_in_range(pcg_state, pcg_inc, 1, 5)
    if starved == 0 and isinstance(new_status, Playing):
        acres_term = min(acres_after_trade * 20, MASK64)
        inner = acres_term + bushels_final
        numerator = imm_factor * inner
        denominator = 100 * state.population
        immigrants = (
            min((numerator // denominator) + 1, 0xFFFFFFFF)
            if denominator > 0
            else 0
        )
    else:
        immigrants = 0

    final_population = min(survivors + immigrants, 0xFFFFFFFF)

    # Step 7: Next Year Land Price Draw
    # Draw 6: pcg.next_int_in_range(17, 26) -> next_land_price (u32)
    pcg_state, next_land_price = pcg32_next_int_in_range(
        pcg_state, pcg_inc, 17, 26
    )

    # Step 8: Turn Advance & Rating
    next_year = min(state.year + 1, 255)
    if next_year == 11 and isinstance(new_status, Playing):
        avg_starved = new_percent_sum // 10
        acres_per_person = (
            (acres_after_trade // final_population) if final_population > 0 else 0
        )
        if avg_starved <= 3 and acres_per_person >= 10:
            rating = Rating.CHARLEMAGNE
        elif avg_starved <= 10 and acres_per_person >= 9:
            rating = Rating.ACRE
        elif avg_starved <= 33 and acres_per_person >= 7:
            rating = Rating.NERO
        else:
            rating = Rating.DISASTER
        new_status = Won(rating=rating)

    return HamurabiState(
        year=next_year,
        population=final_population,
        bushels=bushels_final,
        acres=acres_after_trade,
        land_price=next_land_price,
        harvest_per_acre=harvest_per_acre,
        rats_eaten=rats_eaten,
        starved_last_year=starved,
        starved_total=new_starved_total,
        percent_starved_sum=new_percent_sum,
        immigrants_last_year=immigrants,
        plague_struck=plague_struck,
        status=new_status,
        rng_state=pcg_state,
        rng_inc=pcg_inc,
    )


def format_report(state: HamurabiState) -> str:
    """Formats annual report or final evaluation in authentic teletype styling."""
    if isinstance(state.status, Impeached):
        return (
            f"YOU STARVED {state.status.starved_this_turn} PEOPLE IN ONE YEAR!!!\n"
            "DUE TO THIS EXTREME MISMANAGEMENT YOU HAVE BEEN IMPEACHED AND "
            "THROWN OUT OF OFFICE!\n"
        )
    elif isinstance(state.status, GameOverExtinction):
        return "EVERYONE HAS DIED. THE CITY OF LAGASH IS EXTINCT!\n"
    elif isinstance(state.status, Won):
        avg_starved = state.percent_starved_sum // 10
        acres_per_person = (
            (state.acres // state.population) if state.population > 0 else 0
        )
        r = state.status.rating
        if r == Rating.CHARLEMAGNE or r == "Charlemagne":
            rating_msg = (
                "A FANTASTIC PERFORMANCE! CHARLEMAGNE, DISRAELI, AND JEFFERSON "
                "COMBINED COULD NOT HAVE DONE BETTER!"
            )
        elif r == Rating.ACRE or r == "Acre":
            rating_msg = (
                "YOUR PERFORMANCE COULD HAVE BEEN WORSE, BUT YOU LEFT THEM "
                "REASONABLE ACREAGE."
            )
        elif r == Rating.NERO or r == "Nero":
            rating_msg = (
                "YOUR PERFORMANCE LEAVES MUCH TO BE DESIRED. NERO WOULD HAVE "
                "BEEN PROUD OF YOU."
            )
        else:
            rating_msg = (
                "YOU DID SO POORLY YOU HAVE BEEN DECLARED AN OFFICIAL "
                "NATIONAL DISASTER!"
            )

        return (
            f"IN YOUR 10-YEAR TERM OF OFFICE, {avg_starved} PERCENT OF THE\n"
            "POPULATION STARVED PER YEAR ON AVERAGE.\n"
            f"A TOTAL OF {state.starved_total} PEOPLE DIED!!\n"
            "YOU STARTED WITH 10 ACRES PER PERSON AND ENDED WITH\n"
            f"{acres_per_person} ACRES PER PERSON.\n\n"
            f"{rating_msg}\n"
        )
    else:  # Playing
        lines = [
            "HAMURABI: I BEG TO REPORT TO YOU,",
            (
                f"IN YEAR {state.year}, {state.starved_last_year} PEOPLE STARVED, "
                f"{state.immigrants_last_year} CAME TO THE CITY."
            ),
        ]
        if state.plague_struck:
            lines.append("A HORRIBLE PLAGUE STRUCK! HALF THE PEOPLE DIED.")
        lines.extend(
            [
                f"POPULATION IS NOW {state.population}.",
                f"THE CITY NOW OWNS {state.acres} ACRES.",
                f"YOU HARVESTED {state.harvest_per_acre} BUSHELS PER ACRE.",
                f"RATS ATE {state.rats_eaten} BUSHELS.",
                f"YOU NOW HAVE {state.bushels} BUSHELS IN STORE.",
                f"LAND IS TRADING AT {state.land_price} BUSHELS PER ACRE.",
            ]
        )
        return "\n".join(lines) + "\n"


__all__ = [
    "DEFAULT_INC",
    "HAMURABI_STREAM_ID",
    "MASK32",
    "MASK64",
    "PCG_MULTIPLIER",
    "GameAlreadyEnded",
    "GameOverExtinction",
    "GameStatus",
    "HamurabiError",
    "HamurabiState",
    "Impeached",
    "InsufficientAcresToPlant",
    "InsufficientAcresToSell",
    "InsufficientBushelsForLand",
    "InsufficientBushelsToFeed",
    "InsufficientGrainForSeed",
    "InsufficientPopulationToPlant",
    "InvalidOrderBounds",
    "Playing",
    "Rating",
    "TurnOrder",
    "Won",
    "format_report",
    "new_game",
    "parse_status",
    "pcg32_next_int_in_range",
    "pcg32_step",
    "step_year",
    "validate_order",
]
