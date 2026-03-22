"""
parser_grammar.py — Parser and validator for .grammar files.

A .grammar file describes the syntactic structure of a programming language
using EBNF (Extended Backus-Naur Form). Where a .tokens file says "these
are the words," a .grammar file says "these are the sentences."

EBNF: a brief history
---------------------

BNF (Backus-Naur Form) was invented in the late 1950s by John Backus and
Peter Naur to describe the syntax of ALGOL 60. It was one of the first
formal notations for programming language grammars. EBNF extends BNF with
three conveniences:

    { x }   — zero or more repetitions of x (replaces recursive rules)
    [ x ]   — optional x (shorthand for x | epsilon)
    ( x )   — grouping (to clarify precedence in alternations)

These extensions don't add any theoretical power — anything expressible in
EBNF can be written in plain BNF — but they make grammars dramatically
more readable. Compare:

    BNF:   statements ::= <empty> | statement statements
    EBNF:  statements = { statement } ;

The recursive descent parser
----------------------------

This module contains a hand-written recursive descent parser for the EBNF
notation used in .grammar files. This is the "chicken-and-egg" solution
mentioned in the README: we need a parser to read grammar files, so we
write one by hand.

A recursive descent parser works by having one function per grammar rule.
Each function:
  1. Looks at the current token (character or word)
  2. Decides which alternative to take
  3. Calls other parsing functions as needed
  4. Returns an AST node

For our EBNF parser, the grammar of the grammar (the "meta-grammar") is:

    grammar_file  = { rule } ;
    rule          = rule_name "=" body ";" ;
    body          = sequence { "|" sequence } ;
    sequence      = { element } ;
    element       = rule_ref | token_ref | literal
                  | "{" body "}"
                  | "[" body "]"
                  | "(" body ")" ;
    rule_ref      = lowercase_identifier ;
    token_ref     = UPPERCASE_IDENTIFIER ;
    literal       = '"' characters '"' ;

Each level of this meta-grammar becomes a method in our parser class.

Why not use regex? Because EBNF has nested structure (parentheses inside
braces inside brackets), and regex cannot handle arbitrary nesting. This
is exactly the kind of problem that context-free grammars (and recursive
descent parsers) were invented to solve.
"""

from __future__ import annotations

from dataclasses import dataclass, field


# ---------------------------------------------------------------------------
# Exceptions
# ---------------------------------------------------------------------------


class ParserGrammarError(Exception):
    """Raised when a .grammar file cannot be parsed.

    Attributes:
        message: Human-readable description of the problem.
        line_number: 1-based line number where the error occurred.
    """

    def __init__(self, message: str, line_number: int) -> None:
        self.message = message
        self.line_number = line_number
        super().__init__(f"Line {line_number}: {message}")


# ---------------------------------------------------------------------------
# AST node types (the "grammar elements")
# ---------------------------------------------------------------------------
# These dataclasses form a tree that represents the parsed body of a grammar
# rule. Together they can express anything that EBNF can express.
#
# The type alias GrammarElement is a union of all node types, which lets us
# write functions that accept "any node" and switch on its type. This is
# Python's equivalent of a tagged union / sum type / algebraic data type.
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class RuleReference:
    """A reference to another grammar rule (lowercase name) or a token
    (UPPERCASE name).

    In EBNF, ``expression`` refers to the rule named "expression", while
    ``NUMBER`` refers to the token type NUMBER from the .tokens file. We
    distinguish these by checking whether the name is all-uppercase.

    Attributes:
        name: The identifier being referenced.
        is_token: True if the name is UPPERCASE (a token reference),
            False if lowercase (a rule reference).
    """

    name: str
    is_token: bool


@dataclass(frozen=True)
class Literal:
    """A literal string match in the grammar, written as "..." in EBNF.

    This is less common than token references — usually you define tokens
    in the .tokens file and reference them by name. But sometimes it is
    convenient to write a literal directly in the grammar.

    Attributes:
        value: The literal string to match (without quotes).
    """

    value: str


