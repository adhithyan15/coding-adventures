"""Error types for the CLI Builder library.

=== Design philosophy ===

CLI Builder distinguishes two categories of failure:

1. **Spec errors** — the JSON specification file itself is wrong. These are
   caught at load time, before any argv is ever processed. A spec error means
   the developer made a mistake (circular requires, duplicate IDs, etc.). The
   user cannot fix them; only the developer can.

2. **Parse errors** — the user's invocation of the program is wrong. These are
   caught at parse time, per invocation. A parse error means the user passed
   an unrecognized flag, forgot a required argument, etc. They are displayed to
   the user with a helpful message.

Both categories flow through a common base class so callers can catch either
with ``except CliBuilderError``.

=== ParseError as a dataclass ===

``ParseError`` is a dataclass (not an exception) because many parse errors can
occur simultaneously. We collect them all and surface the full list at once,
giving the user a complete picture of what went wrong. The containing exception
is ``ParseErrors``, which formats them all for display.

=== Analogy: compiler diagnostics ===

A good compiler doesn't stop at the first syntax error — it reports all errors
it can find in one pass. CLI Builder does the same for argument validation.
"""

from __future__ import annotations

from dataclasses import dataclass, field


class CliBuilderError(Exception):
    """Base class for all CLI Builder errors.

    Both ``SpecError`` (developer mistake) and ``ParseErrors`` (user mistake)
    inherit from this, so a single ``except CliBuilderError`` catches all
    library failures.
    """


class SpecError(CliBuilderError):
    """Raised when the JSON specification file is invalid.

    Spec errors are fatal and are detected at load time. The library refuses
    to parse any argv until the spec is fixed.

    Examples:
        - Circular ``requires`` dependency (A requires B requires A)
        - Duplicate flag ``id`` within the same scope
        - Unknown flag ID in ``conflicts_with``
        - ``type: "enum"`` without ``enum_values``

    The ``message`` attribute contains a human-readable explanation.
    """

    def __init__(self, message: str) -> None:
        super().__init__(message)
        self.message = message

    def __str__(self) -> str:
        return f"CliBuilder spec error: {self.message}"


@dataclass
class ParseError:
    """A single parse-time error.

    This is a **dataclass**, not an exception. Multiple parse errors can occur
    simultaneously (e.g., missing required flag + invalid value). We collect
    all of them and surface the full list via ``ParseErrors``.

    Attributes:
        error_type: Machine-readable snake_case identifier. One of:
            ``unknown_command``, ``unknown_flag``, ``missing_required_flag``,
            ``missing_required_argument``, ``conflicting_flags``,
            ``missing_dependency_flag``, ``too_few_arguments``,
            ``too_many_arguments``, ``invalid_value``, ``invalid_enum_value``,
            ``exclusive_group_violation``, ``missing_exclusive_group``,
            ``duplicate_flag``, ``invalid_stack``.
        message: Human-readable sentence explaining the error.
        suggestion: Optional corrective hint (e.g., a fuzzy-matched flag name).
        context: The ``command_path`` at the point the error was detected.
    """

    error_type: str
    message: str
    suggestion: str | None = None
    context: list[str] = field(default_factory=list)

    def format(self) -> str:
        """Format this error as a user-facing string.

        Example output::

            error: unknown flag '--mesage'
              Did you mean: --message
              Context: git commit

        Returns:
            A formatted multi-line string.
        """
        lines = [f"error[{self.error_type}]: {self.message}"]
        if self.suggestion:
            lines.append(f"  Did you mean: {self.suggestion}")
        if self.context:
            lines.append(f"  Context: {' '.join(self.context)}")
        return "\n".join(lines)


class ParseErrors(CliBuilderError):
    """Raised when one or more parse-time errors are collected.

    Instead of failing on the first error, the parser collects all errors it
    can find and raises this exception at the end. This gives the user a
    complete picture of what is wrong with their invocation.

    Attributes:
        errors: The list of ``ParseError`` objects collected during parsing.

    Example::

        try:
            result = Parser("spec.json", argv).parse()
        except ParseErrors as e:
            print(e)   # formatted list of all errors
            raise SystemExit(1)
    """

    def __init__(self, errors: list[ParseError]) -> None:
        super().__init__(f"{len(errors)} parse error(s) found")
        self.errors: list[ParseError] = errors

    def __str__(self) -> str:
        """Format all errors, separated by blank lines."""
        parts = [e.format() for e in self.errors]
        return "\n\n".join(parts)
