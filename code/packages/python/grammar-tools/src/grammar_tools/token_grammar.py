"""
token_grammar.py — Parser and validator for .tokens files.

A .tokens file is a declarative description of the lexical grammar of a
programming language. It lists every token the lexer should recognize, in
priority order (first match wins), along with an optional keywords section
for reserved words.

This module solves the "front half" of the grammar-tools pipeline: it reads
a plain-text token specification and produces a structured TokenGrammar
object that downstream tools (lexer generators, cross-validators) can
consume.

File format overview
--------------------

Each non-blank, non-comment line in a .tokens file has one of three forms:

  TOKEN_NAME = /regex_pattern/      — a regex-based token
  TOKEN_NAME = "literal_string"     — a literal-string token
  keywords:                         — begins the keywords section

Lines starting with # are comments. Blank lines are ignored.

The keywords section lists one reserved word per line (indented). Keywords
are identifiers that the lexer recognizes as NAME tokens but then
reclassifies. For instance, `if` matches the NAME pattern but is promoted
to an IF keyword.

Design decisions
----------------

Why hand-parse instead of using regex or a parser library? Because the
format is simple enough that a line-by-line parser is clearer, faster, and
produces better error messages than any generic tool would. Every error
includes the line number where the problem occurred, which matters a lot
when users are writing grammars by hand.

Why dataclasses instead of dicts? Because we want type safety. A
TokenDefinition with `.name`, `.pattern`, `.is_regex`, and `.line_number`
fields is self-documenting and mypy-checkable. A dict with string keys
would silently accept typos.
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
    """

    name: str
    pattern: str
    is_regex: bool
    line_number: int


@dataclass
class TokenGrammar:
    """The complete contents of a parsed .tokens file.

    Attributes:
        definitions: Ordered list of token definitions. Order matters
            because the lexer uses first-match-wins semantics.
        keywords: List of reserved words from the keywords: section.
    """

    definitions: list[TokenDefinition] = field(default_factory=list)
    keywords: list[str] = field(default_factory=list)

    def token_names(self) -> set[str]:
        """Return the set of all defined token names.

        This is useful for cross-validation: the parser grammar references
        tokens by name, and we need to check that every referenced token
        actually exists.
        """
        return {d.name for d in self.definitions}


# ---------------------------------------------------------------------------
# Parser
# ---------------------------------------------------------------------------


