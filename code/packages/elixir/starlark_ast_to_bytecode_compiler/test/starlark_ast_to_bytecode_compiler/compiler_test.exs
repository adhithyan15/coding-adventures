defmodule CodingAdventures.StarlarkAstToBytecodeCompiler.CompilerTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.StarlarkAstToBytecodeCompiler.Compiler
  alias CodingAdventures.StarlarkAstToBytecodeCompiler.Opcodes, as: Op
  alias CodingAdventures.VirtualMachine.Types.CodeObject

  # Helper to find instructions by opcode
  defp find_opcodes(code, opcode) do
    Enum.filter(code.instructions, fn instr -> instr.opcode == opcode end)
  end

  defp has_opcode?(code, opcode) do
    Enum.any?(code.instructions, fn instr -> instr.opcode == opcode end)
  end

  # ===========================================================================
  # Basic Compilation
  # ===========================================================================

  test "compile_starlark returns a CodeObject" do
    code = Compiler.compile_starlark("x = 1\n")
    assert %CodeObject{} = code
    assert is_list(code.instructions)
    assert is_list(code.constants)
    assert is_list(code.names)
  end

  test "compiled code ends with HALT" do
    code = Compiler.compile_starlark("x = 1\n")
    last = List.last(code.instructions)
    assert last.opcode == Op.halt()
  end

  test "create_compiler returns a configured compiler" do
    compiler = Compiler.create_compiler()
    assert map_size(compiler.dispatch) > 40
  end

  # ===========================================================================
  # Integer Literals
  # ===========================================================================

  test "compiles integer literal" do
    code = Compiler.compile_starlark("x = 42\n")
    assert 42 in code.constants
    assert has_opcode?(code, Op.load_const())
    assert has_opcode?(code, Op.store_name())
  end

  test "compiles zero" do
    code = Compiler.compile_starlark("x = 0\n")
    assert 0 in code.constants
  end

  test "compiles large integer" do
    code = Compiler.compile_starlark("x = 999999\n")
    assert 999999 in code.constants
  end

  test "compiles negative integer via unary minus" do
    code = Compiler.compile_starlark("x = -5\n")
    assert 5 in code.constants
    assert has_opcode?(code, Op.negate())
  end

  # ===========================================================================
  # Float Literals
  # ===========================================================================

  test "compiles float literal" do
    code = Compiler.compile_starlark("x = 3.14\n")
    assert 3.14 in code.constants
  end

  # ===========================================================================
  # String Literals
  # ===========================================================================

  test "compiles double-quoted string" do
    code = Compiler.compile_starlark("x = \"hello\"\n")
    assert "hello" in code.constants
  end

  test "compiles single-quoted string" do
    code = Compiler.compile_starlark("x = 'world'\n")
    assert "world" in code.constants
  end

  test "compiles empty string" do
    code = Compiler.compile_starlark("x = \"\"\n")
    assert "" in code.constants
  end

  # ===========================================================================
  # Boolean and None Literals
  # ===========================================================================

  test "compiles True" do
    code = Compiler.compile_starlark("x = True\n")
    assert has_opcode?(code, Op.load_true())
  end

  test "compiles False" do
    code = Compiler.compile_starlark("x = False\n")
    assert has_opcode?(code, Op.load_false())
  end

  test "compiles None" do
    code = Compiler.compile_starlark("x = None\n")
    assert has_opcode?(code, Op.load_none())
  end

  # ===========================================================================
  # Arithmetic Operations
  # ===========================================================================

  test "compiles addition" do
    code = Compiler.compile_starlark("x = 1 + 2\n")
    assert has_opcode?(code, Op.add())
  end

  test "compiles subtraction" do
    code = Compiler.compile_starlark("x = 5 - 3\n")
    assert has_opcode?(code, Op.sub())
  end

  test "compiles multiplication" do
    code = Compiler.compile_starlark("x = 3 * 4\n")
    assert has_opcode?(code, Op.mul())
  end

  test "compiles division" do
    code = Compiler.compile_starlark("x = 10 / 3\n")
    assert has_opcode?(code, Op.div_op())
  end

  test "compiles floor division" do
    code = Compiler.compile_starlark("x = 10 // 3\n")
    assert has_opcode?(code, Op.floor_div())
  end

  test "compiles modulo" do
    code = Compiler.compile_starlark("x = 10 % 3\n")
    assert has_opcode?(code, Op.mod())
  end

  test "compiles power" do
    code = Compiler.compile_starlark("x = 2 ** 3\n")
    assert has_opcode?(code, Op.power())
  end

  test "compiles unary negate" do
    code = Compiler.compile_starlark("x = -y\n")
    assert has_opcode?(code, Op.negate())
  end

  test "compiles bitwise and" do
    code = Compiler.compile_starlark("x = 5 & 3\n")
    assert has_opcode?(code, Op.bit_and())
  end

  test "compiles bitwise or" do
    code = Compiler.compile_starlark("x = 5 | 3\n")
    assert has_opcode?(code, Op.bit_or())
  end

  test "compiles bitwise xor" do
    code = Compiler.compile_starlark("x = 5 ^ 3\n")
    assert has_opcode?(code, Op.bit_xor())
  end

  test "compiles left shift" do
    code = Compiler.compile_starlark("x = 1 << 3\n")
    assert has_opcode?(code, Op.lshift())
  end

  test "compiles right shift" do
    code = Compiler.compile_starlark("x = 8 >> 2\n")
    assert has_opcode?(code, Op.rshift())
  end

  # ===========================================================================
  # Comparison Operations
  # ===========================================================================

  test "compiles equal comparison" do
    code = Compiler.compile_starlark("x = 1 == 2\n")
    assert has_opcode?(code, Op.cmp_eq())
  end

  test "compiles not-equal comparison" do
    code = Compiler.compile_starlark("x = 1 != 2\n")
    assert has_opcode?(code, Op.cmp_ne())
  end

  test "compiles less-than comparison" do
    code = Compiler.compile_starlark("x = 1 < 2\n")
    assert has_opcode?(code, Op.cmp_lt())
  end

  test "compiles greater-than comparison" do
    code = Compiler.compile_starlark("x = 1 > 2\n")
    assert has_opcode?(code, Op.cmp_gt())
  end

  test "compiles less-equal comparison" do
    code = Compiler.compile_starlark("x = 1 <= 2\n")
    assert has_opcode?(code, Op.cmp_le())
  end

  test "compiles greater-equal comparison" do
    code = Compiler.compile_starlark("x = 1 >= 2\n")
    assert has_opcode?(code, Op.cmp_ge())
  end

  # ===========================================================================
  # Boolean Operations
  # ===========================================================================

  test "compiles not expression" do
    code = Compiler.compile_starlark("x = not True\n")
    assert has_opcode?(code, Op.logical_not())
  end

  test "compiles and expression with short-circuit" do
    code = Compiler.compile_starlark("x = True and False\n")
    assert has_opcode?(code, Op.jump_if_false_or_pop())
  end

  test "compiles or expression with short-circuit" do
    code = Compiler.compile_starlark("x = False or True\n")
    assert has_opcode?(code, Op.jump_if_true_or_pop())
  end

  # ===========================================================================
  # Variables
  # ===========================================================================

  test "compiles variable assignment and lookup" do
    code = Compiler.compile_starlark("x = 1\ny = x\n")
    assert has_opcode?(code, Op.store_name())
    assert has_opcode?(code, Op.load_name())
    assert "x" in code.names
    assert "y" in code.names
  end

  test "variable name deduplication in name pool" do
    code = Compiler.compile_starlark("x = 1\nx = 2\n")
    x_count = Enum.count(code.names, fn n -> n == "x" end)
    assert x_count == 1
  end

  test "constant deduplication in constant pool" do
    code = Compiler.compile_starlark("x = 42\ny = 42\n")
    count_42 = Enum.count(code.constants, fn c -> c == 42 end)
    assert count_42 == 1
  end

  # ===========================================================================
  # Augmented Assignment
  # ===========================================================================

  test "compiles += augmented assignment" do
    code = Compiler.compile_starlark("x = 1\nx += 2\n")
    assert has_opcode?(code, Op.add())
  end

  test "compiles -= augmented assignment" do
    code = Compiler.compile_starlark("x = 5\nx -= 3\n")
    assert has_opcode?(code, Op.sub())
  end

  test "compiles *= augmented assignment" do
    code = Compiler.compile_starlark("x = 2\nx *= 4\n")
    assert has_opcode?(code, Op.mul())
  end

  # ===========================================================================
  # Control Flow
  # ===========================================================================

  test "compiles if statement" do
    code = Compiler.compile_starlark("if True:\n  x = 1\n")
    assert has_opcode?(code, Op.jump_if_false())
  end

  test "compiles for loop" do
    code = Compiler.compile_starlark("for x in y:\n  z = x\n")
    assert has_opcode?(code, Op.get_iter())
    assert has_opcode?(code, Op.for_iter())
    assert has_opcode?(code, Op.jump())
  end

  test "compiles pass statement (no-op)" do
    code = Compiler.compile_starlark("pass\n")
    # pass should not emit any opcode except HALT
    non_halt = Enum.reject(code.instructions, fn i -> i.opcode == Op.halt() end)
    assert length(non_halt) == 0
  end

  # ===========================================================================
  # Collections
  # ===========================================================================

  test "compiles empty list" do
    code = Compiler.compile_starlark("x = []\n")
    assert has_opcode?(code, Op.build_list())
    list_instrs = find_opcodes(code, Op.build_list())
    assert hd(list_instrs).operand == 0
  end

  test "compiles list with elements" do
    code = Compiler.compile_starlark("x = [1, 2, 3]\n")
    assert has_opcode?(code, Op.build_list())
    list_instrs = find_opcodes(code, Op.build_list())
    assert hd(list_instrs).operand == 3
  end

  test "compiles empty dict" do
    code = Compiler.compile_starlark("x = {}\n")
    assert has_opcode?(code, Op.build_dict())
    dict_instrs = find_opcodes(code, Op.build_dict())
    assert hd(dict_instrs).operand == 0
  end

  test "compiles dict with entries" do
    code = Compiler.compile_starlark("x = {\"a\": 1, \"b\": 2}\n")
    assert has_opcode?(code, Op.build_dict())
  end

  test "compiles empty tuple" do
    code = Compiler.compile_starlark("x = ()\n")
    assert has_opcode?(code, Op.build_tuple())
  end

  # ===========================================================================
  # Function Calls
  # ===========================================================================

  test "compiles function call with no args" do
    code = Compiler.compile_starlark("f()\n")
    assert has_opcode?(code, Op.call_function())
    call_instrs = find_opcodes(code, Op.call_function())
    assert hd(call_instrs).operand == 0
  end

  test "compiles function call with positional args" do
    code = Compiler.compile_starlark("f(1, 2)\n")
    assert has_opcode?(code, Op.call_function())
    call_instrs = find_opcodes(code, Op.call_function())
    assert hd(call_instrs).operand == 2
  end

  test "compiles return statement with value" do
    code = Compiler.compile_starlark("return 42\n")
    assert has_opcode?(code, Op.return_op())
    assert 42 in code.constants
  end

  test "compiles return statement without value" do
    code = Compiler.compile_starlark("return\n")
    assert has_opcode?(code, Op.return_op())
    assert has_opcode?(code, Op.load_none())
  end

  # ===========================================================================
  # Expression Statements
  # ===========================================================================

  test "expression statement pops result" do
    code = Compiler.compile_starlark("42\n")
    assert has_opcode?(code, Op.pop())
  end

  # ===========================================================================
  # Compile AST Directly
  # ===========================================================================

  test "compile_ast compiles an AST node" do
    ast = %{rule_name: "file", children: [
      %{rule_name: "assign_stmt", children: [
        %{rule_name: "identifier", children: [%{type: "NAME", value: "x"}]},
        %{type: "EQUALS", value: "="},
        %{rule_name: "number", children: [%{type: "INT", value: "5"}]}
      ]}
    ]}

    code = Compiler.compile_ast(ast)
    assert %CodeObject{} = code
    assert 5 in code.constants
    assert "x" in code.names
  end

  # ===========================================================================
  # Complex Expressions
  # ===========================================================================

  test "compiles nested arithmetic: 1 + 2 * 3" do
    code = Compiler.compile_starlark("x = 1 + 2 * 3\n")
    assert has_opcode?(code, Op.add())
    assert has_opcode?(code, Op.mul())
  end

  test "compiles string assignment" do
    code = Compiler.compile_starlark("name = \"Alice\"\n")
    assert "Alice" in code.constants
    assert "name" in code.names
  end

  test "compiles multiple assignments" do
    code = Compiler.compile_starlark("x = 1\ny = 2\nz = 3\n")
    assert "x" in code.names
    assert "y" in code.names
    assert "z" in code.names
    assert 1 in code.constants
    assert 2 in code.constants
    assert 3 in code.constants
  end

  test "compiles list comprehension" do
    code = Compiler.compile_starlark("x = [i for i in y]\n")
    assert has_opcode?(code, Op.build_list())
    assert has_opcode?(code, Op.get_iter())
    assert has_opcode?(code, Op.for_iter())
    assert has_opcode?(code, Op.list_append())
  end
end
