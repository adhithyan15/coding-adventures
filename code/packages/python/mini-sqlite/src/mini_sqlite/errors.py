"""
PEP 249 DB-API 2.0 exception hierarchy — and the translation layer
that maps errors from the underlying pipeline packages onto it.

PEP 249 prescribes a rigid tree::

    Exception
     └── Warning
     └── Error
          ├── InterfaceError      — misuse of the database interface itself
          └── DatabaseError       — problems with the database, not us
               ├── DataError           — type conversion / out-of-range values
               ├── OperationalError    — runtime failures (missing table, etc.)
               ├── IntegrityError      — constraint violations
               ├── InternalError       — backend bug
               ├── ProgrammingError    — bad SQL (syntax, wrong param count)
               └── NotSupportedError   — feature not implemented

Application code is expected to catch at whichever level is most
appropriate — and a compliant driver must raise errors in this exact
hierarchy for portability.

The pipeline packages (sql-planner / sql-codegen / sql-vm /
sql-backend) have their own error hierarchies. This module owns the
mapping. Every entry point into the facade funnels through
:func:`translate` so the user only ever sees PEP 249 exceptions.
"""

from __future__ import annotations

# --------------------------------------------------------------------------
# PEP 249 base classes.
# --------------------------------------------------------------------------


class Warning(Exception):  # noqa: A001, N818 — PEP 249 spelling
    """Important warnings like data truncations. Not an error."""


class Error(Exception):
    """Root of every database-related error. PEP 249 base class."""


class InterfaceError(Error):
    """Errors related to the database interface itself (not the DB)."""


class DatabaseError(Error):
    """Errors related to the database."""


class DataError(DatabaseError):
    """Problems with the processed data (bad values, out of range)."""


class OperationalError(DatabaseError):
    """Errors related to the database's operation — runtime failures."""


class IntegrityError(DatabaseError):
    """Constraint violations (unique, not-null, foreign key)."""


class InternalError(DatabaseError):
    """Database internal errors — bugs in the underlying engine."""


class ProgrammingError(DatabaseError):
    """Programming errors — bad SQL, wrong param count, misuse."""


class NotSupportedError(DatabaseError):
    """Feature not supported by the database."""


# --------------------------------------------------------------------------
# Translation. Called at every pipeline-entry boundary.
# --------------------------------------------------------------------------


def translate(exc: BaseException) -> Exception:
    """Map a pipeline exception onto the corresponding PEP 249 error.

    Strategy: check for known pipeline exception *types* first, then fall
    back to ``InternalError`` for anything unrecognized. We import error
    classes lazily inside the function so that users who never hit an
    error path don't pay the import cost of every pipeline package.
    """

    # sql-lexer / sql-parser: syntax errors — the user wrote invalid SQL.
    try:
        from lang_parser.grammar_parser import GrammarParseError

        if isinstance(exc, GrammarParseError):
            return ProgrammingError(str(exc))
    except ImportError:  # pragma: no cover
        pass
    try:
        from lexer.tokenizer import LexerError

        if isinstance(exc, LexerError):
            return ProgrammingError(str(exc))
    except ImportError:  # pragma: no cover
        pass

    # sql-planner: planning-time errors — bad SQL semantics.
    try:
        from sql_planner.errors import (
            AmbiguousColumn,
            InvalidAggregate,
            UnknownColumn,
            UnknownTable,
            UnsupportedStatement,
        )

        if isinstance(exc, AmbiguousColumn | InvalidAggregate):
            return ProgrammingError(str(exc))
        if isinstance(exc, UnknownTable | UnknownColumn):
            return OperationalError(str(exc))
        if isinstance(exc, UnsupportedStatement):
            return NotSupportedError(str(exc))
    except ImportError:  # pragma: no cover — sql-planner is a hard dep
        pass

    # sql-codegen: unsupported IR nodes.
    try:
        from sql_codegen import CodegenError, UnsupportedNode

        if isinstance(exc, UnsupportedNode):
            return NotSupportedError(str(exc))
        if isinstance(exc, CodegenError):
            return InternalError(str(exc))
    except ImportError:  # pragma: no cover
        pass

    # sql-vm: runtime errors — table lookup, constraints, types.
    try:
        from sql_vm import (
            ColumnNotFound,
            ConstraintViolation,
            DivisionByZero,
            TableAlreadyExists,
            TableNotFound,
            TypeMismatch,
            VmError,
        )

        if isinstance(exc, TableNotFound | ColumnNotFound | TableAlreadyExists):
            return OperationalError(str(exc))
        if isinstance(exc, ConstraintViolation):
            return IntegrityError(str(exc))
        if isinstance(exc, TypeMismatch):
            return DataError(str(exc))
        if isinstance(exc, DivisionByZero):
            return OperationalError(str(exc))
        if isinstance(exc, VmError):
            return InternalError(str(exc))
    except ImportError:  # pragma: no cover
        pass

    # sql-backend: storage-layer errors.
    try:
        import sql_backend.errors as be

        if isinstance(exc, be.Unsupported):
            return NotSupportedError(str(exc))
        if isinstance(exc, be.ConstraintViolation):
            return IntegrityError(str(exc))
        if isinstance(exc, be.TableNotFound | be.TableAlreadyExists | be.ColumnNotFound):
            return OperationalError(str(exc))
        if isinstance(exc, be.BackendError):
            return InternalError(str(exc))
    except ImportError:  # pragma: no cover
        pass

    # Facade-level programming errors pass through unchanged.
    if isinstance(exc, Error):
        return exc

    # Anything else: treat as an internal bug.
    return InternalError(f"unexpected error: {type(exc).__name__}: {exc}")
