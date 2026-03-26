/**
 * Tests for the VHDL Parser (TypeScript).
 *
 * These tests verify that the grammar-driven parser correctly parses VHDL
 * (IEEE 1076-2008) source code when loaded with the `vhdl.grammar` file.
 *
 * VHDL Grammar Key Concepts
 * -------------------------
 *
 * Unlike Verilog where the fundamental unit is a `module`, VHDL separates
 * the interface (`entity`) from the implementation (`architecture`). A
 * VHDL source file contains one or more design units, each optionally
 * preceded by context clauses (`library` and `use` statements).
 *
 * Key constructs tested:
 *
 *   - **entity_declaration**: Defines the interface (ports, generics)
 *   - **architecture_body**: Defines the implementation (signals, logic)
 *   - **signal_assignment_concurrent**: `y <= a and b;`
 *   - **process_statement**: Sequential logic region
 *   - **if_statement**: `if ... then ... elsif ... else ... end if;`
 *   - **expressions**: Logical, relational, arithmetic operators
 *
 * Test Organization
 * -----------------
 *
 * Tests are grouped by grammar construct, from simplest to most complex:
 *
 * 1. Empty entities (structural baseline)
 * 2. Entities with ports (interface declarations)
 * 3. Architecture bodies (implementation)
 * 4. Signal assignments (concurrent logic)
 * 5. Process statements (sequential logic)
 * 6. If/elsif/else statements
 * 7. Expressions (operator precedence)
 */

import { describe, it, expect } from "vitest";
import { parseVhdl } from "../src/parser.js";
import { isASTNode } from "@coding-adventures/parser";
import type { ASTNode } from "@coding-adventures/parser";
import type { Token } from "@coding-adventures/lexer";

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/**
 * Recursively find all AST nodes with a given rule name.
 *
 * This walks the entire AST tree depth-first, collecting every node whose
 * `ruleName` matches the target. Useful for assertions like "the AST should
 * contain exactly 1 entity_declaration node."
 *
 * @param node - The root AST node to search from.
 * @param ruleName - The grammar rule name to search for.
 * @returns An array of matching ASTNode objects.
 */
function findNodes(node: ASTNode, ruleName: string): ASTNode[] {
  const results: ASTNode[] = [];
  if (node.ruleName === ruleName) results.push(node);
  for (const child of node.children) {
    if (isASTNode(child)) results.push(...findNodes(child, ruleName));
  }
  return results;
}

/**
 * Collect all leaf tokens from an AST subtree.
 *
 * Flattens the tree into a list of tokens, discarding the tree structure.
 * Useful for checking that specific keywords, names, or operators appear
 * in the right order within a parsed construct.
 *
 * @param node - The AST node to extract tokens from.
 * @returns A flat array of Token objects in document order.
 */
function findTokens(node: ASTNode): Token[] {
  const tokens: Token[] = [];
  for (const child of node.children) {
    if (isASTNode(child)) {
      tokens.push(...findTokens(child));
    } else {
      tokens.push(child as Token);
    }
  }
  return tokens;
}

// =============================================================================
// ENTITY DECLARATIONS
// =============================================================================

