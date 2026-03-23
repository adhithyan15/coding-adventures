"""Grammar-Driven Parser — Parsing from .grammar Files
======================================================

Instead of hardcoding grammar rules as Python methods (one method per rule),
this parser reads grammar rules from a .grammar file and interprets them
at runtime. The same Python code can parse Python, Ruby, or any language —
just swap the .grammar file.

This is how tools like ANTLR and Yacc work: you define a grammar, the tool
generates (or interprets) a parser. We're doing the same thing at runtime.

=============================================================================
WHY A GRAMMAR-DRIVEN PARSER?
=============================================================================

The hand-written ``Parser`` in ``parser.py`` is great for learning — you can
see exactly how each grammar rule maps to a Python method. But it has a
limitation: the grammar is *baked into the code*. If you want to change the
grammar (add a new operator, a new statement type), you have to modify Python
code.

A grammar-driven parser separates the *grammar* from the *parsing engine*.
The grammar lives in a ``.grammar`` file (like ``python.grammar``), and the
parsing engine reads that file and follows its rules. To support a new
language, you write a new ``.grammar`` file — no Python changes needed.

This separation of concerns is the same principle behind:

- **ANTLR**: You write a ``.g4`` grammar file, ANTLR generates a parser.
- **Yacc/Bison**: You write a ``.y`` grammar file, the tool generates C code.
- **PEG parsers**: You write a PEG grammar, the runtime interprets it.

Our approach is closest to PEG (Parsing Expression Grammars) — we interpret
the grammar at runtime with backtracking, rather than generating code ahead
of time.

=============================================================================
HOW IT WORKS
=============================================================================

The ``GrammarParser`` receives two inputs:

1. A ``ParserGrammar`` — the parsed representation of a ``.grammar`` file,
   produced by ``grammar_tools.parse_parser_grammar()``. This contains a list
   of ``GrammarRule`` objects, each with a name and a body (a tree of EBNF
   elements like ``Sequence``, ``Alternation``, ``Repetition``, etc.).

2. A list of ``Token`` objects — the output of the lexer.

The parser walks the grammar rule tree, trying to match each element against
the token stream. The key insight is that each EBNF element type has a
natural interpretation:

- **RuleReference** (uppercase, e.g., ``NUMBER``): Match a token of that type.
- **RuleReference** (lowercase, e.g., ``expression``): Recursively parse that
  grammar rule.
- **Sequence** (``A B C``): Match A, then B, then C — all must succeed.
- **Alternation** (``A | B | C``): Try A first; if it fails, backtrack and
  try B; if B fails, try C.
- **Repetition** (``{ A }``): Match A zero or more times.
- **Optional** (``[ A ]``): Match A zero or one time.
- **Literal** (``"++"``): Match a token whose text value is exactly that string.
- **Group** (``( A )``): Just a parenthesized sub-expression.

=============================================================================
BACKTRACKING AND PACKRAT MEMOIZATION
=============================================================================

When an ``Alternation`` tries its first choice and it fails, the parser needs
to "undo" any tokens it consumed during that failed attempt. This is called
**backtracking**. We implement it by saving the position before each attempt
and restoring it on failure.

Backtracking makes the parser simple but potentially slow for ambiguous
grammars (exponential in the worst case). To prevent this, we use **packrat
memoization**: the first time we try to parse a given rule at a given
position, we cache the result. If we ever try the same (rule, position) pair
again, we return the cached result immediately.

This is the key insight behind `Packrat Parsing
<https://en.wikipedia.org/wiki/Parsing_expression_grammar#Packrat_parsing>`_:
memoization converts a potentially exponential backtracking parser into one
that runs in O(n × R) time, where n is the input length and R is the number
of grammar rules. The space cost is also O(n × R), but for the grammar
sizes we handle (< 100 rules), this is entirely acceptable.

=============================================================================
SIGNIFICANT NEWLINES
=============================================================================

Some grammars explicitly reference ``NEWLINE`` tokens (e.g., Starlark's
grammar uses NEWLINE as a statement terminator). In these grammars, newlines
are **significant** — the parser must not skip them.

Other grammars (like the simple ``python.grammar`` for expressions) don't
reference NEWLINE at all. In those grammars, newlines are **insignificant**
and the parser skips them automatically when matching token references.

The parser detects which mode to use by scanning the grammar rules for
NEWLINE references. If any rule mentions NEWLINE, newlines become significant.

=============================================================================
STRING-BASED TOKEN TYPES
=============================================================================

The grammar-driven lexer can emit tokens with either ``TokenType`` enum
values (for simple grammars) or string-based types (for extended grammars
like Starlark that define 45+ custom token names). The parser handles both
by comparing token types flexibly:

- If the token type is a ``TokenType`` enum member, compare by name.
- If the token type is a string, compare directly.

This means the parser works with both the hand-written lexer (enum types)
and the grammar-driven lexer (string or enum types) interchangeably.

=============================================================================
GENERIC AST NODES
=============================================================================

Unlike the hand-written parser (which produces specific nodes like
``NumberLiteral``, ``BinaryOp``, etc.), the grammar-driven parser produces
**generic** ``ASTNode`` objects. Each node records:

- ``rule_name``: Which grammar rule produced it (e.g., "expression", "factor").
- ``children``: The matched elements — a mix of sub-nodes and raw tokens.

This makes the grammar-driven parser language-agnostic. The same ``ASTNode``
type works for Python, Ruby, JavaScript, or any language whose grammar is
written in a ``.grammar`` file.

The trade-off is that consumers (like a bytecode compiler) need to inspect
``rule_name`` and walk ``children`` to extract meaning, rather than pattern-
matching on specific node types. An adapter layer can bridge this gap if
needed.
"""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass

