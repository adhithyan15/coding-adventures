"""HardwareVM — event-driven simulator for HIR.

v0.1.0 scope:
- Combinational continuous assignments (ContAssign)
- Top-level port driving (drive a port from outside via `set_input`)
- Output port reading (`read`)
- Subscriber callbacks for value-change events (consumed by vcd-writer)

Behavioral processes (always blocks, initial blocks, wait/@/#) are documented
as v0.2.0 work; this v0.1.0 simulator is sufficient for the canonical 4-bit
adder smoke test and any pure combinational design.
"""

from __future__ import annotations

import heapq
import itertools
from collections.abc import Callable
from dataclasses import dataclass, field

from hdl_ir import (
    HIR,
    Concat,
    ContAssign,
    Direction,
    Expr,
    NetRef,
    PortRef,
    Slice,
)
from hdl_ir.types import width as ty_width

from hardware_vm.eval import evaluate, referenced_signals


@dataclass
class Event:
    """A signal value-change event. Subscribers receive these."""

    time: int
    signal: str
    new_value: int
    old_value: int


@dataclass
class RunResult:
    """Stats from a simulation run."""

    final_time: int
    event_count: int
    cont_assign_runs: int


@dataclass(order=True)
class _ScheduledUpdate:
    """One pending signal update in the event queue.

    Ordered by (time, delta, seq) for deterministic delta-cycle ordering.
    """

    time: int
    delta: int
    seq: int = field(compare=True)
    signal: str = field(compare=False)
    new_value: int = field(compare=False)


