"""Errors raised by ``algol-iir-compiler``."""

from __future__ import annotations


class AlgolIIRCompileError(Exception):
    """Base class for ALGOL to InterpreterIR lowering errors."""


class AlgolIIRUnsupportedError(AlgolIIRCompileError):
    """Raised when valid ALGOL uses a feature outside this IIR slice."""
