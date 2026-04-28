"""
Virtual-machine error hierarchy
===============================

The VM surfaces every runtime failure as a :class:`VmError` subclass. The goal
is that the faceade layer (``mini-sqlite``) can translate these 1:1 into the
PEP 249 exception hierarchy — ``TypeMismatch`` → ``DataError``,
``ConstraintViolation`` → ``IntegrityError``, and so on — without inspecting
messages or guessing at intent.

Why not reuse :class:`sql_backend.BackendError` directly?
----------------------------------------------------------

The backend layer knows nothing about SQL types, arithmetic, or labels. Its
errors describe *storage-level* failures (no such table, constraint violated).
The VM produces its own errors on top of that (stack underflow, type
mismatch, unknown label). Keeping two hierarchies side by side makes the
layering explicit:

- If the failure is purely storage: ``BackendError`` with the concrete variant.
- If the failure comes from the dispatch loop or value semantics: ``VmError``.

The VM wraps every backend error it catches in :class:`BackendError` below so
*everything* propagating out of ``execute`` descends from ``VmError``. The
façade has a single root class to catch.

Dataclasses
-----------

We use frozen dataclasses with ``eq=True`` for the same reason the backend does:
cheap structural equality in tests and readable reprs in failed assertions.
"""

from __future__ import annotations

from dataclasses import dataclass


class VmError(Exception):
    """Root of every error the VM may raise.

    A single ``except VmError`` in the façade is enough to catch every possible
    failure during query execution. Subclasses carry the structured fields a
    caller might want to introspect.
    """


@dataclass(eq=True)
class TableNotFound(VmError):
    """Raised when an instruction references a table the backend doesn't know."""

    table: str

    def __str__(self) -> str:
        return f"table not found: {self.table!r}"


@dataclass(eq=True)
class ColumnNotFound(VmError):
    """Raised when ``LoadColumn`` asks for a column the backend's row lacks."""

    cursor_id: int
    column: str

    def __str__(self) -> str:
        return f"column not found: cursor={self.cursor_id} column={self.column!r}"


@dataclass(eq=True)
class TypeMismatch(VmError):
    """Raised when arithmetic or comparison receives incompatible types.

    ``context`` names the instruction that produced the mismatch so the message
    is self-explanatory without a stack trace: ``"BinaryOp(ADD)"``,
    ``"UnaryOp(NEG)"``.
    """

    expected: str
    got: str
    context: str

    def __str__(self) -> str:
        return f"type mismatch in {self.context}: expected {self.expected}, got {self.got}"


class DivisionByZero(VmError):
    """Raised when the second operand of DIV or MOD is zero."""

    def __str__(self) -> str:
        return "division by zero"


@dataclass(eq=True)
class ConstraintViolation(VmError):
    """Raised for backend-reported NOT NULL / UNIQUE / PK failures.

    We wrap the backend's ``ConstraintViolation`` so callers only see VmError.
    """

    table: str
    column: str
    message: str

    def __str__(self) -> str:
        return self.message


@dataclass(eq=True)
class TableAlreadyExists(VmError):
    """Raised when ``CreateTable(if_not_exists=False)`` finds the table present."""

    table: str

    def __str__(self) -> str:
        return f"table already exists: {self.table!r}"


@dataclass(eq=True)
class ColumnAlreadyExists(VmError):
    """Raised when ``AlterTable`` tries to add a column that already exists."""

    table: str
    column: str

    def __str__(self) -> str:
        return f"column already exists: {self.table!r}.{self.column!r}"


class StackUnderflow(VmError):
    """Raised by ``pop()`` on an empty stack. Indicates a codegen bug."""

    def __str__(self) -> str:
        return "stack underflow"


@dataclass(eq=True)
class InvalidLabel(VmError):
    """Raised when a ``Jump*`` instruction's label is not in ``program.labels``.

    Only possible if the program was assembled by something other than the
    vetted ``sql_codegen.compile`` — i.e. a codegen bug.
    """

    label: str

    def __str__(self) -> str:
        return f"invalid label: {self.label!r}"


@dataclass(eq=True)
class BackendError(VmError):
    """Wraps an exception raised by the backend.

    ``original`` is the underlying :class:`sql_backend.BackendError`. Callers
    that want to distinguish e.g. ``Unsupported`` can ``isinstance``-check it.
    """

    message: str
    original: Exception | None = None

    def __str__(self) -> str:
        return self.message


@dataclass(eq=True)
class InternalError(VmError):
    """Raised when an invariant is violated (should not happen in production).

    Useful in ``assert``-style checks that defend against malformed programs.
    """

    message: str

    def __str__(self) -> str:
        return self.message


@dataclass(eq=True)
class UnsupportedFunction(VmError):
    """Raised by ``CallScalar`` when the function name is not in the registry.

    This happens when SQL uses a function that mini-sqlite does not yet
    implement (e.g. a vendor extension or a user-defined function not
    registered via :func:`~sql_vm.vm.register_scalar`).

    The ``name`` field is the lower-cased function name as it appears in
    the SQL source (e.g. ``"json_extract"``, ``"my_custom_fn"``).
    """

    name: str

    def __str__(self) -> str:
        return f"unknown scalar function: {self.name!r}"


@dataclass(eq=True)
class TransactionError(VmError):
    """Raised when a transaction-control instruction is used incorrectly.

    Common causes:

    - ``BEGIN`` while a transaction is already active (nested transactions
      are not supported in v1).
    - ``COMMIT`` or ``ROLLBACK`` when no transaction is active.

    ``message`` describes the specific problem; callers should treat this as
    a programming error rather than a transient failure.
    """

    message: str

    def __str__(self) -> str:
        return self.message


@dataclass(eq=True)
class WrongNumberOfArguments(VmError):
    """Raised when a scalar function is called with the wrong argument count.

    ``name`` is the function name; ``expected`` describes the arity
    (e.g. ``"1"`` or ``"1 or 2"`` for optional arguments); ``got`` is the
    actual count supplied by the caller.
    """

    name: str
    expected: str
    got: int

    def __str__(self) -> str:
        return (
            f"wrong number of arguments to {self.name!r}: "
            f"expected {self.expected}, got {self.got}"
        )


@dataclass(eq=True)
class TriggerDepthError(VmError):
    """Raised when trigger recursion exceeds the maximum depth (16).

    Recursive triggers — where a trigger body causes the same trigger to
    fire again — are supported up to depth 16.  Beyond that the engine
    stops and raises this error to prevent infinite recursion.

    ``trigger_name`` is the name of the trigger whose firing pushed the
    recursion past the limit.
    """

    trigger_name: str

    def __str__(self) -> str:
        return f"trigger recursion depth exceeded in {self.trigger_name!r}"


@dataclass(eq=True)
class CardinalityError(VmError):
    """Raised when a scalar subquery returns more than one row.

    SQL requires a scalar subquery — ``(SELECT expr FROM …)`` used in an
    expression position — to return *at most one row*.  When the inner query
    produces two or more rows the result is undefined in SQL and a runtime
    error in this implementation.  Returning zero rows is not an error; the
    value is ``NULL`` in that case.
    """

    message: str = "scalar subquery returned more than one row"

    def __str__(self) -> str:
        return self.message
