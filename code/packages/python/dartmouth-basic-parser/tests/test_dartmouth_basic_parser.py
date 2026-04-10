"""Tests for the Dartmouth BASIC 1964 parser.

These tests verify that the grammar-driven parser, configured with
``dartmouth_basic.grammar``, correctly parses every statement type in the
original 1964 Dartmouth BASIC language into ASTs.

Why 1964 Dartmouth BASIC?
--------------------------
This is the language that made computing accessible. John Kemeny and Thomas
Kurtz designed it so that a student with no math background beyond high-school
algebra could write a working program in one sitting. The simplicity that made
it learnable in 1964 also makes it ideal for testing a grammar-driven parser:
the grammar is small enough to exercise completely in a single test suite.

How the Tests Work
-------------------
Each test passes a string of BASIC source code to ``parse_dartmouth_basic``
and checks properties of the returned ``ASTNode`` tree. The root is always
``rule_name="program"``. Helper functions walk the tree to find nodes by
rule name, which makes assertions concise and readable.

Test Coverage Goals
--------------------
- All 17 statement types parse without error
- All relational operators in IF statements
- Expression precedence (+ vs * vs ^)
- Right-associativity of ^ (exponentiation)
- Unary minus
- Parenthesised sub-expressions
- All 11 built-in functions (SIN, COS, TAN, ATN, EXP, LOG, ABS, SQR, INT, RND, SGN)
- User-defined function calls (FNA-style)
- Array subscript access
- Multi-line programs
- Bare line numbers (empty statements)
- Error cases (missing = in LET, missing THEN in IF, missing TO in FOR)
"""

from __future__ import annotations

import pytest

from dartmouth_basic_parser import create_dartmouth_basic_parser, parse_dartmouth_basic
from lang_parser import ASTNode, GrammarParser, GrammarParseError
from lexer import Token


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def find_nodes(node: ASTNode, rule_name: str) -> list[ASTNode]:
    """Recursively find all ASTNode descendants with a given rule_name.

    Walks the entire tree depth-first. Useful for asserting that specific
    grammar rules were matched somewhere in the tree, regardless of depth.
    """
    results: list[ASTNode] = []
    if node.rule_name == rule_name:
        results.append(node)
    for child in node.children:
        if isinstance(child, ASTNode):
            results.extend(find_nodes(child, rule_name))
    return results


def collect_tokens(node: ASTNode) -> list[Token]:
    """Recursively collect all Token leaf nodes from an AST.

    Returns every token in the tree in depth-first left-to-right order.
    Useful for checking that specific token types/values appear.
    """
    tokens: list[Token] = []
    for child in node.children:
        if isinstance(child, Token):
            tokens.append(child)
        elif isinstance(child, ASTNode):
            tokens.extend(collect_tokens(child))
    return tokens


def token_type_name(token: Token) -> str:
    """Return the token type as a string, handling both enum and str types."""
    return token.type if isinstance(token.type, str) else token.type.name


def token_values(node: ASTNode, type_name: str) -> list[str]:
    """Collect all token values of a given type from an AST."""
    return [
        t.value
        for t in collect_tokens(node)
        if token_type_name(t) == type_name
    ]


# ---------------------------------------------------------------------------
# Factory function tests
# ---------------------------------------------------------------------------


class TestFactory:
    """Tests for the create_dartmouth_basic_parser factory function."""

    def test_returns_grammar_parser(self) -> None:
        """create_dartmouth_basic_parser should return a GrammarParser instance."""
        parser = create_dartmouth_basic_parser("10 END\n")
        assert isinstance(parser, GrammarParser)

    def test_factory_produces_ast(self) -> None:
        """The factory-created parser should produce a valid AST."""
        parser = create_dartmouth_basic_parser("10 END\n")
        ast = parser.parse()
        assert isinstance(ast, ASTNode)
        assert ast.rule_name == "program"


# ---------------------------------------------------------------------------
# Root node
# ---------------------------------------------------------------------------


