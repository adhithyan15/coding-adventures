"""
The driver — ``optimize`` and ``optimize_with_passes``.

``optimize`` runs the fixed pipeline defined in the spec. A caller who
wants to test a single pass in isolation uses ``optimize_with_passes``
with their own list; every pass is both a callable and an instance of
:class:`Pass` (see :mod:`sql_optimizer.pass_protocol`).

No fixpoint iteration — each pass runs exactly once in the given order.
Convergence loops are easy to misconfigure and the v1 pass set has no
mutual-recursion patterns that would benefit.
"""

from __future__ import annotations

from sql_planner import LogicalPlan

from .constant_folding import ConstantFolding
from .dead_code import DeadCodeElimination
from .limit_pushdown import LimitPushdown
from .pass_protocol import Pass
from .predicate_pushdown import PredicatePushdown
from .projection_pruning import ProjectionPruning


def default_passes() -> list[Pass]:
    """The default pipeline. Order is fixed by the spec."""
    return [
        ConstantFolding(),
        PredicatePushdown(),
        ProjectionPruning(),
        DeadCodeElimination(),
        LimitPushdown(),
    ]


def optimize(plan: LogicalPlan) -> LogicalPlan:
    """Run the default pass pipeline over ``plan``."""
    return optimize_with_passes(plan, default_passes())


def optimize_with_passes(plan: LogicalPlan, passes: list[Pass]) -> LogicalPlan:
    """Run ``passes`` in order. Callers control pass selection and order."""
    current = plan
    for p in passes:
        current = p(current)
    return current
