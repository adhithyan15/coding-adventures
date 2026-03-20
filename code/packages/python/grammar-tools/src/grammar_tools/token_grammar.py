"""
token_grammar.py — Parser and validator for .tokens files.

A .tokens file is a declarative description of the lexical grammar of a
programming language. It lists every token the lexer should recognize, in
priority order (first match wins), along with optional sections for
keywords, reserved words, skip patterns, and lexer mode configuration.

This module solves the "front half" of the grammar-tools pipeline: it reads
a plain-text token specification and produces a structured TokenGrammar
object that downstream tools (lexer generators, cross-validators) can
consume.

File format overview
--------------------

Each non-blank, non-comment line in a .tokens file has one of these forms:

  TOKEN_NAME = /regex_pattern/           — a regex-based token
  TOKEN_NAME = "literal_string"          — a literal-string token
  TOKEN_NAME = /regex/ -> ALIAS          — emits token type ALIAS instead
  TOKEN_NAME = "literal" -> ALIAS        — same for literals
  mode: indentation                      — sets the lexer mode
  keywords:                              — begins the keywords section
  reserved:                              — begins the reserved keywords section
  skip:                                  — begins the skip patterns section

Lines starting with # are comments. Blank lines are ignored.

Extended format (Starlark support)
----------------------------------

The original format supported only token definitions and keywords. To handle
languages like Starlark (a Python subset), we added four extensions:

1. **mode:** directive — tells the lexer which special mode to activate.
   Currently the only supported mode is ``indentation``, which enables
   Python-style INDENT/DEDENT/NEWLINE tracking.

2. **skip:** section — defines patterns that are matched and consumed but
   do NOT produce tokens. Used for comments and inline whitespace.

3. **-> ALIAS** suffix — multiple token patterns can emit the same token
   type. For example, ``STRING_DQ`` and ``STRING_SQ`` can both emit
   ``STRING``. This keeps the grammar simple while the tokens file handles
   the lexical complexity.

4. **reserved:** section — keywords that are syntax errors if used as
   identifiers. Unlike regular keywords (which produce KEYWORD tokens),
   reserved words cause immediate lex errors.
"""

from __future__ import annotations

import re
from dataclasses import dataclass, field


# ---------------------------------------------------------------------------
# Exceptions
# ---------------------------------------------------------------------------


class TokenGrammarError(Exception):
    """Raised when a .tokens file cannot be parsed.

    Attributes:
        message: Human-readable description of the problem.
        line_number: 1-based line number where the error occurred.
    """

    def __init__(self, message: str, line_number: int) -> None:
        self.message = message
        self.line_number = line_number
        super().__init__(f"Line {line_number}: {message}")


# ---------------------------------------------------------------------------
# Data model
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class TokenDefinition:
    """A single token rule from a .tokens file.

    Attributes:
        name: The token name, e.g. "NUMBER" or "PLUS".
        pattern: The pattern string — either a regex (without delimiters)
            or a literal string (without quotes).
        is_regex: True if the pattern was written as /regex/, False if it
            was written as "literal".
        line_number: The 1-based line number where this definition appeared.
        alias: Optional type alias. When set, the lexer emits this as the
            token type instead of ``name``. For example, STRING_DQ with
            alias="STRING" means the lexer produces a STRING token. This
            keeps the grammar simple (it references STRING) while the tokens
            file handles the lexical detail (matching double-quoted strings
            specifically).
    """

    name: str
    pattern: str
    is_regex: bool
    line_number: int
    alias: str | None = None


