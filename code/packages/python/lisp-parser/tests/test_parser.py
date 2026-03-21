"""Tests for the Lisp parser.

These tests verify that the Lisp parser correctly produces ASTs for:
- Atoms: numbers, symbols, strings
- Lists: simple, nested, empty
- Quoted forms: 'x, '(1 2 3)
- Dotted pairs: (a . b)
- Multiple top-level expressions
- The factorial definition
"""

from __future__ import annotations

from lisp_parser import parse_lisp


def _get_children(node: object) -> list[object]:
    """Get children of an AST node, handling both ASTNode and Token."""
    if hasattr(node, "children"):
        return node.children
    return []


def _get_rule(node: object) -> str | None:
    """Get the rule name of an AST node."""
    if hasattr(node, "rule_name"):
        return node.rule_name
    return None


def _get_token_type(node: object) -> str | None:
    """Get the token type if this is a terminal."""
    if hasattr(node, "type"):
        return node.type
    return None


def _get_token_value(node: object) -> str | None:
    """Get the token value if this is a terminal."""
    if hasattr(node, "value"):
        return node.value
    return None


def _find_atoms(node: object) -> list[str]:
    """Recursively find all atom values in an AST."""
    results = []
    if _get_rule(node) == "atom":
        children = _get_children(node)
        if children:
            val = _get_token_value(children[0])
            if val is not None:
                results.append(val)
    for child in _get_children(node):
        results.extend(_find_atoms(child))
    return results


def _count_rule(node: object, rule_name: str) -> int:
    """Count how many times a rule appears in the AST."""
    count = 1 if _get_rule(node) == rule_name else 0
    for child in _get_children(node):
        count += _count_rule(child, rule_name)
    return count


# -------------------------------------------------------------------------
# Basic structure
# -------------------------------------------------------------------------


class TestBasicStructure:
    """Tests for the top-level AST structure."""

    def test_program_root(self) -> None:
        """The root node should be 'program'."""
        ast = parse_lisp("42")
        assert _get_rule(ast) == "program"

    def test_empty_program(self) -> None:
        """Empty input should parse as a program with no children."""
        ast = parse_lisp("")
        assert _get_rule(ast) == "program"
        # Program may have 0 sexpr children
        sexprs = [c for c in _get_children(ast) if _get_rule(c) == "sexpr"]
        assert len(sexprs) == 0

    def test_multiple_top_level(self) -> None:
        """Multiple top-level expressions should be separate sexpr children."""
        ast = parse_lisp("1 2 3")
        sexprs = [c for c in _get_children(ast) if _get_rule(c) == "sexpr"]
        assert len(sexprs) == 3


# -------------------------------------------------------------------------
# Atoms
# -------------------------------------------------------------------------


class TestAtoms:
    """Tests for parsing atomic values."""

    def test_number(self) -> None:
        """A number should parse as sexpr > atom > NUMBER."""
        ast = parse_lisp("42")
        atoms = _find_atoms(ast)
        assert atoms == ["42"]

    def test_negative_number(self) -> None:
        """A negative number should parse correctly."""
        ast = parse_lisp("-7")
        atoms = _find_atoms(ast)
        assert atoms == ["-7"]

    def test_symbol(self) -> None:
        """A symbol should parse as sexpr > atom > SYMBOL."""
        ast = parse_lisp("define")
        atoms = _find_atoms(ast)
        assert atoms == ["define"]

    def test_operator_symbol(self) -> None:
        """Operator symbols should parse correctly."""
        ast = parse_lisp("+")
        atoms = _find_atoms(ast)
        assert atoms == ["+"]

    def test_string(self) -> None:
        """A string should parse as sexpr > atom > STRING."""
        ast = parse_lisp('"hello"')
        atoms = _find_atoms(ast)
        assert len(atoms) == 1


# -------------------------------------------------------------------------
# Lists
# -------------------------------------------------------------------------