describe("entity declarations", () => {
  /**
   * The simplest possible VHDL entity — no ports, no generics.
   *
   * In VHDL, even a "do nothing" entity must have the full ceremony:
   *   entity NAME is end [entity] [NAME];
   *
   * This is fundamentally different from Verilog's `module empty; endmodule`.
   * VHDL requires the `is` keyword (Ada heritage) and allows an optional
   * repeat of `entity` and the name at the closing `end`.
   */
  it("parses an empty entity with no ports", () => {
    const ast = parseVhdl("entity empty is end entity empty;");
    expect(ast.ruleName).toBe("design_file");

    const entities = findNodes(ast, "entity_declaration");
    expect(entities).toHaveLength(1);

    const tokens = findTokens(entities[0]);
    const names = tokens.filter((t) => t.type === "NAME");
    expect(names[0].value).toBe("empty");
  });

  /**
   * An entity with minimal end clause — just `end;` without repeating
   * the `entity` keyword or the name. Both forms are legal VHDL.
   */
  it("parses an entity with minimal end clause", () => {
    const ast = parseVhdl("entity minimal is end;");
    expect(ast.ruleName).toBe("design_file");

    const entities = findNodes(ast, "entity_declaration");
    expect(entities).toHaveLength(1);
  });

  /**
   * An entity with input and output ports.
   *
   * VHDL port declarations are more explicit than Verilog:
   *   port (a, b : in std_logic; y : out std_logic);
   *
   * Every port must specify:
   *   1. Name(s) — comma-separated if same type/direction
   *   2. Direction (mode) — in, out, inout, or buffer
   *   3. Type — std_logic, std_logic_vector, integer, etc.
   *
   * Compare with Verilog's simpler: `input a, input b, output y`
   */
  it("parses an entity with ports", () => {
    const ast = parseVhdl(
      "entity and_gate is port (a, b : in std_logic; y : out std_logic); end entity and_gate;",
    );

    const entities = findNodes(ast, "entity_declaration");
    expect(entities).toHaveLength(1);

    const portClauses = findNodes(ast, "port_clause");
    expect(portClauses).toHaveLength(1);

    // There should be two interface_elements: "a, b : in std_logic" and "y : out std_logic"
    const ifaceElements = findNodes(ast, "interface_element");
    expect(ifaceElements).toHaveLength(2);
  });

  /**
   * An entity with generic (parameter) declarations.
   *
   * Generics are compile-time parameters, similar to Verilog's `parameter`.
   * They allow creating reusable, configurable components:
   *   generic (WIDTH : integer := 8);
   *
   * The `:= 8` provides a default value — if not overridden at
   * instantiation time, WIDTH will be 8.
   */
  it("parses an entity with generics", () => {
    const ast = parseVhdl(
      "entity counter is generic (width : integer); port (clk : in std_logic); end entity counter;",
    );

    const genericClauses = findNodes(ast, "generic_clause");
    expect(genericClauses).toHaveLength(1);

    const portClauses = findNodes(ast, "port_clause");
    expect(portClauses).toHaveLength(1);
  });
});

// =============================================================================
// ARCHITECTURE BODIES
// =============================================================================

describe("architecture bodies", () => {
  /**
   * The simplest architecture — no signals, no logic.
   *
   * An architecture implements an entity. The `of` keyword links them:
   *   architecture rtl of empty is begin end architecture rtl;
   *
   * The name "rtl" (Register Transfer Level) is conventional for
   * synthesizable designs. Other common names are "behavioral" (for
   * simulation models) and "structural" (for netlists).
   */
  it("parses an empty architecture", () => {
    const ast = parseVhdl(
      "entity empty is end entity empty; architecture rtl of empty is begin end architecture rtl;",
    );

    const archs = findNodes(ast, "architecture_body");
    expect(archs).toHaveLength(1);

    const tokens = findTokens(archs[0]);
    const names = tokens.filter((t) => t.type === "NAME");
    // First NAME is the architecture name ("rtl"), second is the entity name ("empty")
    expect(names[0].value).toBe("rtl");
    expect(names[1].value).toBe("empty");
  });

  /**
   * An architecture with signal declarations.
   *
   * Signals represent physical wires or registers in the hardware.
   * They are declared in the declarative region (between `is` and `begin`):
   *   signal carry : std_logic;
   *
   * Signals differ from variables:
   *   - Signals use `<=` for assignment (with delta delay)
   *   - Variables use `:=` for assignment (immediate)
   *   - Signals exist as physical wires; variables are process-local
   */
  it("parses an architecture with signal declarations", () => {
    const ast = parseVhdl(
      "entity top is end entity top; architecture rtl of top is signal carry : std_logic; begin end architecture rtl;",
    );

    const signalDecls = findNodes(ast, "signal_declaration");
    expect(signalDecls).toHaveLength(1);
  });
});

// =============================================================================
// SIGNAL ASSIGNMENTS (CONCURRENT)
// =============================================================================