@dataclass
class TokenGrammar:
    """The complete contents of a parsed .tokens file.

    Attributes:
        definitions: Ordered list of token definitions. Order matters
            because the lexer uses first-match-wins semantics.
        keywords: List of reserved words from the keywords: section.
        mode: Optional lexer mode. Currently the only supported value is
            ``"indentation"``, which activates Python-style INDENT/DEDENT
            tracking. When active, the lexer maintains an indentation stack
            and emits synthetic INDENT, DEDENT, and NEWLINE tokens.
        skip_definitions: Token definitions for patterns that should be
            matched and consumed without producing tokens. Typically used
            for comments and inline whitespace.
        reserved_keywords: Keywords that are syntax errors if used as
            identifiers. Unlike regular keywords (which produce KEYWORD
            tokens), reserved words cause an immediate lex error. This
            catches mistakes like ``class Foo`` in Starlark at lex time
            instead of producing a confusing parse error.
    """

    definitions: list[TokenDefinition] = field(default_factory=list)
    keywords: list[str] = field(default_factory=list)
    mode: str | None = None
    skip_definitions: list[TokenDefinition] = field(default_factory=list)
    reserved_keywords: list[str] = field(default_factory=list)

    def token_names(self) -> set[str]:
        """Return the set of all defined token names.

        When a definition has an alias, the alias is included in the set
        (since that is the name the parser grammar references). The original
        definition name is also included for completeness.

        This is useful for cross-validation: the parser grammar references
        tokens by name, and we need to check that every referenced token
        actually exists.
        """
        names = set()
        for d in self.definitions:
            names.add(d.name)
            if d.alias:
                names.add(d.alias)
        return names

    def effective_token_names(self) -> set[str]:
        """Return the set of token names as the parser will see them.

        For definitions with aliases, this returns the alias (not the
        definition name), because that is what the lexer will emit and
        what the parser grammar references.

        For definitions without aliases, this returns the definition name.
        """
        return {d.alias if d.alias else d.name for d in self.definitions}


# ---------------------------------------------------------------------------
# Parser
# ---------------------------------------------------------------------------


def _parse_definition(
    pattern_part: str,
    name_part: str,
    line_number: int,
) -> TokenDefinition:
    """Parse a single token definition's pattern and optional alias.

    The pattern_part may have a ``-> ALIAS`` suffix after the closing
    delimiter. For example::

        /regex/           → no alias
        /regex/ -> STRING → alias is "STRING"
        "literal"         → no alias
        "lit" -> PLUS     → alias is "PLUS"

    Args:
        pattern_part: Everything after the ``=`` sign, stripped.
        name_part: The token name (for error messages).
        line_number: 1-based line number (for error messages).

    Returns:
        A ``TokenDefinition`` with the parsed pattern and optional alias.

    Raises:
        TokenGrammarError: If the pattern cannot be parsed.
    """
    alias = None

    # Check for -> ALIAS suffix. We need to be careful not to confuse
    # the -> in the alias with characters inside a regex pattern.
    # Strategy: find the closing delimiter first, then check for ->.
    if pattern_part.startswith("/"):
        # Regex pattern — find the closing /
        # The pattern could contain escaped slashes, but our format
        # doesn't support that (slashes inside regex use [/] or other
        # workarounds). So we find the LAST / as the closing delimiter.
        last_slash = pattern_part.rfind("/")
        if last_slash == 0:
            raise TokenGrammarError(
                f"Unclosed regex pattern for token {name_part!r}",
                line_number,
            )
        regex_body = pattern_part[1:last_slash]
        remainder = pattern_part[last_slash + 1 :].strip()

        if not regex_body:
            raise TokenGrammarError(
                f"Empty regex pattern for token {name_part!r}",
                line_number,
            )

        # Check for -> ALIAS in remainder
        if remainder.startswith("->"):
            alias = remainder[2:].strip()
            if not alias:
                raise TokenGrammarError(
                    f"Missing alias after '->' for token {name_part!r}",
                    line_number,
                )
        elif remainder:
            raise TokenGrammarError(
                f"Unexpected text after pattern for token {name_part!r}: "
                f"{remainder!r}",
                line_number,
            )

        return TokenDefinition(
            name=name_part,
            pattern=regex_body,
            is_regex=True,
            line_number=line_number,
            alias=alias,
        )

    elif pattern_part.startswith('"'):
        # Literal pattern — find the closing "
        close_quote = pattern_part.find('"', 1)
        if close_quote == -1:
            raise TokenGrammarError(
                f"Unclosed literal pattern for token {name_part!r}",
                line_number,
            )
        literal_body = pattern_part[1:close_quote]
        remainder = pattern_part[close_quote + 1 :].strip()

        if not literal_body:
            raise TokenGrammarError(
                f"Empty literal pattern for token {name_part!r}",
                line_number,
            )

        # Check for -> ALIAS in remainder
        if remainder.startswith("->"):
            alias = remainder[2:].strip()
            if not alias:
                raise TokenGrammarError(
                    f"Missing alias after '->' for token {name_part!r}",
                    line_number,
                )
        elif remainder:
            raise TokenGrammarError(
                f"Unexpected text after pattern for token {name_part!r}: "
                f"{remainder!r}",
                line_number,
            )

        return TokenDefinition(
            name=name_part,
            pattern=literal_body,
            is_regex=False,
            line_number=line_number,
            alias=alias,
        )

    else:
        raise TokenGrammarError(
            f"Pattern for token {name_part!r} must be /regex/ or "
            f'"literal", got: {pattern_part!r}',
            line_number,
        )