@dataclass(frozen=True)
class Sequence:
    """A sequence of elements that must appear in order.

    In EBNF, juxtaposition means sequencing: ``A B C`` means "A followed
    by B followed by C." This is the most fundamental combinator.

    Attributes:
        elements: The ordered list of sub-elements.
    """

    elements: list[GrammarElement]


@dataclass(frozen=True)
class Alternation:
    """A choice between alternatives, written with ``|`` in EBNF.

    ``A | B | C`` means "either A, or B, or C." The parser tries each
    alternative in order (for predictive parsers) or uses lookahead to
    decide.

    Attributes:
        choices: The list of alternatives.
    """

    choices: list[GrammarElement]


@dataclass(frozen=True)
class Repetition:
    """Zero-or-more repetition, written as ``{ x }`` in EBNF.

    ``{ statement }`` means "zero or more statements." This replaces
    the recursive rules that plain BNF requires.

    Attributes:
        element: The sub-element to repeat.
    """

    element: GrammarElement


@dataclass(frozen=True)
class Optional:
    """Optional element, written as ``[ x ]`` in EBNF.

    ``[ ELSE block ]`` means "optionally an ELSE followed by a block."
    Equivalent to ``x | epsilon`` in BNF.

    Attributes:
        element: The sub-element that may or may not appear.
    """

    element: GrammarElement


@dataclass(frozen=True)
class Group:
    """Explicit grouping, written as ``( x )`` in EBNF.

    ``( PLUS | MINUS )`` groups the alternation so it can be used as a
    single element in a sequence: ``term { ( PLUS | MINUS ) term }``.

    Attributes:
        element: The grouped sub-element.
    """

    element: GrammarElement


# The union of all grammar element types. This is what recursive functions
# over the grammar tree accept and pattern-match on.
GrammarElement = (
    RuleReference | Literal | Alternation | Sequence | Repetition | Optional | Group
)


# ---------------------------------------------------------------------------
# Data model for the complete grammar
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class GrammarRule:
    """A single rule from a .grammar file.

    Attributes:
        name: The rule name (lowercase identifier).
        body: The parsed EBNF body as a tree of GrammarElement nodes.
        line_number: The 1-based line number where this rule appeared.
    """

    name: str
    body: GrammarElement
    line_number: int


@dataclass
class ParserGrammar:
    """The complete contents of a parsed .grammar file.

    Attributes:
        rules: Ordered list of grammar rules. The first rule is the
            entry point (start symbol).
    """

    rules: list[GrammarRule] = field(default_factory=list)

    def rule_names(self) -> set[str]:
        """Return all defined rule names."""
        return {r.name for r in self.rules}

    def token_references(self) -> set[str]:
        """Return all UPPERCASE names referenced anywhere in the grammar.

        These should correspond to token names in the .tokens file.
        """
        refs: set[str] = set()
        for rule in self.rules:
            _collect_token_refs(rule.body, refs)
        return refs

    def rule_references(self) -> set[str]:
        """Return all lowercase names referenced anywhere in the grammar.

        These should correspond to other rule names in this grammar.
        """
        refs: set[str] = set()
        for rule in self.rules:
            _collect_rule_refs(rule.body, refs)
        return refs


def _collect_token_refs(node: GrammarElement, refs: set[str]) -> None:
    """Walk the AST and collect all token (UPPERCASE) references."""
    match node:
        case RuleReference(name=name, is_token=True):
            refs.add(name)
        case RuleReference():
            pass
        case Literal():
            pass
        case Sequence(elements=elems):
            for e in elems:
                _collect_token_refs(e, refs)
        case Alternation(choices=choices):
            for c in choices:
                _collect_token_refs(c, refs)
        case Repetition(element=e) | Optional(element=e) | Group(element=e):
            _collect_token_refs(e, refs)


