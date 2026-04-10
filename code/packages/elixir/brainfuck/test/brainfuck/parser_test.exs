defmodule CodingAdventures.Brainfuck.ParserTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.Brainfuck.Parser
  alias CodingAdventures.Parser.ASTNode

  # =========================================================================
  # Helpers
  # =========================================================================

  # Parse source and assert success, returning the AST node.
  defp parse!(source) do
    {:ok, ast} = Parser.parse(source)
    ast
  end

  # Recursively collect all token leaf values from an AST node.
  # Walks the tree depth-first, gathering every non-ASTNode child.
  defp collect_token_types(%ASTNode{children: children}) do
    Enum.flat_map(children, fn
      %ASTNode{} = child -> collect_token_types(child)
      token -> [token.type]
    end)
  end

  defp collect_token_types(_), do: []

  # Find all ASTNode descendants with the given rule_name.
  defp find_nodes(%ASTNode{rule_name: rn, children: children} = node, rule) do
    matched = if rn == rule, do: [node], else: []

    nested =
      children
      |> Enum.filter(&match?(%ASTNode{}, &1))
      |> Enum.flat_map(&find_nodes(&1, rule))

    matched ++ nested
  end

  defp find_nodes(_, _), do: []

  # =========================================================================
  # Empty program
  # =========================================================================
  # Brainfuck's grammar rule `program = { instruction }` allows zero
  # instructions. An empty program is a valid parse result.

  describe "empty program" do
    test "empty string returns ok tuple" do
      result = Parser.parse("")
      assert {:ok, _} = result
    end

    test "empty string produces a program AST node" do
      ast = parse!("")
      assert %ASTNode{} = ast
    end

    test "empty program has rule_name 'program'" do
      ast = parse!("")
      assert ast.rule_name == "program"
    end

    test "empty program has no command tokens in tree" do
      ast = parse!("")
      # Only EOF may appear — no command tokens
      types =
        collect_token_types(ast)
        |> Enum.reject(&(&1 == :eof))
      assert types == []
    end
  end

  # =========================================================================
  # Simple commands
  # =========================================================================
  # A flat sequence of commands (no loops) parses into a program node
  # whose instruction children each wrap a single command token.

  describe "simple commands" do
    test "single '+' returns ok" do
      assert {:ok, _} = Parser.parse("+")
    end

    test "single '+' produces program node" do
      ast = parse!("+")
      assert ast.rule_name == "program"
    end

    test "single '+' has INC token in tree" do
      ast = parse!("+")
      types = collect_token_types(ast) |> Enum.reject(&(&1 == :eof))
      assert types == ["INC"]
    end

    test "all six non-bracket commands produce correct token types" do
      # The six non-loop commands in order.
      ast = parse!("><+-.,")
      types = collect_token_types(ast) |> Enum.reject(&(&1 == :eof))
      assert types == ["RIGHT", "LEFT", "INC", "DEC", "OUTPUT", "INPUT"]
    end

    test "command token values are preserved through parsing" do
      ast = parse!("+->")
      tokens =
        collect_token_types(ast)
        |> Enum.reject(&(&1 == :eof))
      # We can only check types here (not values) since collect_token_types
      # returns types, not the full token struct. The lexer tests cover values.
      assert tokens == ["INC", "DEC", "RIGHT"]
    end

    test "program with only comments produces empty command stream" do
      ast = parse!("this is a comment with no commands")
      types = collect_token_types(ast) |> Enum.reject(&(&1 == :eof))
      assert types == []
    end
  end

  # =========================================================================
  # Loop structure
  # =========================================================================
  # The grammar rule `loop = LOOP_START { instruction } LOOP_END` produces
  # a "loop" node in the AST. Loops contain the bracket tokens and any
  # instruction children.

  describe "loop structure" do
    test "empty loop '[]' parses successfully" do
      # [] is a valid Brainfuck idiom — clears a cell (or no-op if zero).
      assert {:ok, _} = Parser.parse("[]")
    end

    test "empty loop '[]' produces one loop node" do
      ast = parse!("[]")
      loops = find_nodes(ast, "loop")
      assert length(loops) == 1
    end

    test "loop with body '[+]' produces one loop node" do
      ast = parse!("[+]")
      loops = find_nodes(ast, "loop")
      assert length(loops) == 1
    end

    test "loop body commands are inside the loop node" do
      ast = parse!("[+]")
      [loop | _] = find_nodes(ast, "loop")
      # The INC command should be inside the loop subtree
      commands_in_loop = find_nodes(loop, "command")
      refute commands_in_loop == [],
        "The '+' inside [+] should produce a command node inside the loop"
    end

    test "nested loops produce the right count of loop nodes" do
      # [[+]] has an outer loop containing an inner loop.
      ast = parse!("[[+]]")
      loops = find_nodes(ast, "loop")
      assert length(loops) >= 2,
        "[[+]] should have at least 2 loop nodes (outer and inner)"
    end

    test "LOOP_START and LOOP_END tokens appear in the tree" do
      ast = parse!("[+]")
      types = collect_token_types(ast) |> Enum.reject(&(&1 == :eof))
      assert "LOOP_START" in types
      assert "LOOP_END" in types
    end
  end

  # =========================================================================
  # Unmatched brackets
  # =========================================================================
  # The grammar requires a LOOP_END for every LOOP_START. Unmatched
  # brackets must result in {:error, _}.

  describe "unmatched brackets" do
    test "unmatched LOOP_START '[+' returns error" do
      result = Parser.parse("[+")
      assert {:error, _} = result,
        "Unmatched '[' should return {:error, _}"
    end

    test "unmatched LOOP_END '+]' returns error" do
      # A "]" without a preceding "[" is a syntax error.
      result = Parser.parse("+]")
      assert {:error, _} = result,
        "Unmatched ']' should return {:error, _}"
    end
  end

  # =========================================================================
  # Canonical Brainfuck example: ++[>+<-]
  # =========================================================================
  # The standard "add cell" idiom. Exercises commands before a loop,
  # commands inside a loop, and loop nesting — all in one program.
  #
  #   ++      cell 0 = 2
  #   [       while cell 0 != 0
  #     >+    move right, increment cell 1
  #     <-    move left, decrement cell 0
  #   ]       exit when cell 0 = 0

  describe "canonical ++[>+<-]" do
    test "parses successfully" do
      assert {:ok, _} = Parser.parse("++[>+<-]")
    end

    test "root node is 'program'" do
      ast = parse!("++[>+<-]")
      assert ast.rule_name == "program"
    end

    test "produces correct token sequence in tree" do
      ast = parse!("++[>+<-]")
      types = collect_token_types(ast) |> Enum.reject(&(&1 == :eof))
      expected = ["INC", "INC", "LOOP_START", "RIGHT", "INC", "LEFT", "DEC", "LOOP_END"]
      assert types == expected
    end

    test "has exactly one loop node" do
      ast = parse!("++[>+<-]")
      loops = find_nodes(ast, "loop")
      assert length(loops) == 1
    end

    test "has six command nodes total" do
      # ++  (2 commands) + >+<- (4 commands inside loop) = 6 command nodes
      ast = parse!("++[>+<-]")
      commands = find_nodes(ast, "command")
      assert length(commands) == 6
    end

    test "annotated version parses identically" do
      source = "++ set cell0\n[ loop while nonzero\n  >+ right and inc\n  <- left and dec\n]"
      ast = parse!(source)
      types = collect_token_types(ast) |> Enum.reject(&(&1 == :eof))
      expected = ["INC", "INC", "LOOP_START", "RIGHT", "INC", "LEFT", "DEC", "LOOP_END"]
      assert types == expected
    end
  end

  # =========================================================================
  # Deeply nested loops
  # =========================================================================

  describe "deeply nested loops" do
    test "three levels of nesting parse correctly" do
      # +[+[+[+]]] — outer, middle, and inner loops
      ast = parse!("+[+[+[+]]]")
      assert ast.rule_name == "program"
    end

    test "three levels produce three loop nodes" do
      ast = parse!("+[+[+[+]]]")
      loops = find_nodes(ast, "loop")
      assert length(loops) == 3
    end

    test "grammar file exists and parses into a grammar struct" do
      # Calling create_parser/0 directly verifies that the grammar file
      # is readable and parses without error.
      grammar = Parser.create_parser()
      refute is_nil(grammar)
    end
  end
end