class TestRoot:
    """The root of every parse is a 'program' node."""

    def test_root_is_program(self) -> None:
        """The root rule_name is always 'program'."""
        ast = parse_dartmouth_basic("10 END\n")
        assert ast.rule_name == "program"

    def test_empty_program(self) -> None:
        """An empty source string produces a program node with no lines."""
        ast = parse_dartmouth_basic("")
        assert ast.rule_name == "program"
        # No line nodes for an empty program
        lines = find_nodes(ast, "line")
        assert len(lines) == 0

    def test_bare_line_number(self) -> None:
        """A bare line number with no statement is valid BASIC.

        In the original Dartmouth REPL, typing a bare line number deleted that
        line. In stored programs, it produces a no-op. The grammar rule is:
            line = LINE_NUM [ statement ] NEWLINE
        The [ statement ] part is optional, so LINE_NUM NEWLINE is valid.
        """
        ast = parse_dartmouth_basic("10\n")
        assert ast.rule_name == "program"
        lines = find_nodes(ast, "line")
        assert len(lines) == 1


# ---------------------------------------------------------------------------
# LET statement
# ---------------------------------------------------------------------------


class TestLetStatement:
    """Tests for the LET assignment statement.

    Grammar: let_stmt = "LET" variable EQ expr ;

    LET is explicit assignment. Unlike languages that allow bare assignment
    (X = 5), 1964 Dartmouth BASIC requires the LET keyword. This made the
    assignment operation syntactically unambiguous.
    """

    def test_let_simple(self) -> None:
        """LET X = 5 parses without error."""
        ast = parse_dartmouth_basic("10 LET X = 5\n")
        stmts = find_nodes(ast, "let_stmt")
        assert len(stmts) == 1

    def test_let_expr(self) -> None:
        """LET with an arithmetic expression."""
        ast = parse_dartmouth_basic("10 LET X = 2 + 3 * 4\n")
        stmts = find_nodes(ast, "let_stmt")
        assert len(stmts) == 1

    def test_let_variable_rhs(self) -> None:
        """LET with a variable on the right side."""
        ast = parse_dartmouth_basic("10 LET Y = X\n")
        stmts = find_nodes(ast, "let_stmt")
        assert len(stmts) == 1

    def test_let_array_subscript(self) -> None:
        """LET with an array subscript target."""
        ast = parse_dartmouth_basic("10 LET A(3) = 7\n")
        stmts = find_nodes(ast, "let_stmt")
        assert len(stmts) == 1


# ---------------------------------------------------------------------------
# PRINT statement
# ---------------------------------------------------------------------------


class TestPrintStatement:
    """Tests for the PRINT output statement.

    Grammar:
        print_stmt = "PRINT" [ print_list ] ;
        print_list = print_item { print_sep print_item } [ print_sep ] ;
        print_sep  = COMMA | SEMICOLON ;
        print_item = expr | STRING ;

    PRINT in 1964 BASIC is surprisingly rich:
    - PRINT with nothing outputs a blank line.
    - PRINT X outputs the value of X followed by a newline.
    - PRINT X, Y outputs X and Y in fixed 15-character columns (comma sep).
    - PRINT X; Y outputs X and Y with no extra space (semicolon sep).
    - A trailing comma or semicolon suppresses the final newline.
    """

    def test_print_bare(self) -> None:
        """PRINT with no arguments outputs a blank line."""
        ast = parse_dartmouth_basic("10 PRINT\n")
        stmts = find_nodes(ast, "print_stmt")
        assert len(stmts) == 1

    def test_print_expr(self) -> None:
        """PRINT with an expression."""
        ast = parse_dartmouth_basic("10 PRINT X + 1\n")
        stmts = find_nodes(ast, "print_stmt")
        assert len(stmts) == 1

    def test_print_string_literal(self) -> None:
        """PRINT with a string literal.

        String literals are double-quoted in 1964 BASIC. They can only be
        used with PRINT (there are no string variables in the original spec).
        """
        ast = parse_dartmouth_basic('10 PRINT "HELLO"\n')
        stmts = find_nodes(ast, "print_stmt")
        assert len(stmts) == 1

    def test_print_comma_separated(self) -> None:
        """PRINT X, Y — comma-separated values in fixed columns."""
        ast = parse_dartmouth_basic("10 PRINT X, Y\n")
        stmts = find_nodes(ast, "print_stmt")
        assert len(stmts) == 1

    def test_print_semicolon_separated(self) -> None:
        """PRINT X; Y — semicolon-separated values with no extra space."""
        ast = parse_dartmouth_basic("10 PRINT X; Y\n")
        stmts = find_nodes(ast, "print_stmt")
        assert len(stmts) == 1

    def test_print_trailing_comma(self) -> None:
        """PRINT X, — trailing comma suppresses the final newline."""
        ast = parse_dartmouth_basic("10 PRINT X,\n")
        stmts = find_nodes(ast, "print_stmt")
        assert len(stmts) == 1