def _collect_rule_refs(node: GrammarElement, refs: set[str]) -> None:
    """Walk the AST and collect all rule (lowercase) references."""
    match node:
        case RuleReference(name=name, is_token=False):
            refs.add(name)
        case RuleReference():
            pass
        case Literal():
            pass
        case Sequence(elements=elems):
            for e in elems:
                _collect_rule_refs(e, refs)
        case Alternation(choices=choices):
            for c in choices:
                _collect_rule_refs(c, refs)
        case Repetition(element=e) | Optional(element=e) | Group(element=e):
            _collect_rule_refs(e, refs)


# ---------------------------------------------------------------------------
# Tokenizer for .grammar files
# ---------------------------------------------------------------------------
# Before we can parse the EBNF, we need to break the raw text into tokens.
# This is a simple hand-written tokenizer — much simpler than the lexers we
# are trying to generate, because the grammar notation uses only a few token
# types.
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class _Token:
    """Internal token type for the grammar file tokenizer."""

    kind: str  # "IDENT", "STRING", "EQUALS", "SEMI", "PIPE", "LBRACE", etc.
    value: str
    line: int


def _tokenize_grammar(source: str) -> list[_Token]:
    """Break .grammar source text into tokens.

    Token types:
        IDENT   — an identifier (rule name or token reference)
        STRING  — a quoted literal "..."
        EQUALS  — the = sign separating rule name from body
        SEMI    — the ; terminating a rule
        PIPE    — the | alternation operator
        LBRACE / RBRACE — { }
        LBRACKET / RBRACKET — [ ]
        LPAREN / RPAREN — ( )
        EOF     — end of input
    """
    tokens: list[_Token] = []
    lines = source.split("\n")

    for line_number, raw_line in enumerate(lines, start=1):
        line = raw_line.rstrip()
        stripped = line.strip()

        # Skip blanks and comments.
        if stripped == "" or stripped.startswith("#"):
            continue

        i = 0
        while i < len(line):
            ch = line[i]

            # Skip whitespace.
            if ch in (" ", "\t"):
                i += 1
                continue

            # Skip inline comments.
            if ch == "#":
                break  # Rest of line is a comment.

            # Single-character tokens.
            if ch == "=":
                tokens.append(_Token("EQUALS", "=", line_number))
                i += 1
            elif ch == ";":
                tokens.append(_Token("SEMI", ";", line_number))
                i += 1
            elif ch == "|":
                tokens.append(_Token("PIPE", "|", line_number))
                i += 1
            elif ch == "{":
                tokens.append(_Token("LBRACE", "{", line_number))
                i += 1
            elif ch == "}":
                tokens.append(_Token("RBRACE", "}", line_number))
                i += 1
            elif ch == "[":
                tokens.append(_Token("LBRACKET", "[", line_number))
                i += 1
            elif ch == "]":
                tokens.append(_Token("RBRACKET", "]", line_number))
                i += 1
            elif ch == "(":
                tokens.append(_Token("LPAREN", "(", line_number))
                i += 1
            elif ch == ")":
                tokens.append(_Token("RPAREN", ")", line_number))
                i += 1

            # Quoted string literal.
            elif ch == '"':
                j = i + 1
                while j < len(line) and line[j] != '"':
                    if line[j] == "\\":
                        j += 1  # Skip escaped character.
                    j += 1
                if j >= len(line):
                    raise ParserGrammarError(
                        "Unterminated string literal",
                        line_number,
                    )
                # Include the quotes in the value so we can distinguish
                # literals from identifiers later.
                tokens.append(_Token("STRING", line[i + 1 : j], line_number))
                i = j + 1

            # Identifier (rule name or token reference).
            elif ch.isalpha() or ch == "_":
                j = i
                while j < len(line) and (line[j].isalnum() or line[j] == "_"):
                    j += 1
                tokens.append(_Token("IDENT", line[i:j], line_number))
                i = j

            else:
                raise ParserGrammarError(
                    f"Unexpected character: {ch!r}",
                    line_number,
                )

    tokens.append(_Token("EOF", "", len(lines)))
    return tokens