from grammar_tools import (
    Alternation,
    GrammarRule,
    Group,
    Literal,
    ParserGrammar,
    Repetition,
    RuleReference,
    Sequence,
)
from grammar_tools import (
    Optional as OptionalElement,
)
from lexer import Token, TokenType

# A grammar element is any node in the EBNF rule tree.
GrammarElement = (
    Sequence
    | Alternation
    | Repetition
    | OptionalElement
    | Group
    | RuleReference
    | Literal
)

# =============================================================================
# GENERIC AST NODES
# =============================================================================


@dataclass
class ASTNode:
    """A generic AST node produced by grammar-driven parsing.

    Every node in the grammar-driven AST is an instance of this class.
    The ``rule_name`` tells you *what* grammar rule created the node, and
    ``children`` contains the matched sub-structure.

    Attributes:
        rule_name: Which grammar rule produced this node.
        children: The matched elements — a mix of ``ASTNode`` and ``Token``.
    """

    rule_name: str
    children: list[ASTNode | Token]

    @property
    def is_leaf(self) -> bool:
        """True if this node wraps a single token (no sub-structure)."""
        return len(self.children) == 1 and isinstance(self.children[0], Token)

    @property
    def token(self) -> Token | None:
        """The token if this is a leaf node, None otherwise."""
        if self.is_leaf and isinstance(self.children[0], Token):
            return self.children[0]
        return None


# =============================================================================
# PARSE ERROR
# =============================================================================


class GrammarParseError(Exception):
    """Error during grammar-driven parsing.

    Raised when the grammar-driven parser encounters a token that doesn't
    match any of the expected grammar alternatives.

    Attributes:
        token: The token where the error was detected, or None if at EOF.
    """

    def __init__(self, message: str, token: Token | None = None) -> None:
        self.token = token
        if token:
            super().__init__(
                f"Parse error at {token.line}:{token.column}: {message}"
            )
        else:
            super().__init__(f"Parse error: {message}")


# =============================================================================
# HELPER: TOKEN TYPE NAME EXTRACTION
# =============================================================================


def _token_type_name(token: Token) -> str:
    """Extract the type name from a token, handling both enum and string types.

    When the lexer emits tokens with ``TokenType`` enum values, we extract
    the ``.name`` attribute (e.g., ``TokenType.NUMBER`` → ``"NUMBER"``).
    When the lexer emits tokens with string types (from extended grammars),
    we use the string directly (e.g., ``"INT"`` → ``"INT"``).

    Args:
        token: A token to extract the type name from.

    Returns:
        The type name as a string.
    """
    if isinstance(token.type, str):
        return token.type
    return token.type.name