# ---------------------------------------------------------------------------
# INPUT statement
# ---------------------------------------------------------------------------


class TestInputStatement:
    """Tests for the INPUT statement.

    Grammar: input_stmt = "INPUT" variable { COMMA variable } ;

    INPUT pauses execution, prints a ? prompt, and reads values typed by the
    user. Multiple variables are separated by commas; the user types
    corresponding values separated by commas.
    """

    def test_input_single(self) -> None:
        """INPUT X — reads one value."""
        ast = parse_dartmouth_basic("10 INPUT X\n")
        stmts = find_nodes(ast, "input_stmt")
        assert len(stmts) == 1

    def test_input_multiple(self) -> None:
        """INPUT A, B, C — reads three values."""
        ast = parse_dartmouth_basic("10 INPUT A, B, C\n")
        stmts = find_nodes(ast, "input_stmt")
        assert len(stmts) == 1


# ---------------------------------------------------------------------------
# IF statement and relational operators
# ---------------------------------------------------------------------------


class TestIfStatement:
    """Tests for the IF...THEN conditional branch.

    Grammar:
        if_stmt = "IF" expr relop expr "THEN" LINE_NUM ;
        relop   = EQ | LT | GT | LE | GE | NE ;

    1964 BASIC has no ELSE clause and no block structure. The branch target
    must be a literal line number. The condition compares two expressions
    with one of six relational operators.
    """

    def test_if_greater_than(self) -> None:
        """IF X > 0 THEN 100."""
        ast = parse_dartmouth_basic("10 IF X > 0 THEN 100\n")
        stmts = find_nodes(ast, "if_stmt")
        assert len(stmts) == 1

    def test_if_less_than(self) -> None:
        """IF X < 10 THEN 200."""
        ast = parse_dartmouth_basic("10 IF X < 10 THEN 200\n")
        stmts = find_nodes(ast, "if_stmt")
        assert len(stmts) == 1

    def test_if_equal(self) -> None:
        """IF X = Y THEN 50."""
        ast = parse_dartmouth_basic("10 IF X = Y THEN 50\n")
        stmts = find_nodes(ast, "if_stmt")
        assert len(stmts) == 1

    def test_if_not_equal(self) -> None:
        """IF X <> Y THEN 30 — using the not-equal operator."""
        ast = parse_dartmouth_basic("10 IF X <> Y THEN 30\n")
        stmts = find_nodes(ast, "if_stmt")
        assert len(stmts) == 1

    def test_if_less_equal(self) -> None:
        """IF X <= 5 THEN 40."""
        ast = parse_dartmouth_basic("10 IF X <= 5 THEN 40\n")
        stmts = find_nodes(ast, "if_stmt")
        assert len(stmts) == 1

    def test_if_greater_equal(self) -> None:
        """IF X >= 5 THEN 60."""
        ast = parse_dartmouth_basic("10 IF X >= 5 THEN 60\n")
        stmts = find_nodes(ast, "if_stmt")
        assert len(stmts) == 1

    def test_if_with_expressions(self) -> None:
        """IF with full arithmetic expressions on both sides."""
        ast = parse_dartmouth_basic("10 IF X + 1 > Y * 2 THEN 99\n")
        stmts = find_nodes(ast, "if_stmt")
        assert len(stmts) == 1


# ---------------------------------------------------------------------------
# GOTO statement
# ---------------------------------------------------------------------------