class TestLists:
    """Tests for parsing list expressions."""

    def test_empty_list(self) -> None:
        """An empty list () should parse correctly."""
        ast = parse_lisp("()")
        assert _count_rule(ast, "list") == 1

    def test_simple_list(self) -> None:
        """A simple list should contain its elements."""
        ast = parse_lisp("(1 2 3)")
        atoms = _find_atoms(ast)
        assert atoms == ["1", "2", "3"]

    def test_nested_list(self) -> None:
        """Nested lists should parse correctly."""
        ast = parse_lisp("((1 2) (3 4))")
        assert _count_rule(ast, "list") == 3  # outer + 2 inner

    def test_function_call(self) -> None:
        """A function call (+ 1 2) should parse correctly."""
        ast = parse_lisp("(+ 1 2)")
        atoms = _find_atoms(ast)
        assert atoms == ["+", "1", "2"]

    def test_define(self) -> None:
        """A define expression should parse correctly."""
        ast = parse_lisp("(define x 42)")
        atoms = _find_atoms(ast)
        assert atoms == ["define", "x", "42"]

    def test_deeply_nested(self) -> None:
        """Deeply nested lists should parse correctly."""
        ast = parse_lisp("(+ (* 2 3) (- 10 4))")
        atoms = _find_atoms(ast)
        assert atoms == ["+", "*", "2", "3", "-", "10", "4"]


# -------------------------------------------------------------------------
# Quoted forms
# -------------------------------------------------------------------------


class TestQuoted:
    """Tests for parsing quoted forms."""

    def test_quoted_symbol(self) -> None:
        """'foo should parse as a quoted form."""
        ast = parse_lisp("'foo")
        assert _count_rule(ast, "quoted") == 1
        atoms = _find_atoms(ast)
        assert atoms == ["foo"]

    def test_quoted_list(self) -> None:
        """'(1 2 3) should parse as a quoted list."""
        ast = parse_lisp("'(1 2 3)")
        assert _count_rule(ast, "quoted") == 1
        atoms = _find_atoms(ast)
        assert atoms == ["1", "2", "3"]

    def test_quoted_in_expression(self) -> None:
        """Quoted forms should work inside expressions."""
        ast = parse_lisp("(eq 'foo 'bar)")
        assert _count_rule(ast, "quoted") == 2


# -------------------------------------------------------------------------
# Dotted pairs
# -------------------------------------------------------------------------


class TestDottedPairs:
    """Tests for parsing dotted pair notation."""

    def test_simple_dotted_pair(self) -> None:
        """(a . b) should parse correctly."""
        ast = parse_lisp("(a . b)")
        atoms = _find_atoms(ast)
        assert atoms == ["a", "b"]

    def test_numeric_dotted_pair(self) -> None:
        """(1 . 2) should parse correctly."""
        ast = parse_lisp("(1 . 2)")
        atoms = _find_atoms(ast)
        assert atoms == ["1", "2"]


# -------------------------------------------------------------------------
# Complex expressions
# -------------------------------------------------------------------------


class TestComplexExpressions:
    """Tests for parsing real Lisp code."""

    def test_lambda(self) -> None:
        """A lambda expression should parse correctly."""
        ast = parse_lisp("(lambda (x) (* x x))")
        atoms = _find_atoms(ast)
        assert "lambda" in atoms
        assert "x" in atoms
        assert "*" in atoms

    def test_cond(self) -> None:
        """A cond expression should parse correctly."""
        ast = parse_lisp("(cond ((eq x 0) 1) (t x))")
        atoms = _find_atoms(ast)
        assert "cond" in atoms
        assert "eq" in atoms
        assert "t" in atoms

    def test_factorial(self) -> None:
        """The factorial definition should parse without errors."""
        source = """
        (define factorial
          (lambda (n)
            (cond ((eq n 0) 1)
                  (t (* n (factorial (- n 1)))))))
        """
        ast = parse_lisp(source)
        assert _get_rule(ast) == "program"
        atoms = _find_atoms(ast)
        assert "define" in atoms
        assert "factorial" in atoms
        assert "lambda" in atoms
        assert "cond" in atoms

    def test_multiple_definitions(self) -> None:
        """Multiple definitions should all parse."""
        source = """
        (define x 10)
        (define y 20)
        (+ x y)
        """
        ast = parse_lisp(source)
        sexprs = [c for c in _get_children(ast) if _get_rule(c) == "sexpr"]
        assert len(sexprs) == 3

    def test_cons_car_cdr(self) -> None:
        """cons/car/cdr expressions should parse correctly."""
        ast = parse_lisp("(car (cons 1 2))")
        atoms = _find_atoms(ast)
        assert atoms == ["car", "cons", "1", "2"]