# =============================================================================
# THE GRAMMAR-DRIVEN PARSER
# =============================================================================


class GrammarParser:
    """A parser driven by a ParserGrammar (parsed from a .grammar file).

    This parser interprets EBNF grammar rules at runtime, using backtracking
    with packrat memoization. It produces a tree of generic ``ASTNode``
    objects.

    ==========================================================================
    HOW TO USE
    ==========================================================================

    1. Parse a ``.grammar`` file using ``grammar_tools.parse_parser_grammar()``.
    2. Tokenize source code using ``lexer.Lexer`` or ``lexer.GrammarLexer``.
    3. Feed both to ``GrammarParser`` and call ``.parse()``.

    Example::

        from grammar_tools import parse_parser_grammar
        from lexer import Lexer

        grammar = parse_parser_grammar(open("python.grammar").read())
        tokens = Lexer("x = 1 + 2").tokenize()
        ast = GrammarParser(tokens, grammar).parse()

    Attributes:
        _tokens: The complete list of tokens from the lexer.
        _grammar: The parsed grammar (from a .grammar file).
        _pos: Current position in the token list.
        _rules: Lookup dict mapping rule names to GrammarRule objects.
        _newlines_significant: Whether NEWLINE tokens should be matched
            explicitly (True) or skipped automatically (False).
        _memo: Packrat memoization cache. Maps (rule_name, position) to
            (result, end_position) for O(1) re-parsing of the same rule
            at the same position.
        _furthest_pos: The furthest position reached during parsing. Used
            for error reporting — the furthest position is often the best
            indicator of where the actual error is.
        _furthest_expected: What was expected at the furthest position.
    """

    def __init__(self, tokens: list[Token], grammar: ParserGrammar) -> None:
        """Initialize the grammar-driven parser.

        Args:
            tokens: A list of Token objects from the lexer, typically ending
                with an EOF token.
            grammar: A ParserGrammar containing the rules to parse by.
        """
        self._tokens = tokens
        self._grammar = grammar
        self._pos = 0

        # Build a lookup dict: rule_name -> GrammarRule
        self._rules: dict[str, GrammarRule] = {
            rule.name: rule for rule in grammar.rules
        }

        # Detect whether newlines are significant in this grammar.
        # If any rule references NEWLINE as a token, newlines are significant
        # and the parser must not skip them. Otherwise, newlines are
        # insignificant whitespace (like in simple expression grammars).
        self._newlines_significant = self._grammar_references_newline()

        # Packrat memoization cache.
        # Key: (rule_name, position_before_parsing)
        # Value: (result_children_or_None, position_after_parsing)
        self._memo: dict[tuple[str, int], tuple[list[ASTNode | Token] | None, int]] = {}

        # Furthest failure tracking for better error messages.
        self._furthest_pos = 0
        self._furthest_expected: list[str] = []

        # Transform hooks — pluggable pipeline stages for language-specific
        # processing. See the lexer's hooks for the general pattern.
        self._pre_parse_hooks: list[Callable[[list[Token]], list[Token]]] = []
        self._post_parse_hooks: list[Callable[[ASTNode], ASTNode]] = []

    def add_pre_parse(self, hook: Callable[[list[Token]], list[Token]]) -> None:
        """Register a token transform to run before parsing.

        The hook receives the token list and returns a (possibly modified)
        token list. Runs after all lexer hooks have completed.

        Use cases:
        - Token-level disambiguation
        - Injecting synthetic tokens for parser guidance

        Args:
            hook: A function list[Token] → list[Token].
        """
        self._pre_parse_hooks.append(hook)

    def add_post_parse(self, hook: Callable[[ASTNode], ASTNode]) -> None:
        """Register an AST transform to run after parsing.

        The hook receives the root ASTNode and returns a (possibly modified)
        ASTNode. Multiple hooks compose left-to-right.

        Use cases:
        - Lisp defmacro expansion
        - Desugaring (syntactic sugar → core forms)
        - AST optimization passes

        Args:
            hook: A function ASTNode → ASTNode.
        """
        self._post_parse_hooks.append(hook)

    def _grammar_references_newline(self) -> bool:
        """Check if any grammar rule references the NEWLINE token.

        Walks all grammar rules looking for RuleReference(name="NEWLINE",
        is_token=True). If found, newlines are significant in this grammar
        and the parser should not skip them.

        Returns:
            True if any rule references NEWLINE, False otherwise.
        """
        for rule in self._grammar.rules:
            if self._element_references_newline(rule.body):
                return True
        return False

    def _element_references_newline(
        self,
        element: GrammarElement,
    ) -> bool:
        """Recursively check if a grammar element references NEWLINE."""
        if isinstance(element, RuleReference):
            return element.is_token and element.name == "NEWLINE"
        if isinstance(element, Sequence):
            return any(self._element_references_newline(e) for e in element.elements)
        if isinstance(element, Alternation):
            return any(self._element_references_newline(c) for c in element.choices)
        if isinstance(element, (Repetition, OptionalElement, Group)):
            return self._element_references_newline(element.element)
        return False

    def parse(self) -> ASTNode:
        """Parse the token stream using the first grammar rule as entry point.

        The parsing pipeline has three stages:

        1. **Pre-parse hooks** — transform the token list before parsing.
           Each hook receives a token list and returns a token list. Multiple
           hooks compose left-to-right (A → B → C).

        2. **Core parsing** — the existing grammar-driven recursive descent
           parser, using the first rule as the start symbol.

        3. **Post-parse hooks** — transform the AST after parsing.
           Each hook receives an ASTNode and returns an ASTNode.

        When no hooks are registered, this is equivalent to the original
        parse() — zero overhead.

        Returns:
            An ASTNode representing the complete parse tree.

        Raises:
            GrammarParseError: If the grammar has no rules, the input doesn't
                match, or there are unconsumed tokens.
        """
        # Stage 1: Pre-parse hooks transform the token list.
        # Common use cases: token disambiguation, injecting synthetic tokens.
        # Each hook is list[Token] → list[Token].
        if self._pre_parse_hooks:
            tokens = self._tokens
            for hook in self._pre_parse_hooks:
                tokens = hook(tokens)
            self._tokens = tokens

        if not self._grammar.rules:
            raise GrammarParseError("Grammar has no rules")

        entry_rule = self._grammar.rules[0]
        result = self._parse_rule(entry_rule.name)

        # Skip trailing newlines (insignificant whitespace at end of file).
        while self._pos < len(self._tokens):
            tok = self._current()
            type_name = _token_type_name(tok)
            if type_name == "NEWLINE":
                self._pos += 1
            else:
                break

        # Verify we consumed all tokens (except EOF).
        if self._pos < len(self._tokens):
            tok = self._current()
            type_name = _token_type_name(tok)
            if type_name != "EOF":
                # Use furthest failure info for a better error message.
                if self._furthest_expected and self._furthest_pos > self._pos:
                    expected_str = " or ".join(self._furthest_expected[:5])
                    furthest_tok = (
                        self._tokens[self._furthest_pos]
                        if self._furthest_pos < len(self._tokens)
                        else tok
                    )
                    raise GrammarParseError(
                        f"Expected {expected_str}, got "
                        f"{furthest_tok.value!r}",
                        furthest_tok,
                    )
                raise GrammarParseError(
                    f"Unexpected token: {tok.value!r}",
                    tok,
                )

        # Stage 3: Post-parse hooks transform the AST.
        # Common use cases: Lisp defmacro expansion, desugaring,
        # AST optimization passes. Each hook is ASTNode → ASTNode.
        if self._post_parse_hooks:
            for hook in self._post_parse_hooks:
                result = hook(result)

        return result

    # =========================================================================
    # HELPERS
    # =========================================================================

    def _current(self) -> Token:
        """Get the current token without consuming it.

        If past the end of the token list, returns the last token (EOF).
        """
        if self._pos < len(self._tokens):
            return self._tokens[self._pos]
        return self._tokens[-1]  # EOF

    def _record_failure(self, expected: str) -> None:
        """Record an expected token/rule at the current position.

        If the current position is the furthest we've reached, update
        the furthest failure info. If it's further than the previous
        furthest, reset the expected list.

        This information is used for error messages: "Expected X or Y,
        got Z" is much more helpful than "Unexpected token: Z".
        """
        if self._pos > self._furthest_pos:
            self._furthest_pos = self._pos
            self._furthest_expected = [expected]
        elif self._pos == self._furthest_pos:
            if expected not in self._furthest_expected:
                self._furthest_expected.append(expected)

    # =========================================================================
    # RULE PARSING (with packrat memoization)
    # =========================================================================

    def _parse_rule(self, rule_name: str) -> ASTNode:
        """Parse a named grammar rule with memoization and left-recursion support.

        This method implements the seed-and-grow technique from Warth et al.,
        "Packrat Parsers Can Support Left Recursion" (2008). The algorithm
        handles left-recursive rules like:

            expression = expression PLUS term | term

        Without this technique, a left-recursive rule would cause infinite
        recursion: ``expression`` calls ``expression`` calls ``expression``...

        The seed-and-grow algorithm breaks the cycle in three steps:

        1. **Seed**: Before parsing the rule body, plant a failure entry in the
           memo cache. If the rule references itself at the same position, the
           memo check finds this failure entry and returns None, breaking the
           infinite recursion.

        2. **Initial parse**: Parse the rule body. The left-recursive alternative
           fails (hits the seed), but a non-recursive alternative may succeed.
           For ``expression = expression PLUS term | term``, the ``expression``
           alternative fails, but ``term`` succeeds.

        3. **Grow**: If the initial parse succeeded, iteratively re-parse the
           rule body with the previous successful result cached. Each iteration
           lets the left-recursive alternative consume more input:
           - First grow: ``expression`` (= term) ``PLUS term`` → succeeds
           - Second grow: ``expression`` (= term + term) ``PLUS term`` → succeeds
           - ...until no more input is consumed.

        Args:
            rule_name: The name of the grammar rule to parse.

        Returns:
            An ASTNode with ``rule_name`` and matched children.

        Raises:
            GrammarParseError: If the rule is undefined or doesn't match.
        """
        if rule_name not in self._rules:
            raise GrammarParseError(f"Undefined rule: {rule_name}")

        # Check memo cache.
        memo_key = (rule_name, self._pos)
        if memo_key in self._memo:
            cached_result, cached_end_pos = self._memo[memo_key]
            self._pos = cached_end_pos
            if cached_result is None:
                raise GrammarParseError(
                    f"Expected {rule_name}, got {self._current().value!r}",
                    self._current(),
                )
            return ASTNode(rule_name=rule_name, children=cached_result)

        start_pos = self._pos
        rule = self._rules[rule_name]

        # Left-recursion guard: seed the memo with a failure entry BEFORE
        # parsing the rule body. If the rule references itself (directly or
        # indirectly) at the same position, the memo check above will find
        # this failure entry and raise GrammarParseError, which the caller
        # in _match_element catches and treats as "no match" — breaking the
        # infinite recursion cycle.
        self._memo[memo_key] = (None, start_pos)

        children = self._match_element(rule.body)

        # Cache the result.
        self._memo[memo_key] = (children, self._pos)

        # If the initial parse succeeded, try to grow the match.
        # This is the iterative growth phase of the seed-and-grow algorithm.
        # Each iteration re-parses the rule body with the previous successful
        # result cached, allowing the left-recursive alternative to consume
        # more input.
        if children is not None:
            while True:
                prev_end = self._pos
                self._pos = start_pos
                self._memo[memo_key] = (children, prev_end)
                new_children = self._match_element(rule.body)
                if new_children is None or self._pos <= prev_end:
                    # Could not grow the match — restore the best result.
                    self._pos = prev_end
                    self._memo[memo_key] = (children, prev_end)
                    break
                children = new_children

        if children is None:
            self._pos = start_pos
            self._record_failure(rule_name)
            raise GrammarParseError(
                f"Expected {rule_name}, got {self._current().value!r}",
                self._current(),
            )

        return ASTNode(rule_name=rule_name, children=children)

    # =========================================================================
    # ELEMENT MATCHING — THE CORE OF THE GRAMMAR INTERPRETER
    # =========================================================================

    def _match_element(
        self,
        element: GrammarElement,
    ) -> list[ASTNode | Token] | None:
        """Try to match a grammar element against the token stream.

        This is a recursive interpreter for EBNF grammar elements. Position
        is restored on failure (backtracking).

        Args:
            element: A grammar element (Sequence, Alternation, etc.).

        Returns:
            A list of matched children on success, or None on failure.
        """
        save_pos = self._pos

        # -----------------------------------------------------------------
        # SEQUENCE: A B C — match all elements in order
        # -----------------------------------------------------------------
        if isinstance(element, Sequence):
            children: list[ASTNode | Token] = []
            for sub in element.elements:
                result = self._match_element(sub)
                if result is None:
                    self._pos = save_pos
                    return None
                children.extend(result)
            return children

        # -----------------------------------------------------------------
        # ALTERNATION: A | B | C — try each choice, first wins
        # -----------------------------------------------------------------
        elif isinstance(element, Alternation):
            for choice in element.choices:
                self._pos = save_pos
                result = self._match_element(choice)
                if result is not None:
                    return result
            self._pos = save_pos
            return None

        # -----------------------------------------------------------------
        # REPETITION: { A } — zero or more matches
        # -----------------------------------------------------------------
        elif isinstance(element, Repetition):
            children = []
            while True:
                save_rep = self._pos
                result = self._match_element(element.element)
                if result is None:
                    self._pos = save_rep
                    break
                children.extend(result)
            return children

        # -----------------------------------------------------------------
        # OPTIONAL: [ A ] — zero or one match
        # -----------------------------------------------------------------
        elif isinstance(element, OptionalElement):
            result = self._match_element(element.element)
            if result is None:
                return []
            return result

        # -----------------------------------------------------------------
        # GROUP: ( A ) — parenthesized sub-expression
        # -----------------------------------------------------------------
        elif isinstance(element, Group):
            return self._match_element(element.element)

        # -----------------------------------------------------------------
        # RULE REFERENCE — match a token type or parse a sub-rule
        # -----------------------------------------------------------------
        elif isinstance(element, RuleReference):
            if element.is_token:
                return self._match_token_reference(element)
            else:
                # lowercase: parse another grammar rule recursively.
                try:
                    node = self._parse_rule(element.name)
                    return [node]
                except GrammarParseError:
                    self._pos = save_pos
                    return None

        # -----------------------------------------------------------------
        # LITERAL — match a token by exact text value
        # -----------------------------------------------------------------
        elif isinstance(element, Literal):
            token = self._current()

            # Skip insignificant newlines before literal matching.
            if not self._newlines_significant:
                while _token_type_name(token) == "NEWLINE":
                    self._pos += 1
                    token = self._current()

            if token.value == element.value:
                self._pos += 1
                return [token]
            self._record_failure(f'"{element.value}"')
            return None

        return None  # pragma: no cover

    def _match_token_reference(
        self, element: RuleReference,
    ) -> list[Token] | None:
        """Match a token reference (UPPERCASE name) against the current token.

        Handles both ``TokenType`` enum values and string-based token types.
        Skips insignificant newlines when matching non-NEWLINE tokens.

        Args:
            element: A RuleReference with is_token=True.

        Returns:
            A list containing the matched token, or None on failure.
        """
        token = self._current()

        # Skip newlines when matching non-NEWLINE tokens, but only if
        # newlines are not significant in this grammar.
        if not self._newlines_significant and element.name != "NEWLINE":
            while _token_type_name(token) == "NEWLINE":
                self._pos += 1
                token = self._current()

        # Get the type name of the current token.
        type_name = _token_type_name(token)

        # Direct string comparison — works for both enum and string types.
        # For enum types: token.type.name == element.name
        # For string types: token.type == element.name
        if type_name == element.name:
            self._pos += 1
            return [token]

        # If the grammar references a name that exists in TokenType, also
        # try matching by enum value (backward compatibility).
        try:
            expected_type = TokenType[element.name]
            if token.type == expected_type:
                self._pos += 1
                return [token]
        except KeyError:
            pass

        self._record_failure(element.name)
        return None
