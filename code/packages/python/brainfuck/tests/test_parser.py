"""Tests for the Brainfuck parser thin wrapper.

These tests verify that the grammar-driven parser, configured with
``brainfuck.grammar``, correctly parses Brainfuck source text into ASTs.

The Brainfuck grammar's top-level rule is ``program`` — any Brainfuck source
file is a single program consisting of zero or more instructions.

Test Strategy
-------------

Each test parses a Brainfuck string and then uses helper functions to walk the
resulting AST, looking for specific node types and tokens. This approach is
robust against changes in how the grammar wraps nodes.

Test Categories
---------------

1. Empty program — empty source produces a "program" root with no instructions
2. Simple commands — flat sequences of commands (no loops)
3. Simple loops — a single loop with a body
4. Nested loops — loops inside loops
5. Unmatched brackets — should raise an exception
6. Canonical ``++[>+<-]`` — verify the exact AST structure
7. Comments stripped — comment text does not appear in AST
8. All 6 non-bracket commands appear in the AST
"""

from __future__ import annotations

import pytest

from brainfuck.parser import create_brainfuck_parser, parse_brainfuck
from lang_parser import ASTNode, is_ast_node


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def find_nodes(node: ASTNode, rule_name: str) -> list[ASTNode]:
    """Recursively find all AST nodes with a given rule name.

    This is the core helper for tree inspection. Because the grammar wraps
    commands in multiple layers (program -> instruction -> command), we need
    to search the entire tree to count nodes of a given type.

    Args:
        node: The root node to search from.
        rule_name: The grammar rule name to look for (e.g., "loop", "command").

    Returns:
        A list of all ``ASTNode`` instances in the tree with the given
        ``rule_name``.
    """
    results: list[ASTNode] = []
    if node.rule_name == rule_name:
        results.append(node)
    for child in node.children:
        if is_ast_node(child):
            results.extend(find_nodes(child, rule_name))
    return results


def find_tokens(node: ASTNode) -> list:
    """Collect all leaf tokens from an AST subtree.

    Flattens the tree into a list of tokens. This makes it easy to check
    which token types appear in a subtree without worrying about nesting.

    Args:
        node: The root node to flatten.

    Returns:
        A list of all token objects that are leaves (non-ASTNode children)
        in the subtree.
    """
    tokens: list = []
    for child in node.children:
        if is_ast_node(child):
            tokens.extend(find_tokens(child))
        else:
            tokens.append(child)
    return tokens


def token_type(token: object) -> str:
    """Extract the type name from a token as a string.

    Handles both string-typed and enum-typed token types.
    """
    t = getattr(token, "type", None)
    if isinstance(t, str):
        return t
    return t.name  # type: ignore[union-attr]


# ---------------------------------------------------------------------------
# Empty program
# ---------------------------------------------------------------------------


class TestEmptyProgram:
    """Verify behaviour for empty and comment-only sources."""

    def test_empty_source_produces_program_root(self) -> None:
        """Parsing ``""`` produces an AST with rule_name ``"program"``.

        The grammar rule ``program = { instruction }`` allows zero instructions,
        so an empty source is a valid Brainfuck program.
        """
        ast = parse_brainfuck("")
        assert ast.rule_name == "program"

    def test_empty_source_has_no_instruction_nodes(self) -> None:
        """An empty program has no instruction, loop, or command nodes."""
        ast = parse_brainfuck("")
        assert find_nodes(ast, "instruction") == []
        assert find_nodes(ast, "loop") == []
        assert find_nodes(ast, "command") == []

    def test_comment_only_source_is_empty_program(self) -> None:
        """A source with only comment text parses as an empty program.

        After lexing, the token stream is just EOF (all text is discarded).
        The parser sees the same input as for ``""``.
        """
        ast = parse_brainfuck("this is a comment with no commands")
        assert ast.rule_name == "program"
        assert find_nodes(ast, "command") == []


# ---------------------------------------------------------------------------
# Simple commands
# ---------------------------------------------------------------------------


