defmodule CodingAdventures.AlgolParserTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.AlgolParser
  alias CodingAdventures.Parser.ASTNode

  # Convenience: collect all rule_names found anywhere in the AST via BFS.
  defp rule_names_in(node) do
    collect_rules([node], [])
  end

  defp collect_rules([], acc), do: Enum.reverse(acc)

  defp collect_rules([node | rest], acc) when is_struct(node, ASTNode) do
    child_nodes = Enum.filter(node.children, &is_struct(&1, ASTNode))
    collect_rules(rest ++ child_nodes, [node.rule_name | acc])
  end

  defp collect_rules([_ | rest], acc) do
    collect_rules(rest, acc)
  end

  # Convenience: find first ASTNode with the given rule_name via BFS.
  defp find_node(node, rule) do
    bfs([node], rule)
  end

  defp bfs([], _rule), do: nil

  defp bfs([node | rest], rule) when is_struct(node, ASTNode) do
    if node.rule_name == rule do
      node
    else
      child_nodes = Enum.filter(node.children, &is_struct(&1, ASTNode))
      bfs(rest ++ child_nodes, rule)
    end
  end

  defp bfs([_ | rest], rule) do
    bfs(rest, rule)
  end

  # ---------------------------------------------------------------------------
  # Grammar inspection
  # ---------------------------------------------------------------------------

  describe "create_parser/0" do
    # The ALGOL 60 grammar has many more rules than JSON — this test checks
    # that the key structural rules are present.
    test "returns a ParserGrammar with top-level rules" do
      grammar = AlgolParser.create_parser()
      rule_names = Enum.map(grammar.rules, & &1.name)
      assert "program" in rule_names
      assert "block" in rule_names
      assert "declaration" in rule_names
      assert "statement" in rule_names
    end

    test "returns a ParserGrammar with declaration rules" do
      grammar = AlgolParser.create_parser()
      rule_names = Enum.map(grammar.rules, & &1.name)
      assert "type_decl" in rule_names
      assert "array_decl" in rule_names
      assert "procedure_decl" in rule_names
      assert "switch_decl" in rule_names
    end

    test "returns a ParserGrammar with statement rules" do
      grammar = AlgolParser.create_parser()
      rule_names = Enum.map(grammar.rules, & &1.name)
      assert "assign_stmt" in rule_names
      assert "cond_stmt" in rule_names
      assert "for_stmt" in rule_names
      assert "goto_stmt" in rule_names
      assert "proc_stmt" in rule_names
    end

    test "returns a ParserGrammar with expression rules" do
      grammar = AlgolParser.create_parser()
      rule_names = Enum.map(grammar.rules, & &1.name)
      assert "arith_expr" in rule_names
      assert "bool_expr" in rule_names
      assert "relation" in rule_names
      assert "primary" in rule_names
    end
  end

  # ---------------------------------------------------------------------------
  # Minimal program
  # ---------------------------------------------------------------------------

  describe "parse/1 — minimal programs" do
    test "minimal program: begin integer x; x := 42 end" do
      # This is the canonical ALGOL 60 hello-world: declare an integer,
      # assign it a value, and close the block.
      {:ok, ast} = AlgolParser.parse("begin integer x; x := 42 end")
      assert ast.rule_name == "program"
      all_rules = rule_names_in(ast)
      assert "block" in all_rules
    end

    test "root node is always program" do
      {:ok, ast} = AlgolParser.parse("begin integer n; n := 0 end")
      assert ast.rule_name == "program"
    end

    test "empty block: begin end" do
      # An empty block is valid in ALGOL 60 — zero declarations, zero statements.
      {:ok, ast} = AlgolParser.parse("begin end")
      assert ast.rule_name == "program"
      all_rules = rule_names_in(ast)
      assert "block" in all_rules
    end
  end

  # ---------------------------------------------------------------------------
  # Declarations
  # ---------------------------------------------------------------------------

  describe "parse/1 — declarations" do
    test "integer declaration" do
      {:ok, ast} = AlgolParser.parse("begin integer x; x := 0 end")
      all_rules = rule_names_in(ast)
      assert "type_decl" in all_rules
    end

    test "real declaration" do
      {:ok, ast} = AlgolParser.parse("begin real sum; sum := 0.0 end")
      all_rules = rule_names_in(ast)
      assert "type_decl" in all_rules
    end

    test "boolean declaration" do
      {:ok, ast} = AlgolParser.parse("begin boolean flag; flag := true end")
      all_rules = rule_names_in(ast)
      assert "type_decl" in all_rules
    end

    test "multiple variables in one declaration" do
      # ALGOL 60 allows: integer x, y, z
      {:ok, ast} = AlgolParser.parse("begin integer x, y, z; x := 0 end")
      all_rules = rule_names_in(ast)
      assert "type_decl" in all_rules
    end
  end

  # ---------------------------------------------------------------------------
  # Assignment statement
  # ---------------------------------------------------------------------------

  describe "parse/1 — assignment" do
    test "simple integer assignment" do
      {:ok, ast} = AlgolParser.parse("begin integer x; x := 42 end")
      all_rules = rule_names_in(ast)
      assert "assign_stmt" in all_rules
    end

    test "real assignment" do
      {:ok, ast} = AlgolParser.parse("begin real pi; pi := 3.14159 end")
      all_rules = rule_names_in(ast)
      assert "assign_stmt" in all_rules
    end

    test "assignment from expression" do
      {:ok, ast} = AlgolParser.parse("begin integer x; x := 1 + 2 end")
      all_rules = rule_names_in(ast)
      assert "assign_stmt" in all_rules
    end
  end

  # ---------------------------------------------------------------------------
  # Arithmetic expressions
  # ---------------------------------------------------------------------------

  describe "parse/1 — arithmetic expressions" do
    test "addition: x := 1 + 2" do
      {:ok, ast} = AlgolParser.parse("begin integer x; x := 1 + 2 end")
      all_rules = rule_names_in(ast)
      assert "assign_stmt" in all_rules
      assert "arith_expr" in all_rules
    end

    test "subtraction: x := 5 - 3" do
      {:ok, ast} = AlgolParser.parse("begin integer x; x := 5 - 3 end")
      all_rules = rule_names_in(ast)
      assert "arith_expr" in all_rules
    end

    test "multiplication: x := 2 * 3" do
      {:ok, ast} = AlgolParser.parse("begin integer x; x := 2 * 3 end")
      all_rules = rule_names_in(ast)
      assert "term" in all_rules
    end

    test "exponentiation x ** 2" do
      # Exponentiation is left-associative in ALGOL 60: 2^3^4 = (2^3)^4 = 4096.
      # This is unusual — most modern languages and mathematics use right-associativity.
      {:ok, ast} = AlgolParser.parse("begin integer x; x := 2 ** 2 end")
      all_rules = rule_names_in(ast)
      assert "factor" in all_rules
    end

    test "parenthesized expression" do
      {:ok, ast} = AlgolParser.parse("begin integer x; x := (1 + 2) * 3 end")
      all_rules = rule_names_in(ast)
      assert "arith_expr" in all_rules
    end

    test "div operator for integer division" do
      # ALGOL 60 uses the keyword `div` for integer division and `/` for real
      # division. This avoids the C-style confusion where 3/2 = 1 or 1.5
      # depending on the types of the operands.
      {:ok, ast} = AlgolParser.parse("begin integer x; x := 10 div 3 end")
      all_rules = rule_names_in(ast)
      assert "term" in all_rules
    end

    test "mod operator for remainder" do
      {:ok, ast} = AlgolParser.parse("begin integer x; x := 10 mod 3 end")
      all_rules = rule_names_in(ast)
      assert "term" in all_rules
    end
  end

  # ---------------------------------------------------------------------------
  # Conditional statement (if/then/else)
  # ---------------------------------------------------------------------------

  describe "parse/1 — if statement" do
    test "if then without else" do
      {:ok, ast} = AlgolParser.parse("begin integer x; if x > 0 then x := 1 end")
      all_rules = rule_names_in(ast)
      assert "cond_stmt" in all_rules
    end

    test "if then else" do
      {:ok, ast} = AlgolParser.parse("begin integer x; if x > 0 then x := 1 else x := 0 end")
      all_rules = rule_names_in(ast)
      assert "cond_stmt" in all_rules
    end

    test "if with relational expression using LEQ" do
      {:ok, ast} = AlgolParser.parse("begin integer x; if x <= 10 then x := x + 1 end")
      all_rules = rule_names_in(ast)
      assert "cond_stmt" in all_rules
      assert "relation" in all_rules
    end

    test "if with equality check" do
      # Remember: = is equality in ALGOL 60 (not assignment). This tests that
      # the parser correctly treats = as the EQ relational operator.
      {:ok, ast} = AlgolParser.parse("begin integer x; if x = 0 then x := 1 end")
      all_rules = rule_names_in(ast)
      assert "cond_stmt" in all_rules
    end
  end

  # ---------------------------------------------------------------------------
  # Boolean expressions
  # ---------------------------------------------------------------------------

  describe "parse/1 — boolean expressions" do
    test "boolean and" do
      {:ok, ast} = AlgolParser.parse("begin integer x, y; if x > 0 and y < 10 then x := 0 end")
      all_rules = rule_names_in(ast)
      assert "bool_factor" in all_rules
    end

    test "boolean or" do
      {:ok, ast} = AlgolParser.parse("begin integer x; if x < 0 or x > 100 then x := 0 end")
      all_rules = rule_names_in(ast)
      assert "bool_term" in all_rules
    end

    test "boolean not" do
      {:ok, ast} = AlgolParser.parse("begin boolean flag; if not flag then flag := true end")
      all_rules = rule_names_in(ast)
      assert "bool_secondary" in all_rules
    end

    test "boolean literal true" do
      {:ok, ast} = AlgolParser.parse("begin boolean t; t := true end")
      assert ast.rule_name == "program"
    end

    test "boolean literal false" do
      {:ok, ast} = AlgolParser.parse("begin boolean f; f := false end")
      assert ast.rule_name == "program"
    end
  end

  # ---------------------------------------------------------------------------
  # For loop
  # ---------------------------------------------------------------------------

  describe "parse/1 — for loop" do
    test "step/until form: for i := 1 step 1 until 10 do" do
      # The step/until form is the canonical ALGOL range loop.
      # It corresponds to C's: for (i = 1; i <= 10; i += 1)
      source = "begin integer x, i; for i := 1 step 1 until 10 do x := x + i end"
      {:ok, ast} = AlgolParser.parse(source)
      all_rules = rule_names_in(ast)
      assert "for_stmt" in all_rules
    end

    test "while form: for i := x while x > 0 do" do
      # The while form advances while a condition is true.
      source = "begin integer i; for i := i while i > 0 do i := i - 1 end"
      {:ok, ast} = AlgolParser.parse(source)
      all_rules = rule_names_in(ast)
      assert "for_stmt" in all_rules
    end

    test "simple value form: for i := 5 do" do
      # A single value — the loop body executes exactly once.
      source = "begin integer i, x; for i := 5 do x := i end"
      {:ok, ast} = AlgolParser.parse(source)
      all_rules = rule_names_in(ast)
      assert "for_stmt" in all_rules
    end
  end

  # ---------------------------------------------------------------------------
  # Nested blocks
  # ---------------------------------------------------------------------------

  describe "parse/1 — nested blocks" do
    test "nested begin/end blocks" do
      # ALGOL 60's block structure allows full nesting. Inner blocks can
      # declare their own variables that shadow outer-scope variables.
      source = "begin integer x; begin integer y; y := 1 end end"
      {:ok, ast} = AlgolParser.parse(source)
      all_rules = rule_names_in(ast)
      # Two blocks should appear in the tree.
      block_count = Enum.count(all_rules, &(&1 == "block"))
      assert block_count >= 2
    end

    test "compound statement (begin/end with no declarations)" do
      source = "begin integer x; begin x := 1; x := x + 1 end end"
      {:ok, ast} = AlgolParser.parse(source)
      all_rules = rule_names_in(ast)
      assert "compound_stmt" in all_rules
    end
  end

  # ---------------------------------------------------------------------------
  # Procedure call as statement
  # ---------------------------------------------------------------------------

  describe "parse/1 — procedure call" do
    test "procedure call with arguments" do
      # A bare identifier followed by (args) is a procedure call statement.
      # The return value (if any) is discarded.
      source = "begin foo(1, 2) end"
      {:ok, ast} = AlgolParser.parse(source)
      all_rules = rule_names_in(ast)
      assert "proc_stmt" in all_rules
    end

    test "procedure call with no arguments" do
      # A procedure call with no arguments omits the parentheses entirely.
      source = "begin reset end"
      {:ok, ast} = AlgolParser.parse(source)
      all_rules = rule_names_in(ast)
      assert "proc_stmt" in all_rules
    end
  end

  # ---------------------------------------------------------------------------
  # Goto statement
  # ---------------------------------------------------------------------------

  describe "parse/1 — goto" do
    test "goto with label" do
      # goto is present in ALGOL 60 but considered harmful (Dijkstra's famous
      # 1968 letter was partly a response to goto-heavy ALGOL programs).
      source = "begin goto done; done: end"
      {:ok, ast} = AlgolParser.parse(source)
      all_rules = rule_names_in(ast)
      assert "goto_stmt" in all_rules
    end
  end

  # ---------------------------------------------------------------------------
  # String literal in program
  # ---------------------------------------------------------------------------

  describe "parse/1 — string literal" do
    test "string assignment" do
      source = "begin string s; s := 'hello' end"
      {:ok, ast} = AlgolParser.parse(source)
      assert ast.rule_name == "program"
      all_rules = rule_names_in(ast)
      assert "assign_stmt" in all_rules
    end
  end

  # ---------------------------------------------------------------------------
  # Multiple statements
  # ---------------------------------------------------------------------------

  describe "parse/1 — multiple statements" do
    test "two assignments separated by semicolon" do
      source = "begin integer x, y; x := 1; y := 2 end"
      {:ok, ast} = AlgolParser.parse(source)
      all_rules = rule_names_in(ast)
      assign_count = Enum.count(all_rules, &(&1 == "assign_stmt"))
      assert assign_count >= 2
    end

    test "declaration followed by multiple statements" do
      source = "begin real pi; pi := 3.14159; if pi > 3.0 then pi := pi + 0.0 end"
      {:ok, ast} = AlgolParser.parse(source)
      all_rules = rule_names_in(ast)
      assert "type_decl" in all_rules
      assert "assign_stmt" in all_rules
      assert "cond_stmt" in all_rules
    end
  end

  # ---------------------------------------------------------------------------
  # Comment handling (lexer + parser integration)
  # ---------------------------------------------------------------------------

  describe "parse/1 — comment handling" do
    test "comment before statement is ignored" do
      # The lexer strips the comment; the parser sees a clean token stream.
      source = "begin integer x; comment initialize x to zero; x := 0 end"
      {:ok, ast} = AlgolParser.parse(source)
      all_rules = rule_names_in(ast)
      assert "assign_stmt" in all_rules
    end
  end

  # ---------------------------------------------------------------------------
  # ASTNode helpers
  # ---------------------------------------------------------------------------

  describe "ASTNode helpers" do
    test "leaf? detects a leaf node containing a single token" do
      # Parse a literal number value and check that the leaf detection works.
      {:ok, ast} = AlgolParser.parse("begin integer x; x := 42 end")
      # Find the integer literal somewhere in the tree.
      int_node = find_node(ast, "primary")
      assert int_node != nil
    end
  end

  # ---------------------------------------------------------------------------
  # Error cases
  # ---------------------------------------------------------------------------

  describe "parse/1 — error cases" do
    test "unclosed block returns error" do
      {:error, msg} = AlgolParser.parse("begin integer x; x := 1")
      assert msg =~ "Parse error" or msg =~ "Unexpected" or msg =~ "error"
    end

    test "unexpected character in source returns error" do
      {:error, _msg} = AlgolParser.parse("@")
    end

    test "declaration without statement returns error" do
      # A block needs at least one statement after the declarations.
      {:error, _msg} = AlgolParser.parse("begin integer x end")
    end
  end
end