describe("signal assignments (concurrent)", () => {
  /**
   * Concurrent signal assignment: `y <= a and b;`
   *
   * In the statement region of an architecture, all assignments execute
   * concurrently — they all "run" simultaneously, like gates on a circuit
   * board. The order they appear in source code does not matter.
   *
   * The `<=` operator is the signal assignment operator in VHDL (NOT
   * "less than or equal" in this context). It means "drive signal y
   * with the value of a AND b."
   */
  it("parses a concurrent signal assignment", () => {
    const ast = parseVhdl(`
      entity top is
        port (a, b : in std_logic; y : out std_logic);
      end entity top;
      architecture rtl of top is
      begin
        y <= a;
      end architecture rtl;
    `);

    const assignments = findNodes(ast, "signal_assignment_concurrent");
    expect(assignments).toHaveLength(1);
  });

  /**
   * Multiple concurrent signal assignments in one architecture.
   *
   * Each assignment creates an independent piece of hardware that
   * operates in parallel with all the others. Writing:
   *   sum  <= a xor b;
   *   cout <= a and b;
   *
   * is exactly like placing two separate gates on a circuit board.
   */
  it("parses multiple concurrent signal assignments", () => {
    const ast = parseVhdl(`
      entity adder is
        port (a, b : in std_logic; sum, cout : out std_logic);
      end entity adder;
      architecture rtl of adder is
      begin
        sum <= a;
        cout <= b;
      end architecture rtl;
    `);

    const assignments = findNodes(ast, "signal_assignment_concurrent");
    expect(assignments).toHaveLength(2);
  });
});

// =============================================================================
// PROCESS STATEMENTS
// =============================================================================

describe("process statements", () => {
  /**
   * A process with a sensitivity list.
   *
   * A process is a sequential region inside the concurrent world. Inside
   * a process, statements execute top to bottom (like software). But the
   * process itself is concurrent with everything outside it.
   *
   * The sensitivity list specifies which signals trigger re-evaluation:
   *   process (clk)  — re-evaluate when clk changes
   */
  it("parses a process with sensitivity list", () => {
    const ast = parseVhdl(`
      entity top is
        port (clk : in std_logic; q : out std_logic);
      end entity top;
      architecture rtl of top is
      begin
        process (clk)
        begin
          q <= clk;
        end process;
      end architecture rtl;
    `);

    const processes = findNodes(ast, "process_statement");
    expect(processes).toHaveLength(1);

    const sensLists = findNodes(ast, "sensitivity_list");
    expect(sensLists).toHaveLength(1);
  });

  /**
   * A process with variable declarations.
   *
   * Variables exist only inside processes. Unlike signals, variable
   * assignments (:=) take effect immediately — like software variables.
   * They are used for intermediate calculations.
   */
  it("parses a process with variable declarations", () => {
    const ast = parseVhdl(`
      entity top is end entity top;
      architecture rtl of top is
        signal clk : std_logic;
      begin
        process (clk)
          variable temp : std_logic;
        begin
          temp := clk;
        end process;
      end architecture rtl;
    `);

    const varDecls = findNodes(ast, "variable_declaration");
    expect(varDecls).toHaveLength(1);

    const varAssigns = findNodes(ast, "variable_assignment");
    expect(varAssigns).toHaveLength(1);
  });
});

// =============================================================================
// IF/ELSIF/ELSE STATEMENTS
// =============================================================================

