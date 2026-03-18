"""Parser — Recursive Descent Parser for the Computing Stack.

=============================================================================
WHAT IS A PARSER?
=============================================================================

A parser is the second stage of a compiler or interpreter pipeline. It takes
a flat list of tokens (produced by the lexer) and builds a tree structure
called an Abstract Syntax Tree (AST). The AST captures the *meaning* of the
code by encoding the relationships between tokens.

Think of it like diagramming a sentence in English class:

    "The cat sat on the mat"

    Sentence
    ├── Subject: "The cat"
    └── Predicate: "sat on the mat"

Similarly, for code:

    "x = 1 + 2 * 3"

    Assignment
    ├── target: Name("x")
    └── value: BinaryOp("+")
        ├── left: NumberLiteral(1)
        └── right: BinaryOp("*")
            ├── left: NumberLiteral(2)
            └── right: NumberLiteral(3)

Notice how `2 * 3` is deeper in the tree than `1 +`. This means multiplication
gets evaluated FIRST. The tree structure itself encodes operator precedence —
no parentheses needed in the tree!

=============================================================================
RECURSIVE DESCENT PARSING
=============================================================================

This parser uses "recursive descent" — the simplest and most intuitive parsing
technique. The idea is beautifully direct:

    Each grammar rule becomes a Python method.

The grammar:

    program     = statement*
    statement   = assignment | expression_stmt
    assignment  = NAME EQUALS expression NEWLINE
    expression  = term ((PLUS | MINUS) term)*
    term        = factor ((STAR | SLASH) factor)*
    factor      = NUMBER | STRING | NAME | LPAREN expression RPAREN

Maps to methods:

    _parse_program()     → calls _parse_statement() in a loop
    _parse_statement()   → tries _parse_assignment(), falls back to expression
    _parse_expression()  → calls _parse_term(), loops on + and -
    _parse_term()        → calls _parse_factor(), loops on * and /
    _parse_factor()      → handles numbers, strings, names, parenthesized exprs

The "recursive" part comes from the fact that these methods call each other.
For example, _parse_factor() can call _parse_expression() when it encounters
a parenthesized expression — and _parse_expression() will call _parse_term()
which calls _parse_factor() again. It's recursive!

=============================================================================
OPERATOR PRECEDENCE
=============================================================================

Operator precedence is encoded by the *depth* of the grammar rules:

    expression  →  handles + and -     (LOWEST precedence — evaluated LAST)
    term        →  handles * and /     (HIGHER precedence — evaluated FIRST)
    factor      →  handles atoms       (HIGHEST precedence — numbers, names)

When we parse `1 + 2 * 3`:

    1. _parse_expression() calls _parse_term()
    2. _parse_term() calls _parse_factor() → gets NumberLiteral(1)
    3. _parse_term() sees no * or /, returns NumberLiteral(1)
    4. _parse_expression() sees +, advances
    5. _parse_expression() calls _parse_term() again
    6. _parse_term() calls _parse_factor() → gets NumberLiteral(2)
    7. _parse_term() sees *, advances
    8. _parse_term() calls _parse_factor() → gets NumberLiteral(3)
    9. _parse_term() builds BinaryOp(NumberLiteral(2), "*", NumberLiteral(3))
    10. _parse_expression() builds BinaryOp(NumberLiteral(1), "+", BinaryOp(...))

The multiplication was handled at a deeper level, so it becomes a subtree.
"""

from __future__ import annotations

from dataclasses import dataclass

from lexer import Token, TokenType


# =============================================================================
# AST NODE CLASSES
# =============================================================================
#
# These are the building blocks of our Abstract Syntax Tree. Each class
# represents a different kind of syntactic construct in our language.
#
# We use Python's @dataclass decorator for these because AST nodes are
# simple containers of data — they hold their children and nothing else.
# A dataclass automatically generates __init__, __repr__, and __eq__
# methods, which makes them perfect for this purpose.
#
# Think of AST nodes like LEGO bricks. Each type has a specific shape
# (its fields), and you snap them together to build the full tree.
# =============================================================================


@dataclass
class NumberLiteral:
    """A numeric literal like 42 or 7.

    This is a "leaf" node in the AST — it has no children, just a value.
    It's the simplest possible expression: a number that evaluates to itself.

    Example:
        The source code `42` becomes NumberLiteral(value=42).

    Attributes:
        value: The integer value of the literal. We store it as an int
               rather than keeping the string representation because the
               parser's job is to convert syntax into structured meaning.
    """

    value: int


