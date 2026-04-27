"""Immutable state snapshots for the GE-225 simulator."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class GE225Indicators:
    """Frozen snapshot of condition indicators exposed by the simulator."""

    carry: bool
    zero: bool
    negative: bool
    overflow: bool
    parity_error: bool


@dataclass(frozen=True)
class GE225State:
    """Frozen snapshot of GE-225 machine state."""

    a: int
    q: int
    m: int
    n: int
    pc: int
    ir: int
    indicators: GE225Indicators
    overflow: bool
    parity_error: bool
    decimal_mode: bool
    automatic_interrupt_mode: bool
    selected_x_group: int
    n_ready: bool
    typewriter_power: bool
    control_switches: int
    x_words: tuple[int, ...]
    halted: bool
    memory: tuple[int, ...]
