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
BACKTRACKING
=============================================================================

When an ``Alternation`` tries its first choice and it fails, the parser needs
to "undo" any tokens it consumed during that failed attempt. This is called
**backtracking**. We implement it by saving the position before each attempt
and restoring it on failure.

Backtracking makes the parser simple but potentially slow for ambiguous
grammars (exponential in the worst case). For the grammars we use — which are
designed for predictive parsing — backtracking rarely goes more than a few
tokens deep.

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

from dataclasses import dataclass
from typing import Any

from grammar_tools import (
    Alternation,
    GrammarRule,
    Group,
    Literal,
    Optional as OptionalElement,
    ParserGrammar,
    Repetition,
    RuleReference,
    Sequence,
)
from lexer import Token, TokenType


# =============================================================================
# GENERIC AST NODES
# =============================================================================
#
# These are the building blocks of the grammar-driven AST. Unlike the hand-
# written parser's specific node types (NumberLiteral, BinaryOp, etc.), these
# are generic containers that work for any grammar.
#
# Think of it like the difference between a custom-built bookshelf (hand-
# written parser) and a modular shelving system (grammar-driven parser).
# The custom bookshelf fits perfectly but only holds books of certain sizes.
# The modular system adapts to anything, but you need to label the shelves
# yourself.
# =============================================================================


@dataclass
class ASTNode:
    """A generic AST node produced by grammar-driven parsing.

    Every node in the grammar-driven AST is an instance of this class.
    The ``rule_name`` tells you *what* grammar rule created the node, and
    ``children`` contains the matched sub-structure.

    For example, parsing ``1 + 2`` with a grammar rule like::

        expression = term { ( PLUS | MINUS ) term } ;

    Might produce::

        ASTNode(
            rule_name="expression",
            children=[
                ASTNode(rule_name="term", children=[Token(NUMBER, "1")]),
                Token(PLUS, "+"),
                ASTNode(rule_name="term", children=[Token(NUMBER, "2")]),
            ]
        )

    Attributes:
        rule_name: Which grammar rule produced this node (e.g., "expression",
            "assignment", "factor"). This is the key for interpreting the node.
        children: The matched elements — a mix of ``ASTNode`` (from parsing
            sub-rules) and ``Token`` (from matching token types or literals).
    """

    rule_name: str
    children: list[ASTNode | Token]

    @property
    def is_leaf(self) -> bool:
        """True if this node wraps a single token (no sub-structure).

        Leaf nodes typically come from grammar rules like::

            factor = NUMBER | STRING | NAME ;

        Where the rule matches exactly one token. This property is a
        convenience for code that needs to distinguish leaf nodes from
        interior nodes.

        Returns:
            True if ``children`` contains exactly one element and that
            element is a ``Token``.
        """
        return len(self.children) == 1 and isinstance(self.children[0], Token)

    @property
    def token(self) -> Token | None:
        """The token if this is a leaf node, None otherwise.

        This is a shortcut for the common pattern of checking ``is_leaf``
        and then accessing ``children[0]``. It returns ``None`` for non-leaf
        nodes so callers can use it safely without checking first.

        Returns:
            The ``Token`` if this is a leaf node, ``None`` otherwise.
        """
        if self.is_leaf and isinstance(self.children[0], Token):
            return self.children[0]
        return None


# =============================================================================
# PARSE ERROR
# =============================================================================