def parse_token_grammar(source: str) -> TokenGrammar:
    """Parse the text of a .tokens file into a TokenGrammar.

    The parser operates line-by-line with several modes:

    1. **Definition mode** (default) — each line is either a comment, a
       blank, a section header, or a token definition.

    2. **Keywords mode** — entered on ``keywords:``. Indented lines are
       keywords. Exits on non-indented content.

    3. **Reserved mode** — entered on ``reserved:``. Same format as keywords
       but populates ``reserved_keywords``.

    4. **Skip mode** — entered on ``skip:``. Indented lines are token
       definitions (same ``NAME = /pattern/`` format) that produce skip
       patterns instead of regular tokens.

    The ``mode:`` directive is a standalone line that sets the lexer mode
    (e.g., ``mode: indentation``). It can appear anywhere outside a section.

    Args:
        source: The full text content of a .tokens file.

    Returns:
        A TokenGrammar containing all parsed definitions, keywords,
        reserved keywords, skip patterns, and mode.

    Raises:
        TokenGrammarError: If any line cannot be parsed.
    """
    lines = source.split("\n")
    grammar = TokenGrammar()

    # Section tracking. We use a string to track which section we're in,
    # since sections are mutually exclusive and we can only be in one at
    # a time (or in no section = definition mode).
    current_section: str | None = None  # "keywords", "reserved", "skip"

    for line_number, raw_line in enumerate(lines, start=1):
        line = raw_line.rstrip()
        stripped = line.strip()

        # --- Blank lines and comments are always skipped ---
        if stripped == "" or stripped.startswith("#"):
            continue

        # --- mode: directive ---
        # Can appear anywhere outside a section. Sets the lexer mode.
        if stripped.startswith("mode:"):
            mode_value = stripped[5:].strip()
            if not mode_value:
                raise TokenGrammarError(
                    "Missing value after 'mode:'",
                    line_number,
                )
            grammar.mode = mode_value
            current_section = None
            continue

        # --- Section headers ---
        if stripped in ("keywords:", "keywords :"):
            current_section = "keywords"
            continue

        if stripped in ("reserved:", "reserved :"):
            current_section = "reserved"
            continue

        if stripped in ("skip:", "skip :"):
            current_section = "skip"
            continue

        # --- Inside a section ---
        if current_section is not None:
            # Sections contain indented lines. A non-indented line exits
            # the section and is processed as a normal definition.
            if line[0] in (" ", "\t"):
                if current_section == "keywords":
                    if stripped:
                        grammar.keywords.append(stripped)
                elif current_section == "reserved":
                    if stripped:
                        grammar.reserved_keywords.append(stripped)
                elif current_section == "skip":
                    # Skip section contains token definitions
                    if "=" not in stripped:
                        raise TokenGrammarError(
                            f"Expected skip pattern definition "
                            f"(NAME = pattern), got: {stripped!r}",
                            line_number,
                        )
                    eq_index = stripped.index("=")
                    skip_name = stripped[:eq_index].strip()
                    skip_pattern = stripped[eq_index + 1 :].strip()
                    if not skip_name or not skip_pattern:
                        raise TokenGrammarError(
                            f"Incomplete skip pattern definition: "
                            f"{stripped!r}",
                            line_number,
                        )
                    defn = _parse_definition(
                        skip_pattern, skip_name, line_number
                    )
                    grammar.skip_definitions.append(defn)
                continue
            else:
                # Non-indented line — exit section, fall through
                current_section = None

        # --- Token definition ---
        if "=" not in line:
            raise TokenGrammarError(
                f"Expected token definition (NAME = pattern), got: "
                f"{stripped!r}",
                line_number,
            )

        eq_index = line.index("=")
        name_part = line[:eq_index].strip()
        pattern_part = line[eq_index + 1 :].strip()

        if not name_part:
            raise TokenGrammarError(
                "Missing token name before '='",
                line_number,
            )

        if not re.match(r"^[a-zA-Z_][a-zA-Z0-9_]*$", name_part):
            raise TokenGrammarError(
                f"Invalid token name: {name_part!r} "
                "(must be an identifier like NAME or PLUS_EQUALS)",
                line_number,
            )

        if not pattern_part:
            raise TokenGrammarError(
                f"Missing pattern after '=' for token {name_part!r}",
                line_number,
            )

        defn = _parse_definition(pattern_part, name_part, line_number)
        grammar.definitions.append(defn)

    return grammar


