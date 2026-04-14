"""Tests for the VHDL Parser.

These tests verify that the grammar-driven parser, when loaded with the
``vhdl.grammar`` file, correctly parses VHDL source code into ASTs.

VHDL describes hardware, not software. Each test exercises a different
hardware description construct:

- **Entities** define the interface (ports) of a hardware block.
- **Architectures** define the implementation.
- **Signals** represent physical wires; assigned with ``<=``.
- **Variables** exist inside processes; assigned with ``:=``.
- **Processes** are sequential regions in the concurrent world.
- **If/elsif/else** and **case/when** control flow inside processes.
- **Component instantiation** connects modules via port maps.
- **Expressions** use keyword operators (``and``, ``or``, ``xor``).

VHDL vs Verilog Key Differences
--------------------------------

+-------------------+--------------------+
| VHDL              | Verilog            |
+-------------------+--------------------+
| entity            | module (interface) |
| architecture      | module (body)      |
| signal ``<=``     | non-blocking ``<=``|
| variable ``:=``   | blocking ``=``     |
| process           | always block       |
| port map          | instance ports     |
| ``and``/``or``    | ``&``/``|``        |
+-------------------+--------------------+

Token Type Handling
-------------------

Token types from the lexer can be either ``TokenType`` enum values (with a
``.name`` attribute) or plain strings. When checking token types, we use::

    t.type.name if hasattr(t.type, "name") else t.type

This ensures compatibility regardless of which form the lexer produces.
"""

from __future__ import annotations

from lang_parser import ASTNode
from lexer import Token

from vhdl_parser import create_vhdl_parser, parse_vhdl


# ============================================================================
# Helpers
# ============================================================================


def _token_type_name(token: Token) -> str:
    """Get the type name of a token, handling both enum and string types.

    The lexer may produce tokens whose ``.type`` is either a ``TokenType``
    enum value (with a ``.name`` attribute) or a plain string. This helper
    normalizes both forms to a string for comparison.
    """
    return token.type.name if hasattr(token.type, "name") else token.type


def find_nodes(node: ASTNode, rule_name: str) -> list[ASTNode]:
    """Recursively find all descendant nodes with the given rule name.

    This is the AST equivalent of "find all elements matching a CSS selector."
    It performs a depth-first search through the tree, collecting every node
    whose ``rule_name`` matches.

    Args:
        node: The root node to search from.
        rule_name: The grammar rule name to search for.

    Returns:
        A list of matching ``ASTNode`` objects, in depth-first order.
    """
    results: list[ASTNode] = []
    if node.rule_name == rule_name:
        results.append(node)
    for child in node.children:
        if isinstance(child, ASTNode):
            results.extend(find_nodes(child, rule_name))
    return results


def find_tokens(node: ASTNode) -> list[Token]:
    """Recursively collect all Token leaves from an AST.

    Tokens are the leaves of the AST tree — the actual pieces of source code
    (keywords, identifiers, operators, literals). This function collects them
    all in order, which is useful for verifying that the parser consumed the
    right tokens.

    Args:
        node: The root node to collect tokens from.

    Returns:
        A list of ``Token`` objects in left-to-right source order.
    """
    tokens: list[Token] = []
    for child in node.children:
        if isinstance(child, Token):
            tokens.append(child)
        elif isinstance(child, ASTNode):
            tokens.extend(find_tokens(child))
    return tokens


def find_tokens_by_type(node: ASTNode, type_name: str) -> list[Token]:
    """Find all tokens in an AST with the given type name.

    Args:
        node: The root node to search.
        type_name: The token type name (e.g., "KEYWORD", "NAME").

    Returns:
        A list of matching tokens.
    """
    return [t for t in find_tokens(node) if _token_type_name(t) == type_name]


# ============================================================================
# Test: Empty Entity
# ============================================================================