def parse_token_grammar(source: str) -> TokenGrammar:
    """Parse the text of a .tokens file into a TokenGrammar.

    The parser operates line-by-line. It has two modes:

    1. **Definition mode** (default) — each line is either a comment, a
       blank, or a token definition of the form ``NAME = /pattern/`` or
       ``NAME = "literal"``.

    2. **Keywords mode** — entered when the parser encounters a line
       matching ``keywords:``. Each subsequent indented line is treated as
       a keyword until a non-indented, non-blank, non-comment line is found
       (or EOF).

    Args:
        source: The full text content of a .tokens file.

    Returns:
        A TokenGrammar containing all parsed definitions and keywords.

    Raises:
        TokenGrammarError: If any line cannot be parsed.
    """
    lines = source.split("\n")
    grammar = TokenGrammar()
    in_keywords = False

    for line_number, raw_line in enumerate(lines, start=1):
        # Strip trailing whitespace but preserve leading whitespace
        # (we need it to detect keyword entries).
        line = raw_line.rstrip()

        # --- Blank lines and comments are always skipped ---
        stripped = line.strip()
        if stripped == "" or stripped.startswith("#"):
            continue

        # --- Keywords section header ---
        if stripped == "keywords:" or stripped == "keywords :":
            in_keywords = True
            continue

        # --- Inside keywords section ---
        if in_keywords:
            # Keywords are indented lines. A non-indented line that isn't
            # blank or a comment means we've left the keywords section.
            if line[0] in (" ", "\t"):
                keyword = stripped
                if keyword:
                    grammar.keywords.append(keyword)
                continue
            else:
                # We've exited the keywords section. Fall through to
                # parse this line as a normal definition.
                in_keywords = False

        # --- Token definition ---
        # Expected format: NAME = /pattern/  or  NAME = "literal"
        # We split on the first '=' to separate name from pattern.
        if "=" not in line:
            raise TokenGrammarError(
                f"Expected token definition (NAME = pattern), got: {stripped!r}",
                line_number,
            )

        eq_index = line.index("=")
        name_part = line[:eq_index].strip()
        pattern_part = line[eq_index + 1 :].strip()

        # Validate that we got a name.
        if not name_part:
            raise TokenGrammarError(
                "Missing token name before '='",
                line_number,
            )

        # Validate the name looks like an identifier.
        if not re.match(r"^[a-zA-Z_][a-zA-Z0-9_]*$", name_part):
            raise TokenGrammarError(
                f"Invalid token name: {name_part!r} "
                "(must be an identifier like NAME or PLUS_EQUALS)",
                line_number,
            )

        # Parse the pattern: either /regex/ or "literal".
        if not pattern_part:
            raise TokenGrammarError(
                f"Missing pattern after '=' for token {name_part!r}",
                line_number,
            )

        if pattern_part.startswith("/") and pattern_part.endswith("/"):
            # Regex pattern — strip the delimiters.
            regex_body = pattern_part[1:-1]
            if not regex_body:
                raise TokenGrammarError(
                    f"Empty regex pattern for token {name_part!r}",
                    line_number,
                )
            grammar.definitions.append(
                TokenDefinition(
                    name=name_part,
                    pattern=regex_body,
                    is_regex=True,
                    line_number=line_number,
                )
            )

        elif pattern_part.startswith('"') and pattern_part.endswith('"'):
            # Literal pattern — strip the quotes.
            literal_body = pattern_part[1:-1]
            if not literal_body:
                raise TokenGrammarError(
                    f"Empty literal pattern for token {name_part!r}",
                    line_number,
                )
            grammar.definitions.append(
                TokenDefinition(
                    name=name_part,
                    pattern=literal_body,
                    is_regex=False,
                    line_number=line_number,
                )
            )

        else:
            raise TokenGrammarError(
                f"Pattern for token {name_part!r} must be /regex/ or "
                f'"literal", got: {pattern_part!r}',
                line_number,
            )

    return grammar


# ---------------------------------------------------------------------------
# Validator
# ---------------------------------------------------------------------------


def validate_token_grammar(grammar: TokenGrammar) -> list[str]:
    """Check a parsed TokenGrammar for common problems.

    This is a *lint* pass, not a parse pass — the grammar has already been
    parsed successfully. We are looking for semantic issues that would cause
    problems downstream:

    - **Duplicate token names**: Two definitions with the same name. The
      second would shadow the first, which is almost certainly a mistake.
    - **Invalid regex patterns**: A pattern written as /regex/ that Python's
      ``re`` module cannot compile. Caught here rather than at lexer-
      generation time so the user gets an early, clear error.
    - **Empty patterns**: Should have been caught during parsing, but we
      double-check here for safety.
    - **Non-UPPER_CASE names**: By convention, token names are UPPER_CASE.
      This helps distinguish them from parser rule names (lowercase) in
      .grammar files.

    Args:
        grammar: A parsed TokenGrammar to validate.

    Returns:
        A list of warning/error strings. An empty list means no issues.
    """
    issues: list[str] = []
    seen_names: dict[str, int] = {}

    for defn in grammar.definitions:
        # --- Duplicate check ---
        if defn.name in seen_names:
            issues.append(
                f"Line {defn.line_number}: Duplicate token name "
                f"'{defn.name}' (first defined on line {seen_names[defn.name]})"
            )
        else:
            seen_names[defn.name] = defn.line_number

        # --- Empty pattern check ---
        if not defn.pattern:
            issues.append(
                f"Line {defn.line_number}: Empty pattern for token "
                f"'{defn.name}'"
            )

        # --- Invalid regex check ---
        if defn.is_regex:
            try:
                re.compile(defn.pattern)
            except re.error as e:
                issues.append(
                    f"Line {defn.line_number}: Invalid regex for token "
                    f"'{defn.name}': {e}"
                )

        # --- Naming convention check ---
        if defn.name != defn.name.upper():
            issues.append(
                f"Line {defn.line_number}: Token name '{defn.name}' "
                f"should be UPPER_CASE"
            )

    return issues