# ---------------------------------------------------------------------------
# Validator
# ---------------------------------------------------------------------------


def _validate_definitions(
    definitions: list[TokenDefinition],
    label: str,
) -> list[str]:
    """Validate a list of token definitions (shared logic for both
    regular definitions and skip definitions).

    Args:
        definitions: The list of TokenDefinition objects to validate.
        label: A label for error messages (e.g., "token" or "skip pattern").

    Returns:
        A list of issue strings.
    """
    issues: list[str] = []
    seen_names: dict[str, int] = {}

    for defn in definitions:
        # --- Duplicate check ---
        if defn.name in seen_names:
            issues.append(
                f"Line {defn.line_number}: Duplicate {label} name "
                f"'{defn.name}' (first defined on line "
                f"{seen_names[defn.name]})"
            )
        else:
            seen_names[defn.name] = defn.line_number

        # --- Empty pattern check ---
        if not defn.pattern:
            issues.append(
                f"Line {defn.line_number}: Empty pattern for {label} "
                f"'{defn.name}'"
            )

        # --- Invalid regex check ---
        if defn.is_regex:
            try:
                re.compile(defn.pattern)
            except re.error as e:
                issues.append(
                    f"Line {defn.line_number}: Invalid regex for {label} "
                    f"'{defn.name}': {e}"
                )

        # --- Naming convention check ---
        if defn.name != defn.name.upper():
            issues.append(
                f"Line {defn.line_number}: Token name '{defn.name}' "
                f"should be UPPER_CASE"
            )

        # --- Alias convention check ---
        if defn.alias and defn.alias != defn.alias.upper():
            issues.append(
                f"Line {defn.line_number}: Alias '{defn.alias}' for "
                f"token '{defn.name}' should be UPPER_CASE"
            )

    return issues


def validate_token_grammar(grammar: TokenGrammar) -> list[str]:
    """Check a parsed TokenGrammar for common problems.

    This is a *lint* pass, not a parse pass — the grammar has already been
    parsed successfully. We are looking for semantic issues that would cause
    problems downstream:

    - **Duplicate token names**: Two definitions with the same name.
    - **Invalid regex patterns**: A regex that Python's ``re`` module
      cannot compile.
    - **Empty patterns**: Should have been caught during parsing, but we
      double-check here for safety.
    - **Non-UPPER_CASE names**: By convention, token names are UPPER_CASE.
    - **Invalid aliases**: Alias names should also be UPPER_CASE.
    - **Invalid mode**: Only ``"indentation"`` is currently supported.
    - **Skip definition issues**: Same checks as regular definitions.

    Args:
        grammar: A parsed TokenGrammar to validate.

    Returns:
        A list of warning/error strings. An empty list means no issues.
    """
    issues: list[str] = []

    # Validate regular definitions
    issues.extend(_validate_definitions(grammar.definitions, "token"))

    # Validate skip definitions
    issues.extend(_validate_definitions(grammar.skip_definitions, "skip pattern"))

    # Validate mode
    if grammar.mode is not None and grammar.mode != "indentation":
        issues.append(
            f"Unknown lexer mode '{grammar.mode}' "
            f"(only 'indentation' is supported)"
        )

    return issues