class TestSimpleCommands:
    """Verify parsing of flat command sequences (no loops)."""

    def test_single_inc_command(self) -> None:
        """Parsing ``"+"`` produces one command node containing an INC token.

        The full tree is: program -> instruction -> command -> Token(INC, '+').
        """
        ast = parse_brainfuck("+")
        assert ast.rule_name == "program"

        command_nodes = find_nodes(ast, "command")
        assert len(command_nodes) == 1

        tokens = find_tokens(ast)
        inc_tokens = [t for t in tokens if token_type(t) == "INC"]
        assert len(inc_tokens) == 1

    def test_two_inc_commands(self) -> None:
        """Parsing ``"++"`` produces two command nodes."""
        ast = parse_brainfuck("++")
        command_nodes = find_nodes(ast, "command")
        assert len(command_nodes) == 2

    def test_all_six_non_bracket_commands(self) -> None:
        """``"><+-.,``" produces 6 command nodes, one per non-bracket command.

        LOOP_START (``[``) and LOOP_END (``]``) are handled by the ``loop``
        rule, not the ``command`` rule. So there are exactly 6 command types.
        """
        ast = parse_brainfuck("><+-.,")

        command_nodes = find_nodes(ast, "command")
        assert len(command_nodes) == 6

        tokens = find_tokens(ast)
        types = [token_type(t) for t in tokens if token_type(t) != "EOF"]

        assert "RIGHT" in types
        assert "LEFT" in types
        assert "INC" in types
        assert "DEC" in types
        assert "OUTPUT" in types
        assert "INPUT" in types

    def test_ten_consecutive_inc_commands(self) -> None:
        """``"++++++++++``" (10 ``+`` characters) produces 10 command nodes.

        This verifies that the ``{ instruction }`` repetition in the grammar
        handles long sequences correctly.
        """
        ast = parse_brainfuck("+" * 10)
        command_nodes = find_nodes(ast, "command")
        assert len(command_nodes) == 10


# ---------------------------------------------------------------------------
# Simple loops
# ---------------------------------------------------------------------------


class TestSimpleLoops:
    """Verify parsing of single-level loops."""

    def test_empty_loop(self) -> None:
        """Parsing ``"[]"`` produces one loop node with no instruction children.

        ``[]`` is a legal Brainfuck construct: if the current cell is nonzero,
        it loops forever (usually a bug). If the cell is zero, the ``[``
        immediately jumps past ``]`` — a no-op.
        """
        ast = parse_brainfuck("[]")
        loop_nodes = find_nodes(ast, "loop")
        assert len(loop_nodes) == 1

    def test_clear_cell_loop(self) -> None:
        """Parsing ``"[-]"`` produces one loop node containing a DEC command.

        ``[-]`` is the idiomatic Brainfuck clear-cell loop. It decrements the
        current cell once per iteration until it reaches zero.
        """
        ast = parse_brainfuck("[-]")

        loop_nodes = find_nodes(ast, "loop")
        assert len(loop_nodes) == 1

        tokens = find_tokens(ast)
        types = [token_type(t) for t in tokens]
        assert "LOOP_START" in types
        assert "LOOP_END" in types
        assert "DEC" in types

    def test_copy_loop_body_has_four_commands(self) -> None:
        """Parsing ``"[>+<-]"`` produces one loop with 4 body commands.

        ``[>+<-]`` copies the value from cell 0 to cell 1 while decrementing
        cell 0 to zero. The body has 4 commands: RIGHT, INC, LEFT, DEC.
        """
        ast = parse_brainfuck("[>+<-]")

        loop_nodes = find_nodes(ast, "loop")
        assert len(loop_nodes) == 1

        command_nodes = find_nodes(ast, "command")
        assert len(command_nodes) == 4  # >, +, <, -


# ---------------------------------------------------------------------------
# Nested loops
# ---------------------------------------------------------------------------


class TestNestedLoops:
    """Verify parsing of nested loop structures."""

    def test_two_sequential_loops(self) -> None:
        """Parsing ``"[-][+]"`` produces 2 loop nodes at the top level.

        Sequential loops are not nested — they are siblings in the program's
        instruction list. The parser should find exactly 2 loop nodes.
        """
        ast = parse_brainfuck("[-][+]")
        loop_nodes = find_nodes(ast, "loop")
        assert len(loop_nodes) == 2

    def test_trivially_nested_loop(self) -> None:
        """Parsing ``"[[]]"`` produces 2 loop nodes: outer and inner.

        The inner loop body is empty. The parser must handle nesting because
        the ``loop`` rule recurses: ``loop = LOOP_START { instruction } LOOP_END``
        and ``instruction = loop | command``.
        """
        ast = parse_brainfuck("[[]]")
        loop_nodes = find_nodes(ast, "loop")
        assert len(loop_nodes) == 2

    def test_deeply_nested_loop(self) -> None:
        """Parsing ``"[[[-]]]"`` produces 3 loop nodes at 3 levels of nesting.

        Three levels: outer -> middle -> inner (the ``[-]`` clear-cell idiom).
        """
        ast = parse_brainfuck("[[[-]]]")
        loop_nodes = find_nodes(ast, "loop")
        assert len(loop_nodes) == 3

    def test_multiply_loop(self) -> None:
        """Parsing ``"[->+>+<<]"`` produces 1 loop with 7 body commands.

        ``[->+>+<<]`` is a common multiplication helper that copies the source
        cell's value into two destination cells. The body has 7 commands:
        -, >, +, >, +, <, <.
        """
        ast = parse_brainfuck("[->+>+<<]")

        loop_nodes = find_nodes(ast, "loop")
        assert len(loop_nodes) == 1

        command_nodes = find_nodes(ast, "command")
        assert len(command_nodes) == 7