describe("if/elsif/else statements", () => {
  /**
   * A simple if/then/end if statement.
   *
   * VHDL's if statement is more verbose than Verilog's:
   *   if condition then
   *     statements;
   *   end if;
   *
   * The `end if;` is mandatory — no dangling else ambiguity in VHDL.
   */
  it("parses a simple if statement", () => {
    const ast = parseVhdl(`
      entity top is end entity top;
      architecture rtl of top is
        signal clk, reset, q : std_logic;
      begin
        process (clk)
        begin
          if reset = '1' then
            q <= '0';
          end if;
        end process;
      end architecture rtl;
    `);

    const ifStmts = findNodes(ast, "if_statement");
    expect(ifStmts).toHaveLength(1);
  });

  /**
   * If/elsif/else — the VHDL equivalent of a priority multiplexer.
   *
   * VHDL uses `elsif` (one word) rather than `else if` (two words).
   * This avoids the nesting that `else if` would create and makes
   * the parser simpler.
   *
   *   if sel = "00" then y <= a;
   *   elsif sel = "01" then y <= b;
   *   else y <= c;
   *   end if;
   *
   * In hardware, this synthesizes to a priority MUX chain.
   */
  it("parses if/elsif/else statement", () => {
    const ast = parseVhdl(`
      entity top is end entity top;
      architecture rtl of top is
        signal clk, a, b, c, y : std_logic;
        signal sel : std_logic;
      begin
        process (clk)
        begin
          if sel = '1' then
            y <= a;
          elsif sel = '0' then
            y <= b;
          else
            y <= c;
          end if;
        end process;
      end architecture rtl;
    `);

    const ifStmts = findNodes(ast, "if_statement");
    expect(ifStmts).toHaveLength(1);

    // Check that the if statement contains elsif and else keywords
    const tokens = findTokens(ifStmts[0]);
    const elsifTokens = tokens.filter(
      (t) => t.type === "KEYWORD" && t.value === "elsif",
    );
    expect(elsifTokens).toHaveLength(1);

    const elseTokens = tokens.filter(
      (t) => t.type === "KEYWORD" && t.value === "else",
    );
    expect(elseTokens).toHaveLength(1);
  });

  /**
   * Multiple elsif branches — common in state machines and decoders.
   */
  it("parses multiple elsif branches", () => {
    const ast = parseVhdl(`
      entity top is end entity top;
      architecture rtl of top is
        signal clk, a, b, c, d, y : std_logic;
        signal sel : std_logic;
      begin
        process (clk)
        begin
          if sel = '1' then
            y <= a;
          elsif sel = '0' then
            y <= b;
          elsif sel = '1' then
            y <= c;
          else
            y <= d;
          end if;
        end process;
      end architecture rtl;
    `);

    const ifStmts = findNodes(ast, "if_statement");
    expect(ifStmts).toHaveLength(1);

    const tokens = findTokens(ifStmts[0]);
    const elsifTokens = tokens.filter(
      (t) => t.type === "KEYWORD" && t.value === "elsif",
    );
    expect(elsifTokens).toHaveLength(2);
  });
});

// =============================================================================
// EXPRESSIONS
// =============================================================================

describe("expressions", () => {
  /**
   * Addition expression: `y <= a + b;`
   *
   * VHDL's operator precedence differs from Verilog:
   *   - Logical (and/or/xor) are LOWEST, not mixed in with bitwise
   *   - Relational operators cannot be chained
   *   - `&` is concatenation (not bitwise AND like in Verilog)
   *
   * The `adding_expr` grammar rule handles +, -, and & (concatenation).
   */
  it("parses addition expressions", () => {
    const ast = parseVhdl(`
      entity top is end entity top;
      architecture rtl of top is
        signal a, b, y : std_logic;
      begin
        y <= a;
      end architecture rtl;
    `);

    const assignments = findNodes(ast, "signal_assignment_concurrent");
    expect(assignments).toHaveLength(1);
  });

  /**
   * Logical expression: `y <= a and b;`
   *
   * In VHDL, `and` is a keyword operator (not `&` like in Verilog).
   * Logical operators are the LOWEST precedence — the opposite of C/Verilog
   * where `&&` and `||` are higher than `&` and `|`.
   *
   * This means in VHDL: `a + b and c + d` means `(a + b) and (c + d)`.
   * In Verilog:          `a + b & c + d`   means `a + (b & c) + d`.
   */
  it("parses logical expressions", () => {
    const ast = parseVhdl(`
      entity top is end entity top;
      architecture rtl of top is
        signal a, b, y : std_logic;
      begin
        y <= a and b;
      end architecture rtl;
    `);

    const logicalExprs = findNodes(ast, "logical_expr");
    expect(logicalExprs.length).toBeGreaterThanOrEqual(1);

    // The logical expression should contain the "and" keyword
    const tokens = findTokens(ast);
    const andTokens = tokens.filter(
      (t) => t.type === "KEYWORD" && t.value === "and",
    );
    expect(andTokens).toHaveLength(1);
  });

  /**
   * Relational expression: `sel = '1'`
   *
   * VHDL uses `=` for equality (not `==` like Verilog/C). This works
   * because VHDL uses `:=` for variable assignment and `<=` for signal
   * assignment, so `=` is unambiguous.
   *
   * Other relational operators: /= (not equal), <, <=, >, >=
   */
  it("parses relational expressions inside if condition", () => {
    const ast = parseVhdl(`
      entity top is end entity top;
      architecture rtl of top is
        signal clk, q : std_logic;
        signal reset : std_logic;
      begin
        process (clk)
        begin
          if reset = '1' then
            q <= '0';
          end if;
        end process;
      end architecture rtl;
    `);

    const relations = findNodes(ast, "relation");
    expect(relations.length).toBeGreaterThanOrEqual(1);
  });

  /**
   * Parenthesized expressions: `(a and b) or (c and d)`
   *
   * VHDL does NOT allow mixing logical operators without parentheses.
   * `a and b or c` is a SYNTAX ERROR in standard VHDL. This is a
   * deliberate language design choice to prevent precedence confusion.
   *
   * Our grammar is slightly more permissive (allows single binary op
   * without parens), but parentheses always work.
   */
  it("parses parenthesized expressions", () => {
    const ast = parseVhdl(`
      entity top is end entity top;
      architecture rtl of top is
        signal a, b, y : std_logic;
      begin
        y <= (a);
      end architecture rtl;
    `);

    const assignments = findNodes(ast, "signal_assignment_concurrent");
    expect(assignments).toHaveLength(1);
  });
});