class TestGotoStatement:
    """Tests for the GOTO unconditional jump.

    Grammar: goto_stmt = "GOTO" LINE_NUM ;

    GOTO is the simplest control-flow statement. It jumps execution to the
    named line number unconditionally. In structured programming, GOTO is
    considered harmful; but in 1964, it was the only loop mechanism other
    than FOR...NEXT.
    """

    def test_goto(self) -> None:
        """GOTO 50 — jumps to line 50."""
        ast = parse_dartmouth_basic("10 GOTO 50\n")
        stmts = find_nodes(ast, "goto_stmt")
        assert len(stmts) == 1


# ---------------------------------------------------------------------------
# GOSUB / RETURN statements
# ---------------------------------------------------------------------------


class TestGosubReturn:
    """Tests for the GOSUB/RETURN subroutine mechanism.

    Grammar:
        gosub_stmt  = "GOSUB" LINE_NUM ;
        return_stmt = "RETURN" ;

    GOSUB pushes the return address onto a call stack and jumps to the target
    line. RETURN pops the return address and resumes from there. This gives
    1964 BASIC a primitive form of subroutine, predating proper functions.
    """

    def test_gosub(self) -> None:
        """GOSUB 200 — calls the subroutine at line 200."""
        ast = parse_dartmouth_basic("10 GOSUB 200\n")
        stmts = find_nodes(ast, "gosub_stmt")
        assert len(stmts) == 1

    def test_return(self) -> None:
        """RETURN — returns from the current subroutine."""
        ast = parse_dartmouth_basic("200 RETURN\n")
        stmts = find_nodes(ast, "return_stmt")
        assert len(stmts) == 1


# ---------------------------------------------------------------------------
# FOR / NEXT loop
# ---------------------------------------------------------------------------


class TestForNext:
    """Tests for the FOR...NEXT counted loop.

    Grammar:
        for_stmt  = "FOR" NAME EQ expr "TO" expr [ "STEP" expr ] ;
        next_stmt = "NEXT" NAME ;

    FOR...NEXT is the only structured loop in 1964 BASIC. It iterates a
    named variable from an initial value to a final value. STEP defaults
    to 1; a negative STEP counts down.

    The loop body consists of any statements between FOR and NEXT. Loops
    can nest but must not interleave (no crossing). The NEXT statement must
    name the same variable as the corresponding FOR.
    """

    def test_for_without_step(self) -> None:
        """FOR I = 1 TO 10 with no STEP (defaults to +1)."""
        ast = parse_dartmouth_basic("10 FOR I = 1 TO 10\n20 NEXT I\n")
        for_stmts = find_nodes(ast, "for_stmt")
        next_stmts = find_nodes(ast, "next_stmt")
        assert len(for_stmts) == 1
        assert len(next_stmts) == 1

    def test_for_with_step(self) -> None:
        """FOR I = 10 TO 1 STEP -1 — count down."""
        ast = parse_dartmouth_basic("10 FOR I = 10 TO 1 STEP -1\n20 NEXT I\n")
        for_stmts = find_nodes(ast, "for_stmt")
        assert len(for_stmts) == 1

    def test_for_with_positive_step(self) -> None:
        """FOR with explicit positive STEP."""
        ast = parse_dartmouth_basic("10 FOR I = 0 TO 100 STEP 5\n20 NEXT I\n")
        for_stmts = find_nodes(ast, "for_stmt")
        assert len(for_stmts) == 1


# ---------------------------------------------------------------------------
# END and STOP statements
# ---------------------------------------------------------------------------


class TestEndStop:
    """Tests for the program termination statements.

    Grammar:
        end_stmt  = "END" ;
        stop_stmt = "STOP" ;

    END is the normal program terminus. In the 1964 spec, every program must
    end with an END statement on the highest-numbered line.

    STOP halts execution with a ``STOP IN LINE n`` message and returns the user
    to the DTSS prompt. The program can be resumed from the STOP point.
    """

    def test_end(self) -> None:
        """END — normal program termination."""
        ast = parse_dartmouth_basic("10 END\n")
        stmts = find_nodes(ast, "end_stmt")
        assert len(stmts) == 1

    def test_stop(self) -> None:
        """STOP — halt with possibility of resume."""
        ast = parse_dartmouth_basic("10 STOP\n")
        stmts = find_nodes(ast, "stop_stmt")
        assert len(stmts) == 1


