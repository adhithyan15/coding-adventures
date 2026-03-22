"""CLI Builder — declarative CLI argument parsing via directed graphs and state machines.

=== What is CLI Builder? ===

CLI Builder separates *what a tool accepts* (the spec) from *what a tool does*
(the business logic). A developer writes a JSON specification file describing
the CLI's structure, and CLI Builder handles all parsing, validation, help
generation, and error reporting.

=== Layer 8 of the computing stack ===

This package sits at Layer 8, built directly on:
- ``coding-adventures-directed-graph`` (Layer 3) — command routing graph,
  flag dependency graph, cycle detection, transitive closure
- ``coding-adventures-state-machine`` (Layer 4) — modal parse mode tracking

=== Quick start ===

    from cli_builder import Parser, ParseResult, HelpResult, VersionResult, ParseErrors

    try:
        result = Parser("myapp.json", ["myapp", "--verbose", "file.txt"]).parse()
    except ParseErrors as e:
        print(e)
        raise SystemExit(1)

    if isinstance(result, HelpResult):
        print(result.text)
        raise SystemExit(0)
    elif isinstance(result, VersionResult):
        print(result.version)
        raise SystemExit(0)
    else:
        # isinstance(result, ParseResult) guaranteed
        verbose = result.flags["verbose"]   # True
        path = result.arguments["file"]     # "file.txt"
"""

from cli_builder.errors import CliBuilderError, ParseError, ParseErrors, SpecError
from cli_builder.flag_validator import FlagValidator
from cli_builder.help_generator import HelpGenerator
from cli_builder.parser import Parser
from cli_builder.positional_resolver import PositionalResolver
from cli_builder.spec_loader import SpecLoader
from cli_builder.token_classifier import TokenClassifier
from cli_builder.types import HelpResult, ParseResult, ValidationResult, VersionResult
from cli_builder.validate import validate_spec, validate_spec_string

__all__ = [
    # Main entry point
    "Parser",
    # Result types
    "ParseResult",
    "HelpResult",
    "VersionResult",
    "ValidationResult",
    # Standalone validation
    "validate_spec",
    "validate_spec_string",
    # Error types
    "CliBuilderError",
    "SpecError",
    "ParseError",
    "ParseErrors",
    # Sub-components (for advanced use)
    "SpecLoader",
    "TokenClassifier",
    "PositionalResolver",
    "FlagValidator",
    "HelpGenerator",
]