class TestEmptyEntity:
    """Test parsing of the simplest possible VHDL entity.

    An empty entity is the "hello world" of VHDL::

        entity e is end entity e;

    It has no ports, no generics — just a name. This is the minimal valid
    VHDL design unit. In hardware terms, it describes a component with no
    pins and no internal logic.

    Note how VHDL requires the entity name to appear twice: once after
    ``entity`` and optionally again after ``end entity``. This redundancy
    is intentional — it helps catch errors in large files where the end
    of a block might be hundreds of lines from its beginning.
    """

    def test_empty_entity_parses(self) -> None:
        """Parse ``entity e is end entity e;`` — the simplest VHDL entity."""
        ast = parse_vhdl("entity e is end entity e;")
        assert ast.rule_name == "design_file"

        # The AST should contain exactly one entity_declaration.
        entities = find_nodes(ast, "entity_declaration")
        assert len(entities) == 1


class TestVersions:
    """Version-selection behaviour for compiled VHDL grammars."""

    def test_default_version_matches_explicit_2008(self) -> None:
        default_ast = parse_vhdl("entity e is end entity e;")
        explicit_ast = parse_vhdl("entity e is end entity e;", version="2008")
        assert default_ast.rule_name == explicit_ast.rule_name

    def test_rejects_unknown_version(self) -> None:
        try:
            parse_vhdl("entity e is end entity e;", version="2099")
        except ValueError as exc:
            assert "Unknown VHDL version" in str(exc)
        else:
            raise AssertionError("Expected ValueError for unknown VHDL version")

    def test_empty_entity_has_correct_name(self) -> None:
        """The entity name should be captured as a NAME token."""
        ast = parse_vhdl("entity e is end entity e;")
        entities = find_nodes(ast, "entity_declaration")
        tokens = find_tokens(entities[0])

        names = [t for t in tokens if _token_type_name(t) == "NAME"]
        assert len(names) >= 1
        assert names[0].value == "e"

    def test_empty_entity_keywords(self) -> None:
        """The entity should have 'entity', 'is', and 'end' keywords."""
        ast = parse_vhdl("entity e is end entity e;")
        entities = find_nodes(ast, "entity_declaration")
        tokens = find_tokens(entities[0])

        keywords = [t for t in tokens if _token_type_name(t) == "KEYWORD"]
        keyword_values = [k.value for k in keywords]
        assert "entity" in keyword_values
        assert "is" in keyword_values
        assert "end" in keyword_values

    def test_empty_entity_without_trailing_name(self) -> None:
        """Parse ``entity e is end entity;`` — trailing name is optional."""
        ast = parse_vhdl("entity e is end entity;")
        entities = find_nodes(ast, "entity_declaration")
        assert len(entities) == 1

    def test_empty_entity_without_entity_keyword_at_end(self) -> None:
        """Parse ``entity e is end;`` — 'entity' after 'end' is optional."""
        ast = parse_vhdl("entity e is end;")
        entities = find_nodes(ast, "entity_declaration")
        assert len(entities) == 1


# ============================================================================
# Test: Entity with Ports
# ============================================================================


class TestEntityWithPorts:
    """Test parsing of entities with port declarations.

    Ports are the entity's interface to the outside world — like pins on a
    chip. Every signal entering or leaving must be declared in the port clause
    with its direction (in, out, inout, buffer) and type.

    Example::

        entity and_gate is
            port(a, b : in std_logic; y : out std_logic);
        end entity and_gate;

    This describes a component with two input pins (a, b) and one output
    pin (y), all of type ``std_logic`` (the 9-valued logic type from IEEE).
    """

    def test_entity_with_ports(self) -> None:
        """Parse an entity with input and output ports."""
        ast = parse_vhdl("""
            entity and_gate is
                port(a, b : in std_logic; y : out std_logic);
            end entity and_gate;
        """)
        entities = find_nodes(ast, "entity_declaration")
        assert len(entities) == 1

        # Should have a port_clause node.
        port_clauses = find_nodes(entities[0], "port_clause")
        assert len(port_clauses) == 1

    def test_port_names_captured(self) -> None:
        """Port names (a, b, y) should appear as NAME tokens in the port clause."""
        ast = parse_vhdl("""
            entity and_gate is
                port(a, b : in std_logic; y : out std_logic);
            end entity and_gate;
        """)
        entities = find_nodes(ast, "entity_declaration")
        port_clauses = find_nodes(entities[0], "port_clause")
        tokens = find_tokens(port_clauses[0])

        names = [t for t in tokens if _token_type_name(t) == "NAME"]
        name_values = [n.value for n in names]
        assert "a" in name_values
        assert "b" in name_values
        assert "y" in name_values

    def test_port_modes(self) -> None:
        """Port modes (in, out) should appear as keywords in the interface."""
        ast = parse_vhdl("""
            entity and_gate is
                port(a, b : in std_logic; y : out std_logic);
            end entity and_gate;
        """)
        entities = find_nodes(ast, "entity_declaration")
        port_clauses = find_nodes(entities[0], "port_clause")
        modes = find_nodes(port_clauses[0], "mode")

        # Two interface_elements: "a, b : in std_logic" and "y : out std_logic"
        assert len(modes) == 2


