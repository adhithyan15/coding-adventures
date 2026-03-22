defmodule CodingAdventures.StarlarkVm.VmTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.StarlarkVm
  alias CodingAdventures.StarlarkVm.Vm
  alias CodingAdventures.StarlarkVm.Handlers.StarlarkResult

  # Helper to execute and return result
  defp exec(source) do
    Vm.execute_starlark(source)
  end

  # ===========================================================================
  # Factory
  # ===========================================================================

  test "create_starlark_vm returns a configured VM" do
    vm = Vm.create_starlark_vm()
    assert map_size(vm.handlers) > 40
    assert map_size(vm.builtins) >= 23
  end

  test "create_starlark_vm with custom max recursion depth" do
    vm = Vm.create_starlark_vm(max_recursion_depth: 50)
    assert vm.max_recursion_depth == 50
  end

  # ===========================================================================
  # Integer Arithmetic
  # ===========================================================================

  test "integer addition" do
    result = exec("x = 1 + 2\n")
    assert result.variables["x"] == 3
  end

  test "integer subtraction" do
    result = exec("x = 10 - 3\n")
    assert result.variables["x"] == 7
  end

  test "integer multiplication" do
    result = exec("x = 4 * 5\n")
    assert result.variables["x"] == 20
  end

  test "integer floor division" do
    result = exec("x = 7 // 2\n")
    assert result.variables["x"] == 3
  end

  test "integer modulo" do
    result = exec("x = 10 % 3\n")
    assert result.variables["x"] == 1
  end

  test "integer power" do
    result = exec("x = 2 ** 10\n")
    assert result.variables["x"] == 1024
  end

  test "integer negation" do
    result = exec("x = -5\n")
    assert result.variables["x"] == -5
  end

  # ===========================================================================
  # Float Arithmetic
  # ===========================================================================

  test "float literal" do
    result = exec("x = 3.14\n")
    assert_in_delta result.variables["x"], 3.14, 0.001
  end

  test "float division" do
    result = exec("x = 10 / 4\n")
    assert result.variables["x"] == 2.5
  end

  # ===========================================================================
  # String Operations
  # ===========================================================================

  test "string assignment" do
    result = exec("x = \"hello\"\n")
    assert result.variables["x"] == "hello"
  end

  test "string concatenation" do
    result = exec("x = \"hello\" + \" world\"\n")
    assert result.variables["x"] == "hello world"
  end

  test "string repetition" do
    result = exec("x = \"ab\" * 3\n")
    assert result.variables["x"] == "ababab"
  end

  # ===========================================================================
  # Boolean Operations
  # ===========================================================================

  test "True literal" do
    result = exec("x = True\n")
    assert result.variables["x"] == true
  end

  test "False literal" do
    result = exec("x = False\n")
    assert result.variables["x"] == false
  end

  test "None literal" do
    result = exec("x = None\n")
    assert result.variables["x"] == nil
  end

  test "not True" do
    result = exec("x = not True\n")
    assert result.variables["x"] == false
  end

  test "not False" do
    result = exec("x = not False\n")
    assert result.variables["x"] == true
  end

  # ===========================================================================
  # Comparison Operations
  # ===========================================================================

  test "equal comparison" do
    result = exec("x = 1 == 1\n")
    assert result.variables["x"] == true
  end

  test "not equal comparison" do
    result = exec("x = 1 != 2\n")
    assert result.variables["x"] == true
  end

  test "less than comparison" do
    result = exec("x = 1 < 2\n")
    assert result.variables["x"] == true
  end

  test "greater than comparison" do
    result = exec("x = 2 > 1\n")
    assert result.variables["x"] == true
  end

  test "less or equal comparison" do
    result = exec("x = 2 <= 2\n")
    assert result.variables["x"] == true
  end

  test "greater or equal comparison" do
    result = exec("x = 3 >= 2\n")
    assert result.variables["x"] == true
  end

  # ===========================================================================
  # Bitwise Operations
  # ===========================================================================

  test "bitwise and" do
    result = exec("x = 5 & 3\n")
    assert result.variables["x"] == 1
  end

  test "bitwise or" do
    result = exec("x = 5 | 3\n")
    assert result.variables["x"] == 7
  end

  test "bitwise xor" do
    result = exec("x = 5 ^ 3\n")
    assert result.variables["x"] == 6
  end

  test "left shift" do
    result = exec("x = 1 << 3\n")
    assert result.variables["x"] == 8
  end

  test "right shift" do
    result = exec("x = 8 >> 2\n")
    assert result.variables["x"] == 2
  end

  # ===========================================================================
  # Short-Circuit Boolean
  # ===========================================================================

  test "and short-circuit false" do
    result = exec("x = False and True\n")
    assert result.variables["x"] == false
  end

  test "or short-circuit true" do
    result = exec("x = True or False\n")
    assert result.variables["x"] == true
  end

  # ===========================================================================
  # Variable Operations
  # ===========================================================================

  test "variable assignment and lookup" do
    result = exec("x = 42\ny = x\n")
    assert result.variables["x"] == 42
    assert result.variables["y"] == 42
  end

  test "multiple assignments" do
    result = exec("a = 1\nb = 2\nc = a + b\n")
    assert result.variables["c"] == 3
  end

  test "augmented assignment +=" do
    result = exec("x = 5\nx += 3\n")
    assert result.variables["x"] == 8
  end

  test "augmented assignment -=" do
    result = exec("x = 10\nx -= 3\n")
    assert result.variables["x"] == 7
  end

  test "augmented assignment *=" do
    result = exec("x = 4\nx *= 3\n")
    assert result.variables["x"] == 12
  end

  # ===========================================================================
  # Collections
  # ===========================================================================

  test "empty list" do
    result = exec("x = []\n")
    assert result.variables["x"] == []
  end

  test "list with elements" do
    result = exec("x = [1, 2, 3]\n")
    assert result.variables["x"] == [1, 2, 3]
  end

  test "list concatenation" do
    result = exec("x = [1, 2] + [3, 4]\n")
    assert result.variables["x"] == [1, 2, 3, 4]
  end

  test "empty dict" do
    result = exec("x = {}\n")
    assert result.variables["x"] == %{}
  end

  test "dict with entries" do
    result = exec("x = {\"a\": 1, \"b\": 2}\n")
    assert result.variables["x"] == %{"a" => 1, "b" => 2}
  end

  test "empty tuple" do
    result = exec("x = ()\n")
    assert result.variables["x"] == {}
  end

  # ===========================================================================
  # Control Flow
  # ===========================================================================

  test "if statement true branch" do
    result = exec("x = 0\nif True:\n  x = 1\n")
    assert result.variables["x"] == 1
  end

  test "if statement false branch skips body" do
    result = exec("x = 0\nif False:\n  x = 1\n")
    assert result.variables["x"] == 0
  end

  # ===========================================================================
  # Iteration
  # ===========================================================================

  test "for loop over list" do
    result = exec("total = 0\nfor x in [1, 2, 3]:\n  total = total + x\n")
    assert result.variables["total"] == 6
  end

  # ===========================================================================
  # Built-in Functions
  # ===========================================================================

  test "len of list" do
    result = exec("x = len([1, 2, 3])\n")
    assert result.variables["x"] == 3
  end

  test "len of string" do
    result = exec("x = len(\"hello\")\n")
    assert result.variables["x"] == 5
  end

  test "len of dict" do
    result = exec("x = len({\"a\": 1})\n")
    assert result.variables["x"] == 1
  end

  test "range with one arg" do
    result = exec("x = range(5)\n")
    assert result.variables["x"] == [0, 1, 2, 3, 4]
  end

  test "range with two args" do
    result = exec("x = range(2, 5)\n")
    assert result.variables["x"] == [2, 3, 4]
  end

  test "range with step" do
    result = exec("x = range(0, 10, 2)\n")
    assert result.variables["x"] == [0, 2, 4, 6, 8]
  end

  test "type of int" do
    result = exec("x = type(42)\n")
    assert result.variables["x"] == "int"
  end

  test "type of string" do
    result = exec("x = type(\"hi\")\n")
    assert result.variables["x"] == "string"
  end

  test "type of bool" do
    result = exec("x = type(True)\n")
    assert result.variables["x"] == "bool"
  end

  test "type of None" do
    result = exec("x = type(None)\n")
    assert result.variables["x"] == "NoneType"
  end

  test "type of list" do
    result = exec("x = type([])\n")
    assert result.variables["x"] == "list"
  end

  test "type of dict" do
    result = exec("x = type({})\n")
    assert result.variables["x"] == "dict"
  end

  test "bool of truthy value" do
    result = exec("x = bool(1)\n")
    assert result.variables["x"] == true
  end

  test "bool of falsy value" do
    result = exec("x = bool(0)\n")
    assert result.variables["x"] == false
  end

  test "int conversion from float" do
    result = exec("x = int(3.14)\n")
    assert result.variables["x"] == 3
  end

  test "int conversion from string" do
    result = exec("x = int(\"42\")\n")
    assert result.variables["x"] == 42
  end

  test "str conversion" do
    result = exec("x = str(42)\n")
    assert result.variables["x"] == "42"
  end

  test "sorted list" do
    result = exec("x = sorted([3, 1, 2])\n")
    assert result.variables["x"] == [1, 2, 3]
  end

  test "reversed list" do
    result = exec("x = reversed([1, 2, 3])\n")
    assert result.variables["x"] == [3, 2, 1]
  end

  test "min of args" do
    result = exec("x = min(3, 1, 2)\n")
    assert result.variables["x"] == 1
  end

  test "max of args" do
    result = exec("x = max(3, 1, 2)\n")
    assert result.variables["x"] == 3
  end

  test "abs of negative" do
    result = exec("x = abs(-5)\n")
    assert result.variables["x"] == 5
  end

  test "all with all truthy" do
    result = exec("x = all([1, 2, 3])\n")
    assert result.variables["x"] == true
  end

  test "all with a falsy" do
    result = exec("x = all([1, 0, 3])\n")
    assert result.variables["x"] == false
  end

  test "any with a truthy" do
    result = exec("x = any([0, 0, 1])\n")
    assert result.variables["x"] == true
  end

  test "any with all falsy" do
    result = exec("x = any([0, 0, 0])\n")
    assert result.variables["x"] == false
  end

  # ===========================================================================
  # Expression Statements
  # ===========================================================================

  test "expression statement evaluated for side effects" do
    result = exec("x = 1\n42\n")
    assert result.variables["x"] == 1
  end

  # ===========================================================================
  # Complex Programs
  # ===========================================================================

  test "sum of range" do
    result = exec("total = 0\nfor i in range(5):\n  total = total + i\n")
    assert result.variables["total"] == 10
  end

  test "nested arithmetic" do
    result = exec("x = 2 + 3 * 4\n")
    assert result.variables["x"] == 14
  end

  test "string in list" do
    result = exec("x = [\"a\", \"b\", \"c\"]\n")
    assert result.variables["x"] == ["a", "b", "c"]
  end

  test "pass statement does nothing" do
    result = exec("pass\nx = 1\n")
    assert result.variables["x"] == 1
  end

  test "list comprehension" do
    result = exec("x = [i for i in [1, 2, 3]]\n")
    assert result.variables["x"] == [1, 2, 3]
  end

  # ===========================================================================
  # StarlarkVm Delegator Module Tests
  # ===========================================================================

  test "StarlarkVm.create_starlark_vm/0 delegates correctly" do
    vm = StarlarkVm.create_starlark_vm()
    assert map_size(vm.handlers) > 40
    assert map_size(vm.builtins) >= 23
  end

  test "StarlarkVm.create_starlark_vm/1 delegates with opts" do
    vm = StarlarkVm.create_starlark_vm(max_recursion_depth: 100)
    assert vm.max_recursion_depth == 100
  end

  test "StarlarkVm.execute_starlark/1 delegates correctly" do
    result = StarlarkVm.execute_starlark("x = 42\n")
    assert %StarlarkResult{} = result
    assert result.variables["x"] == 42
  end

  # ===========================================================================
  # Additional Coverage — Frozen VM
  # ===========================================================================

  test "create_starlark_vm with frozen option" do
    vm = Vm.create_starlark_vm(frozen: true)
    assert vm.frozen == true
  end

  # ===========================================================================
  # Function Definition and Call
  # ===========================================================================

  test "simple function definition and call" do
    result = exec("def add(a, b):\n  return a + b\nx = add(3, 4)\n")
    assert result.variables["x"] == 7
  end

  test "function with default arguments" do
    result = exec("def greet(name, greeting = \"hello\"):\n  return greeting + \" \" + name\nx = greet(\"world\")\n")
    assert result.variables["x"] == "hello world"
  end

  # ===========================================================================
  # Subscript Operations via execute
  # ===========================================================================

  test "list subscript" do
    result = exec("x = [10, 20, 30]\ny = x[1]\n")
    assert result.variables["y"] == 20
  end

  test "dict subscript" do
    result = exec("x = {\"a\": 1}\ny = x[\"a\"]\n")
    assert result.variables["y"] == 1
  end

  test "string subscript" do
    result = exec("x = \"hello\"\ny = x[0]\n")
    assert result.variables["y"] == "h"
  end

  # ===========================================================================
  # Membership Testing via execute
  # ===========================================================================

  test "in operator with list" do
    result = exec("x = 2 in [1, 2, 3]\n")
    assert result.variables["x"] == true
  end

  test "not in operator with list" do
    result = exec("x = 5 not in [1, 2, 3]\n")
    assert result.variables["x"] == true
  end

  # ===========================================================================
  # If/Else via execute
  # ===========================================================================

  test "if else statement" do
    result = exec("x = 0\nif False:\n  x = 1\nelse:\n  x = 2\n")
    assert result.variables["x"] == 2
  end

  # ===========================================================================
  # Print via execute
  # ===========================================================================

  test "print function captures output" do
    result = exec("print(42)\n")
    assert result.output == ["42"]
  end

  test "print multiple args" do
    result = exec("print(1, 2, 3)\n")
    assert result.output == ["1 2 3"]
  end

  # ===========================================================================
  # Division by zero
  # ===========================================================================

  test "division by zero raises" do
    assert_raise CodingAdventures.VirtualMachine.Errors.DivisionByZeroError, fn ->
      exec("x = 1 / 0\n")
    end
  end

  # ===========================================================================
  # Bitwise not via execute
  # ===========================================================================

  test "bitwise not" do
    result = exec("x = ~0\n")
    assert result.variables["x"] == -1
  end

  # ===========================================================================
  # Multiple prints
  # ===========================================================================

  test "multiple print statements" do
    result = exec("print(\"a\")\nprint(\"b\")\n")
    # Output is reversed in execute_starlark (Enum.reverse on vm.output)
    assert result.output == ["b", "a"]
  end
end
