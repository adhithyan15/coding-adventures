defmodule CodingAdventures.BytecodeCompiler.GenericCompilerTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.BytecodeCompiler.GenericCompiler
  alias CodingAdventures.BytecodeCompiler.GenericCompiler.{CompilerScope, CompilerError, UnhandledRuleError}
  alias CodingAdventures.VirtualMachine.Types.{Instruction, CodeObject}

  # ---------------------------------------------------------------------------
  # Opcode constants used throughout tests
  # ---------------------------------------------------------------------------

  @load_const 0x01
  @store_name 0x02
  # @load_name 0x03  # reserved for future use
  @add 0x10
  @jump 0x20
  @jump_if_false 0x21
  @halt 0xFF

  # ---------------------------------------------------------------------------
  # Helper: build a toy "number" AST node
  # ---------------------------------------------------------------------------

  defp number_node(value) do
    %{rule_name: "number", children: [%{type: "NUMBER", value: Integer.to_string(value)}]}
  end

  defp make_number_handler do
    fn compiler, node ->
      token = hd(node.children)
      value = String.to_integer(token.value)
      {index, compiler} = GenericCompiler.add_constant(compiler, value)
      {_idx, compiler} = GenericCompiler.emit(compiler, @load_const, index)
      compiler
    end
  end

  # ---------------------------------------------------------------------------
  # 1. Constructor
  # ---------------------------------------------------------------------------

  describe "new/0" do
    test "creates a compiler with empty state" do
      compiler = GenericCompiler.new()
      assert compiler.instructions == []
      assert compiler.constants == []
      assert compiler.names == []
      assert compiler.dispatch == %{}
      assert compiler.scope == nil
    end
  end

  # ---------------------------------------------------------------------------
  # 2. Rule Registration
  # ---------------------------------------------------------------------------

  describe "register_rule/3" do
    test "registers a handler for a rule name" do
      compiler = GenericCompiler.new()
      handler = fn c, _node -> c end
      compiler = GenericCompiler.register_rule(compiler, "number", handler)
      assert Map.has_key?(compiler.dispatch, "number")
    end

    test "overwrites a previously registered handler" do
      compiler = GenericCompiler.new()
      handler1 = fn c, _node -> c end
      handler2 = fn c, _node -> c end
      compiler = GenericCompiler.register_rule(compiler, "expr", handler1)
      compiler = GenericCompiler.register_rule(compiler, "expr", handler2)
      assert compiler.dispatch["expr"] == handler2
    end

    test "can register multiple different rules" do
      compiler = GenericCompiler.new()
      compiler = GenericCompiler.register_rule(compiler, "number", fn c, _ -> c end)
      compiler = GenericCompiler.register_rule(compiler, "string", fn c, _ -> c end)
      assert map_size(compiler.dispatch) == 2
    end
  end

  # ---------------------------------------------------------------------------
  # 3. Instruction Emission
  # ---------------------------------------------------------------------------

  describe "emit/2 and emit/3" do
    test "emits an instruction with opcode and operand" do
      compiler = GenericCompiler.new()
      {idx, compiler} = GenericCompiler.emit(compiler, @load_const, 0)
      assert idx == 0
      assert length(compiler.instructions) == 1
      assert hd(compiler.instructions) == %Instruction{opcode: @load_const, operand: 0}
    end

    test "emits an instruction with opcode only (nil operand)" do
      compiler = GenericCompiler.new()
      {idx, compiler} = GenericCompiler.emit(compiler, @add)
      assert idx == 0
      assert hd(compiler.instructions) == %Instruction{opcode: @add, operand: nil}
    end

    test "returns incrementing indices for multiple emissions" do
      compiler = GenericCompiler.new()
      {idx0, compiler} = GenericCompiler.emit(compiler, @load_const, 0)
      {idx1, compiler} = GenericCompiler.emit(compiler, @load_const, 1)
      {idx2, _compiler} = GenericCompiler.emit(compiler, @add)
      assert idx0 == 0
      assert idx1 == 1
      assert idx2 == 2
    end

    test "instructions are stored in emission order" do
      compiler = GenericCompiler.new()
      {_, compiler} = GenericCompiler.emit(compiler, @load_const, 0)
      {_, compiler} = GenericCompiler.emit(compiler, @load_const, 1)
      {_, compiler} = GenericCompiler.emit(compiler, @add)

      assert Enum.map(compiler.instructions, & &1.opcode) == [@load_const, @load_const, @add]
    end
  end

  # ---------------------------------------------------------------------------
  # 4. Jump Emission and Patching
  # ---------------------------------------------------------------------------

  describe "emit_jump/2 and patch_jump/3" do
    test "emit_jump emits with placeholder target 0" do
      compiler = GenericCompiler.new()
      {idx, compiler} = GenericCompiler.emit_jump(compiler, @jump)
      assert idx == 0
      assert hd(compiler.instructions).operand == 0
    end

    test "patch_jump updates the target to current offset by default" do
      compiler = GenericCompiler.new()
      {jump_idx, compiler} = GenericCompiler.emit_jump(compiler, @jump_if_false)
      # Emit some instructions after the jump
      {_, compiler} = GenericCompiler.emit(compiler, @load_const, 0)
      {_, compiler} = GenericCompiler.emit(compiler, @add)
      # Patch: jump should now target instruction 3 (the next to be emitted)
      compiler = GenericCompiler.patch_jump(compiler, jump_idx)
      patched = Enum.at(compiler.instructions, jump_idx)
      assert patched.operand == 3
    end

    test "patch_jump with explicit target" do
      compiler = GenericCompiler.new()
      {jump_idx, compiler} = GenericCompiler.emit_jump(compiler, @jump)
      {_, compiler} = GenericCompiler.emit(compiler, @load_const, 0)
      compiler = GenericCompiler.patch_jump(compiler, jump_idx, 99)
      assert Enum.at(compiler.instructions, jump_idx).operand == 99
    end

    test "patch_jump preserves the original opcode" do
      compiler = GenericCompiler.new()
      {jump_idx, compiler} = GenericCompiler.emit_jump(compiler, @jump_if_false)
      compiler = GenericCompiler.patch_jump(compiler, jump_idx, 10)
      assert Enum.at(compiler.instructions, jump_idx).opcode == @jump_if_false
    end
  end

  # ---------------------------------------------------------------------------
  # 5. current_offset
  # ---------------------------------------------------------------------------

  describe "current_offset/1" do
    test "returns 0 for empty compiler" do
      assert GenericCompiler.current_offset(GenericCompiler.new()) == 0
    end

    test "returns instruction count" do
      compiler = GenericCompiler.new()
      {_, compiler} = GenericCompiler.emit(compiler, @load_const, 0)
      {_, compiler} = GenericCompiler.emit(compiler, @add)
      assert GenericCompiler.current_offset(compiler) == 2
    end
  end

  # ---------------------------------------------------------------------------
  # 6. Constant Pool
  # ---------------------------------------------------------------------------

  describe "add_constant/2" do
    test "adds a constant and returns its index" do
      compiler = GenericCompiler.new()
      {idx, compiler} = GenericCompiler.add_constant(compiler, 42)
      assert idx == 0
      assert compiler.constants == [42]
    end

    test "deduplicates identical constants" do
      compiler = GenericCompiler.new()
      {idx1, compiler} = GenericCompiler.add_constant(compiler, 42)
      {idx2, compiler} = GenericCompiler.add_constant(compiler, 42)
      assert idx1 == 0
      assert idx2 == 0
      assert compiler.constants == [42]
    end

    test "assigns sequential indices for distinct values" do
      compiler = GenericCompiler.new()
      {idx1, compiler} = GenericCompiler.add_constant(compiler, 10)
      {idx2, compiler} = GenericCompiler.add_constant(compiler, 20)
      {idx3, _compiler} = GenericCompiler.add_constant(compiler, 30)
      assert {idx1, idx2, idx3} == {0, 1, 2}
    end

    test "distinguishes between integer and float with ===" do
      compiler = GenericCompiler.new()
      {idx1, compiler} = GenericCompiler.add_constant(compiler, 1)
      {idx2, _compiler} = GenericCompiler.add_constant(compiler, 1.0)
      assert idx1 == 0
      assert idx2 == 1
    end

    test "handles string constants" do
      compiler = GenericCompiler.new()
      {idx, compiler} = GenericCompiler.add_constant(compiler, "hello")
      assert idx == 0
      assert compiler.constants == ["hello"]
    end
  end

  # ---------------------------------------------------------------------------
  # 7. Name Pool
  # ---------------------------------------------------------------------------

  describe "add_name/2" do
    test "adds a name and returns its index" do
      compiler = GenericCompiler.new()
      {idx, compiler} = GenericCompiler.add_name(compiler, "x")
      assert idx == 0
      assert compiler.names == ["x"]
    end

    test "deduplicates identical names" do
      compiler = GenericCompiler.new()
      {_, compiler} = GenericCompiler.add_name(compiler, "x")
      {idx, compiler} = GenericCompiler.add_name(compiler, "x")
      assert idx == 0
      assert length(compiler.names) == 1
    end

    test "assigns sequential indices for distinct names" do
      compiler = GenericCompiler.new()
      {idx1, compiler} = GenericCompiler.add_name(compiler, "x")
      {idx2, _compiler} = GenericCompiler.add_name(compiler, "y")
      assert {idx1, idx2} == {0, 1}
    end
  end

  # ---------------------------------------------------------------------------
  # 8. CompilerScope
  # ---------------------------------------------------------------------------

  describe "CompilerScope" do
    test "add_local assigns sequential indices" do
      scope = %CompilerScope{}
      {idx1, scope} = CompilerScope.add_local(scope, "a")
      {idx2, scope} = CompilerScope.add_local(scope, "b")
      assert {idx1, idx2} == {0, 1}
      assert scope.locals == %{"a" => 0, "b" => 1}
    end

    test "add_local returns existing index for duplicate" do
      scope = %CompilerScope{}
      {_, scope} = CompilerScope.add_local(scope, "x")
      {idx, scope} = CompilerScope.add_local(scope, "x")
      assert idx == 0
      assert CompilerScope.num_locals(scope) == 1
    end

    test "get_local returns index or nil" do
      scope = %CompilerScope{}
      {_, scope} = CompilerScope.add_local(scope, "x")
      assert CompilerScope.get_local(scope, "x") == 0
      assert CompilerScope.get_local(scope, "y") == nil
    end

    test "num_locals counts variables" do
      scope = %CompilerScope{}
      assert CompilerScope.num_locals(scope) == 0
      {_, scope} = CompilerScope.add_local(scope, "a")
      {_, scope} = CompilerScope.add_local(scope, "b")
      assert CompilerScope.num_locals(scope) == 2
    end
  end

  # ---------------------------------------------------------------------------
  # 9. Scope Management
  # ---------------------------------------------------------------------------

  describe "enter_scope/2 and exit_scope/1" do
    test "enter_scope creates a new scope" do
      compiler = GenericCompiler.new()
      {scope, compiler} = GenericCompiler.enter_scope(compiler)
      assert scope == compiler.scope
      assert scope.locals == %{}
      assert scope.parent == nil
    end

    test "enter_scope with params pre-populates locals" do
      compiler = GenericCompiler.new()
      {scope, compiler} = GenericCompiler.enter_scope(compiler, ["a", "b", "c"])
      assert scope.locals == %{"a" => 0, "b" => 1, "c" => 2}
      assert compiler.scope == scope
    end

    test "nested scopes chain via parent" do
      compiler = GenericCompiler.new()
      {outer, compiler} = GenericCompiler.enter_scope(compiler, ["x"])
      {inner, compiler} = GenericCompiler.enter_scope(compiler, ["y"])
      assert inner.parent == outer
      assert compiler.scope == inner
    end

    test "exit_scope returns to parent" do
      compiler = GenericCompiler.new()
      {_outer, compiler} = GenericCompiler.enter_scope(compiler, ["x"])
      {_inner, compiler} = GenericCompiler.enter_scope(compiler, ["y"])
      {exited, compiler} = GenericCompiler.exit_scope(compiler)
      assert exited.locals == %{"y" => 0}
      assert compiler.scope.locals == %{"x" => 0}
    end

    test "exit_scope raises when no scope exists" do
      compiler = GenericCompiler.new()

      assert_raise CompilerError, ~r/Cannot exit scope/, fn ->
        GenericCompiler.exit_scope(compiler)
      end
    end
  end

  # ---------------------------------------------------------------------------
  # 10. Token Compilation
  # ---------------------------------------------------------------------------

  describe "compile_node/2 with tokens" do
    test "token nodes are no-ops by default" do
      compiler = GenericCompiler.new()
      result = GenericCompiler.compile_node(compiler, %{type: "NEWLINE", value: "\n"})
      assert result == compiler
    end

    test "multiple token types are all no-ops" do
      compiler = GenericCompiler.new()
      for type <- ["INDENT", "DEDENT", "NEWLINE", "EOF"] do
        result = GenericCompiler.compile_node(compiler, %{type: type, value: ""})
        assert result == compiler
      end
    end
  end

  # ---------------------------------------------------------------------------
  # 11. Pass-through for Single-Child Nodes
  # ---------------------------------------------------------------------------

  describe "compile_node/2 pass-through" do
    test "single-child node without handler passes through" do
      compiler = GenericCompiler.new()
      compiler = GenericCompiler.register_rule(compiler, "number", make_number_handler())

      # expr -> term -> number  (no handlers for expr or term)
      ast = %{
        rule_name: "expr",
        children: [
          %{
            rule_name: "term",
            children: [number_node(42)]
          }
        ]
      }

      compiler = GenericCompiler.compile_node(compiler, ast)
      assert length(compiler.instructions) == 1
      assert hd(compiler.instructions).opcode == @load_const
      assert compiler.constants == [42]
    end
  end

  # ---------------------------------------------------------------------------
  # 12. UnhandledRuleError
  # ---------------------------------------------------------------------------

  describe "compile_node/2 error cases" do
    test "raises UnhandledRuleError for multi-child node without handler" do
      compiler = GenericCompiler.new()

      ast = %{
        rule_name: "binary_op",
        children: [
          %{type: "NUMBER", value: "1"},
          %{type: "PLUS", value: "+"},
          %{type: "NUMBER", value: "2"}
        ]
      }

      assert_raise UnhandledRuleError, ~r/No handler registered for rule 'binary_op'/, fn ->
        GenericCompiler.compile_node(compiler, ast)
      end
    end

    test "error message includes the rule name and child count" do
      compiler = GenericCompiler.new()

      ast = %{
        rule_name: "if_stmt",
        children: [%{type: "IF", value: "if"}, %{type: "TRUE", value: "true"}]
      }

      assert_raise UnhandledRuleError, ~r/2 children/, fn ->
        GenericCompiler.compile_node(compiler, ast)
      end
    end
  end

  # ---------------------------------------------------------------------------
  # 13. Toy Handler: Compile Number
  # ---------------------------------------------------------------------------

  describe "compile with number handler" do
    test "compiles a number node to LOAD_CONST" do
      compiler = GenericCompiler.new()
      compiler = GenericCompiler.register_rule(compiler, "number", make_number_handler())

      compiler = GenericCompiler.compile_node(compiler, number_node(42))

      assert compiler.constants == [42]
      assert length(compiler.instructions) == 1
      instr = hd(compiler.instructions)
      assert instr.opcode == @load_const
      assert instr.operand == 0
    end
  end

  # ---------------------------------------------------------------------------
  # 14. Toy Handler: Assignment (STORE_NAME)
  # ---------------------------------------------------------------------------

  describe "compile with assignment handler" do
    test "compiles x = 42 to LOAD_CONST + STORE_NAME" do
      compiler = GenericCompiler.new()
      compiler = GenericCompiler.register_rule(compiler, "number", make_number_handler())

      assign_handler = fn compiler, node ->
        # children: [name_token, "=", expr]
        [name_token, _eq, expr] = node.children
        # Compile the expression first (pushes value onto stack)
        compiler = GenericCompiler.compile_node(compiler, expr)
        # Then store into the named variable
        {name_idx, compiler} = GenericCompiler.add_name(compiler, name_token.value)
        {_idx, compiler} = GenericCompiler.emit(compiler, @store_name, name_idx)
        compiler
      end

      compiler = GenericCompiler.register_rule(compiler, "assignment", assign_handler)

      ast = %{
        rule_name: "assignment",
        children: [
          %{type: "NAME", value: "x"},
          %{type: "EQUALS", value: "="},
          number_node(42)
        ]
      }

      compiler = GenericCompiler.compile_node(compiler, ast)

      assert compiler.constants == [42]
      assert compiler.names == ["x"]
      assert length(compiler.instructions) == 2

      [load, store] = compiler.instructions
      assert load.opcode == @load_const
      assert load.operand == 0
      assert store.opcode == @store_name
      assert store.operand == 0
    end
  end

  # ---------------------------------------------------------------------------
  # 15. Toy Handler: Binary Operation
  # ---------------------------------------------------------------------------

  describe "compile with binary op handler" do
    test "compiles 3 + 5 to LOAD_CONST, LOAD_CONST, ADD" do
      compiler = GenericCompiler.new()
      compiler = GenericCompiler.register_rule(compiler, "number", make_number_handler())

      binop_handler = fn compiler, node ->
        [left, _op, right] = node.children
        compiler = GenericCompiler.compile_node(compiler, left)
        compiler = GenericCompiler.compile_node(compiler, right)
        {_idx, compiler} = GenericCompiler.emit(compiler, @add)
        compiler
      end

      compiler = GenericCompiler.register_rule(compiler, "binary_op", binop_handler)

      ast = %{
        rule_name: "binary_op",
        children: [
          number_node(3),
          %{type: "PLUS", value: "+"},
          number_node(5)
        ]
      }

      compiler = GenericCompiler.compile_node(compiler, ast)

      assert compiler.constants == [3, 5]
      opcodes = Enum.map(compiler.instructions, & &1.opcode)
      assert opcodes == [@load_const, @load_const, @add]
    end
  end

  # ---------------------------------------------------------------------------
  # 16. compile_nested
  # ---------------------------------------------------------------------------

  describe "compile_nested/2" do
    test "produces a standalone CodeObject without affecting parent" do
      compiler = GenericCompiler.new()
      compiler = GenericCompiler.register_rule(compiler, "number", make_number_handler())

      # Add something to the parent first
      {_, compiler} = GenericCompiler.add_constant(compiler, 999)
      {_, compiler} = GenericCompiler.emit(compiler, @load_const, 0)

      # Compile nested
      {nested, compiler} = GenericCompiler.compile_nested(compiler, number_node(42))

      # Nested should have its own constants and instructions
      assert nested.constants == [42]
      assert length(nested.instructions) == 1
      assert hd(nested.instructions).opcode == @load_const

      # Parent state should be restored
      assert compiler.constants == [999]
      assert length(compiler.instructions) == 1
    end

    test "nested compilation preserves dispatch table" do
      compiler = GenericCompiler.new()
      compiler = GenericCompiler.register_rule(compiler, "number", make_number_handler())

      {nested, _compiler} = GenericCompiler.compile_nested(compiler, number_node(7))

      assert nested.constants == [7]
      assert length(nested.instructions) == 1
    end
  end

  # ---------------------------------------------------------------------------
  # 17. Full compile/3
  # ---------------------------------------------------------------------------

  describe "compile/3" do
    test "produces a CodeObject with HALT at the end" do
      compiler = GenericCompiler.new()
      compiler = GenericCompiler.register_rule(compiler, "number", make_number_handler())

      {code, _compiler} = GenericCompiler.compile(compiler, number_node(42))

      assert %CodeObject{} = code
      assert code.constants == [42]
      assert length(code.instructions) == 2

      [load, halt] = code.instructions
      assert load.opcode == @load_const
      assert halt.opcode == @halt
    end

    test "uses custom halt opcode" do
      compiler = GenericCompiler.new()
      compiler = GenericCompiler.register_rule(compiler, "number", make_number_handler())

      {code, _compiler} = GenericCompiler.compile(compiler, number_node(1), 0x00)

      last = List.last(code.instructions)
      assert last.opcode == 0x00
    end

    test "compile returns updated compiler state" do
      compiler = GenericCompiler.new()
      compiler = GenericCompiler.register_rule(compiler, "number", make_number_handler())

      {_code, compiler} = GenericCompiler.compile(compiler, number_node(10))

      assert compiler.constants == [10]
      assert length(compiler.instructions) == 2
    end

    test "compile with empty pass-through chain" do
      compiler = GenericCompiler.new()
      compiler = GenericCompiler.register_rule(compiler, "number", make_number_handler())

      ast = %{
        rule_name: "program",
        children: [
          %{
            rule_name: "statement",
            children: [
              %{rule_name: "expression", children: [number_node(77)]}
            ]
          }
        ]
      }

      {code, _compiler} = GenericCompiler.compile(compiler, ast)
      assert code.constants == [77]
      # LOAD_CONST + HALT
      assert length(code.instructions) == 2
    end
  end

  # ---------------------------------------------------------------------------
  # 18. Delegation from top-level module
  # ---------------------------------------------------------------------------

  describe "CodingAdventures.BytecodeCompiler delegation" do
    alias CodingAdventures.BytecodeCompiler

    test "new/0 delegates correctly" do
      compiler = BytecodeCompiler.new()
      assert %GenericCompiler{} = compiler
    end

    test "register_rule/3 delegates correctly" do
      compiler = BytecodeCompiler.new()
      compiler = BytecodeCompiler.register_rule(compiler, "test", fn c, _ -> c end)
      assert Map.has_key?(compiler.dispatch, "test")
    end

    test "compile/2 delegates correctly" do
      compiler = BytecodeCompiler.new()
      compiler = BytecodeCompiler.register_rule(compiler, "number", make_number_handler())
      {code, _} = BytecodeCompiler.compile(compiler, number_node(5))
      assert %CodeObject{} = code
    end

    test "emit/2 delegates correctly (no operand)" do
      compiler = BytecodeCompiler.new()
      {idx, compiler} = BytecodeCompiler.emit(compiler, @add)
      assert idx == 0
      assert hd(compiler.instructions).opcode == @add
      assert hd(compiler.instructions).operand == nil
    end

    test "emit/3 delegates correctly (with operand)" do
      compiler = BytecodeCompiler.new()
      {idx, compiler} = BytecodeCompiler.emit(compiler, @load_const, 0)
      assert idx == 0
      assert hd(compiler.instructions).opcode == @load_const
      assert hd(compiler.instructions).operand == 0
    end

    test "compile/3 with custom halt delegates correctly" do
      compiler = BytecodeCompiler.new()
      compiler = BytecodeCompiler.register_rule(compiler, "number", make_number_handler())
      {code, _} = BytecodeCompiler.compile(compiler, number_node(5), 0x00)
      assert List.last(code.instructions).opcode == 0x00
    end
  end
end
