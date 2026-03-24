"""Output types for CLI Builder parse results and validation.

=== Three kinds of parse result ===

A CLI invocation can produce one of three outcomes:

1. **ParseResult** — the user's arguments were valid and fully parsed. This is
   the normal case. Flags and arguments are ready for the application to use.

2. **HelpResult** — the user passed ``--help`` or ``-h``. The library generates
   and returns the help text for the deepest resolved command. The application
   should print it and exit 0.

3. **VersionResult** — the user passed ``--version``. The library returns the
   version string from the spec. The application should print it and exit 0.

=== Using isinstance to dispatch ===

The idiomatic way to handle these results is::

    result = Parser("spec.json", argv).parse()

    if isinstance(result, HelpResult):
        print(result.text)
        raise SystemExit(0)
    elif isinstance(result, VersionResult):
        print(result.version)
        raise SystemExit(0)
    else:
        # isinstance(result, ParseResult) guaranteed here
        run_application(result.flags, result.arguments)

This pattern is the same as Rust's ``match`` on an enum variant — each arm
handles a structurally distinct outcome with no ambiguity.
"""

from __future__ import annotations

from dataclasses import dataclass, field


@dataclass
class ParseResult:
    """A fully-parsed, validated CLI invocation.

    All flags in scope are present in ``flags``. Absent optional flags have
    ``False`` (for booleans) or ``None`` (for value-taking flags), or the
    ``default`` value from the spec.

    Attributes:
        program: ``argv[0]`` — the program name as invoked.
        command_path: Full path of commands from root to resolved leaf.
            For a root-level invocation: ``["program-name"]``.
            For ``git remote add``: ``["git", "remote", "add"]``.
        flags: Map from flag ``id`` to parsed, coerced value.
            Repeatable flags produce lists. Count flags produce integers.
        arguments: Map from argument ``id`` to parsed, coerced value.
            Variadic arguments produce lists.
        explicit_flags: List of flag IDs that were explicitly set by the user
            on the command line. Every time a flag token is consumed from argv,
            its ID is appended here. A flag that appears multiple times will
            appear multiple times in this list. This enables callers to
            distinguish "user passed --color=auto" from "auto is the default".
    """

    program: str
    command_path: list[str]
    flags: dict[str, object]
    arguments: dict[str, object] = field(default_factory=dict)
    explicit_flags: list[str] = field(default_factory=list)


@dataclass
class HelpResult:
    """Result of a ``--help`` / ``-h`` flag.

    The application should print ``text`` and exit 0.

    Attributes:
        text: The formatted help text, generated per spec §9.
        command_path: The command path for which help was generated.
            Used to identify which command's help was shown.
    """

    text: str
    command_path: list[str]


@dataclass
class VersionResult:
    """Result of a ``--version`` flag.

    The application should print ``version`` and exit 0.

    Attributes:
        version: The version string from the spec's ``version`` field.
    """

    version: str


# =========================================================================
# Validation result
# =========================================================================


@dataclass
class ValidationResult:
    """The outcome of validating a CLI Builder JSON spec.

    === Why a result instead of an exception? ===

    The ``SpecLoader.load()`` method raises ``SpecError`` on the first problem
    it finds. That's the right behavior for *running* a CLI — you want to fail
    fast and loudly.

    But sometimes you want to *check* a spec without crashing. For example:

    - A linter that validates spec files in CI
    - An editor plugin that shows red squiggles on invalid specs
    - A test suite that checks "does this spec produce the right error?"

    ``ValidationResult`` captures the outcome as data, not control flow.
    The caller can inspect ``valid`` and ``errors`` without a try/except.

    === Usage ===

    ::

        from cli_builder import validate_spec

        result = validate_spec("myapp.json")
        if result.valid:
            print("Spec is valid!")
        else:
            for error in result.errors:
                print(f"  - {error}")

    Attributes:
        valid: ``True`` if the spec passed all validation checks.
        errors: A list of human-readable error strings. Empty when ``valid``
            is ``True``. Contains one or more messages when ``valid`` is
            ``False``.
    """

    valid: bool
    errors: list[str] = field(default_factory=list)