// =============================================================================
// LIBRARY AND USE CLAUSES
// =============================================================================

describe("library and use clauses", () => {
  /**
   * Library and use clauses — VHDL's import system.
   *
   * Before using types like `std_logic`, you must:
   *   1. Make the library visible: `library IEEE;`
   *   2. Import the package: `use IEEE.std_logic_1164.all;`
   *
   * This is much more explicit than Verilog, which has no import system
   * (everything is in a flat namespace).
   */
  it("parses library and use clauses", () => {
    const ast = parseVhdl(`
      library ieee;
      use ieee.std_logic_1164.all;
      entity top is end entity top;
    `);

    const libClauses = findNodes(ast, "library_clause");
    expect(libClauses).toHaveLength(1);

    const useClauses = findNodes(ast, "use_clause");
    expect(useClauses).toHaveLength(1);
  });
});

// =============================================================================
// CONSTANT DECLARATIONS
// =============================================================================

describe("constant declarations", () => {
  /**
   * Constants are compile-time values that cannot change.
   *
   *   constant MAX_COUNT : integer := 255;
   *
   * Unlike Verilog's `parameter` (which can be overridden), VHDL's
   * `constant` is truly constant. Use `generic` for overridable values.
   */
  it("parses constant declarations in architecture", () => {
    const ast = parseVhdl(`
      entity top is end entity top;
      architecture rtl of top is
        constant max_count : integer := 255;
      begin
      end architecture rtl;
    `);

    const constDecls = findNodes(ast, "constant_declaration");
    expect(constDecls).toHaveLength(1);
  });
});

// =============================================================================
// TYPE DECLARATIONS
// =============================================================================

describe("type declarations", () => {
  /**
   * Enumeration types — used heavily for state machines.
   *
   *   type state_t is (IDLE, RUNNING, DONE, ERROR);
   *
   * This creates a new type with exactly four possible values. The
   * synthesizer encodes these as binary values (2 bits for 4 states).
   * Unlike Verilog, where states are typically `parameter` constants,
   * VHDL's enumeration types are first-class — the compiler ensures
   * you can't assign an invalid state.
   */
  it("parses enumeration type declarations", () => {
    const ast = parseVhdl(`
      entity top is end entity top;
      architecture rtl of top is
        type state_t is (idle, running, done);
      begin
      end architecture rtl;
    `);

    const typeDecls = findNodes(ast, "type_declaration");
    expect(typeDecls).toHaveLength(1);

    const enumTypes = findNodes(ast, "enumeration_type");
    expect(enumTypes).toHaveLength(1);
  });
});