# ---------------------------------------------------------------------------
# REM statement
# ---------------------------------------------------------------------------


class TestRemStatement:
    """Tests for the REM comment statement.

    Grammar: rem_stmt = "REM" ;

    In the grammar, rem_stmt has an empty body after the REM keyword. This
    works because the lexer's post-tokenize hook strips everything between
    REM and NEWLINE from the token stream. By the time the parser sees the
    tokens, a REM line looks like: LINE_NUM KEYWORD("REM") NEWLINE.
    """

    def test_rem_with_text(self) -> None:
        """REM with a comment body — the text is stripped by the lexer."""
        ast = parse_dartmouth_basic("10 REM THIS IS A COMMENT\n")
        stmts = find_nodes(ast, "rem_stmt")
        assert len(stmts) == 1

    def test_rem_empty(self) -> None:
        """REM with no following text."""
        ast = parse_dartmouth_basic("10 REM\n")
        stmts = find_nodes(ast, "rem_stmt")
        assert len(stmts) == 1


# ---------------------------------------------------------------------------
# READ / DATA / RESTORE statements
# ---------------------------------------------------------------------------


class TestReadDataRestore:
    """Tests for the sequential data pool statements.

    Grammar:
        read_stmt    = "READ" variable { COMMA variable } ;
        data_stmt    = "DATA" NUMBER { COMMA NUMBER } ;
        restore_stmt = "RESTORE" ;

    DATA defines a pool of numeric constants embedded in the program text.
    READ pops values from this pool sequentially. RESTORE resets the read
    pointer to the beginning of the pool.

    This mechanism allows programs to embed lookup tables and initial datasets
    without the complexity of reading from files (which 1964 BASIC didn't have
    on the student-accessible DTSS system).
    """

    def test_read_single(self) -> None:
        """READ X — reads one value from the DATA pool."""
        ast = parse_dartmouth_basic("10 READ X\n")
        stmts = find_nodes(ast, "read_stmt")
        assert len(stmts) == 1

    def test_read_multiple(self) -> None:
        """READ A, B, C — reads three values from the DATA pool."""
        ast = parse_dartmouth_basic("10 READ A, B, C\n")
        stmts = find_nodes(ast, "read_stmt")
        assert len(stmts) == 1

    def test_data_single(self) -> None:
        """DATA 42 — a pool with one value."""
        ast = parse_dartmouth_basic("10 DATA 42\n")
        stmts = find_nodes(ast, "data_stmt")
        assert len(stmts) == 1

    def test_data_multiple(self) -> None:
        """DATA 1, 2, 3 — a pool with multiple values."""
        ast = parse_dartmouth_basic("10 DATA 1, 2, 3\n")
        stmts = find_nodes(ast, "data_stmt")
        assert len(stmts) == 1

    def test_restore(self) -> None:
        """RESTORE — resets the DATA pool read pointer."""
        ast = parse_dartmouth_basic("10 RESTORE\n")
        stmts = find_nodes(ast, "restore_stmt")
        assert len(stmts) == 1


# ---------------------------------------------------------------------------
# DIM statement
# ---------------------------------------------------------------------------


class TestDimStatement:
    """Tests for the DIM array dimensioning statement.

    Grammar:
        dim_stmt = "DIM" dim_decl { COMMA dim_decl } ;
        dim_decl = NAME LPAREN NUMBER RPAREN ;

    Without DIM, array indices default to 0–10 (11 elements). DIM allows
    programs to declare larger arrays. The size argument is a literal integer,
    not an expression — you can't DIM A(N) where N is a variable.
    """

    def test_dim_single(self) -> None:
        """DIM A(10) — single array dimensioning."""
        ast = parse_dartmouth_basic("10 DIM A(10)\n")
        stmts = find_nodes(ast, "dim_stmt")
        assert len(stmts) == 1

    def test_dim_multiple(self) -> None:
        """DIM A(10), B(20) — dimensioning two arrays in one statement."""
        ast = parse_dartmouth_basic("10 DIM A(10), B(20)\n")
        stmts = find_nodes(ast, "dim_stmt")
        assert len(stmts) == 1
        decls = find_nodes(ast, "dim_decl")
        assert len(decls) == 2