# ============================================================================
# Test: Architecture with Signal Assignment
# ============================================================================


class TestArchitectureSignalAssignment:
    """Test parsing of architecture bodies with concurrent signal assignments.

    An architecture defines the implementation of an entity. Concurrent
    signal assignments model combinational logic — circuits whose output
    is always a function of the current inputs, with no memory.

    Example::

        architecture rtl of and_gate is
        begin
            y <= a and b;
        end architecture rtl;

    This implements the and_gate entity: output ``y`` is continuously
    driven by ``a AND b``. The ``<=`` operator is signal assignment
    (not "less than or equal" — context disambiguates).
    """

    def test_architecture_parses(self) -> None:
        """Parse a simple architecture with one signal assignment."""
        ast = parse_vhdl("""
            entity e is
                port(a, b : in std_logic; y : out std_logic);
            end entity e;

            architecture rtl of e is
            begin
                y <= a and b;
            end architecture rtl;
        """)
        archs = find_nodes(ast, "architecture_body")
        assert len(archs) == 1

    def test_architecture_has_signal_assignment(self) -> None:
        """The architecture should contain a concurrent signal assignment."""
        ast = parse_vhdl("""
            entity e is
                port(a, b : in std_logic; y : out std_logic);
            end entity e;

            architecture rtl of e is
            begin
                y <= a and b;
            end architecture rtl;
        """)
        archs = find_nodes(ast, "architecture_body")
        assignments = find_nodes(archs[0], "signal_assignment_concurrent")
        assert len(assignments) == 1

    def test_architecture_name(self) -> None:
        """The architecture name ('rtl') and entity name should be captured."""
        ast = parse_vhdl("""
            entity e is end entity e;

            architecture rtl of e is
            begin
            end architecture rtl;
        """)
        archs = find_nodes(ast, "architecture_body")
        tokens = find_tokens(archs[0])
        names = [t for t in tokens if _token_type_name(t) == "NAME"]

        # First NAME is the architecture name, second is the entity name.
        assert names[0].value == "rtl"
        assert names[1].value == "e"


# ============================================================================
# Test: Process with Sensitivity List
# ============================================================================


class TestProcessStatement:
    """Test parsing of process statements.

    A process is a sequential region inside the concurrent world. Inside
    a process, statements execute top to bottom (like software). But the
    process itself is concurrent with everything outside it.

    The sensitivity list specifies which signals trigger the process:
      - ``process (clk)`` — re-evaluate when clk changes (sequential logic)
      - ``process (a, b, sel)`` — re-evaluate when any input changes (comb.)

    Example::

        process (clk)
        begin
            if rising_edge(clk) then
                q <= d;
            end if;
        end process;
    """

    def test_process_parses(self) -> None:
        """Parse a process with a sensitivity list."""
        ast = parse_vhdl("""
            entity e is
                port(a, b, sel : in std_logic; y : out std_logic);
            end entity e;

            architecture rtl of e is
            begin
                process (a, b, sel)
                begin
                    if sel = '1' then
                        y <= a;
                    else
                        y <= b;
                    end if;
                end process;
            end architecture rtl;
        """)
        processes = find_nodes(ast, "process_statement")
        assert len(processes) == 1

    def test_sensitivity_list(self) -> None:
        """The sensitivity list should contain the trigger signal names."""
        ast = parse_vhdl("""
            entity e is
                port(a, b, sel : in std_logic; y : out std_logic);
            end entity e;

            architecture rtl of e is
            begin
                process (a, b, sel)
                begin
                    if sel = '1' then
                        y <= a;
                    else
                        y <= b;
                    end if;
                end process;
            end architecture rtl;
        """)
        sensitivity = find_nodes(ast, "sensitivity_list")
        assert len(sensitivity) == 1

        tokens = find_tokens(sensitivity[0])
        names = [t for t in tokens if _token_type_name(t) == "NAME"]
        name_values = [n.value for n in names]
        assert "a" in name_values
        assert "b" in name_values
        assert "sel" in name_values