# ---------------------------------------------------------------------------
# Unmatched brackets
# ---------------------------------------------------------------------------


class TestUnmatchedBrackets:
    """Verify that unmatched brackets raise parse errors."""

    def test_unmatched_open_bracket_raises(self) -> None:
        """``"[+"`` has an opening bracket with no matching ``]``.

        The ``loop`` grammar rule requires LOOP_START { instruction } LOOP_END,
        so the missing LOOP_END causes a parse error at parse time — before
        the program ever runs.
        """
        with pytest.raises(Exception):
            parse_brainfuck("[+")

    def test_unmatched_close_bracket_raises(self) -> None:
        """``"+]"`` has a closing bracket with no matching ``[``.

        The grammar's ``instruction`` rule produces ``]`` tokens only inside
        a loop body. A standalone ``]`` in the top-level instruction stream
        cannot be matched by any grammar rule.
        """
        with pytest.raises(Exception):
            parse_brainfuck("+]")


# ---------------------------------------------------------------------------
# Canonical ++[>+<-] pattern
# ---------------------------------------------------------------------------


class TestCanonicalPattern:
    """Tests for the canonical ``++[>+<-]`` Brainfuck idiom."""

    def test_root_rule_name_is_program(self) -> None:
        """Parsing ``"++[>+<-]"`` produces an AST rooted at ``"program"``."""
        ast = parse_brainfuck("++[>+<-]")
        assert ast.rule_name == "program"

    def test_two_top_level_inc_commands_plus_one_loop(self) -> None:
        """``"++[>+<-]"`` has 2 top-level commands (INC, INC) and 1 loop.

        The full structure:
            program
              instruction -> command -> INC
              instruction -> command -> INC
              instruction -> loop -> [>, +, <, -]

        Total: 1 loop node, 6 command nodes (2 outer + 4 in loop body).
        """
        ast = parse_brainfuck("++[>+<-]")

        loop_nodes = find_nodes(ast, "loop")
        assert len(loop_nodes) == 1

        command_nodes = find_nodes(ast, "command")
        assert len(command_nodes) == 6  # ++ outside + >+<- inside

    def test_loop_body_contains_correct_token_types(self) -> None:
        """The loop in ``"++[>+<-]"`` contains the correct command tokens."""
        ast = parse_brainfuck("++[>+<-]")

        loop_nodes = find_nodes(ast, "loop")
        loop_tokens = find_tokens(loop_nodes[0])
        loop_types = [token_type(t) for t in loop_tokens]

        assert "LOOP_START" in loop_types
        assert "RIGHT" in loop_types
        assert "INC" in loop_types
        assert "LEFT" in loop_types
        assert "DEC" in loop_types
        assert "LOOP_END" in loop_types

    def test_comments_do_not_change_ast_structure(self) -> None:
        """Adding comments to ``"++[>+<-]"`` does not change the AST.

        The lexer discards all comment text. The parser sees the same
        token stream whether or not comments are present.
        """
        clean_ast = parse_brainfuck("++[>+<-]")
        commented_ast = parse_brainfuck("++ setup  [ copy loop >+ cell1 <-  ]")

        clean_commands = len(find_nodes(clean_ast, "command"))
        commented_commands = len(find_nodes(commented_ast, "command"))
        assert clean_commands == commented_commands

        clean_loops = len(find_nodes(clean_ast, "loop"))
        commented_loops = len(find_nodes(commented_ast, "loop"))
        assert clean_loops == commented_loops


# ---------------------------------------------------------------------------
# create_brainfuck_parser factory
# ---------------------------------------------------------------------------


class TestCreateBrainfuckParser:
    """Verify that the ``create_brainfuck_parser`` factory function works."""

    def test_factory_returns_grammar_parser(self) -> None:
        """``create_brainfuck_parser`` returns a ``GrammarParser`` instance."""
        from lang_parser import GrammarParser

        parser = create_brainfuck_parser("+")
        assert isinstance(parser, GrammarParser)

    def test_factory_parse_matches_convenience_function(self) -> None:
        """``create_brainfuck_parser(s).parse()`` equals ``parse_brainfuck(s)``.

        Both code paths should produce ASTs with the same structure.
        """
        source = "++[>+<-]"

        via_factory = create_brainfuck_parser(source).parse()
        via_convenience = parse_brainfuck(source)

        factory_commands = len(find_nodes(via_factory, "command"))
        convenience_commands = len(find_nodes(via_convenience, "command"))
        assert factory_commands == convenience_commands