class GrammarParseError(Exception):
    """Error during grammar-driven parsing.

    Raised when the grammar-driven parser encounters a token that doesn't
    match any of the expected grammar alternatives. Like the hand-written
    parser's ``ParseError``, this includes the problematic token so error
    messages can report the exact location.

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
# THE GRAMMAR-DRIVEN PARSER
# =============================================================================
#
# This is the heart of the module. The GrammarParser takes a ParserGrammar
# (the parsed .grammar file) and a token list, and produces an AST by
# interpreting the grammar rules at runtime.
#
# The key method is _match_element(), which dispatches on the type of
# grammar element (Sequence, Alternation, etc.) and recursively matches
# the token stream. This is essentially a tree-walking interpreter for
# EBNF grammars.
# =============================================================================


class GrammarParser:
    """A parser driven by a ParserGrammar (parsed from a .grammar file).

    This parser interprets EBNF grammar rules at runtime, using backtracking
    to handle alternations. It produces a tree of generic ``ASTNode`` objects.

    ==========================================================================
    HOW TO USE
    ==========================================================================

    1. Parse a ``.grammar`` file using ``grammar_tools.parse_parser_grammar()``.
    2. Tokenize source code using ``lexer.Lexer``.
    3. Feed both to ``GrammarParser`` and call ``.parse()``.

    Example::

        from grammar_tools import parse_parser_grammar
        from lexer import Lexer

        grammar_text = open("python.grammar").read()
        grammar = parse_parser_grammar(grammar_text)

        tokens = Lexer("x = 1 + 2").tokenize()
        parser = GrammarParser(tokens, grammar)
        ast = parser.parse()

    ==========================================================================
    GRAMMAR ELEMENT INTERPRETATION
    ==========================================================================

    Each EBNF element type is interpreted as follows:

    - **RuleReference** (uppercase, e.g., ``NUMBER``): Match a token whose
      ``TokenType`` name matches. Skips newlines automatically unless the
      reference is to ``NEWLINE`` itself.

    - **RuleReference** (lowercase, e.g., ``expression``): Recursively parse
      the named grammar rule. If parsing fails, backtrack.

    - **Sequence** (``A B C``): Match all elements in order. If any fails,
      the whole sequence fails and we backtrack.

    - **Alternation** (``A | B | C``): Try each choice in order. The first
      one that succeeds wins. Failed choices backtrack automatically.

    - **Repetition** (``{ A }``): Match zero or more occurrences of A.
      Always succeeds (zero matches is fine). Stops when A fails to match.

    - **Optional** (``[ A ]``): Match zero or one occurrence of A.
      Always succeeds (no match returns an empty list).

    - **Literal** (``"+"``): Match a token whose ``.value`` equals the
      literal string exactly.

    - **Group** (``( A )``): Just delegates to the sub-element. Groups
      exist only for syntactic clarity in the grammar.

    Attributes:
        _tokens: The complete list of tokens from the lexer.
        _grammar: The parsed grammar (from a .grammar file).
        _pos: Current position in the token list.
        _rules: Lookup dict mapping rule names to GrammarRule objects.
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
        # This lets us find any rule in O(1) time when we encounter a
        # RuleReference during parsing.
        self._rules: dict[str, GrammarRule] = {
            rule.name: rule for rule in grammar.rules
        }

    def parse(self) -> ASTNode:
        """Parse the token stream using the first grammar rule as entry point.

        The first rule in a ``.grammar`` file is always the start symbol
        (the top-level rule). For our Python subset, this is ``program``.

        After parsing the start rule, we verify that all tokens have been
        consumed (except for trailing newlines and the EOF token). If there
        are leftover tokens, something went wrong — the grammar didn't
        account for all the input.

        Returns:
            An ASTNode representing the complete parse tree.

        Raises:
            GrammarParseError: If the grammar has no rules, the input doesn't
                match the grammar, or there are unconsumed tokens.
        """
        if not self._grammar.rules:
            raise GrammarParseError("Grammar has no rules")

        entry_rule = self._grammar.rules[0]
        result = self._parse_rule(entry_rule.name)

        # Skip trailing newlines — these are insignificant whitespace
        # that often appears at the end of source files.
        while (
            self._pos < len(self._tokens)
            and self._current().type == TokenType.NEWLINE
        ):
            self._pos += 1

        # Verify we consumed all tokens (except EOF).
        # If there are leftover tokens, the grammar didn't fully describe
        # the input — there's something the parser doesn't understand.
        if (
            self._pos < len(self._tokens)
            and self._current().type != TokenType.EOF
        ):
            raise GrammarParseError(
                f"Unexpected token: {self._current().value!r}",
                self._current(),
            )

        return result

    # =========================================================================
    # HELPERS
    # =========================================================================

    def _current(self) -> Token:
        """Get the current token without consuming it.

        If we're past the end of the token list, returns the last token
        (which should be EOF). This prevents index-out-of-bounds errors.

        Returns:
            The token at the current position.
        """
        if self._pos < len(self._tokens):
            return self._tokens[self._pos]
        return self._tokens[-1]  # EOF

    # =========================================================================
    # RULE PARSING
    # =========================================================================

    def _parse_rule(self, rule_name: str) -> ASTNode:
        """Parse a named grammar rule.

        This is the entry point for parsing any grammar rule. It looks up
        the rule by name, matches its body against the token stream, and
        wraps the result in an ``ASTNode`` tagged with the rule name.

        Args:
            rule_name: The name of the grammar rule to parse.

        Returns:
            An ASTNode with ``rule_name`` set to the rule name and
            ``children`` containing all matched elements.

        Raises:
            GrammarParseError: If the rule is undefined or the token stream
                doesn't match the rule's body.
        """
        if rule_name not in self._rules:
            raise GrammarParseError(f"Undefined rule: {rule_name}")

        rule = self._rules[rule_name]
        children = self._match_element(rule.body)

        if children is None:
            raise GrammarParseError(
                f"Expected {rule_name}, got {self._current().value!r}",
                self._current(),
            )

        return ASTNode(rule_name=rule_name, children=children)

    # =========================================================================
    # ELEMENT MATCHING — THE CORE OF THE GRAMMAR INTERPRETER
    # =========================================================================
    #
    # _match_element() is the workhorse of the grammar-driven parser. It
    # takes a single grammar element (Sequence, Alternation, etc.) and tries
    # to match it against the token stream starting at the current position.
    #
    # It returns either:
    #   - A list of matched children (tokens and sub-nodes) on SUCCESS
    #   - None on FAILURE (no match)
    #
    # On failure, the position is restored to where it was before the attempt
    # (backtracking). This is critical for Alternation — if the first choice
    # fails, we need to try the second choice from the same position.
    #
    # The method dispatches on the type of grammar element using isinstance
    # checks. Each element type has its own matching logic, documented inline.
    # =========================================================================

    def _match_element(
        self, element: Any,
    ) -> list[ASTNode | Token] | None:
        """Try to match a grammar element against the token stream.

        This is a recursive interpreter for EBNF grammar elements. It
        handles each element type differently:

        - **Sequence**: Match all sub-elements in order; fail if any fails.
        - **Alternation**: Try each choice; succeed on first match.
        - **Repetition**: Match zero or more times; always succeeds.
        - **Optional**: Match zero or one time; always succeeds.
        - **Group**: Delegate to the inner element.
        - **RuleReference** (token): Match a token by type.
        - **RuleReference** (rule): Recursively parse the named rule.
        - **Literal**: Match a token by exact text value.

        Args:
            element: A grammar element (Sequence, Alternation, etc.) from
                the parsed grammar.

        Returns:
            A list of matched children (ASTNode and Token objects) on
            success, or None if the element doesn't match. Position is
            restored on failure.
        """
        save_pos = self._pos

        # -----------------------------------------------------------------
        # SEQUENCE: A B C — match all elements in order
        # -----------------------------------------------------------------
        # A sequence succeeds only if ALL its elements match in order.
        # If any element fails, the entire sequence fails and we restore
        # the position to before the first element.
        #
        # Example: The grammar rule ``NAME EQUALS expression`` is a
        # Sequence of three elements. We must match a NAME token, then
        # an EQUALS token, then the expression sub-rule — in that order.
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
        # An alternation tries each choice in order. The first choice that
        # matches wins. If a choice fails, we restore the position and try
        # the next one. If ALL choices fail, the alternation fails.
        #
        # This is where backtracking happens. For example, in the grammar:
        #   statement = assignment | expression_stmt ;
        #
        # We first try to parse an assignment. If that fails (because the
        # input isn't NAME EQUALS ...), we backtrack and try expression_stmt.
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
        # A repetition matches its element as many times as possible, then
        # stops. It ALWAYS succeeds — zero matches is fine. This implements
        # the Kleene star (*) from regular expressions.
        #
        # Example: ``{ statement }`` matches zero or more statements.
        # We keep parsing statements until we can't parse another one.
        elif isinstance(element, Repetition):
            children = []
            while True:
                save_rep = self._pos
                result = self._match_element(element.element)
                if result is None:
                    self._pos = save_rep
                    break
                children.extend(result)
            return children  # Always succeeds (zero matches is fine)

        # -----------------------------------------------------------------
        # OPTIONAL: [ A ] — zero or one match
        # -----------------------------------------------------------------
        # An optional element matches zero or one time. Like repetition,
        # it always succeeds — if the element doesn't match, we return
        # an empty list (no children).
        #
        # Example: ``[ ELSE block ]`` optionally matches an else clause.
        elif isinstance(element, OptionalElement):
            result = self._match_element(element.element)
            if result is None:
                return []  # Optional: no match is fine
            return result

        # -----------------------------------------------------------------
        # GROUP: ( A ) — just a parenthesized sub-expression
        # -----------------------------------------------------------------
        # Groups exist purely for syntactic clarity in the grammar file.
        # ``( PLUS | MINUS )`` groups the alternation so it can be used as
        # a single element in a sequence. We just delegate to the inner
        # element.
        elif isinstance(element, Group):
            return self._match_element(element.element)

        # -----------------------------------------------------------------
        # RULE REFERENCE (uppercase) — match a token by type
        # -----------------------------------------------------------------
        # An uppercase RuleReference like ``NUMBER`` means "match a token
        # whose TokenType is NUMBER." We look up the name in the TokenType
        # enum and compare.
        #
        # Special handling: we skip NEWLINE tokens when looking for non-
        # NEWLINE token types. This is because newlines are significant
        # as statement terminators but should be transparent within
        # expressions. For example, in multi-line expressions or when the
        # grammar doesn't explicitly include NEWLINE tokens.
        elif isinstance(element, RuleReference):
            if element.is_token:
                # UPPERCASE: match a token type
                token = self._current()

                # Skip newlines when matching non-NEWLINE tokens.
                # Newlines are statement terminators, not part of expressions.
                while (
                    token.type == TokenType.NEWLINE
                    and element.name != "NEWLINE"
                ):
                    self._pos += 1
                    token = self._current()

                # Look up the expected token type in the TokenType enum.
                # If the name isn't a valid TokenType, this reference can't
                # match anything.
                try:
                    expected_type = TokenType[element.name]
                except KeyError:
                    return None

                if token.type == expected_type:
                    self._pos += 1
                    return [token]

                return None
            else:
                # lowercase: parse another grammar rule recursively.
                # If parsing fails, we catch the error and backtrack.
                try:
                    node = self._parse_rule(element.name)
                    return [node]
                except GrammarParseError:
                    self._pos = save_pos
                    return None

        # -----------------------------------------------------------------
        # LITERAL — match a token by exact text value
        # -----------------------------------------------------------------
        # A literal like ``"++"`` matches a token whose .value equals
        # the literal string. This is less common than token references
        # but useful for matching specific keywords or symbols that don't
        # have their own token type.
        elif isinstance(element, Literal):
            token = self._current()
            if token.value == element.value:
                self._pos += 1
                return [token]
            return None

        # If we get here, we encountered an unknown grammar element type.
        # This shouldn't happen if the grammar was produced by grammar_tools.
        return None  # pragma: no cover