# ============================================================================
# Test: If / Elsif / Else
# ============================================================================


class TestIfStatement:
    """Test parsing of if/elsif/else statements inside processes.

    VHDL's if statement is similar to other languages but ends with
    ``end if;`` (two keywords) instead of a closing brace. VHDL uses
    ``elsif`` (one word, no space) rather than ``else if``.

    Example::

        if sel = "00" then
            y <= a;
        elsif sel = "01" then
            y <= b;
        else
            y <= c;
        end if;

    In combinational logic, forgetting the ``else`` branch creates a
    latch (unintended memory element) — a common VHDL synthesis mistake.
    """

    def test_simple_if(self) -> None:
        """Parse a simple if/then/end if statement."""
        ast = parse_vhdl("""
            entity e is
                port(a, en : in std_logic; y : out std_logic);
            end entity e;

            architecture rtl of e is
            begin
                process (a, en)
                begin
                    if en = '1' then
                        y <= a;
                    end if;
                end process;
            end architecture rtl;
        """)
        if_stmts = find_nodes(ast, "if_statement")
        assert len(if_stmts) == 1

    def test_if_else(self) -> None:
        """Parse an if/else statement — synthesizes to a 2-to-1 mux."""
        ast = parse_vhdl("""
            entity e is
                port(a, b, sel : in std_logic; y : out std_logic);
            end entity e;

            architecture rtl of e is
            begin
                process (a, b, sel)
                begin
                    if sel = '1' then
                        y <= a;
                    else
                        y <= b;
                    end if;
                end process;
            end architecture rtl;
        """)
        if_stmts = find_nodes(ast, "if_statement")
        assert len(if_stmts) == 1

        # The if_statement should contain 'else' keyword.
        tokens = find_tokens(if_stmts[0])
        keywords = [t for t in tokens if _token_type_name(t) == "KEYWORD"]
        keyword_values = [k.value for k in keywords]
        assert "if" in keyword_values
        assert "else" in keyword_values

    def test_if_elsif_else(self) -> None:
        """Parse an if/elsif/else chain — synthesizes to a priority encoder.

        VHDL's ``elsif`` is a single keyword (no space). This is different
        from languages that use ``else if`` as two separate keywords.
        """
        ast = parse_vhdl("""
            entity e is
                port(a, b, c, sel1, sel2 : in std_logic; y : out std_logic);
            end entity e;

            architecture rtl of e is
            begin
                process (a, b, c, sel1, sel2)
                begin
                    if sel1 = '1' then
                        y <= a;
                    elsif sel2 = '1' then
                        y <= b;
                    else
                        y <= c;
                    end if;
                end process;
            end architecture rtl;
        """)
        if_stmts = find_nodes(ast, "if_statement")
        assert len(if_stmts) == 1

        # Should have 'elsif' keyword in the token stream.
        tokens = find_tokens(if_stmts[0])
        keywords = [t for t in tokens if _token_type_name(t) == "KEYWORD"]
        keyword_values = [k.value for k in keywords]
        assert "elsif" in keyword_values


# ============================================================================
# Test: Case / When
# ============================================================================


