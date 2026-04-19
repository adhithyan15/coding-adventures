"""sql_optimizer — pure rewrite passes over sql-planner's LogicalPlan tree."""

from .constant_folding import ConstantFolding
from .dead_code import DeadCodeElimination
from .driver import default_passes, optimize, optimize_with_passes
from .limit_pushdown import LimitPushdown
from .pass_protocol import Pass
from .predicate_pushdown import PredicatePushdown
from .projection_pruning import ProjectionPruning

__all__ = [
    "optimize",
    "optimize_with_passes",
    "default_passes",
    "Pass",
    "ConstantFolding",
    "PredicatePushdown",
    "ProjectionPruning",
    "DeadCodeElimination",
    "LimitPushdown",
]