# ---------------------------------------------------------------------------
# Recursive descent parser for EBNF
# ---------------------------------------------------------------------------
# The parser consumes the token list produced by _tokenize_grammar and
# builds a tree of GrammarElement nodes. Each method corresponds to one
# level of the meta-grammar:
#
#   parse_grammar_file  ->  { rule }
#   _parse_rule         ->  name "=" body ";"
#   _parse_body         ->  sequence { "|" sequence }
#   _parse_sequence     ->  { element }
#   _parse_element      ->  ident | string | "{" body "}" | "[" body "]" | "(" body ")"
# ---------------------------------------------------------------------------


class _Parser:
    """Recursive descent parser for .grammar file EBNF notation."""

    def __init__(self, tokens: list[_Token]) -> None:
        self._tokens = tokens
        self._pos = 0

    def _peek(self) -> _Token:
        """Look at the current token without consuming it."""
        return self._tokens[self._pos]

    def _advance(self) -> _Token:
        """Consume and return the current token."""
        tok = self._tokens[self._pos]
        self._pos += 1
        return tok

    def _expect(self, kind: str) -> _Token:
        """Consume a token of the expected kind, or raise an error."""
        tok = self._advance()
        if tok.kind != kind:
            raise ParserGrammarError(
                f"Expected {kind}, got {tok.kind} ({tok.value!r})",
                tok.line,
            )
        return tok

    # --- Top level: grammar file = { rule } ---

    def parse(self) -> list[GrammarRule]:
        """Parse all rules in the grammar file."""
        rules: list[GrammarRule] = []
        while self._peek().kind != "EOF":
            rules.append(self._parse_rule())
        return rules

    # --- rule = name "=" body ";" ---

    def _parse_rule(self) -> GrammarRule:
        """Parse a single grammar rule."""
        name_tok = self._expect("IDENT")
        self._expect("EQUALS")
        body = self._parse_body()
        self._expect("SEMI")
        return GrammarRule(name=name_tok.value, body=body, line_number=name_tok.line)

    # --- body = sequence { "|" sequence } ---

    def _parse_body(self) -> GrammarElement:
        """Parse alternation: one or more sequences separated by '|'.

        If there is only one sequence (no '|'), we return it directly
        rather than wrapping it in an Alternation node. This keeps the
        AST clean — a rule like ``factor = NUMBER ;`` produces a simple
        RuleReference, not an Alternation with one choice containing a
        Sequence with one element.
        """
        first = self._parse_sequence()
        alternatives = [first]

        while self._peek().kind == "PIPE":
            self._advance()  # consume '|'
            alternatives.append(self._parse_sequence())

        if len(alternatives) == 1:
            return alternatives[0]
        return Alternation(choices=alternatives)

    # --- sequence = { element } ---

    def _parse_sequence(self) -> GrammarElement:
        """Parse a sequence of elements.

        A sequence ends when we hit something that cannot start an element:
        '|', ';', '}', ']', ')' or EOF. If the sequence has only one
        element, we return it directly (no Sequence wrapper).
        """
        elements: list[GrammarElement] = []

        while self._peek().kind not in (
            "PIPE",
            "SEMI",
            "RBRACE",
            "RBRACKET",
            "RPAREN",
            "EOF",
        ):
            elements.append(self._parse_element())

        if len(elements) == 0:
            raise ParserGrammarError(
                "Expected at least one element in sequence",
                self._peek().line,
            )
        if len(elements) == 1:
            return elements[0]
        return Sequence(elements=elements)

    # --- element = ident | string | "{" body "}" | "[" body "]" | "(" body ")" ---

    def _parse_element(self) -> GrammarElement:
        """Parse a single grammar element.

        This is where the recursive descent happens: braces, brackets,
        and parentheses cause us to recurse back into _parse_body.
        """
        tok = self._peek()

        if tok.kind == "IDENT":
            self._advance()
            # UPPERCASE = token reference, lowercase = rule reference.
            is_token = tok.value == tok.value.upper() and tok.value[0].isalpha()
            return RuleReference(name=tok.value, is_token=is_token)

        if tok.kind == "STRING":
            self._advance()
            return Literal(value=tok.value)

        if tok.kind == "LBRACE":
            self._advance()
            body = self._parse_body()
            self._expect("RBRACE")
            return Repetition(element=body)

        if tok.kind == "LBRACKET":
            self._advance()
            body = self._parse_body()
            self._expect("RBRACKET")
            return Optional(element=body)

        if tok.kind == "LPAREN":
            self._advance()
            body = self._parse_body()
            self._expect("RPAREN")
            return Group(element=body)

        raise ParserGrammarError(
            f"Unexpected token: {tok.kind} ({tok.value!r})",
            tok.line,
        )


# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------


def parse_parser_grammar(source: str) -> ParserGrammar:
    """Parse the text of a .grammar file into a ParserGrammar.

    This function tokenizes the source, then runs a recursive descent
    parser over the token stream to produce an AST of grammar elements.

    Args:
        source: The full text content of a .grammar file.

    Returns:
        A ParserGrammar containing all parsed rules.

    Raises:
        ParserGrammarError: If the source cannot be parsed.
    """
    tokens = _tokenize_grammar(source)
    parser = _Parser(tokens)
    rules = parser.parse()
    return ParserGrammar(rules=rules)


def validate_parser_grammar(
    grammar: ParserGrammar,
    token_names: set[str] | None = None,
) -> list[str]:
    """Check a parsed ParserGrammar for common problems.

    Validation checks:
    - **Undefined rule references**: A lowercase name is used in a rule
      body but never defined as a rule. This means the parser would have
      no idea how to parse that construct.
    - **Undefined token references**: An UPPERCASE name is used but does
      not appear in the provided token_names set. (Only checked if
      token_names is provided.)
    - **Duplicate rule names**: Two rules with the same name. The second
      would shadow the first.
    - **Non-lowercase rule names**: By convention, rule names are lowercase
      to distinguish them from token names.
    - **Unreachable rules**: A rule that is defined but never referenced
      by any other rule. The first rule (start symbol) is exempt.

    Args:
        grammar: A parsed ParserGrammar to validate.
        token_names: Optional set of valid token names from a .tokens file.
            If provided, UPPERCASE references are checked against it.

    Returns:
        A list of warning/error strings. An empty list means no issues.
    """
    issues: list[str] = []
    defined = grammar.rule_names()
    referenced_rules = grammar.rule_references()
    referenced_tokens = grammar.token_references()

    # --- Duplicate rule names ---
    seen: dict[str, int] = {}
    for rule in grammar.rules:
        if rule.name in seen:
            issues.append(
                f"Line {rule.line_number}: Duplicate rule name "
                f"'{rule.name}' (first defined on line {seen[rule.name]})"
            )
        else:
            seen[rule.name] = rule.line_number

    # --- Non-lowercase rule names ---
    for rule in grammar.rules:
        if rule.name != rule.name.lower():
            issues.append(
                f"Line {rule.line_number}: Rule name '{rule.name}' "
                f"should be lowercase"
            )

    # --- Undefined rule references ---
    for ref in sorted(referenced_rules):
        if ref not in defined:
            issues.append(f"Undefined rule reference: '{ref}'")

    # --- Undefined token references ---
    if token_names is not None:
        # Synthetic tokens are always valid — the lexer produces these
        # implicitly without needing a .tokens definition:
        #   NEWLINE — emitted at bare '\n' when skip pattern excludes newlines
        #   INDENT/DEDENT — emitted in indentation mode
        #   EOF — always emitted at end of input
        synthetic_tokens = {"NEWLINE", "INDENT", "DEDENT", "EOF"}
        for ref in sorted(referenced_tokens):
            if ref not in token_names and ref not in synthetic_tokens:
                issues.append(f"Undefined token reference: '{ref}'")

    # --- Unreachable rules ---
    if grammar.rules:
        start_rule = grammar.rules[0].name
        for rule in grammar.rules:
            if rule.name != start_rule and rule.name not in referenced_rules:
                issues.append(
                    f"Line {rule.line_number}: Rule '{rule.name}' is "
                    f"defined but never referenced (unreachable)"
                )

    return issues
