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

  # ===========================================================================
  # Top-Level Module Delegation
  # ===========================================================================

  test "top-level compile_starlark delegates correctly" do
    alias CodingAdventures.StarlarkAstToBytecodeCompiler, as: TopLevel
    code = TopLevel.compile_starlark("x = 1\n")
    assert %CodeObject{} = code
    assert 1 in code.constants
  end

  test "top-level create_compiler delegates correctly" do
    alias CodingAdventures.StarlarkAstToBytecodeCompiler, as: TopLevel
    compiler = TopLevel.create_compiler()
    assert map_size(compiler.dispatch) > 40
  end

  # ===========================================================================
  # If/Elif/Else Statements
  # ===========================================================================

  test "compiles if/else statement" do
    code = Compiler.compile_starlark("if True:\n  x = 1\nelse:\n  x = 2\n")
    assert has_opcode?(code, Op.jump_if_false())
    assert has_opcode?(code, Op.jump())
    assert 1 in code.constants
    assert 2 in code.constants
  end

  test "compiles if/elif/else statement" do
    source = "if x:\n  a = 1\nelif y:\n  a = 2\nelse:\n  a = 3\n"
    code = Compiler.compile_starlark(source)
    assert has_opcode?(code, Op.jump_if_false())
    assert has_opcode?(code, Op.jump())
    assert 1 in code.constants
    assert 2 in code.constants
    assert 3 in code.constants
  end

  test "compiles if with multiple elif clauses" do
    source = "if a:\n  x = 1\nelif b:\n  x = 2\nelif c:\n  x = 3\n"
    code = Compiler.compile_starlark(source)
    # Multiple JUMP_IF_FALSE for each condition branch
    false_jumps = find_opcodes(code, Op.jump_if_false())
    assert length(false_jumps) >= 2
  end

  # ===========================================================================
  # Ternary Expressions
  # ===========================================================================

  test "compiles ternary expression" do
    code = Compiler.compile_starlark("x = 1 if True else 2\n")
    assert has_opcode?(code, Op.jump_if_false())
    assert has_opcode?(code, Op.jump())
    assert 1 in code.constants
    assert 2 in code.constants
  end

  # ===========================================================================
  # Function Definitions
  # ===========================================================================

  test "compiles simple function definition" do
    source = "def greet():\n  return 42\n"
    code = Compiler.compile_starlark(source)
    assert has_opcode?(code, Op.make_function())
    assert has_opcode?(code, Op.store_name())
    assert "greet" in code.names
  end

  test "compiles function definition with parameters" do
    source = "def add(a, b):\n  return a\n"
    code = Compiler.compile_starlark(source)
    assert has_opcode?(code, Op.make_function())
    assert "add" in code.names
  end

  test "compiles function definition with default parameter values" do
    source = "def greet(name, greeting = \"hello\"):\n  return name\n"
    code = Compiler.compile_starlark(source)
    assert has_opcode?(code, Op.make_function())
    assert has_opcode?(code, Op.build_tuple())
    assert "greet" in code.names
  end

  # ===========================================================================
  # Lambda Expressions
  # ===========================================================================

  test "compiles lambda expression" do
    code = Compiler.compile_starlark("f = lambda x: x\n")
    assert has_opcode?(code, Op.make_function())
    assert "f" in code.names
  end

  test "compiles lambda with multiple parameters" do
    code = Compiler.compile_starlark("f = lambda a, b: a\n")
    assert has_opcode?(code, Op.make_function())
  end

  # ===========================================================================
  # Dict Comprehension
  # ===========================================================================

  test "compiles dict comprehension" do
    code = Compiler.compile_starlark("x = {k: v for k in items}\n")
    assert has_opcode?(code, Op.build_dict())
    assert has_opcode?(code, Op.get_iter())
    assert has_opcode?(code, Op.for_iter())
    assert has_opcode?(code, Op.dict_set())
  end

  test "compiles dict comprehension with filter" do
    code = Compiler.compile_starlark("x = {k: v for k in items if k}\n")
    assert has_opcode?(code, Op.build_dict())
    assert has_opcode?(code, Op.jump_if_false())
    assert has_opcode?(code, Op.dict_set())
  end

  # ===========================================================================
  # Subscript and Slice Operations
  # ===========================================================================

  test "compiles subscript access" do
    code = Compiler.compile_starlark("x = y[0]\n")
    assert has_opcode?(code, Op.load_subscript())
  end

  test "compiles slice with start and stop" do
    code = Compiler.compile_starlark("x = y[1:3]\n")
    assert has_opcode?(code, Op.load_slice())
  end

  test "compiles slice with only stop" do
    code = Compiler.compile_starlark("x = y[:3]\n")
    assert has_opcode?(code, Op.load_slice())
  end

  test "compiles slice with start, stop, and step" do
    code = Compiler.compile_starlark("x = y[0:10:2]\n")
    assert has_opcode?(code, Op.load_slice())
    slice_instrs = find_opcodes(code, Op.load_slice())
    assert hd(slice_instrs).operand == 3
  end

  test "compiles slice with only step" do
    code = Compiler.compile_starlark("x = y[::2]\n")
    assert has_opcode?(code, Op.load_slice())
  end

  # ===========================================================================
  # Dot Access (Attribute Access)
  # ===========================================================================

  test "compiles dot access" do
    code = Compiler.compile_starlark("x = obj.attr\n")
    assert has_opcode?(code, Op.load_attr())
    assert "attr" in code.names
  end

  test "compiles chained dot access" do
    code = Compiler.compile_starlark("x = a.b.c\n")
    attr_instrs = find_opcodes(code, Op.load_attr())
    assert length(attr_instrs) == 2
  end

  # ===========================================================================
  # Load Statement
  # ===========================================================================

  test "compiles load statement" do
    code = Compiler.compile_starlark("load(\"module.star\", \"func1\")\n")
    assert has_opcode?(code, Op.load_module())
    assert has_opcode?(code, Op.import_from())
    assert has_opcode?(code, Op.dup())
    assert has_opcode?(code, Op.pop())
  end

  test "compiles load statement with multiple symbols" do
    code = Compiler.compile_starlark("load(\"mod.star\", \"a\", \"b\")\n")
    dup_instrs = find_opcodes(code, Op.dup())
    import_instrs = find_opcodes(code, Op.import_from())
    assert length(dup_instrs) == 2
    assert length(import_instrs) == 2
  end

  # ===========================================================================
  # Break and Continue
  # ===========================================================================

  test "compiles break statement" do
    source = "for x in y:\n  break\n"
    code = Compiler.compile_starlark(source)
    # break emits a JUMP instruction
    jump_instrs = find_opcodes(code, Op.jump())
    assert length(jump_instrs) >= 2  # one for break, one for loop back
  end

  test "compiles continue statement" do
    source = "for x in y:\n  continue\n"
    code = Compiler.compile_starlark(source)
    jump_instrs = find_opcodes(code, Op.jump())
    assert length(jump_instrs) >= 2
  end

  # ===========================================================================
  # In / Not In Comparisons
  # ===========================================================================

  test "compiles in comparison" do
    code = Compiler.compile_starlark("x = 1 in y\n")
    assert has_opcode?(code, Op.cmp_in())
  end

  test "compiles not in comparison" do
    code = Compiler.compile_starlark("x = 1 not in y\n")
    assert has_opcode?(code, Op.cmp_not_in())
  end

  # ===========================================================================
  # Unary Operations
  # ===========================================================================

  test "compiles bitwise not" do
    code = Compiler.compile_starlark("x = ~5\n")
    assert has_opcode?(code, Op.bit_not())
  end

  test "compiles unary plus (no-op)" do
    code = Compiler.compile_starlark("x = +5\n")
    # Unary + is a no-op, just compiles the operand
    assert 5 in code.constants
  end

  # ===========================================================================
  # String Escape Sequences
  # ===========================================================================

  test "compiles string with newline escape" do
    code = Compiler.compile_starlark("x = \"hello\\nworld\"\n")
    assert "hello\nworld" in code.constants
  end

  test "compiles string with tab escape" do
    code = Compiler.compile_starlark("x = \"col1\\tcol2\"\n")
    assert "col1\tcol2" in code.constants
  end

  test "compiles string with backslash escape" do
    code = Compiler.compile_starlark("x = \"path\\\\file\"\n")
    assert "path\\file" in code.constants
  end

  test "compiles single-quoted string with escape" do
    code = Compiler.compile_starlark("x = 'it\\'s'\n")
    assert "it's" in code.constants
  end

  # ===========================================================================
  # Tuples with Elements
  # ===========================================================================

  test "compiles tuple with elements" do
    code = Compiler.compile_starlark("x = (1, 2, 3)\n")
    assert has_opcode?(code, Op.build_tuple())
    tuple_instrs = find_opcodes(code, Op.build_tuple())
    assert hd(tuple_instrs).operand == 3
  end

  test "compiles two-element tuple" do
    code = Compiler.compile_starlark("x = (1, 2)\n")
    assert has_opcode?(code, Op.build_tuple())
  end

  # ===========================================================================
  # Function Calls with Keyword Args
  # ===========================================================================

  test "compiles function call with keyword arguments" do
    code = Compiler.compile_starlark("f(x = 1, y = 2)\n")
    assert has_opcode?(code, Op.call_function_kw())
  end

  test "compiles function call with mixed positional and keyword args" do
    code = Compiler.compile_starlark("f(1, y = 2)\n")
    assert has_opcode?(code, Op.call_function_kw())
  end

  # ===========================================================================
  # Comments
  # ===========================================================================

  test "compiles source with comments" do
    code = Compiler.compile_starlark("# this is a comment\nx = 1\n")
    assert 1 in code.constants
    assert "x" in code.names
  end

  test "compiles source with inline comment" do
    code = Compiler.compile_starlark("x = 1 # assign one\n")
    assert 1 in code.constants
  end

  # ===========================================================================
  # Additional Augmented Assignments
  # ===========================================================================

  test "compiles /= augmented assignment" do
    code = Compiler.compile_starlark("x = 10\nx /= 2\n")
    assert has_opcode?(code, Op.div_op())
  end

  test "compiles %= augmented assignment" do
    code = Compiler.compile_starlark("x = 10\nx %= 3\n")
    assert has_opcode?(code, Op.mod())
  end

  # ===========================================================================
  # List Comprehension with Filter
  # ===========================================================================

  test "compiles list comprehension with if filter" do
    code = Compiler.compile_starlark("x = [i for i in y if i]\n")
    assert has_opcode?(code, Op.build_list())
    assert has_opcode?(code, Op.get_iter())
    assert has_opcode?(code, Op.for_iter())
    assert has_opcode?(code, Op.list_append())
    assert has_opcode?(code, Op.jump_if_false())
  end

  # ===========================================================================
  # Parenthesized Expressions
  # ===========================================================================

  test "compiles parenthesized expression" do
    code = Compiler.compile_starlark("x = (1 + 2) * 3\n")
    assert has_opcode?(code, Op.add())
    assert has_opcode?(code, Op.mul())
  end

  # ===========================================================================
  # Float Edge Cases
  # ===========================================================================

  test "compiles float with trailing dot" do
    code = Compiler.compile_starlark("x = 3.\n")
    assert 3.0 in code.constants
  end

  # ===========================================================================
  # Complex Programs
  # ===========================================================================

  test "compiles multi-statement program with function and call" do
    source = "def double(n):\n  return n\ndouble(5)\n"
    code = Compiler.compile_starlark(source)
    assert has_opcode?(code, Op.make_function())
    assert has_opcode?(code, Op.call_function())
    assert "double" in code.names
  end

  test "compiles for loop with multiple body statements" do
    source = "for i in items:\n  x = i\n  y = i\n"
    code = Compiler.compile_starlark(source)
    assert has_opcode?(code, Op.get_iter())
    assert has_opcode?(code, Op.for_iter())
  end

  test "compiles nested if in for loop" do
    source = "for i in items:\n  if i:\n    x = i\n"
    code = Compiler.compile_starlark(source)
    assert has_opcode?(code, Op.get_iter())
    assert has_opcode?(code, Op.for_iter())
    assert has_opcode?(code, Op.jump_if_false())
  end

  # ===========================================================================
  # Method Calls (dot access + call)
  # ===========================================================================

  test "compiles method call" do
    code = Compiler.compile_starlark("x.append(1)\n")
    assert has_opcode?(code, Op.load_attr())
    assert has_opcode?(code, Op.call_function())
  end

  # ===========================================================================
  # For Loop with Tuple Unpacking
  # ===========================================================================

  test "compiles for loop with tuple unpacking" do
    source = "for k, v in items:\n  x = k\n"
    code = Compiler.compile_starlark(source)
    assert has_opcode?(code, Op.get_iter())
    assert has_opcode?(code, Op.for_iter())
  end

  # ===========================================================================
  # Dict with Trailing Comma
  # ===========================================================================

  test "compiles dict with trailing comma" do
    code = Compiler.compile_starlark("x = {\"a\": 1,}\n")
    assert has_opcode?(code, Op.build_dict())
  end

  test "compiles list with trailing comma" do
    code = Compiler.compile_starlark("x = [1, 2,]\n")
    assert has_opcode?(code, Op.build_list())
  end

  # ===========================================================================
  # Store to Subscript and Attribute
  # ===========================================================================

  test "compiles store to subscript" do
    code = Compiler.compile_starlark("x[0] = 1\n")
    assert has_opcode?(code, Op.store_subscript())
  end

  test "compiles store to attribute" do
    code = Compiler.compile_starlark("x.y = 1\n")
    assert has_opcode?(code, Op.store_attr())
  end

  # ===========================================================================
  # Augmented Assignment on Subscript
  # ===========================================================================

  test "compiles augmented assignment on subscript" do
    code = Compiler.compile_starlark("x[0] += 1\n")
    assert has_opcode?(code, Op.load_subscript())
    assert has_opcode?(code, Op.add())
    assert has_opcode?(code, Op.store_subscript())
  end

  test "compiles augmented assignment on dot access" do
    code = Compiler.compile_starlark("x.y += 1\n")
    assert has_opcode?(code, Op.load_attr())
    assert has_opcode?(code, Op.add())
    assert has_opcode?(code, Op.store_attr())
  end

  # ===========================================================================
  # Empty Source and Edge Cases
  # ===========================================================================

  test "compiles empty source" do
    code = Compiler.compile_starlark("\n")
    last = List.last(code.instructions)
    assert last.opcode == Op.halt()
  end

  test "compiles source with only comments" do
    code = Compiler.compile_starlark("# just a comment\n")
    last = List.last(code.instructions)
    assert last.opcode == Op.halt()
  end

  test "compiles double-quoted string with escaped quote" do
    code = Compiler.compile_starlark("x = \"she said \\\"hi\\\"\"\n")
    assert "she said \"hi\"" in code.constants
  end

  # ===========================================================================
  # Multiple Dict Entries
  # ===========================================================================

  test "compiles dict with three entries" do
    code = Compiler.compile_starlark("x = {\"a\": 1, \"b\": 2, \"c\": 3}\n")
    assert has_opcode?(code, Op.build_dict())
    dict_instrs = find_opcodes(code, Op.build_dict())
    assert hd(dict_instrs).operand == 3
  end

  # ===========================================================================
  # Chained Comparisons and Boolean Combos
  # ===========================================================================

  test "compiles chained and expressions" do
    code = Compiler.compile_starlark("x = a and b and c\n")
    false_or_pop = find_opcodes(code, Op.jump_if_false_or_pop())
    assert length(false_or_pop) == 2
  end

  test "compiles chained or expressions" do
    code = Compiler.compile_starlark("x = a or b or c\n")
    true_or_pop = find_opcodes(code, Op.jump_if_true_or_pop())
    assert length(true_or_pop) == 2
  end

  test "compiles nested not expressions" do
    code = Compiler.compile_starlark("x = not not True\n")
    not_instrs = find_opcodes(code, Op.logical_not())
    assert length(not_instrs) == 2
  end

  # ===========================================================================
  # Tab Characters in Source
  # ===========================================================================

  test "compiles source with tab indentation" do
    code = Compiler.compile_starlark("if True:\n\tx = 1\n")
    assert has_opcode?(code, Op.jump_if_false())
    assert 1 in code.constants
  end

  # ===========================================================================
  # Lambda Used Directly in Expression
  # ===========================================================================

  test "compiles lambda used in call" do
    code = Compiler.compile_starlark("f(lambda x: x)\n")
    assert has_opcode?(code, Op.make_function())
    assert has_opcode?(code, Op.call_function())
  end

  # ===========================================================================
  # Compile AST with Various Node Types
  # ===========================================================================

  test "compile_ast with suite node" do
    ast = %{rule_name: "file", children: [
      %{rule_name: "suite", children: [
        %{rule_name: "assign_stmt", children: [
          %{rule_name: "identifier", children: [%{type: "NAME", value: "x"}]},
          %{type: "EQUALS", value: "="},
          %{rule_name: "number", children: [%{type: "INT", value: "1"}]}
        ]}
      ]}
    ]}
    code = Compiler.compile_ast(ast)
    assert 1 in code.constants
  end

  test "compile_ast with simple_stmt node" do
    ast = %{rule_name: "file", children: [
      %{rule_name: "simple_stmt", children: [
        %{rule_name: "pass_stmt", children: []}
      ]}
    ]}
    code = Compiler.compile_ast(ast)
    assert %CodeObject{} = code
  end

  test "compile_ast with small_stmt node" do
    ast = %{rule_name: "file", children: [
      %{rule_name: "small_stmt", children: [
        %{rule_name: "expression_stmt", children: [
          %{rule_name: "number", children: [%{type: "INT", value: "42"}]}
        ]}
      ]}
    ]}
    code = Compiler.compile_ast(ast)
    assert 42 in code.constants
  end

  test "compile_ast with compound_stmt node" do
    ast = %{rule_name: "file", children: [
      %{rule_name: "compound_stmt", children: [
        %{rule_name: "pass_stmt", children: []}
      ]}
    ]}
    code = Compiler.compile_ast(ast)
    assert %CodeObject{} = code
  end

  test "compile_ast with expr wrapper node" do
    ast = %{rule_name: "file", children: [
      %{rule_name: "expression_stmt", children: [
        %{rule_name: "expr", children: [
          %{rule_name: "number", children: [%{type: "INT", value: "7"}]}
        ]}
      ]}
    ]}
    code = Compiler.compile_ast(ast)
    assert 7 in code.constants
  end

  test "compile_ast with primary wrapper node" do
    ast = %{rule_name: "file", children: [
      %{rule_name: "expression_stmt", children: [
        %{rule_name: "primary", children: [
          %{rule_name: "number", children: [%{type: "INT", value: "9"}]}
        ]}
      ]}
    ]}
    code = Compiler.compile_ast(ast)
    assert 9 in code.constants
  end

  test "compile_ast with atom wrapping a child node" do
    ast = %{rule_name: "file", children: [
      %{rule_name: "expression_stmt", children: [
        %{rule_name: "atom", children: [
          %{rule_name: "number", children: [%{type: "INT", value: "99"}]}
        ]}
      ]}
    ]}
    code = Compiler.compile_ast(ast)
    assert 99 in code.constants
  end

  test "compile_ast with empty atom node emits load_none" do
    ast = %{rule_name: "file", children: [
      %{rule_name: "expression_stmt", children: [
        %{rule_name: "atom", children: []}
      ]}
    ]}
    code = Compiler.compile_ast(ast)
    assert has_opcode?(code, Op.load_none())
  end

  # ===========================================================================
  # Handler Coverage for factor with single child
  # ===========================================================================

  test "compile_ast with factor node with single child" do
    ast = %{rule_name: "file", children: [
      %{rule_name: "expression_stmt", children: [
        %{rule_name: "factor", children: [
          %{rule_name: "number", children: [%{type: "INT", value: "5"}]}
        ]}
      ]}
    ]}
    code = Compiler.compile_ast(ast)
    assert 5 in code.constants
  end

  # ===========================================================================
  # Direct AST: elif_clause and else_clause handlers
  # ===========================================================================

  test "compile_ast with elif_clause handled directly" do
    ast = %{rule_name: "file", children: [
      %{rule_name: "elif_clause", children: [
        %{rule_name: "atom", children: [%{type: "KEYWORD", value: "True"}]},
        %{rule_name: "assign_stmt", children: [
          %{rule_name: "identifier", children: [%{type: "NAME", value: "x"}]},
          %{type: "EQUALS", value: "="},
          %{rule_name: "number", children: [%{type: "INT", value: "1"}]}
        ]}
      ]}
    ]}
    code = Compiler.compile_ast(ast)
    assert %CodeObject{} = code
  end

  test "compile_ast with else_clause handled directly" do
    ast = %{rule_name: "file", children: [
      %{rule_name: "else_clause", children: [
        %{rule_name: "assign_stmt", children: [
          %{rule_name: "identifier", children: [%{type: "NAME", value: "x"}]},
          %{type: "EQUALS", value: "="},
          %{rule_name: "number", children: [%{type: "INT", value: "2"}]}
        ]}
      ]}
    ]}
    code = Compiler.compile_ast(ast)
    assert 2 in code.constants
  end

  # ===========================================================================
  # Direct AST: call_args and keyword_arg handlers
  # ===========================================================================

  test "compile_ast with call_args handler" do
    ast = %{rule_name: "file", children: [
      %{rule_name: "expression_stmt", children: [
        %{rule_name: "call_args", children: [
          %{rule_name: "number", children: [%{type: "INT", value: "1"}]},
          %{rule_name: "number", children: [%{type: "INT", value: "2"}]}
        ]}
      ]}
    ]}
    code = Compiler.compile_ast(ast)
    assert 1 in code.constants
    assert 2 in code.constants
  end

  test "compile_ast with keyword_arg handler" do
    ast = %{rule_name: "file", children: [
      %{rule_name: "expression_stmt", children: [
        %{rule_name: "keyword_arg", children: [
          %{type: "NAME", value: "key"},
          %{rule_name: "number", children: [%{type: "INT", value: "42"}]}
        ]}
      ]}
    ]}
    code = Compiler.compile_ast(ast)
    assert 42 in code.constants
  end

  # ===========================================================================
  # Direct AST: param_list and param handlers
  # ===========================================================================

  test "compile_ast with param_list handler" do
    ast = %{rule_name: "file", children: [
      %{rule_name: "param_list", children: [
        %{rule_name: "param", children: [%{type: "NAME", value: "x"}]},
        %{rule_name: "param", children: [%{type: "NAME", value: "y"}]}
      ]}
    ]}
    code = Compiler.compile_ast(ast)
    assert %CodeObject{} = code
  end

  test "compile_ast with param handler is a no-op" do
    ast = %{rule_name: "file", children: [
      %{rule_name: "param", children: [%{type: "NAME", value: "x"}]}
    ]}
    code = Compiler.compile_ast(ast)
    assert %CodeObject{} = code
  end

  # ===========================================================================
  # Direct AST: comp_clause and comp_if handlers
  # ===========================================================================

  test "compile_ast with comp_clause handler" do
    ast = %{rule_name: "file", children: [
      %{rule_name: "expression_stmt", children: [
        %{rule_name: "comp_clause", children: [
          %{rule_name: "number", children: [%{type: "INT", value: "10"}]}
        ]}
      ]}
    ]}
    code = Compiler.compile_ast(ast)
    assert 10 in code.constants
  end

  test "compile_ast with comp_if handler" do
    ast = %{rule_name: "file", children: [
      %{rule_name: "expression_stmt", children: [
        %{rule_name: "comp_if", children: [
          %{rule_name: "atom", children: [%{type: "KEYWORD", value: "True"}]}
        ]}
      ]}
    ]}
    code = Compiler.compile_ast(ast)
    assert has_opcode?(code, Op.load_true())
  end

  # ===========================================================================
  # Direct AST: slice handler
  # ===========================================================================

  test "compile_ast with direct slice node" do
    ast = %{rule_name: "file", children: [
      %{rule_name: "expression_stmt", children: [
        %{rule_name: "slice", children: [
          %{rule_name: "number", children: [%{type: "INT", value: "1"}]},
          %{rule_name: "number", children: [%{type: "INT", value: "5"}]}
        ]}
      ]}
    ]}
    code = Compiler.compile_ast(ast)
    assert has_opcode?(code, Op.load_slice())
  end

  # ===========================================================================
  # Number Type Fallback
  # ===========================================================================

  test "compile_ast with unknown number token type defaults to integer" do
    ast = %{rule_name: "file", children: [
      %{rule_name: "expression_stmt", children: [
        %{rule_name: "number", children: [%{type: "UNKNOWN_NUM", value: "7"}]}
      ]}
    ]}
    code = Compiler.compile_ast(ast)
    assert 7 in code.constants
  end

  # ===========================================================================
  # get_string_value fallback paths
  # ===========================================================================

  test "compiles load statement with name-based symbol" do
    # This triggers the get_string_value identifier path
    code = Compiler.compile_starlark("load(\"mod.star\", \"sym\")\n")
    assert has_opcode?(code, Op.load_module())
    assert has_opcode?(code, Op.import_from())
  end

  # ===========================================================================
  # Identifier in scope (no scope context)
  # ===========================================================================

  test "identifier loads via LOAD_NAME when no scope" do
    code = Compiler.compile_starlark("x\n")
    assert has_opcode?(code, Op.load_name())
    assert "x" in code.names
  end

  # ===========================================================================
  # Nested Function Call
  # ===========================================================================

  test "compiles nested function calls" do
    code = Compiler.compile_starlark("f(g(1))\n")
    call_instrs = find_opcodes(code, Op.call_function())
    assert length(call_instrs) == 2
  end

  # ===========================================================================
  # Complex Slice Patterns
  # ===========================================================================

  test "compiles slice with only start" do
    code = Compiler.compile_starlark("x = y[1:]\n")
    assert has_opcode?(code, Op.load_slice())
  end

  # ===========================================================================
  # Empty For Loop Body (pass)
  # ===========================================================================

  test "compiles for loop with pass body" do
    source = "for x in y:\n  pass\n"
    code = Compiler.compile_starlark(source)
    assert has_opcode?(code, Op.get_iter())
    assert has_opcode?(code, Op.for_iter())
  end

  # ===========================================================================
  # Complex Multi-line Programs
  # ===========================================================================

  test "compiles program with def, if, and for" do
    source = """
    def process(items):
      for i in items:
        if i:
          x = i
    """
    code = Compiler.compile_starlark(source)
    assert has_opcode?(code, Op.make_function())
    assert "process" in code.names
  end

  test "compiles if-elif without else" do
    source = "if a:\n  x = 1\nelif b:\n  x = 2\n"
    code = Compiler.compile_starlark(source)
    assert has_opcode?(code, Op.jump_if_false())
  end

  # ===========================================================================
  # Multiple Function Calls and Complex Expressions
  # ===========================================================================

  test "compiles expression with function call and arithmetic" do
    code = Compiler.compile_starlark("x = f(1) + g(2)\n")
    assert has_opcode?(code, Op.call_function())
    assert has_opcode?(code, Op.add())
  end

  test "compiles subscript store to dict" do
    code = Compiler.compile_starlark("d = {}\nd[\"key\"] = 1\n")
    assert has_opcode?(code, Op.store_subscript())
  end

  # ===========================================================================
  # Lambda with No Params
  # ===========================================================================

  test "compiles lambda with no parameters" do
    code = Compiler.compile_starlark("f = lambda: 42\n")
    assert has_opcode?(code, Op.make_function())
  end
end
