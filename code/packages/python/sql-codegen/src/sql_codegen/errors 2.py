"""
Codegen error hierarchy.

The code generator does not perform semantic validation — that was the
planner's job. These errors fire only for plans the compiler cannot
handle (e.g., a node-kind that hasn't been wired up yet) or for
defensive invariants (e.g., cursor-ID overflow in pathological plans).
"""

from __future__ import annotations


class CodegenError(Exception):
    """Base class for all code-generator failures."""


class UnsupportedNode(CodegenError):
    """Raised when a LogicalPlan node has no code-generation rule yet.

    Attributes:
        node_kind: The Python class name of the plan node we couldn't compile.
    """

    def __init__(self, node_kind: str) -> None:
        super().__init__(f"unsupported plan node: {node_kind}")
        self.node_kind = node_kind


class InternalError(CodegenError):
    """An invariant was violated — bug in the code generator itself."""

    def __init__(self, message: str) -> None:
        super().__init__(message)
        self.message = message