@dataclass
class StringLiteral:
    """A string literal like "hello" or "world".

    Similar to NumberLiteral, this is a leaf node. The lexer has already
    stripped the surrounding quotes, so we just store the string content.

    Example:
        The source code `"hello"` becomes StringLiteral(value="hello").

    Attributes:
        value: The string content, without surrounding quotes.
    """

    value: str


@dataclass
class Name:
    """A variable name (identifier) like x, total, or my_var.

    Names are references to values stored elsewhere. When we evaluate
    `x + 1`, the Name("x") node means "look up the current value of x."

    In a compiler or interpreter, Name nodes are resolved by looking up
    the name in a symbol table or environment.

    Example:
        The source code `x` becomes Name(name="x").

    Attributes:
        name: The identifier string. We call it 'name' rather than 'value'
              to emphasize that this is a reference, not a literal value.
    """

    name: str


@dataclass
class BinaryOp:
    """A binary operation like `1 + 2` or `x * y`.

    "Binary" means two operands (left and right), not binary numbers.
    This is an "interior" node in the AST — it has children (the operands).

    The tree structure naturally encodes operator precedence:
        `1 + 2 * 3` becomes:
            BinaryOp(
                left=NumberLiteral(1),
                op="+",
                right=BinaryOp(
                    left=NumberLiteral(2),
                    op="*",
                    right=NumberLiteral(3)
                )
            )

    The multiplication is deeper in the tree, so it gets evaluated first.

    Attributes:
        left:  The left operand (any expression).
        op:    The operator as a string: "+", "-", "*", or "/".
        right: The right operand (any expression).
    """

    left: Expression
    op: str
    right: Expression


@dataclass
class Assignment:
    """A variable assignment like `x = 1 + 2`.

    Assignments bind a name to a value. The target is always a Name node
    (the variable being assigned to), and the value is any expression.

    Example:
        `x = 1 + 2` becomes:
            Assignment(
                target=Name("x"),
                value=BinaryOp(NumberLiteral(1), "+", NumberLiteral(2))
            )

    Attributes:
        target: The Name being assigned to.
        value:  The expression whose result will be stored in the variable.
    """

    target: Name
    value: Expression


@dataclass
class Program:
    """The root node of the AST — represents an entire program.

    A program is simply a sequence of statements executed in order.
    This node exists so that the parser always returns a single root node,
    even when the source code contains multiple statements.

    Example:
        The source code:
            x = 1
            y = 2

        Becomes:
            Program(statements=[
                Assignment(Name("x"), NumberLiteral(1)),
                Assignment(Name("y"), NumberLiteral(2)),
            ])

    Attributes:
        statements: A list of all top-level statements in the program.
    """

    statements: list[Statement]


# =============================================================================
# TYPE ALIASES
# =============================================================================
#
# Python's type union syntax (using |) lets us define which node types can
# appear in which positions. This serves as documentation AND enables
# type-checker enforcement.
#
# Expression: anything that produces a value when evaluated.
# Statement: anything that performs an action (assignment, expression statement).
# =============================================================================

Expression = NumberLiteral | StringLiteral | Name | BinaryOp
"""An Expression is any AST node that evaluates to a value.

Numbers, strings, and names evaluate to themselves (or their bound value).
BinaryOp evaluates by computing its operation on the left and right values.
"""

Statement = Assignment | Expression
"""A Statement is a top-level action in the program.

An Assignment binds a value to a name. An expression used as a statement
is simply evaluated for its side effects (or, in a REPL, its value is printed).
"""


# =============================================================================
# PARSE ERROR
# =============================================================================


class ParseError(Exception):
    """Raised when the parser encounters an unexpected token.

    Parse errors include the problematic token so error messages can report
    the exact line and column where things went wrong. This is essential for
    a good developer experience — nobody likes "syntax error" with no location!

    Attributes:
        message: A human-readable description of what went wrong.
        token:   The token where the error was detected.
    """

    def __init__(self, message: str, token: Token) -> None:
        self.message = message
        self.token = token
        super().__init__(f"{message} at line {token.line}, column {token.column}")


# =============================================================================
# THE PARSER
# =============================================================================


