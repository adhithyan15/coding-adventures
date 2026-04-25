"""HDL coverage measurement.

Subscribes to ``hardware-vm`` value-change events to record:
- Toggle coverage (each signal: 0->1 and 1->0 transitions seen).
- Functional coverage via covergroups: bins matched on signal values.

Code coverage (line/branch) is documented as v0.2.0 work; it requires HIR
provenance instrumentation that's not yet in the simulator path.
"""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass, field

# ---------------------------------------------------------------------------
# Bin definitions
# ---------------------------------------------------------------------------


@dataclass(frozen=True, slots=True)
class Bin:
    """One bin in a coverpoint."""

    name: str
    matcher: Callable[[int], bool]


def bin_value(name: str, value: int) -> Bin:
    """Bin matching exactly one value."""
    return Bin(name, lambda v: v == value)


def bin_range(name: str, min: int, max: int) -> Bin:
    """Bin matching values in [min, max] inclusive."""
    return Bin(name, lambda v: min <= v <= max)


def bin_default() -> Bin:
    """Bin matching anything (catch-all)."""
    return Bin("default", lambda _v: True)


# ---------------------------------------------------------------------------
# Coverpoint
# ---------------------------------------------------------------------------


@dataclass
class Coverpoint:
    """A coverpoint watches one signal; samples produce hits in matching bins."""

    name: str
    signal: str
    bins: list[Bin]
    hits: dict[str, int] = field(default_factory=dict)

    def __post_init__(self) -> None:
        for b in self.bins:
            self.hits.setdefault(b.name, 0)

    def sample(self, value: int) -> None:
        for b in self.bins:
            if b.matcher(value):
                self.hits[b.name] = self.hits.get(b.name, 0) + 1
                return  # first-match-wins

    @property
    def coverage(self) -> float:
        if not self.bins:
            return 1.0
        hit_count = sum(1 for b in self.bins if self.hits.get(b.name, 0) > 0)
        return hit_count / len(self.bins)


@dataclass
class CrossPoint:
    """Cross-product of two or more coverpoints. Records the joint hit-count
    of every (bin_in_a, bin_in_b, ...) combination."""

    name: str
    coverpoints: list[Coverpoint]
    hits: dict[tuple[str, ...], int] = field(default_factory=dict)
    _last_values: dict[str, int] = field(default_factory=dict)

    def sample(self) -> None:
        """Record one sample, using the last-seen value for each constituent
        coverpoint's signal."""
        bins_hit = []
        for cp in self.coverpoints:
            v = self._last_values.get(cp.signal)
            if v is None:
                return
            matched = next((b.name for b in cp.bins if b.matcher(v)), None)
            if matched is None:
                return
            bins_hit.append(matched)
        key = tuple(bins_hit)
        self.hits[key] = self.hits.get(key, 0) + 1

    @property
    def coverage(self) -> float:
        if not self.coverpoints:
            return 1.0
        total_combos = 1
        for cp in self.coverpoints:
            total_combos *= max(1, len(cp.bins))
        hit_combos = sum(1 for v in self.hits.values() if v > 0)
        return hit_combos / total_combos


# ---------------------------------------------------------------------------
# Coverage recorder
# ---------------------------------------------------------------------------


@dataclass
class ToggleStats:
    """Per-signal toggle counts."""

    rising: int = 0
    falling: int = 0


@dataclass
class CoverageReport:
    coverpoints: dict[str, dict[str, int]]
    crosses: dict[str, dict[tuple[str, ...], int]]
    toggle: dict[str, ToggleStats]


class CoverageRecorder:
    """Subscribes to a HardwareVM, records value changes into coverage data."""

    def __init__(self, vm: object) -> None:
        self._vm = vm
        self._coverpoints: dict[str, Coverpoint] = {}
        self._crosses: dict[str, CrossPoint] = {}
        self._toggle_signals: set[str] = set()
        self._toggle: dict[str, ToggleStats] = {}
        self._last_values: dict[str, int] = {}

        # Subscribe.
        if hasattr(vm, "subscribe"):
            vm.subscribe(self._on_event)  # type: ignore[attr-defined]

    # ----- Coverpoint registration -----

    def add_coverpoint(self, cp: Coverpoint) -> None:
        self._coverpoints[cp.name] = cp

    def add_cross(self, cross: CrossPoint) -> None:
        self._crosses[cross.name] = cross
        # Make sure the cross sees subsequent value changes.

    def enable_toggle_coverage(self, signals: list[str]) -> None:
        for s in signals:
            self._toggle_signals.add(s)
            self._toggle.setdefault(s, ToggleStats())

    # ----- Event handling -----

    def _on_event(self, event: object) -> None:
        signal = getattr(event, "signal", None)
        new_value = getattr(event, "new_value", None)
        old_value = getattr(event, "old_value", 0)

        if signal is None or new_value is None:
            return

        # Record value for any covergroup using this signal as a watch.
        self._last_values[signal] = int(new_value)

        # Toggle counting (1-bit basis).
        if signal in self._toggle_signals:
            stats = self._toggle.setdefault(signal, ToggleStats())
            if int(old_value) == 0 and int(new_value) != 0:
                stats.rising += 1
            elif int(old_value) != 0 and int(new_value) == 0:
                stats.falling += 1

        # Sample coverpoints whose signal matches.
        for cp in self._coverpoints.values():
            if cp.signal == signal:
                cp.sample(int(new_value))

        # Notify crosses
        for cross in self._crosses.values():
            cross._last_values[signal] = int(new_value)

    # ----- Manual sampling -----

    def sample_cross(self, cross_name: str | None = None) -> None:
        """Sample one cross (or all crosses). Useful for clock-edge sampling."""
        if cross_name is None:
            for c in self._crosses.values():
                c.sample()
        else:
            self._crosses[cross_name].sample()

    # ----- Reporting -----

    def report(self) -> CoverageReport:
        return CoverageReport(
            coverpoints={name: dict(cp.hits) for name, cp in self._coverpoints.items()},
            crosses={name: dict(c.hits) for name, c in self._crosses.items()},
            toggle=dict(self._toggle),
        )

    @property
    def overall_coverage(self) -> float:
        """Average coverage across all registered coverpoints + crosses."""
        items: list[float] = []
        for cp in self._coverpoints.values():
            items.append(cp.coverage)
        for cr in self._crosses.values():
            items.append(cr.coverage)
        if not items:
            return 0.0
        return sum(items) / len(items)