# ---------------------------------------------------------------------------
# DEF statement
# ---------------------------------------------------------------------------


class TestDefStatement:
    """Tests for the DEF user-defined function statement.

    Grammar: def_stmt = "DEF" USER_FN LPAREN NAME RPAREN EQ expr ;

    DEF defines a single-argument mathematical function named FNA through FNZ
    (26 possible user functions). The body can reference the formal parameter
    and any global variable. This provides a primitive form of parametrised
    computation without the overhead of GOSUB.
    """

    def test_def_simple(self) -> None:
        """DEF FNA(X) = X * X — square function."""
        ast = parse_dartmouth_basic("10 DEF FNA(X) = X * X\n")
        stmts = find_nodes(ast, "def_stmt")
        assert len(stmts) == 1

    def test_def_complex(self) -> None:
        """DEF FNB(T) = SIN(T) / COS(T) — tangent function."""
        ast = parse_dartmouth_basic("10 DEF FNB(T) = SIN(T) / COS(T)\n")
        stmts = find_nodes(ast, "def_stmt")
        assert len(stmts) == 1


# ---------------------------------------------------------------------------
# Expression precedence
# ---------------------------------------------------------------------------


class TestExpressionPrecedence:
    """Tests for arithmetic expression precedence.

    1964 BASIC follows the standard mathematical precedence:
    1. ^ (exponentiation) — highest, right-associative
    2. Unary minus (-)
    3. * and / (multiplication and division) — left-associative
    4. + and - (addition and subtraction) — lowest, left-associative

    Parentheses override precedence as usual.

    Right-associativity of ^
    --------------------------
    In standard mathematics, 2^3^2 = 2^(3^2) = 2^9 = 512, NOT (2^3)^2 = 64.
    The grammar achieves this by making the 'power' rule recurse on its right
    side: power = unary [ CARET power ]. This forces right-to-left grouping
    at the grammar level, with no special annotation needed.
    """

    def test_addition_and_multiplication(self) -> None:
        """2 + 3 * 4 should parse as 2 + (3 * 4) = 14, not (2 + 3) * 4 = 20."""
        ast = parse_dartmouth_basic("10 LET X = 2 + 3 * 4\n")
        stmts = find_nodes(ast, "let_stmt")
        assert len(stmts) == 1
        # There must be an expr node at the top of the expression
        exprs = find_nodes(ast, "expr")
        assert len(exprs) >= 1

    def test_right_assoc_exponentiation(self) -> None:
        """2 ^ 3 ^ 2 should parse as 2 ^ (3 ^ 2) — right-associative."""
        ast = parse_dartmouth_basic("10 LET X = 2 ^ 3 ^ 2\n")
        stmts = find_nodes(ast, "let_stmt")
        assert len(stmts) == 1
        power_nodes = find_nodes(ast, "power")
        # Right-assoc means there are multiple nested power nodes
        assert len(power_nodes) >= 2

    def test_unary_minus(self) -> None:
        """Unary minus: LET X = -Y."""
        ast = parse_dartmouth_basic("10 LET X = -Y\n")
        stmts = find_nodes(ast, "let_stmt")
        assert len(stmts) == 1
        unary_nodes = find_nodes(ast, "unary")
        assert len(unary_nodes) >= 1

    def test_parentheses(self) -> None:
        """Parentheses override default precedence: (2 + 3) * 4."""
        ast = parse_dartmouth_basic("10 LET X = (2 + 3) * 4\n")
        stmts = find_nodes(ast, "let_stmt")
        assert len(stmts) == 1


# ---------------------------------------------------------------------------
# Built-in functions
# ---------------------------------------------------------------------------