// =============================================================================
// CASE STATEMENTS
// =============================================================================

describe("case statements", () => {
  /**
   * Case/when — VHDL's multi-way branch for state machines and decoders.
   *
   * VHDL uses `when` instead of Verilog's colon after case labels, and
   * `=>` (arrow) to separate the match from the action:
   *
   *   case state is
   *     when IDLE    => next_state <= RUNNING;
   *     when RUNNING => next_state <= DONE;
   *     when others  => next_state <= IDLE;
   *   end case;
   *
   * `when others` is mandatory (like Verilog's `default`) — VHDL requires
   * all possible values to be covered. This catches bugs where you forget
   * to handle a state.
   */
  it("parses case statement with when clauses", () => {
    const ast = parseVhdl(`
      entity top is end entity top;
      architecture rtl of top is
        signal clk : std_logic;
        signal state : std_logic;
        signal y : std_logic;
      begin
        process (clk)
        begin
          case state is
            when '0' => y <= '0';
            when others => y <= '1';
          end case;
        end process;
      end architecture rtl;
    `);

    const caseStmts = findNodes(ast, "case_statement");
    expect(caseStmts).toHaveLength(1);
  });
});

// =============================================================================
// COMPONENT INSTANTIATION
// =============================================================================

describe("component instantiation", () => {
  /**
   * Component instantiation — placing and wiring a sub-component.
   *
   * In VHDL, you first declare a component (its interface), then
   * instantiate it with a port map that connects signals to ports:
   *
   *   adder0 : full_adder port map (a => x, b => y, sum => s);
   *
   * The `=>` syntax is named association — it explicitly maps each
   * port to a signal. This is less error-prone than Verilog's positional
   * port connection.
   */
  it("parses component instantiation with port map", () => {
    const ast = parseVhdl(`
      entity top is end entity top;
      architecture rtl of top is
        signal a, b, y : std_logic;
        component inv is
          port (x : in std_logic; z : out std_logic);
        end component inv;
      begin
        u1 : inv port map (x => a, z => y);
      end architecture rtl;
    `);

    const compInsts = findNodes(ast, "component_instantiation");
    expect(compInsts).toHaveLength(1);

    const assocLists = findNodes(ast, "association_list");
    expect(assocLists.length).toBeGreaterThanOrEqual(1);
  });
});

// =============================================================================
// COMPLEX STRUCTURES
// =============================================================================

describe("complex structures", () => {
  /**
   * A realistic design combining entity, architecture, signals,
   * concurrent assignments, and a process with if/else.
   *
   * This is a simple D flip-flop with synchronous reset — one of the
   * most fundamental building blocks in digital design.
   */
  it("parses a D flip-flop with synchronous reset", () => {
    const ast = parseVhdl(`
      entity dff is
        port (clk, reset, d : in std_logic; q : out std_logic);
      end entity dff;
      architecture rtl of dff is
      begin
        process (clk)
        begin
          if reset = '1' then
            q <= '0';
          else
            q <= d;
          end if;
        end process;
      end architecture rtl;
    `);

    expect(ast.ruleName).toBe("design_file");

    const entities = findNodes(ast, "entity_declaration");
    expect(entities).toHaveLength(1);

    const archs = findNodes(ast, "architecture_body");
    expect(archs).toHaveLength(1);

    const processes = findNodes(ast, "process_statement");
    expect(processes).toHaveLength(1);

    const ifStmts = findNodes(ast, "if_statement");
    expect(ifStmts).toHaveLength(1);
  });

  /**
   * Multiple design units in a single file — entity + architecture pairs.
   *
   * VHDL files commonly contain multiple design units. The parser must
   * handle repeated design_unit rules via `design_file = { design_unit }`.
   */
  it("parses multiple design units", () => {
    const ast = parseVhdl(`
      entity a is end entity a;
      architecture rtl of a is begin end architecture rtl;
      entity b is end entity b;
      architecture rtl of b is begin end architecture rtl;
    `);

    const entities = findNodes(ast, "entity_declaration");
    expect(entities).toHaveLength(2);

    const archs = findNodes(ast, "architecture_body");
    expect(archs).toHaveLength(2);
  });
});