class TestCaseStatement:
    """Test parsing of case/when statements.

    VHDL's case statement uses ``when`` clauses with ``=>`` (fat arrow),
    which is different from Verilog's ``case``/``endcase`` with ``:`` syntax.

    Example::

        case state is
            when IDLE    => next_state <= RUNNING;
            when RUNNING => next_state <= DONE;
            when others  => next_state <= IDLE;
        end case;

    The ``when others`` clause is like Verilog's ``default`` — it must be
    present to cover all possible values (VHDL requires exhaustive cases).
    """

    def test_case_statement(self) -> None:
        """Parse a case/when statement with multiple alternatives."""
        ast = parse_vhdl("""
            entity e is
                port(sel : in std_logic; y : out std_logic);
            end entity e;

            architecture rtl of e is
            begin
                process (sel)
                begin
                    case sel is
                        when '0' => y <= '0';
                        when others => y <= '1';
                    end case;
                end process;
            end architecture rtl;
        """)
        case_stmts = find_nodes(ast, "case_statement")
        assert len(case_stmts) == 1

    def test_case_has_when_clauses(self) -> None:
        """The case statement should have choices (when clauses)."""
        ast = parse_vhdl("""
            entity e is
                port(sel : in std_logic; y : out std_logic);
            end entity e;

            architecture rtl of e is
            begin
                process (sel)
                begin
                    case sel is
                        when '0' => y <= '0';
                        when others => y <= '1';
                    end case;
                end process;
            end architecture rtl;
        """)
        case_stmts = find_nodes(ast, "case_statement")
        tokens = find_tokens(case_stmts[0])
        keywords = [t for t in tokens if _token_type_name(t) == "KEYWORD"]
        keyword_values = [k.value for k in keywords]

        # Should have 'case', 'is', 'when', 'others', 'end' keywords.
        assert "case" in keyword_values
        assert "when" in keyword_values
        assert "others" in keyword_values

    def test_case_with_choices(self) -> None:
        """The case should have choices nodes for pattern matching."""
        ast = parse_vhdl("""
            entity e is
                port(sel : in std_logic; y : out std_logic);
            end entity e;

            architecture rtl of e is
            begin
                process (sel)
                begin
                    case sel is
                        when '0' => y <= '0';
                        when others => y <= '1';
                    end case;
                end process;
            end architecture rtl;
        """)
        choices = find_nodes(ast, "choices")
        # Two when clauses = two choices nodes
        assert len(choices) >= 2


# ============================================================================
# Test: Component Instantiation with Port Map
# ============================================================================


class TestComponentInstantiation:
    """Test parsing of component instantiation with port maps.

    Component instantiation is how you connect hardware components together
    in VHDL. It's the equivalent of placing a chip on a circuit board and
    wiring its pins to other chips.

    VHDL uses ``port map`` with named associations (``=>``)::

        adder0 : full_adder port map (
            a    => x(0),
            b    => y(0),
            cin  => '0',
            sum  => s(0),
            cout => carry(0)
        );

    The ``=>`` arrow connects a formal port (left) to an actual signal (right).
    """

    def test_component_instantiation(self) -> None:
        """Parse a component instantiation with port map."""
        ast = parse_vhdl("""
            entity e is
                port(a, b : in std_logic; y : out std_logic);
            end entity e;

            architecture structural of e is
            begin
                u1 : and_gate port map (a => a, b => b, y => y);
            end architecture structural;
        """)
        instantiations = find_nodes(ast, "component_instantiation")
        assert len(instantiations) == 1

    def test_port_map_associations(self) -> None:
        """The port map should have association elements for each port."""
        ast = parse_vhdl("""
            entity e is
                port(a, b : in std_logic; y : out std_logic);
            end entity e;

            architecture structural of e is
            begin
                u1 : and_gate port map (a => a, b => b, y => y);
            end architecture structural;
        """)
        instantiations = find_nodes(ast, "component_instantiation")
        assoc_lists = find_nodes(instantiations[0], "association_list")
        assert len(assoc_lists) == 1

        # Should have association_element nodes for each port connection.
        assoc_elements = find_nodes(assoc_lists[0], "association_element")
        assert len(assoc_elements) == 3

    def test_instance_name(self) -> None:
        """The instance label (u1) should be the first NAME token."""
        ast = parse_vhdl("""
            entity e is
                port(a, b : in std_logic; y : out std_logic);
            end entity e;

            architecture structural of e is
            begin
                u1 : and_gate port map (a => a, b => b, y => y);
            end architecture structural;
        """)
        instantiations = find_nodes(ast, "component_instantiation")
        tokens = find_tokens(instantiations[0])
        names = [t for t in tokens if _token_type_name(t) == "NAME"]
        # First NAME is the instance label.
        assert names[0].value == "u1"


