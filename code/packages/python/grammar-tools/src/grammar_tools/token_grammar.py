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

5. **group NAME:** section — defines a named set of token patterns
   (a "pattern group"). Groups enable context-sensitive lexing: the
   lexer maintains a stack of active groups and only tries patterns from
   the group on top of the stack. Language-specific callback code
   pushes/pops groups in response to matched tokens. For example, an
   XML lexer pushes a "tag" group when it sees ``<`` and pops it on
   ``>``, so attribute-related patterns are only active inside tags.
   Patterns outside any group section belong to the implicit "default"
   group. The grammar file contains no transition logic — just pattern
   definitions labeled by group.
"""

from __future__ import annotations

import re
from dataclasses import dataclass, field

# ---------------------------------------------------------------------------
# Magic comment regex
# ---------------------------------------------------------------------------
# Magic comments are special directives embedded in comment lines, using the
# form:  # @key value
#
# They must appear as complete comment lines (the # is the first non-space
# character on the line). The pattern captures:
#   group 1 — the key (word characters only, e.g. "version")
#   group 2 — the rest of the line after the key (the raw value string)
#
# Examples:
#   # @version 1          → key="version", value="1"
#   # @case_insensitive true  → key="case_insensitive", value="true"
#
# Unknown keys are silently ignored for forward compatibility: a newer
# grammar file can contain directives that an older grammar-tools version
# does not understand, and parsing will still succeed.
_MAGIC_COMMENT_RE = re.compile(r'^#\s*@(\w+)\s*(.*)$')


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


@dataclass(frozen=True)
class PatternGroup:
    """A named set of token definitions that are active together.

    When this group is at the top of the lexer's group stack, only these
    patterns are tried during token matching. Skip patterns are global
    and always tried regardless of the active group.

    Pattern groups enable context-sensitive lexing. For example, an XML
    lexer defines a "tag" group with patterns for attribute names, equals
    signs, and attribute values. These patterns are only active inside
    tags — the callback pushes the "tag" group when ``<`` is matched and
    pops it when ``>`` is matched.

    Attributes:
        name: The group name, e.g. "tag" or "cdata". Must be a lowercase
            identifier matching ``[a-z_][a-z0-9_]*``.
        definitions: Ordered list of token definitions in this group.
            Order matters (first-match-wins), just like the top-level
            definitions list.
    """

    name: str
    definitions: list[TokenDefinition]


@dataclass
class TokenGrammar:
    """The complete contents of a parsed .tokens file.

    Attributes:
        version: Schema version declared with ``# @version N``. Defaults
            to 0 (unversioned). Tools can use this to detect whether a
            file uses features that require a minimum grammar-tools
            release. Forward-compatible: older tools that do not
            understand a version simply ignore the field.
        case_insensitive: When ``True`` (set via ``# @case_insensitive true``),
            the lexer should treat all patterns as case-insensitive. This
            is a global flag: it applies to every token definition in the
            file. Useful for languages like SQL or HTML where keywords are
            conventionally case-insensitive. Defaults to ``False``.
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
        escape_mode: Controls how the lexer processes STRING token values.
            None (default) uses standard escape processing (JSON-style
            escapes like backslash-n, backslash-t, backslash-uXXXX).
            The string "none" disables escape processing — quotes are
            stripped but escape sequences are left as-is. This is useful
            for languages like CSS where escape semantics differ from the
            standard set and should be handled post-parse.
        error_definitions: Token definitions for error recovery patterns.
            When the lexer fails to match any normal token or skip pattern,
            it tries these patterns before raising ``LexerError``. This
            allows graceful degradation for malformed inputs — for example,
            CSS emits ``BAD_STRING`` for unclosed strings instead of
            crashing. Error tokens carry an ``is_error`` marker.
        case_sensitive: Whether the lexer should match patterns
            case-sensitively. Defaults to True. When False, the lexer
            lowercases the source text before matching and performs
            keyword promotion on the lowercased values. This is used
            by case-insensitive languages like VHDL.
    """

    version: int = 0
    case_insensitive: bool = False
    definitions: list[TokenDefinition] = field(default_factory=list)
    keywords: list[str] = field(default_factory=list)
    mode: str | None = None
    skip_definitions: list[TokenDefinition] = field(default_factory=list)
    reserved_keywords: list[str] = field(default_factory=list)
    escape_mode: str | None = None
    error_definitions: list[TokenDefinition] = field(default_factory=list)
    groups: dict[str, PatternGroup] = field(default_factory=dict)
    case_sensitive: bool = True
    layout_keywords: list[str] = field(default_factory=list)
    """Keywords that introduce a Haskell-style layout context when
    ``mode == "layout"``."""
    context_keywords: list[str] = field(default_factory=list)
    """Context-sensitive keywords — words that are keywords in some
    syntactic positions but identifiers in others.

    These are emitted as NAME tokens with the ``TOKEN_CONTEXT_KEYWORD``
    flag set, leaving the final keyword-vs-identifier decision to
    the language-specific parser or callback.

    Examples: JavaScript's ``async``, ``await``, ``yield``, ``get``, ``set``.
    """

    soft_keywords: list[str] = field(default_factory=list)
    """Soft keywords — words that act as keywords only in specific syntactic
    contexts, remaining ordinary identifiers everywhere else.

    Unlike context_keywords (which set a flag on the token), soft keywords
    produce plain NAME tokens with NO special flag. The lexer is completely
    unaware of their keyword status — the parser handles disambiguation
    entirely based on syntactic position.

    This distinction matters because:
      - context_keywords: lexer hints to parser ("this NAME might be special")
      - soft_keywords: lexer ignores them completely, parser owns the decision

    Examples:
      Python 3.10+: ``match``, ``case``, ``_`` (only keywords inside match statements)
      Python 3.12+: ``type`` (only a keyword in ``type X = ...`` statements)

    A ``soft_keywords:`` section in a .tokens file populates this field.
    """

    def token_names(self) -> set[str]:
        """Return the set of all defined token names.

        When a definition has an alias, the alias is included in the set
        (since that is the name the parser grammar references). The original
        definition name is also included for completeness.

        Includes names from all pattern groups, since group tokens can
        also appear in parser grammars.

        This is useful for cross-validation: the parser grammar references
        tokens by name, and we need to check that every referenced token
        actually exists.
        """
        names = set()
        all_defs = list(self.definitions)
        for group in self.groups.values():
            all_defs.extend(group.definitions)
        for d in all_defs:
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

        Includes names from all pattern groups.
        """
        all_defs = list(self.definitions)
        for group in self.groups.values():
            all_defs.extend(group.definitions)
        return {d.alias if d.alias else d.name for d in all_defs}


# ---------------------------------------------------------------------------
# Parser
# ---------------------------------------------------------------------------


def _find_closing_slash(pattern_part: str) -> int:
    """Find the index of the closing ``/`` in a regex pattern string.

    The string is expected to start with ``/``. We scan from index 1
    looking for the first unescaped ``/`` that is NOT inside a ``[...]``
    character class.

    If the bracket-aware scan fails (e.g. an unclosed ``[``), we fall
    back to finding the last ``/`` so the pattern can still be parsed
    and validated downstream.

    Returns the index of the closing ``/``, or ``-1`` if not found.
    """
    i = 1
    in_bracket = False
    n = len(pattern_part)
    while i < n:
        ch = pattern_part[i]
        if ch == "\\":
            # Escaped character — skip next
            i += 2
            continue
        if ch == "[" and not in_bracket:
            in_bracket = True
        elif ch == "]" and in_bracket:
            in_bracket = False
        elif ch == "/" and not in_bracket:
            return i
        i += 1

    # Fallback: if bracket-aware scan found nothing (e.g. unclosed [),
    # try the last / as a best-effort parse.
    last = pattern_part.rfind("/")
    return last if last > 0 else -1


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
        # Regex pattern — find the closing / by scanning character-by-character.
        # We track bracket depth so that / inside [...] character classes is
        # not mistaken for the closing delimiter. We also skip escaped chars.
        last_slash = _find_closing_slash(pattern_part)
        if last_slash == -1:
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
    #
    # For pattern groups, current_section is "group:NAME" where NAME is
    # the group name. This distinguishes groups from other sections.
    current_section: str | None = None  # "keywords", "reserved", "skip", "group:NAME"

    for line_number, raw_line in enumerate(lines, start=1):
        line = raw_line.rstrip()
        stripped = line.strip()

        # --- Blank lines are always skipped ---
        if stripped == "":
            continue

        # --- Comment lines: check for magic comments, then skip ---
        # Regular comments (``# some text``) are silently ignored.
        # Magic comments (``# @key value``) carry structured directives
        # that configure grammar-level metadata. We scan every comment
        # line with _MAGIC_COMMENT_RE before discarding it.
        #
        # Currently recognised keys:
        #   @version N            — set grammar.version to integer N
        #   @case_insensitive B   — set grammar.case_insensitive to bool
        #
        # Unknown keys are silently ignored so that files written for a
        # newer grammar-tools version still parse correctly on older ones.
        if stripped.startswith("#"):
            magic = _MAGIC_COMMENT_RE.match(stripped)
            if magic:
                key = magic.group(1)
                value = magic.group(2).strip()
                if key == "version":
                    try:
                        grammar.version = int(value)
                    except ValueError:
                        pass  # Non-integer version — ignore silently
                elif key == "case_insensitive":
                    grammar.case_insensitive = (value == "true")
                # All other keys are intentionally ignored (forward compat)
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

        # --- escapes: directive ---
        # Controls how STRING tokens are processed. "none" disables
        # escape processing (quotes are stripped but escapes are left
        # as-is). Useful for CSS where escape semantics differ from JSON.
        if stripped.startswith("escapes:"):
            escape_value = stripped[8:].strip()
            if not escape_value:
                raise TokenGrammarError(
                    "Missing value after 'escapes:'",
                    line_number,
                )
            grammar.escape_mode = escape_value
            current_section = None
            continue

        # --- case_sensitive: directive ---
        # Controls whether the lexer should match case-sensitively.
        # ``case_sensitive: false`` makes the lexer lowercase input before
        # matching and perform keyword promotion on lowercased values.
        if stripped.startswith("case_sensitive:"):
            cs_value = stripped[15:].strip().lower()
            if cs_value not in ("true", "false"):
                raise TokenGrammarError(
                    f"Invalid value for 'case_sensitive:': {cs_value!r} "
                    "(expected 'true' or 'false')",
                    line_number,
                )
            grammar.case_sensitive = cs_value == "true"
            current_section = None
            continue

        # --- Group headers ---
        # Pattern groups are declared with ``group NAME:`` where NAME is
        # a lowercase identifier. All subsequent indented lines belong to
        # that group, just like skip: or errors: sections.
        if stripped.startswith("group ") and stripped.endswith(":"):
            group_name = stripped[6:-1].strip()
            if not group_name:
                raise TokenGrammarError(
                    "Missing group name after 'group'",
                    line_number,
                )
            if not re.match(r"^[a-z_][a-z0-9_]*$", group_name):
                raise TokenGrammarError(
                    f"Invalid group name: {group_name!r} "
                    "(must be a lowercase identifier like 'tag' or 'cdata')",
                    line_number,
                )
            reserved_names = {
                "default",
                "skip",
                "keywords",
                "reserved",
                "errors",
                "layout_keywords",
                "context_keywords",
                "soft_keywords",
            }
            if group_name in reserved_names:
                raise TokenGrammarError(
                    f"Reserved group name: {group_name!r} "
                    f"(cannot use {', '.join(sorted(reserved_names))})",
                    line_number,
                )
            if group_name in grammar.groups:
                raise TokenGrammarError(
                    f"Duplicate group name: {group_name!r}",
                    line_number,
                )
            grammar.groups[group_name] = PatternGroup(
                name=group_name, definitions=[]
            )
            current_section = f"group:{group_name}"
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

        if stripped in ("errors:", "errors :"):
            current_section = "errors"
            continue

        if stripped in ("context_keywords:", "context_keywords :"):
            current_section = "context_keywords"
            continue

        if stripped in ("layout_keywords:", "layout_keywords :"):
            current_section = "layout_keywords"
            continue

        if stripped in ("soft_keywords:", "soft_keywords :"):
            current_section = "soft_keywords"
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
                elif current_section == "context_keywords":
                    if stripped:
                        grammar.context_keywords.append(stripped)
                elif current_section == "layout_keywords":
                    if stripped:
                        grammar.layout_keywords.append(stripped)
                elif current_section == "soft_keywords":
                    if stripped:
                        grammar.soft_keywords.append(stripped)
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
                elif current_section == "errors":
                    # Errors section contains token definitions for
                    # error recovery — patterns tried as a fallback when
                    # no normal token matches (e.g., BAD_STRING for
                    # unclosed strings in CSS).
                    if "=" not in stripped:
                        raise TokenGrammarError(
                            f"Expected error pattern definition "
                            f"(NAME = pattern), got: {stripped!r}",
                            line_number,
                        )
                    eq_index = stripped.index("=")
                    err_name = stripped[:eq_index].strip()
                    err_pattern = stripped[eq_index + 1 :].strip()
                    if not err_name or not err_pattern:
                        raise TokenGrammarError(
                            f"Incomplete error pattern definition: "
                            f"{stripped!r}",
                            line_number,
                        )
                    defn = _parse_definition(
                        err_pattern, err_name, line_number
                    )
                    grammar.error_definitions.append(defn)
                elif current_section is not None and current_section.startswith("group:"):
                    # Group section contains token definitions,
                    # same format as skip: and errors: sections.
                    group_name = current_section[6:]
                    if "=" not in stripped:
                        raise TokenGrammarError(
                            f"Expected token definition in group "
                            f"'{group_name}' (NAME = pattern), "
                            f"got: {stripped!r}",
                            line_number,
                        )
                    eq_index = stripped.index("=")
                    g_name = stripped[:eq_index].strip()
                    g_pattern = stripped[eq_index + 1 :].strip()
                    if not g_name or not g_pattern:
                        raise TokenGrammarError(
                            f"Incomplete definition in group "
                            f"'{group_name}': {stripped!r}",
                            line_number,
                        )
                    defn = _parse_definition(
                        g_pattern, g_name, line_number
                    )
                    # PatternGroup is frozen, so we need to create a new
                    # one with the updated definitions list.
                    old_group = grammar.groups[group_name]
                    grammar.groups[group_name] = PatternGroup(
                        name=group_name,
                        definitions=[*old_group.definitions, defn],
                    )
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

    # Validate error definitions
    issues.extend(_validate_definitions(grammar.error_definitions, "error pattern"))

    # Validate mode
    if grammar.mode is not None and grammar.mode not in ("indentation", "layout"):
        issues.append(
            f"Unknown lexer mode '{grammar.mode}' "
            f"(only 'indentation' and 'layout' are supported)"
        )
    if grammar.mode == "layout" and not grammar.layout_keywords:
        issues.append("Layout mode requires a non-empty layout_keywords section")

    # Validate escape mode
    if grammar.escape_mode is not None and grammar.escape_mode != "none":
        issues.append(
            f"Unknown escape mode '{grammar.escape_mode}' "
            f"(only 'none' is supported)"
        )

    # Validate pattern groups
    for group_name, group in grammar.groups.items():
        # Group name format
        if not re.match(r"^[a-z_][a-z0-9_]*$", group_name):
            issues.append(
                f"Invalid group name '{group_name}' "
                f"(must be a lowercase identifier)"
            )

        # Empty group warning
        if not group.definitions:
            issues.append(
                f"Empty pattern group '{group_name}' "
                f"(has no token definitions)"
            )

        # Validate definitions within the group
        issues.extend(
            _validate_definitions(group.definitions, f"group '{group_name}' token")
        )

    return issues
