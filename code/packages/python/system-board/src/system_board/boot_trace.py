"""Boot phases and trace recording for the boot sequence."""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import IntEnum


class BootPhase(IntEnum):
    """Stage in the boot sequence."""

    POWER_ON = 0
    BIOS = 1
    BOOTLOADER = 2
    KERNEL_INIT = 3
    USER_PROGRAM = 4
    IDLE = 5

    def __str__(self: BootPhase) -> str:
        names = {
            0: "PowerOn", 1: "BIOS", 2: "Bootloader",
            3: "KernelInit", 4: "UserProgram", 5: "Idle",
        }
        return names.get(self.value, "Unknown")


@dataclass
class BootEvent:
    """A notable event during the boot sequence."""

    phase: BootPhase
    cycle: int
    description: str


@dataclass
class BootTrace:
    """Records the complete boot sequence."""

    events: list[BootEvent] = field(default_factory=list)

    def add_event(self: BootTrace, phase: BootPhase, cycle: int, description: str) -> None:
        """Append a new event to the trace."""
        self.events.append(BootEvent(phase=phase, cycle=cycle, description=description))

    def phases(self: BootTrace) -> list[BootPhase]:
        """Return distinct phases that occurred, in order."""
        seen: set[BootPhase] = set()
        result: list[BootPhase] = []
        for e in self.events:
            if e.phase not in seen:
                seen.add(e.phase)
                result.append(e.phase)
        return result

    def events_in_phase(self: BootTrace, phase: BootPhase) -> list[BootEvent]:
        """Return all events belonging to the given phase."""
        return [e for e in self.events if e.phase == phase]

    def total_cycles(self: BootTrace) -> int:
        """Return the cycle count of the last event, or 0."""
        if not self.events:
            return 0
        return self.events[-1].cycle

    def phase_start_cycle(self: BootTrace, phase: BootPhase) -> int:
        """Return the cycle at which the given phase began. Returns -1 if not found."""
        for e in self.events:
            if e.phase == phase:
                return e.cycle
        return -1