# ============================================================================
# Test: Variable Assignment (:=)
# ============================================================================


class TestVariableAssignment:
    """Test parsing of variable assignment (``:=``) inside processes.

    Variables exist only inside processes and have immediate-effect
    assignment. This is different from signals (``<=``) where the new
    value takes effect after a delta delay.

    Variables are used for intermediate calculations::

        process (clk)
            variable temp : std_logic;
        begin
            temp := a and b;
            y <= temp;
        end process;

    The ``:=`` operator has no hardware equivalent — it models sequential
    computation within the process. Synthesis tools convert it to
    combinational logic or registers depending on context.
    """

    def test_variable_assignment(self) -> None:
        """Parse a variable assignment inside a process."""
        ast = parse_vhdl("""
            entity e is
                port(a, b : in std_logic; y : out std_logic);
            end entity e;

            architecture rtl of e is
            begin
                process (a, b)
                    variable temp : std_logic;
                begin
                    temp := a;
                    y <= temp;
                end process;
            end architecture rtl;
        """)
        var_assigns = find_nodes(ast, "variable_assignment")
        assert len(var_assigns) >= 1

    def test_variable_assignment_token(self) -> None:
        """The ``:=`` operator should appear as a VAR_ASSIGN token."""
        ast = parse_vhdl("""
            entity e is
                port(a : in std_logic; y : out std_logic);
            end entity e;

            architecture rtl of e is
            begin
                process (a)
                    variable temp : std_logic;
                begin
                    temp := a;
                    y <= temp;
                end process;
            end architecture rtl;
        """)
        var_assigns = find_nodes(ast, "variable_assignment")
        tokens = find_tokens(var_assigns[0])
        var_assign_tokens = [
            t for t in tokens if _token_type_name(t) == "VAR_ASSIGN"
        ]
        assert len(var_assign_tokens) >= 1


# ============================================================================
# Test: Signal Assignment (<=)
# ============================================================================


class TestSignalAssignment:
    """Test parsing of signal assignment (``<=``) inside processes.

    Signal assignment inside a process (sequential signal assignment) looks
    identical to concurrent signal assignment, but behaves differently:
    multiple assignments to the same signal are legal, and only the LAST
    one takes effect.

    Inside a process, ``<=`` is "signal assignment" (not "less than or equal").
    The grammar disambiguates by context: ``<=`` in a statement position is
    assignment; ``<=`` inside an expression is comparison.
    """

    def test_signal_assignment_in_process(self) -> None:
        """Parse a sequential signal assignment inside a process."""
        ast = parse_vhdl("""
            entity e is
                port(a : in std_logic; y : out std_logic);
            end entity e;

            architecture rtl of e is
            begin
                process (a)
                begin
                    y <= a;
                end process;
            end architecture rtl;
        """)
        sig_assigns = find_nodes(ast, "signal_assignment_seq")
        assert len(sig_assigns) >= 1

    def test_signal_assignment_has_less_equals(self) -> None:
        """The ``<=`` operator should appear as LESS_EQUALS token."""
        ast = parse_vhdl("""
            entity e is
                port(a : in std_logic; y : out std_logic);
            end entity e;

            architecture rtl of e is
            begin
                process (a)
                begin
                    y <= a;
                end process;
            end architecture rtl;
        """)
        sig_assigns = find_nodes(ast, "signal_assignment_seq")
        tokens = find_tokens(sig_assigns[0])
        le_tokens = [
            t for t in tokens if _token_type_name(t) == "LESS_EQUALS"
        ]
        assert len(le_tokens) >= 1


# ============================================================================
# Test: Expressions with Keyword Operators
# ============================================================================