class HardwareVM:
    """Event-driven simulator for an HIR document.

    Construct it with an HIR; drive inputs via `set_input`; advance via
    `run` or `step`. Outputs are read with `read`. Subscribe to events
    via `subscribe` (the vcd-writer is the canonical consumer).
    """

    def __init__(self, hir: HIR) -> None:
        self.hir = hir
        self.time: int = 0
        self.delta: int = 0
        self._values: dict[str, int] = {}
        self._widths: dict[str, int] = {}
        self._event_queue: list[_ScheduledUpdate] = []
        self._seq_counter = itertools.count()
        self._subscribers: list[Callable[[Event], None]] = []
        self._cont_assigns: list[tuple[ContAssign, set[str]]] = []
        self._signal_to_cont_assigns: dict[str, list[int]] = {}
        self._cont_assign_runs: int = 0
        self._event_count: int = 0
        self._forced: dict[str, int] = {}

        self._initialize()

    def _initialize(self) -> None:
        """Walk the top module and prepare initial state + sensitivity tables."""
        if self.hir.top not in self.hir.modules:
            raise ValueError(f"top module {self.hir.top!r} not in HIR")

        top_mod = self.hir.modules[self.hir.top]

        # Initialize port values + widths
        for p in top_mod.ports:
            self._values[p.name] = 0
            try:
                self._widths[p.name] = ty_width(p.type)
            except ValueError:
                self._widths[p.name] = 1

        # Initialize net values + widths
        for n in top_mod.nets:
            self._values[n.name] = 0
            try:
                self._widths[n.name] = ty_width(n.type)
            except ValueError:
                self._widths[n.name] = 1

        # Index continuous assignments by signal sensitivity.
        for ca in top_mod.cont_assigns:
            sens = referenced_signals(ca.rhs)
            idx = len(self._cont_assigns)
            self._cont_assigns.append((ca, sens))
            for sig in sens:
                self._signal_to_cont_assigns.setdefault(sig, []).append(idx)

        # Run all continuous assigns once at t=0 to compute initial outputs
        # from initial input values.
        self._evaluate_all_cont_assigns()

    def _evaluate_all_cont_assigns(self) -> None:
        """Bootstrap: run every cont_assign once."""
        for idx, _ in enumerate(self._cont_assigns):
            self._run_cont_assign(idx)

    def _run_cont_assign(self, idx: int) -> None:
        """Evaluate one ContAssign and apply its result."""
        ca, _ = self._cont_assigns[idx]
        self._cont_assign_runs += 1
        rhs_value = evaluate(ca.rhs, self._lookup)
        self._apply_lhs(ca.target, rhs_value)

    def _apply_lhs(self, target: Expr, value: int) -> None:
        """Apply a computed value to an lvalue (port, net, slice, or concat)."""
        if isinstance(target, (NetRef, PortRef)):
            self._update_signal(target.name, value)
            return
        if isinstance(target, Slice):
            base_name = self._extract_base_name(target.base)
            if base_name is None:
                return
            msb, lsb = target.msb, target.lsb
            if msb < lsb:
                msb, lsb = lsb, msb
            width = msb - lsb + 1
            mask = (1 << width) - 1
            old = self._values.get(base_name, 0)
            new_bits = (value & mask) << lsb
            clear_mask = ~(mask << lsb)
            new_total = (old & clear_mask) | new_bits
            self._update_signal(base_name, new_total)
            return
        if isinstance(target, Concat):
            # Decompose value across parts in MSB-first order.
            widths = [self._signal_width(p) for p in target.parts]
            offset = sum(widths)
            for part, w in zip(target.parts, widths, strict=False):
                offset -= w
                part_val = (value >> offset) & ((1 << w) - 1)
                self._apply_lhs(part, part_val)
            return

    def _extract_base_name(self, expr: Expr) -> str | None:
        if isinstance(expr, (NetRef, PortRef)):
            return expr.name
        if isinstance(expr, Slice):
            return self._extract_base_name(expr.base)
        return None

    def _signal_width(self, expr: Expr) -> int:
        """Return the bit-width of an lvalue expression, looking up named
        signals in the VM's width table."""
        if isinstance(expr, (NetRef, PortRef)):
            return self._widths.get(expr.name, 1)
        if isinstance(expr, Slice):
            return abs(expr.msb - expr.lsb) + 1
        if isinstance(expr, Concat):
            return sum(self._signal_width(p) for p in expr.parts)
        return 1

    def _update_signal(self, name: str, new_value: int) -> None:
        if name in self._forced:
            return  # forced signal ignores normal updates
        old = self._values.get(name, 0)
        if old == new_value:
            return
        self._values[name] = new_value
        self._event_count += 1
        # Notify subscribers
        ev = Event(time=self.time, signal=name, new_value=new_value, old_value=old)
        for cb in self._subscribers:
            cb(ev)
        # Trigger dependent ContAssigns
        for idx in self._signal_to_cont_assigns.get(name, []):
            self._run_cont_assign(idx)

    def _lookup(self, name: str) -> int:
        if name in self._forced:
            return self._forced[name]
        return self._values.get(name, 0)

    # ------------------------------------------------------------
    # Public API
    # ------------------------------------------------------------

    def set_input(self, signal: str, value: int) -> None:
        """Drive a top-level input port from outside the simulation."""
        top = self.hir.modules[self.hir.top]
        port = top.find_port(signal)
        if port is None:
            raise ValueError(f"signal {signal!r} not a port of top module")
        if port.direction != Direction.IN and port.direction != Direction.INOUT:
            raise ValueError(
                f"cannot set_input on {signal!r}: direction is {port.direction.value}"
            )
        self._update_signal(signal, value)

    def read(self, signal: str) -> int:
        """Read the current value of any signal (port or net)."""
        return self._lookup(signal)

    def force(self, signal: str, value: int) -> None:
        """Force a signal to a given value, overriding any normal driver."""
        self._forced[signal] = value
        old = self._values.get(signal, 0)
        if old != value:
            self._values[signal] = value
            self._event_count += 1
            ev = Event(time=self.time, signal=signal, new_value=value, old_value=old)
            for cb in self._subscribers:
                cb(ev)

    def release(self, signal: str) -> None:
        """Release a forced signal so normal drivers take over."""
        self._forced.pop(signal, None)
        # Re-evaluate any cont_assign that drives this signal
        for idx, (ca, _) in enumerate(self._cont_assigns):
            if self._extract_base_name(ca.target) == signal:
                self._run_cont_assign(idx)

    def subscribe(self, callback: Callable[[Event], None]) -> None:
        """Register a callback for every signal value-change."""
        self._subscribers.append(callback)

    def step(self) -> bool:
        """Advance one simulation step. Returns True if more events remain.

        For v0.1.0 (combinational only), this just advances time by 1 unit
        and returns False (no clocks, no event queue activity from the kernel).
        """
        if self._event_queue:
            update = heapq.heappop(self._event_queue)
            self.time = update.time
            self._update_signal(update.signal, update.new_value)
            return bool(self._event_queue)
        return False

    def run(self, until_time: int | None = None) -> RunResult:
        """Run until the event queue empties or `until_time` is reached.

        For v0.1.0 combinational designs, this drains any pending updates and
        advances time to `until_time` (default: stay at current time)."""
        while self._event_queue:
            update = self._event_queue[0]
            if until_time is not None and update.time > until_time:
                break
            heapq.heappop(self._event_queue)
            self.time = update.time
            self._update_signal(update.signal, update.new_value)

        if until_time is not None:
            self.time = max(self.time, until_time)

        return RunResult(
            final_time=self.time,
            event_count=self._event_count,
            cont_assign_runs=self._cont_assign_runs,
        )