class TestBuiltinFunctions:
    """Tests for all 11 built-in mathematical functions.

    Grammar: primary = BUILTIN_FN LPAREN expr RPAREN | ...

    The 1964 Dartmouth BASIC spec defines these built-in functions:
    - SIN(x)  — sine of x (radians)
    - COS(x)  — cosine of x (radians)
    - TAN(x)  — tangent of x (radians)
    - ATN(x)  — arctangent of x (result in radians)
    - EXP(x)  — e raised to the power x (natural exponential)
    - LOG(x)  — natural logarithm of x (base e, not base 10)
    - ABS(x)  — absolute value of x
    - SQR(x)  — square root of x
    - INT(x)  — floor integer function (rounds toward negative infinity)
    - RND(x)  — random number between 0 and 1 (x is a dummy argument)
    - SGN(x)  — sign of x: -1, 0, or +1

    Each is called with a single parenthesised argument expression.
    """

    @pytest.mark.parametrize("fn", [
        "SIN", "COS", "TAN", "ATN", "EXP", "LOG",
        "ABS", "SQR", "INT", "RND", "SGN",
    ])
    def test_builtin_function(self, fn: str) -> None:
        """Each built-in function parses as part of an expression."""
        source = f"10 LET Y = {fn}(X)\n"
        ast = parse_dartmouth_basic(source)
        stmts = find_nodes(ast, "let_stmt")
        assert len(stmts) == 1
        primary_nodes = find_nodes(ast, "primary")
        assert len(primary_nodes) >= 1


# ---------------------------------------------------------------------------
# User-defined functions
# ---------------------------------------------------------------------------


class TestUserFunctions:
    """Tests for user-defined function calls.

    Grammar: primary = USER_FN LPAREN expr RPAREN | ...

    After DEF FNA(X) = ..., the function is called as FNA(value). User
    function names are exactly three characters: FN followed by A-Z.
    """

    def test_user_function_call(self) -> None:
        """LET Y = FNA(X) — call user-defined function FNA."""
        ast = parse_dartmouth_basic("10 LET Y = FNA(X)\n")
        stmts = find_nodes(ast, "let_stmt")
        assert len(stmts) == 1

    def test_user_function_with_expr(self) -> None:
        """User function called with an expression argument."""
        ast = parse_dartmouth_basic("10 LET Y = FNB(X + 1)\n")
        stmts = find_nodes(ast, "let_stmt")
        assert len(stmts) == 1


# ---------------------------------------------------------------------------
# Array subscript
# ---------------------------------------------------------------------------


class TestArraySubscript:
    """Tests for array element access.

    Grammar: variable = NAME LPAREN expr RPAREN | NAME ;

    Arrays in 1964 BASIC are single-dimensional. The subscript is an arbitrary
    expression (unlike DIM, where the size must be a literal). Arrays are
    automatically declared with indices 0–10; DIM allows larger ranges.
    """

    def test_array_read(self) -> None:
        """LET X = A(3) — read array element at index 3."""
        ast = parse_dartmouth_basic("10 LET X = A(3)\n")
        stmts = find_nodes(ast, "let_stmt")
        assert len(stmts) == 1

    def test_array_with_variable_index(self) -> None:
        """LET X = A(I) — array indexed by a variable."""
        ast = parse_dartmouth_basic("10 LET X = A(I)\n")
        stmts = find_nodes(ast, "let_stmt")
        assert len(stmts) == 1

    def test_array_write(self) -> None:
        """LET A(5) = 42 — write to array element at index 5."""
        ast = parse_dartmouth_basic("10 LET A(5) = 42\n")
        stmts = find_nodes(ast, "let_stmt")
        assert len(stmts) == 1


# ---------------------------------------------------------------------------
# Multi-line programs
# ---------------------------------------------------------------------------