class Parser:
    """A recursive descent parser that builds an AST from a token stream.

    ==========================================================================
    HOW IT WORKS
    ==========================================================================

    The parser maintains a cursor (self._pos) into the token list. It reads
    tokens one at a time, advancing the cursor as it consumes them. Each
    grammar rule is a method that:

        1. Looks at the current token(s) to decide what to parse
        2. Consumes the tokens it needs
        3. Returns an AST node

    The "recursive descent" comes from the way these methods call each other
    in a pattern that mirrors the grammar rules. Higher-level rules (like
    _parse_expression) call lower-level rules (like _parse_term), which call
    even lower-level rules (like _parse_factor).

    ==========================================================================
    USAGE
    ==========================================================================

        tokens = [Token(TokenType.NUMBER, "42", 1, 1), Token(TokenType.EOF, "", 1, 3)]
        parser = Parser(tokens)
        ast = parser.parse()
        # ast is now a Program node containing the AST

    ==========================================================================
    GRAMMAR
    ==========================================================================

        program         = statement*
        statement       = assignment | expression_stmt
        assignment      = NAME EQUALS expression NEWLINE
        expression_stmt = expression NEWLINE
        expression      = term ((PLUS | MINUS) term)*
        term            = factor ((STAR | SLASH) factor)*
        factor          = NUMBER | STRING | NAME | LPAREN expression RPAREN

    Each grammar rule maps directly to a _parse_* method below.

    Attributes:
        _tokens: The complete list of tokens from the lexer.
        _pos:    The current position in the token list (our "cursor").
    """

    def __init__(self, tokens: list[Token]) -> None:
        """Initialize the parser with a list of tokens.

        The token list should end with an EOF token — this is the standard
        convention from the lexer and serves as a sentinel value so we
        never accidentally read past the end of the input.

        Args:
            tokens: A list of Token objects, typically ending with EOF.
        """
        self._tokens = tokens
        self._pos = 0

    # =========================================================================
    # HELPER METHODS
    # =========================================================================
    #
    # These utility methods abstract away the mechanics of reading tokens.
    # They keep the grammar methods clean and focused on structure rather
    # than bookkeeping.
    # =========================================================================

    def _peek(self) -> Token:
        """Look at the current token WITHOUT consuming it.

        This is like glancing at the next card in a deck without picking it up.
        We use _peek() when we need to decide what to do based on the current
        token, but haven't committed to consuming it yet.

        Returns:
            The token at the current position. If we've reached the end
            of the token list, returns the last token (which should be EOF).
        """
        if self._pos < len(self._tokens):
            return self._tokens[self._pos]
        # Safety: return the last token (should be EOF) if we're past the end.
        # This prevents index-out-of-bounds errors in edge cases.
        return self._tokens[-1]

    def _advance(self) -> Token:
        """Consume the current token and move to the next one.

        This is like picking up a card from the deck. Once consumed, we
        can't go back (this is a simple parser with no backtracking).

        Returns:
            The token that was consumed (the one at the old position).
        """
        token = self._peek()
        self._pos += 1
        return token

    def _expect(self, token_type: TokenType) -> Token:
        """Consume the current token, but ONLY if it has the expected type.

        This is used when the grammar requires a specific token. For example,
        after seeing `(`, we know a `)` must eventually appear. If it doesn't,
        that's a syntax error.

        Think of it like a bouncer at a door: "I'm expecting a RPAREN.
        If you're not an RPAREN, you can't come in."

        Args:
            token_type: The TokenType we expect to see.

        Returns:
            The consumed token (if it matched).

        Raises:
            ParseError: If the current token doesn't match the expected type.
        """
        token = self._peek()
        if token.type != token_type:
            raise ParseError(
                f"Expected {token_type.name}, got {token.type.name} ({token.value!r})",
                token,
            )
        return self._advance()

    def _match(self, *token_types: TokenType) -> Token | None:
        """Consume the current token if it matches ANY of the given types.

        This is a "soft" version of _expect(). Instead of raising an error
        when there's no match, it simply returns None. This is perfect for
        optional grammar elements — like the operator in `expression`:

            expression = term ((PLUS | MINUS) term)*

        The `(PLUS | MINUS)` part is optional (the `*` means zero or more).
        We use _match() to check: "Is there a + or - here? If so, consume it.
        If not, that's fine — we're done with this expression."

        Args:
            *token_types: One or more TokenTypes to try matching.

        Returns:
            The consumed token if it matched, or None if it didn't.
        """
        token = self._peek()
        if token.type in token_types:
            return self._advance()
        return None

    def _at_end(self) -> bool:
        """Check if we've reached the end of the token stream.

        Returns:
            True if the current token is EOF, False otherwise.
        """
        return self._peek().type == TokenType.EOF

    def _skip_newlines(self) -> None:
        """Skip over any NEWLINE tokens at the current position.

        Newlines are significant in our grammar (they terminate statements),
        but between statements we want to skip any extra blank lines.
        """
        while self._peek().type == TokenType.NEWLINE:
            self._advance()

    # =========================================================================
    # PUBLIC API
    # =========================================================================

    def parse(self) -> Program:
        """Parse the token stream and return a complete AST.

        This is the main entry point. It delegates to _parse_program(),
        which handles the top-level grammar rule.

        Returns:
            A Program node containing all parsed statements.

        Raises:
            ParseError: If the token stream contains syntax errors.
        """
        return self._parse_program()

    # =========================================================================
    # GRAMMAR METHODS
    # =========================================================================
    #
    # Each method below corresponds to one rule in the grammar. The method
    # names follow the pattern _parse_<rule_name>().
    #
    # These methods form the heart of the recursive descent parser. Reading
    # them alongside the grammar rules makes the mapping clear:
    #
    #   Grammar Rule                        → Method
    #   program = statement*                → _parse_program()
    #   statement = assignment | expr_stmt  → _parse_statement()
    #   assignment = NAME EQUALS expr NL    → _parse_assignment()
    #   expression = term ((+|-) term)*     → _parse_expression()
    #   term = factor ((*|/) factor)*       → _parse_term()
    #   factor = NUMBER | STRING | NAME |   → _parse_factor()
    #            LPAREN expr RPAREN
    # =========================================================================

    def _parse_program(self) -> Program:
        """Parse the top-level program: a sequence of statements.

        Grammar: program = statement*

        A program is zero or more statements. We keep parsing statements
        until we hit EOF (end of file). This is the root of our recursive
        descent — everything starts here.

        Returns:
            A Program node containing all parsed statements.
        """
        statements: list[Statement] = []

        # Skip any leading newlines (blank lines at the top of the file).
        self._skip_newlines()

        # Parse statements until we reach the end of the token stream.
        while not self._at_end():
            statement = self._parse_statement()
            statements.append(statement)

            # Skip any newlines between statements (blank lines).
            self._skip_newlines()

        return Program(statements=statements)

    def _parse_statement(self) -> Statement:
        """Parse a single statement: either an assignment or an expression.

        Grammar: statement = assignment | expression_stmt

        The tricky part here is distinguishing assignments from expressions.
        Both can start with a NAME token:
            - `x = 1 + 2`  ← assignment (NAME followed by EQUALS)
            - `x + 1`      ← expression starting with a name

        We use LOOKAHEAD: peek at the current token and the next one.
        If we see NAME followed by EQUALS, it's an assignment.
        Otherwise, it's an expression statement.

        Returns:
            Either an Assignment node or an Expression node.
        """
        # Lookahead: check if this is an assignment (NAME EQUALS ...).
        # We need to look TWO tokens ahead — the current one (NAME) and
        # the next one (EQUALS). This is called "LL(2)" lookahead.
        if (
            self._peek().type == TokenType.NAME
            and self._pos + 1 < len(self._tokens)
            and self._tokens[self._pos + 1].type == TokenType.EQUALS
        ):
            return self._parse_assignment()

        return self._parse_expression_stmt()

    def _parse_assignment(self) -> Assignment:
        """Parse an assignment statement: NAME EQUALS expression NEWLINE.

        Grammar: assignment = NAME EQUALS expression NEWLINE

        An assignment binds the result of an expression to a variable name.
        For example, `x = 1 + 2` means "evaluate 1 + 2 and store the
        result (3) in a variable called x."

        Returns:
            An Assignment node with the target name and value expression.
        """
        # Consume the variable name (the left side of the =).
        name_token = self._expect(TokenType.NAME)
        target = Name(name=name_token.value)

        # Consume the = sign.
        self._expect(TokenType.EQUALS)

        # Parse the value expression (the right side of the =).
        value = self._parse_expression()

        # Consume the newline that terminates the statement.
        # If we're at EOF, that's also acceptable (last line without newline).
        if not self._at_end():
            self._expect(TokenType.NEWLINE)

        return Assignment(target=target, value=value)

    def _parse_expression_stmt(self) -> Expression:
        """Parse an expression used as a statement.

        Grammar: expression_stmt = expression NEWLINE

        Sometimes an expression appears on its own line, not as part of
        an assignment. For example, in a REPL:

            >>> 1 + 2
            3

        The `1 + 2` is an expression statement — it's evaluated and the
        result is displayed.

        Returns:
            An Expression node (the expression itself IS the statement).
        """
        expression = self._parse_expression()

        # Consume the trailing newline (or accept EOF).
        if not self._at_end():
            self._expect(TokenType.NEWLINE)

        return expression

    def _parse_expression(self) -> Expression:
        """Parse an expression: addition and subtraction.

        Grammar: expression = term ((PLUS | MINUS) term)*

        This handles the LOWEST precedence operators: + and -. It calls
        _parse_term() for the operands, which handles * and / (higher
        precedence).

        The pattern `left = ...; while match(op): right = ...; left = BinaryOp(...)`
        is the standard way to handle left-associative operators in recursive
        descent. It ensures that `1 + 2 + 3` parses as `(1 + 2) + 3`.

        HOW LEFT-ASSOCIATIVITY WORKS:

            Parsing `1 + 2 + 3`:

            1. left = NumberLiteral(1)         [parse_term returns 1]
            2. See +, consume it
            3. right = NumberLiteral(2)         [parse_term returns 2]
            4. left = BinaryOp(1, "+", 2)       [combine into tree]
            5. See +, consume it
            6. right = NumberLiteral(3)         [parse_term returns 3]
            7. left = BinaryOp((1+2), "+", 3)   [combine again — LEFT associative!]

        Returns:
            An Expression node representing the parsed expression.
        """
        # Start by parsing the left operand (a term).
        left = self._parse_term()

        # Keep consuming + and - operators, building up the tree.
        while (op_token := self._match(TokenType.PLUS, TokenType.MINUS)) is not None:
            # Parse the right operand (another term).
            right = self._parse_term()
            # Build a BinaryOp node with the left and right operands.
            # This becomes the new "left" for the next iteration, which
            # is what makes the result left-associative.
            left = BinaryOp(left=left, op=op_token.value, right=right)

        return left

    def _parse_term(self) -> Expression:
        """Parse a term: multiplication and division.

        Grammar: term = factor ((STAR | SLASH) factor)*

        This handles HIGHER precedence operators: * and /. It calls
        _parse_factor() for the operands, which handles the highest
        precedence items (numbers, names, parenthesized expressions).

        The structure is identical to _parse_expression() — just with
        different operators and different sub-rule calls. This pattern
        can be extended to any number of precedence levels.

        Returns:
            An Expression node representing the parsed term.
        """
        # Start by parsing the left operand (a factor).
        left = self._parse_factor()

        # Keep consuming * and / operators.
        while (op_token := self._match(TokenType.STAR, TokenType.SLASH)) is not None:
            # Parse the right operand (another factor).
            right = self._parse_factor()
            left = BinaryOp(left=left, op=op_token.value, right=right)

        return left

    def _parse_factor(self) -> Expression:
        """Parse a factor: the highest-precedence items.

        Grammar: factor = NUMBER | STRING | NAME | LPAREN expression RPAREN

        Factors are the "atoms" of our expressions — the indivisible units.
        They include:

        - NUMBER: A numeric literal like 42.
        - STRING: A string literal like "hello".
        - NAME:   A variable reference like x.
        - (expr): A parenthesized expression, which lets the programmer
                  override the default precedence. `(1 + 2) * 3` forces
                  addition to happen before multiplication.

        The parenthesized case is where the "recursive" in "recursive descent"
        really shines. When we see `(`, we call _parse_expression() — which
        might call _parse_term() which calls _parse_factor() which might
        see another `(` and call _parse_expression() again! The call stack
        naturally handles arbitrarily nested parentheses.

        Returns:
            An Expression node for the parsed factor.

        Raises:
            ParseError: If the current token can't start a factor.
        """
        token = self._peek()

        # --- NUMBER literal ---
        # A bare number like 42. We consume it and wrap it in a NumberLiteral.
        if token.type == TokenType.NUMBER:
            self._advance()
            return NumberLiteral(value=int(token.value))

        # --- STRING literal ---
        # A quoted string like "hello". The lexer already stripped the quotes.
        if token.type == TokenType.STRING:
            self._advance()
            return StringLiteral(value=token.value)

        # --- NAME (variable reference) ---
        # A variable name like x or total. We wrap it in a Name node.
        if token.type == TokenType.NAME:
            self._advance()
            return Name(name=token.value)

        # --- Parenthesized expression ---
        # An expression wrapped in ( and ). This lets the programmer override
        # the default operator precedence.
        #
        # We consume the (, parse the inner expression (which can be
        # arbitrarily complex), then consume the closing ).
        if token.type == TokenType.LPAREN:
            self._advance()  # consume the (
            expression = self._parse_expression()
            self._expect(TokenType.RPAREN)  # consume the ) — error if missing!
            return expression

        # --- ERROR: unexpected token ---
        # If we get here, the current token can't start a factor. This is
        # a syntax error. Common causes:
        #   - A stray operator: `+ 1` (nothing on the left)
        #   - A missing operand: `1 +` (nothing on the right)
        #   - A typo or unexpected character
        raise ParseError(
            f"Unexpected token {token.type.name} ({token.value!r})",
            token,
        )
