"""
Pass Protocol
=============

A *pass* is a pure function ``LogicalPlan → LogicalPlan``. We model it as a
:class:`typing.Protocol` with a single callable method, so any class
instance or callable with the right shape is a pass.

Why a Protocol and not a plain function type?
---------------------------------------------

Two reasons:

1. **Composability.** Passes may in the future carry configuration (e.g. a
   predicate-pushdown pass with a cost-cap). A class-shaped Protocol makes
   attaching state trivial without changing the call site.
2. **Debug metadata.** Each pass has a ``name`` property. Pipeline drivers
   can log "before ConstantFolding" / "after ConstantFolding" with the
   class name, so test failures point to the exact pass that mis-rewrote.

Idempotence is a requirement. Every pass in this package runs at most
once per pipeline invocation, but idempotence means re-running is safe —
and that is important for the composition with user-supplied passes.
"""

from __future__ import annotations

from typing import Protocol, runtime_checkable

from sql_planner import LogicalPlan


@runtime_checkable
class Pass(Protocol):
    """One optimization pass. Must be idempotent."""

    name: str

    def __call__(self, plan: LogicalPlan) -> LogicalPlan: ...