class TestMultiLinePrograms:
    """Tests for complete multi-line BASIC programs.

    These tests exercise the grammar's handling of multiple lines and the
    interaction between different statement types.
    """

    def test_hello_world(self) -> None:
        """The classic 'Hello World' in Dartmouth BASIC.

        In 1964, the actual BASIC idiom was PRINT "HELLO WORLD" — no
        parentheses. This was simpler than most modern print statements.
        """
        source = '10 PRINT "HELLO WORLD"\n20 END\n'
        ast = parse_dartmouth_basic(source)
        assert ast.rule_name == "program"
        lines = find_nodes(ast, "line")
        assert len(lines) == 2

    def test_counting_loop(self) -> None:
        """A FOR...NEXT loop that prints 1 through 10."""
        source = (
            "10 FOR I = 1 TO 10\n"
            "20 PRINT I\n"
            "30 NEXT I\n"
            "40 END\n"
        )
        ast = parse_dartmouth_basic(source)
        lines = find_nodes(ast, "line")
        assert len(lines) == 4
        for_stmts = find_nodes(ast, "for_stmt")
        assert len(for_stmts) == 1
        next_stmts = find_nodes(ast, "next_stmt")
        assert len(next_stmts) == 1

    def test_conditional_program(self) -> None:
        """A program that uses IF...THEN for a conditional check."""
        source = (
            "10 LET X = 5\n"
            "20 IF X > 3 THEN 50\n"
            "30 PRINT 0\n"
            "40 GOTO 60\n"
            "50 PRINT 1\n"
            "60 END\n"
        )
        ast = parse_dartmouth_basic(source)
        lines = find_nodes(ast, "line")
        assert len(lines) == 6

    def test_subroutine_program(self) -> None:
        """A program with a subroutine via GOSUB/RETURN."""
        source = (
            "10 GOSUB 100\n"
            "20 GOSUB 100\n"
            "30 END\n"
            '100 PRINT "IN SUBROUTINE"\n'
            "110 RETURN\n"
        )
        ast = parse_dartmouth_basic(source)
        gosub_stmts = find_nodes(ast, "gosub_stmt")
        return_stmts = find_nodes(ast, "return_stmt")
        assert len(gosub_stmts) == 2
        assert len(return_stmts) == 1

    def test_read_data_program(self) -> None:
        """A program that reads values from a DATA statement."""
        source = (
            "10 READ X\n"
            "20 READ Y\n"
            "30 LET Z = X + Y\n"
            "40 PRINT Z\n"
            "50 DATA 3, 4\n"
            "60 END\n"
        )
        ast = parse_dartmouth_basic(source)
        read_stmts = find_nodes(ast, "read_stmt")
        data_stmts = find_nodes(ast, "data_stmt")
        assert len(read_stmts) == 2
        assert len(data_stmts) == 1


# ---------------------------------------------------------------------------
# Grammar path
# ---------------------------------------------------------------------------


class TestGrammarPath:
    """Tests for the grammar file location constant."""

    def test_grammar_path_exists(self) -> None:
        """The grammar file should exist at the expected path."""
        from dartmouth_basic_parser.parser import DARTMOUTH_BASIC_GRAMMAR_PATH
        assert DARTMOUTH_BASIC_GRAMMAR_PATH.exists(), (
            f"Grammar file not found at {DARTMOUTH_BASIC_GRAMMAR_PATH}"
        )

    def test_grammar_path_is_file(self) -> None:
        """The grammar path should point to a file, not a directory."""
        from dartmouth_basic_parser.parser import DARTMOUTH_BASIC_GRAMMAR_PATH
        assert DARTMOUTH_BASIC_GRAMMAR_PATH.is_file()


# ---------------------------------------------------------------------------
# Error cases
# ---------------------------------------------------------------------------


class TestErrors:
    """Tests for BASIC syntax errors.

    The parser should raise ``GrammarParseError`` for programs that violate
    the grammar. This tests the unhappy path — making sure the parser
    rejects malformed BASIC rather than silently producing a wrong AST.
    """

    def test_missing_equals_in_let(self) -> None:
        """LET without = should raise a GrammarParseError.

        Grammar: let_stmt = "LET" variable EQ expr
        The EQ token is required. Without it, the token stream doesn't match
        the grammar and the parser raises an error.
        """
        with pytest.raises(GrammarParseError):
            parse_dartmouth_basic("10 LET X 5\n")

    def test_missing_then_in_if(self) -> None:
        """IF without THEN should raise a GrammarParseError.

        Grammar: if_stmt = "IF" expr relop expr "THEN" LINE_NUM
        The THEN keyword and target line number are required.
        """
        with pytest.raises(GrammarParseError):
            parse_dartmouth_basic("10 IF X > 0 100\n")

    def test_missing_to_in_for(self) -> None:
        """FOR without TO should raise a GrammarParseError.

        Grammar: for_stmt = "FOR" NAME EQ expr "TO" expr [ "STEP" expr ]
        The TO keyword is required to separate the initial value from the
        final value.
        """
        with pytest.raises(GrammarParseError):
            parse_dartmouth_basic("10 FOR I = 1 10\n")