class TestKeywordOperatorExpressions:
    """Test parsing of expressions using VHDL's keyword operators.

    VHDL uses English-word operators instead of symbols for logical operations:

    - ``and``  — logical/bitwise AND (Verilog: ``&``)
    - ``or``   — logical/bitwise OR (Verilog: ``|``)
    - ``xor``  — exclusive OR (Verilog: ``^``)
    - ``nand`` — NOT AND
    - ``nor``  — NOT OR
    - ``xnor`` — NOT XOR
    - ``not``  — unary NOT (Verilog: ``~``)

    This is part of VHDL's Ada heritage — favoring readability over
    conciseness. Compare::

        Verilog: assign y = (a & b) | (c ^ d);
        VHDL:    y <= (a and b) or (c xor d);

    Important: VHDL does NOT allow mixing different logical operators
    without parentheses. ``a and b or c`` is a syntax error. You must
    write ``(a and b) or c``. The grammar enforces this.
    """

    def test_and_expression(self) -> None:
        """Parse ``a and b`` — keyword AND operator."""
        ast = parse_vhdl("""
            entity e is
                port(a, b : in std_logic; y : out std_logic);
            end entity e;

            architecture rtl of e is
            begin
                y <= a and b;
            end architecture rtl;
        """)
        # The expression should produce a logical_expr node containing
        # the 'and' keyword operator.
        logical = find_nodes(ast, "logical_expr")
        assert len(logical) >= 1

        # Find the 'and' keyword in the expression tokens.
        tokens = find_tokens(ast)
        and_tokens = [
            t for t in tokens
            if _token_type_name(t) == "KEYWORD" and t.value == "and"
        ]
        assert len(and_tokens) >= 1

    def test_or_expression(self) -> None:
        """Parse ``a or b`` — keyword OR operator."""
        ast = parse_vhdl("""
            entity e is
                port(a, b : in std_logic; y : out std_logic);
            end entity e;

            architecture rtl of e is
            begin
                y <= a or b;
            end architecture rtl;
        """)
        tokens = find_tokens(ast)
        or_tokens = [
            t for t in tokens
            if _token_type_name(t) == "KEYWORD" and t.value == "or"
        ]
        assert len(or_tokens) >= 1

    def test_xor_expression(self) -> None:
        """Parse ``a xor b`` — keyword XOR operator."""
        ast = parse_vhdl("""
            entity e is
                port(a, b : in std_logic; y : out std_logic);
            end entity e;

            architecture rtl of e is
            begin
                y <= a xor b;
            end architecture rtl;
        """)
        tokens = find_tokens(ast)
        xor_tokens = [
            t for t in tokens
            if _token_type_name(t) == "KEYWORD" and t.value == "xor"
        ]
        assert len(xor_tokens) >= 1

    def test_not_expression(self) -> None:
        """Parse ``not a`` — unary NOT operator."""
        ast = parse_vhdl("""
            entity e is
                port(a : in std_logic; y : out std_logic);
            end entity e;

            architecture rtl of e is
            begin
                y <= not a;
            end architecture rtl;
        """)
        # The unary_expr rule handles 'not'.
        unary = find_nodes(ast, "unary_expr")
        assert len(unary) >= 1


# ============================================================================
# Test: Factory Function
# ============================================================================


class TestCreateVhdlParser:
    """Test the ``create_vhdl_parser()`` factory function."""

    def test_creates_parser(self) -> None:
        """The factory should return a GrammarParser with a parse method."""
        parser = create_vhdl_parser("entity e is end entity e;")
        assert hasattr(parser, "parse")

    def test_factory_produces_same_result(self) -> None:
        """The factory should produce the same AST as parse_vhdl()."""
        source = "entity e is end entity e;"
        ast_direct = parse_vhdl(source)
        ast_factory = create_vhdl_parser(source).parse()

        assert ast_direct.rule_name == ast_factory.rule_name
        assert len(ast_direct.children) == len(ast_factory.children)

    def test_case_insensitivity(self) -> None:
        """VHDL is case-insensitive — ENTITY, Entity, entity are the same."""
        ast = parse_vhdl("ENTITY e IS END ENTITY e;")
        assert ast.rule_name == "design_file"
        entities = find_nodes(ast, "entity_declaration")
        assert len(entities) == 1
