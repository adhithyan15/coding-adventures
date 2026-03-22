defmodule CodingAdventures.StarlarkVm.HandlersTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.StarlarkVm.Handlers
  alias CodingAdventures.StarlarkVm.Handlers.{StarlarkFunction, StarlarkIterator, StarlarkResult}
  alias CodingAdventures.VirtualMachine.GenericVM
  alias CodingAdventures.VirtualMachine.Errors

  # ===========================================================================
  # Struct Tests
  # ===========================================================================

  test "StarlarkFunction has correct default fields" do
    func = %StarlarkFunction{}
    assert func.name == "<lambda>"
    assert func.code == nil
    assert func.params == []
    assert func.defaults == []
    assert func.closure_cells == []
  end

  test "StarlarkIterator has correct default fields" do
    iter = %StarlarkIterator{}
    assert iter.items == []
    assert iter.index == 0
  end

  test "StarlarkResult has correct default fields" do
    result = %StarlarkResult{}
    assert result.variables == %{}
    assert result.output == []
    assert result.traces == []
  end

  # ===========================================================================
  # Truthiness
  # ===========================================================================

  test "nil is falsy" do
    assert Handlers.truthy?(nil) == false
  end

  test "false is falsy" do
    assert Handlers.truthy?(false) == false
  end

  test "0 is falsy" do
    assert Handlers.truthy?(0) == false
  end

  test "0.0 is falsy" do
    assert Handlers.truthy?(0.0) == false
  end

  test "empty string is falsy" do
    assert Handlers.truthy?("") == false
  end

  test "empty list is falsy" do
    assert Handlers.truthy?([]) == false
  end

  test "empty map is falsy" do
    assert Handlers.truthy?(%{}) == false
  end

  test "empty tuple is falsy" do
    assert Handlers.truthy?({}) == false
  end

  test "1 is truthy" do
    assert Handlers.truthy?(1) == true
  end

  test "non-empty string is truthy" do
    assert Handlers.truthy?("hello") == true
  end

  test "non-empty list is truthy" do
    assert Handlers.truthy?([1]) == true
  end

  test "non-empty map is truthy" do
    assert Handlers.truthy?(%{a: 1}) == true
  end

  test "true is truthy" do
    assert Handlers.truthy?(true) == true
  end

  test "non-empty tuple is truthy" do
    assert Handlers.truthy?({1, 2}) == true
  end

  test "negative float is truthy" do
    assert Handlers.truthy?(-1.5) == true
  end

  # ===========================================================================
  # starlark_repr
  # ===========================================================================

  test "repr of nil" do
    assert Handlers.starlark_repr(nil) == "None"
  end

  test "repr of true" do
    assert Handlers.starlark_repr(true) == "True"
  end

  test "repr of false" do
    assert Handlers.starlark_repr(false) == "False"
  end

  test "repr of integer" do
    assert Handlers.starlark_repr(42) == "42"
  end

  test "repr of string" do
    assert Handlers.starlark_repr("hello") == "hello"
  end

  test "repr of list" do
    result = Handlers.starlark_repr([1, 2, 3])
    assert result == "[1, 2, 3]"
  end

  test "repr of empty list" do
    assert Handlers.starlark_repr([]) == "[]"
  end

  test "repr of float" do
    result = Handlers.starlark_repr(3.14)
    assert is_binary(result)
    assert String.contains?(result, "3.14")
  end

  test "repr of tuple" do
    result = Handlers.starlark_repr({1, 2, 3})
    assert result == "(1, 2, 3)"
  end

  test "repr of empty tuple" do
    assert Handlers.starlark_repr({}) == "()"
  end

  test "repr of map" do
    result = Handlers.starlark_repr(%{"a" => 1})
    assert String.contains?(result, "\"a\"")
    assert String.contains?(result, "1")
  end

  test "repr of empty map" do
    assert Handlers.starlark_repr(%{}) == "{}"
  end

  test "repr of list with strings" do
    result = Handlers.starlark_repr(["a", "b"])
    assert result == "[\"a\", \"b\"]"
  end

  test "repr of tuple with strings" do
    result = Handlers.starlark_repr({"hello"})
    assert result == "(\"hello\")"
  end

  test "repr of unknown type falls back to inspect" do
    result = Handlers.starlark_repr(:some_atom)
    assert is_binary(result)
  end

  # ===========================================================================
  # Handler Tests — using GenericVM directly
  # ===========================================================================

  # Helper to create a minimal VM and code for testing handlers
  defp make_vm(stack \\ []) do
    vm = GenericVM.new()
    Enum.reduce(stack, vm, fn val, acc -> GenericVM.push(acc, val) end)
  end

  defp make_instr(operand \\ nil) do
    %{opcode: 0, operand: operand}
  end

  defp make_code(constants \\ [], names \\ []) do
    %{constants: constants, names: names, instructions: []}
  end

  # ---------------------------------------------------------------------------
  # Stack Operations
  # ---------------------------------------------------------------------------

  test "handle_load_const pushes constant onto stack" do
    vm = make_vm()
    code = make_code([42, "hello"])
    {output, vm} = Handlers.handle_load_const(vm, make_instr(0), code)
    assert output == nil
    assert GenericVM.peek(vm) == 42
  end

  test "handle_pop removes top of stack" do
    vm = make_vm([10, 20])
    {output, vm} = Handlers.handle_pop(vm, make_instr(), make_code())
    assert output == nil
    assert GenericVM.peek(vm) == 10
  end

  test "handle_dup duplicates top of stack" do
    vm = make_vm([42])
    {output, vm} = Handlers.handle_dup(vm, make_instr(), make_code())
    assert output == nil
    {top, vm} = GenericVM.pop(vm)
    assert top == 42
    assert GenericVM.peek(vm) == 42
  end

  test "handle_load_none pushes nil" do
    vm = make_vm()
    {_output, vm} = Handlers.handle_load_none(vm, make_instr(), make_code())
    assert GenericVM.peek(vm) == nil
  end

  test "handle_load_true pushes true" do
    vm = make_vm()
    {_output, vm} = Handlers.handle_load_true(vm, make_instr(), make_code())
    assert GenericVM.peek(vm) == true
  end

  test "handle_load_false pushes false" do
    vm = make_vm()
    {_output, vm} = Handlers.handle_load_false(vm, make_instr(), make_code())
    assert GenericVM.peek(vm) == false
  end

  # ---------------------------------------------------------------------------
  # Variable Operations
  # ---------------------------------------------------------------------------

  test "handle_store_name and handle_load_name roundtrip" do
    vm = make_vm([99])
    code = make_code([], ["x"])
    {_output, vm} = Handlers.handle_store_name(vm, make_instr(0), code)
    assert vm.variables["x"] == 99
  end

  test "handle_load_name loads from variables" do
    vm = make_vm()
    vm = %{vm | variables: %{"x" => 42}}
    code = make_code([], ["x"])
    {_output, vm} = Handlers.handle_load_name(vm, make_instr(0), code)
    assert GenericVM.peek(vm) == 42
  end

  test "handle_load_name loads builtin" do
    vm = make_vm()
    builtin_fn = fn _args, _vm -> 42 end
    vm = GenericVM.register_builtin(vm, "myfunc", builtin_fn)
    code = make_code([], ["myfunc"])
    {_output, vm} = Handlers.handle_load_name(vm, make_instr(0), code)
    {:builtin, _impl} = GenericVM.peek(vm)
  end

  test "handle_load_name raises for undefined name" do
    vm = make_vm()
    code = make_code([], ["undefined_var"])
    assert_raise Errors.UndefinedNameError, fn ->
      Handlers.handle_load_name(vm, make_instr(0), code)
    end
  end

  test "handle_store_local and handle_load_local roundtrip" do
    vm = make_vm([77])
    {_output, vm} = Handlers.handle_store_local(vm, make_instr(0), make_code())
    assert Enum.at(vm.locals, 0) == 77

    {_output, vm} = Handlers.handle_load_local(vm, make_instr(0), make_code())
    assert GenericVM.peek(vm) == 77
  end

  test "handle_store_local expands locals list when needed" do
    vm = make_vm([55])
    {_output, vm} = Handlers.handle_store_local(vm, make_instr(3), make_code())
    assert length(vm.locals) >= 4
    assert Enum.at(vm.locals, 3) == 55
  end

  test "handle_store_closure and handle_load_closure roundtrip" do
    vm = make_vm([88])
    {_output, vm} = Handlers.handle_store_closure(vm, make_instr(0), make_code())
    cells = GenericVM.get_extra(vm, :closure_cells, [])
    assert Enum.at(cells, 0) == 88

    {_output, vm} = Handlers.handle_load_closure(vm, make_instr(0), make_code())
    assert GenericVM.peek(vm) == 88
  end

  test "handle_store_closure expands closure cells" do
    vm = make_vm([33])
    {_output, vm} = Handlers.handle_store_closure(vm, make_instr(2), make_code())
    cells = GenericVM.get_extra(vm, :closure_cells, [])
    assert length(cells) >= 3
    assert Enum.at(cells, 2) == 33
  end

  # ---------------------------------------------------------------------------
  # Arithmetic Operations
  # ---------------------------------------------------------------------------

  test "handle_add with integers" do
    vm = make_vm([3, 4])
    {_output, vm} = Handlers.handle_add(vm, make_instr(), make_code())
    assert GenericVM.peek(vm) == 7
  end

  test "handle_add with strings" do
    vm = make_vm(["hello", " world"])
    {_output, vm} = Handlers.handle_add(vm, make_instr(), make_code())
    assert GenericVM.peek(vm) == "hello world"
  end

  test "handle_add with lists" do
    vm = make_vm([[1, 2], [3, 4]])
    {_output, vm} = Handlers.handle_add(vm, make_instr(), make_code())
    assert GenericVM.peek(vm) == [1, 2, 3, 4]
  end

  test "handle_add with mixed int and float" do
    vm = make_vm([1, 2.5])
    {_output, vm} = Handlers.handle_add(vm, make_instr(), make_code())
    assert GenericVM.peek(vm) == 3.5
  end

  test "handle_add raises on type error" do
    vm = make_vm([1, "hello"])
    assert_raise Errors.VMTypeError, fn ->
      Handlers.handle_add(vm, make_instr(), make_code())
    end
  end

  test "handle_sub subtracts numbers" do
    vm = make_vm([10, 3])
    {_output, vm} = Handlers.handle_sub(vm, make_instr(), make_code())
    assert GenericVM.peek(vm) == 7
  end

  test "handle_mul with integers" do
    vm = make_vm([4, 5])
    {_output, vm} = Handlers.handle_mul(vm, make_instr(), make_code())
    assert GenericVM.peek(vm) == 20
  end

  test "handle_mul string * int" do
    vm = make_vm(["ab", 3])
    {_output, vm} = Handlers.handle_mul(vm, make_instr(), make_code())
    assert GenericVM.peek(vm) == "ababab"
  end

  test "handle_mul int * string" do
    vm = make_vm([3, "xy"])
    {_output, vm} = Handlers.handle_mul(vm, make_instr(), make_code())
    assert GenericVM.peek(vm) == "xyxyxy"
  end

  test "handle_mul list * int" do
    vm = make_vm([[1, 2], 3])
    {_output, vm} = Handlers.handle_mul(vm, make_instr(), make_code())
    assert GenericVM.peek(vm) == [1, 2, 1, 2, 1, 2]
  end

  test "handle_mul int * list" do
    vm = make_vm([2, [1, 2]])
    {_output, vm} = Handlers.handle_mul(vm, make_instr(), make_code())
    assert GenericVM.peek(vm) == [1, 2, 1, 2]
  end

  test "handle_mul floats" do
    vm = make_vm([2.0, 3.0])
    {_output, vm} = Handlers.handle_mul(vm, make_instr(), make_code())
    assert GenericVM.peek(vm) == 6.0
  end

  test "handle_mul raises on type error" do
    vm = make_vm(["a", "b"])
    assert_raise Errors.VMTypeError, fn ->
      Handlers.handle_mul(vm, make_instr(), make_code())
    end
  end

  test "handle_div produces float" do
    vm = make_vm([10, 4])
    {_output, vm} = Handlers.handle_div(vm, make_instr(), make_code())
    assert GenericVM.peek(vm) == 2.5
  end

  test "handle_div raises on division by zero (int)" do
    vm = make_vm([10, 0])
    assert_raise Errors.DivisionByZeroError, fn ->
      Handlers.handle_div(vm, make_instr(), make_code())
    end
  end

  test "handle_div raises on division by zero (float)" do
    vm = make_vm([10, 0.0])
    assert_raise Errors.DivisionByZeroError, fn ->
      Handlers.handle_div(vm, make_instr(), make_code())
    end
  end

  test "handle_floor_div with integers" do
    vm = make_vm([7, 2])
    {_output, vm} = Handlers.handle_floor_div(vm, make_instr(), make_code())
    assert GenericVM.peek(vm) == 3
  end

  test "handle_floor_div with floats" do
    vm = make_vm([7.0, 2])
    {_output, vm} = Handlers.handle_floor_div(vm, make_instr(), make_code())
    assert GenericVM.peek(vm) == 3
  end

  test "handle_floor_div raises on zero" do
    vm = make_vm([10, 0])
    assert_raise Errors.DivisionByZeroError, fn ->
      Handlers.handle_floor_div(vm, make_instr(), make_code())
    end
  end

  test "handle_floor_div raises on zero float" do
    vm = make_vm([10, 0.0])
    assert_raise Errors.DivisionByZeroError, fn ->
      Handlers.handle_floor_div(vm, make_instr(), make_code())
    end
  end

  test "handle_mod with integers" do
    vm = make_vm([10, 3])
    {_output, vm} = Handlers.handle_mod(vm, make_instr(), make_code())
    assert GenericVM.peek(vm) == 1
  end

  test "handle_mod with string (format stub)" do
    vm = make_vm(["hello %s", "world"])
    {_output, vm} = Handlers.handle_mod(vm, make_instr(), make_code())
    assert GenericVM.peek(vm) == "hello %s"
  end

  test "handle_mod raises on zero" do
    vm = make_vm([10, 0])
    assert_raise Errors.DivisionByZeroError, fn ->
      Handlers.handle_mod(vm, make_instr(), make_code())
    end
  end

  test "handle_mod raises on zero float" do
    vm = make_vm([10, 0.0])
    assert_raise Errors.DivisionByZeroError, fn ->
      Handlers.handle_mod(vm, make_instr(), make_code())
    end
  end

  test "handle_power with integers" do
    vm = make_vm([2, 10])
    {_output, vm} = Handlers.handle_power(vm, make_instr(), make_code())
    assert GenericVM.peek(vm) == 1024
  end

  test "handle_power with float exponent" do
    vm = make_vm([4, 0.5])
    {_output, vm} = Handlers.handle_power(vm, make_instr(), make_code())
    assert_in_delta GenericVM.peek(vm), 2.0, 0.001
  end

  test "handle_power with zero exponent" do
    vm = make_vm([5, 0])
    {_output, vm} = Handlers.handle_power(vm, make_instr(), make_code())
    assert GenericVM.peek(vm) == 1
  end

  test "handle_negate negates a number" do
    vm = make_vm([5])
    {_output, vm} = Handlers.handle_negate(vm, make_instr(), make_code())
    assert GenericVM.peek(vm) == -5
  end

  test "handle_bit_and" do
    vm = make_vm([5, 3])
    {_output, vm} = Handlers.handle_bit_and(vm, make_instr(), make_code())
    assert GenericVM.peek(vm) == 1
  end

  test "handle_bit_or" do
    vm = make_vm([5, 3])
    {_output, vm} = Handlers.handle_bit_or(vm, make_instr(), make_code())
    assert GenericVM.peek(vm) == 7
  end

  test "handle_bit_xor" do
    vm = make_vm([5, 3])
    {_output, vm} = Handlers.handle_bit_xor(vm, make_instr(), make_code())
    assert GenericVM.peek(vm) == 6
  end

  test "handle_bit_not" do
    vm = make_vm([0])
    {_output, vm} = Handlers.handle_bit_not(vm, make_instr(), make_code())
    assert GenericVM.peek(vm) == -1
  end

  test "handle_lshift" do
    vm = make_vm([1, 3])
    {_output, vm} = Handlers.handle_lshift(vm, make_instr(), make_code())
    assert GenericVM.peek(vm) == 8
  end

  test "handle_rshift" do
    vm = make_vm([8, 2])
    {_output, vm} = Handlers.handle_rshift(vm, make_instr(), make_code())
    assert GenericVM.peek(vm) == 2
  end

  # ---------------------------------------------------------------------------
  # Comparison Operations
  # ---------------------------------------------------------------------------

  test "handle_cmp_eq true case" do
    vm = make_vm([5, 5])
    {_output, vm} = Handlers.handle_cmp_eq(vm, make_instr(), make_code())
    assert GenericVM.peek(vm) == true
  end

  test "handle_cmp_eq false case" do
    vm = make_vm([5, 6])
    {_output, vm} = Handlers.handle_cmp_eq(vm, make_instr(), make_code())
    assert GenericVM.peek(vm) == false
  end

  test "handle_cmp_ne" do
    vm = make_vm([5, 6])
    {_output, vm} = Handlers.handle_cmp_ne(vm, make_instr(), make_code())
    assert GenericVM.peek(vm) == true
  end

  test "handle_cmp_lt" do
    vm = make_vm([3, 5])
    {_output, vm} = Handlers.handle_cmp_lt(vm, make_instr(), make_code())
    assert GenericVM.peek(vm) == true
  end

  test "handle_cmp_gt" do
    vm = make_vm([5, 3])
    {_output, vm} = Handlers.handle_cmp_gt(vm, make_instr(), make_code())
    assert GenericVM.peek(vm) == true
  end

  test "handle_cmp_le true" do
    vm = make_vm([5, 5])
    {_output, vm} = Handlers.handle_cmp_le(vm, make_instr(), make_code())
    assert GenericVM.peek(vm) == true
  end

  test "handle_cmp_ge true" do
    vm = make_vm([5, 5])
    {_output, vm} = Handlers.handle_cmp_ge(vm, make_instr(), make_code())
    assert GenericVM.peek(vm) == true
  end

  test "handle_cmp_in with list" do
    vm = make_vm([2, [1, 2, 3]])
    {_output, vm} = Handlers.handle_cmp_in(vm, make_instr(), make_code())
    assert GenericVM.peek(vm) == true
  end

  test "handle_cmp_in with map (key check)" do
    vm = make_vm(["a", %{"a" => 1}])
    {_output, vm} = Handlers.handle_cmp_in(vm, make_instr(), make_code())
    assert GenericVM.peek(vm) == true
  end

  test "handle_cmp_in with string" do
    vm = make_vm(["ell", "hello"])
    {_output, vm} = Handlers.handle_cmp_in(vm, make_instr(), make_code())
    assert GenericVM.peek(vm) == true
  end

  test "handle_cmp_in with tuple" do
    vm = make_vm([2, {1, 2, 3}])
    {_output, vm} = Handlers.handle_cmp_in(vm, make_instr(), make_code())
    assert GenericVM.peek(vm) == true
  end

  test "handle_cmp_in fallback returns false" do
    vm = make_vm([1, 42])
    {_output, vm} = Handlers.handle_cmp_in(vm, make_instr(), make_code())
    assert GenericVM.peek(vm) == false
  end

  test "handle_cmp_not_in with list" do
    vm = make_vm([5, [1, 2, 3]])
    {_output, vm} = Handlers.handle_cmp_not_in(vm, make_instr(), make_code())
    assert GenericVM.peek(vm) == true
  end

  test "handle_cmp_not_in with map" do
    vm = make_vm(["b", %{"a" => 1}])
    {_output, vm} = Handlers.handle_cmp_not_in(vm, make_instr(), make_code())
    assert GenericVM.peek(vm) == true
  end

  test "handle_cmp_not_in with string" do
    vm = make_vm(["xyz", "hello"])
    {_output, vm} = Handlers.handle_cmp_not_in(vm, make_instr(), make_code())
    assert GenericVM.peek(vm) == true
  end

  test "handle_cmp_not_in with tuple" do
    vm = make_vm([5, {1, 2, 3}])
    {_output, vm} = Handlers.handle_cmp_not_in(vm, make_instr(), make_code())
    assert GenericVM.peek(vm) == true
  end

  test "handle_cmp_not_in fallback returns true" do
    vm = make_vm([1, 42])
    {_output, vm} = Handlers.handle_cmp_not_in(vm, make_instr(), make_code())
    assert GenericVM.peek(vm) == true
  end

  # ---------------------------------------------------------------------------
  # Boolean Operations
  # ---------------------------------------------------------------------------

  test "handle_not with truthy value" do
    vm = make_vm([1])
    {_output, vm} = Handlers.handle_not(vm, make_instr(), make_code())
    assert GenericVM.peek(vm) == false
  end

  test "handle_not with falsy value" do
    vm = make_vm([0])
    {_output, vm} = Handlers.handle_not(vm, make_instr(), make_code())
    assert GenericVM.peek(vm) == true
  end

  # ---------------------------------------------------------------------------
  # Control Flow
  # ---------------------------------------------------------------------------

  test "handle_jump sets pc to operand" do
    vm = make_vm()
    {_output, vm} = Handlers.handle_jump(vm, make_instr(5), make_code())
    assert vm.pc == 5
  end

  test "handle_jump_if_false jumps when falsy" do
    vm = make_vm([0])
    {_output, vm} = Handlers.handle_jump_if_false(vm, make_instr(10), make_code())
    assert vm.pc == 10
  end

  test "handle_jump_if_false advances when truthy" do
    vm = make_vm([1])
    {_output, vm} = Handlers.handle_jump_if_false(vm, make_instr(10), make_code())
    assert vm.pc == 1
  end

  test "handle_jump_if_true jumps when truthy" do
    vm = make_vm([1])
    {_output, vm} = Handlers.handle_jump_if_true(vm, make_instr(10), make_code())
    assert vm.pc == 10
  end

  test "handle_jump_if_true advances when falsy" do
    vm = make_vm([0])
    {_output, vm} = Handlers.handle_jump_if_true(vm, make_instr(10), make_code())
    assert vm.pc == 1
  end

  test "handle_jump_if_false_or_pop pops and continues when truthy" do
    vm = make_vm([1])
    {_output, vm} = Handlers.handle_jump_if_false_or_pop(vm, make_instr(10), make_code())
    assert vm.pc == 1
    assert vm.stack == []
  end

  test "handle_jump_if_false_or_pop keeps value and jumps when falsy" do
    vm = make_vm([0])
    {_output, vm} = Handlers.handle_jump_if_false_or_pop(vm, make_instr(10), make_code())
    assert vm.pc == 10
    assert GenericVM.peek(vm) == 0
  end

  test "handle_jump_if_true_or_pop keeps value and jumps when truthy" do
    vm = make_vm([1])
    {_output, vm} = Handlers.handle_jump_if_true_or_pop(vm, make_instr(10), make_code())
    assert vm.pc == 10
    assert GenericVM.peek(vm) == 1
  end

  test "handle_jump_if_true_or_pop pops and continues when falsy" do
    vm = make_vm([0])
    {_output, vm} = Handlers.handle_jump_if_true_or_pop(vm, make_instr(10), make_code())
    assert vm.pc == 1
    assert vm.stack == []
  end

  # ---------------------------------------------------------------------------
  # Collection Operations
  # ---------------------------------------------------------------------------

  test "handle_build_list creates list from stack" do
    vm = make_vm([1, 2, 3])
    {_output, vm} = Handlers.handle_build_list(vm, make_instr(3), make_code())
    assert GenericVM.peek(vm) == [1, 2, 3]
  end

  test "handle_build_list with zero creates empty list" do
    vm = make_vm()
    {_output, vm} = Handlers.handle_build_list(vm, make_instr(0), make_code())
    assert GenericVM.peek(vm) == []
  end

  test "handle_build_dict creates dict from stack pairs" do
    vm = make_vm(["a", 1, "b", 2])
    {_output, vm} = Handlers.handle_build_dict(vm, make_instr(2), make_code())
    assert GenericVM.peek(vm) == %{"a" => 1, "b" => 2}
  end

  test "handle_build_dict with zero creates empty dict" do
    vm = make_vm()
    {_output, vm} = Handlers.handle_build_dict(vm, make_instr(0), make_code())
    assert GenericVM.peek(vm) == %{}
  end

  test "handle_build_tuple creates tuple from stack" do
    vm = make_vm([1, 2, 3])
    {_output, vm} = Handlers.handle_build_tuple(vm, make_instr(3), make_code())
    assert GenericVM.peek(vm) == {1, 2, 3}
  end

  test "handle_build_tuple with zero creates empty tuple" do
    vm = make_vm()
    {_output, vm} = Handlers.handle_build_tuple(vm, make_instr(0), make_code())
    assert GenericVM.peek(vm) == {}
  end

  test "handle_list_append appends value to list" do
    # Stack bottom to top: [list, iterator, value]
    the_list = [1, 2]
    iter = %StarlarkIterator{items: [3], index: 0}
    vm = make_vm([the_list, iter, 99])
    {_output, vm} = Handlers.handle_list_append(vm, make_instr(), make_code())
    # After: stack should have [updated_list, iterator]
    {top_iter, vm} = GenericVM.pop(vm)
    assert %StarlarkIterator{} = top_iter
    assert GenericVM.peek(vm) == [1, 2, 99]
  end

  test "handle_dict_set sets dict entry" do
    the_dict = %{"a" => 1}
    iter = %StarlarkIterator{items: [], index: 0}
    vm = make_vm([the_dict, iter, "b", 2])
    {_output, vm} = Handlers.handle_dict_set(vm, make_instr(), make_code())
    {top_iter, vm} = GenericVM.pop(vm)
    assert %StarlarkIterator{} = top_iter
    result_dict = GenericVM.peek(vm)
    assert result_dict == %{"a" => 1, "b" => 2}
  end

  # ---------------------------------------------------------------------------
  # Subscript & Attribute Operations
  # ---------------------------------------------------------------------------

  test "handle_load_subscript on list with positive index" do
    vm = make_vm([[10, 20, 30], 1])
    {_output, vm} = Handlers.handle_load_subscript(vm, make_instr(), make_code())
    assert GenericVM.peek(vm) == 20
  end

  test "handle_load_subscript on list with negative index" do
    vm = make_vm([[10, 20, 30], -1])
    {_output, vm} = Handlers.handle_load_subscript(vm, make_instr(), make_code())
    assert GenericVM.peek(vm) == 30
  end

  test "handle_load_subscript on map" do
    vm = make_vm([%{"key" => "val"}, "key"])
    {_output, vm} = Handlers.handle_load_subscript(vm, make_instr(), make_code())
    assert GenericVM.peek(vm) == "val"
  end

  test "handle_load_subscript on string" do
    vm = make_vm(["hello", 1])
    {_output, vm} = Handlers.handle_load_subscript(vm, make_instr(), make_code())
    assert GenericVM.peek(vm) == "e"
  end

  test "handle_load_subscript on string with negative index" do
    vm = make_vm(["hello", -1])
    {_output, vm} = Handlers.handle_load_subscript(vm, make_instr(), make_code())
    assert GenericVM.peek(vm) == "o"
  end

  test "handle_load_subscript on tuple" do
    vm = make_vm([{10, 20, 30}, 1])
    {_output, vm} = Handlers.handle_load_subscript(vm, make_instr(), make_code())
    assert GenericVM.peek(vm) == 20
  end

  test "handle_load_subscript on tuple with negative index" do
    vm = make_vm([{10, 20, 30}, -1])
    {_output, vm} = Handlers.handle_load_subscript(vm, make_instr(), make_code())
    assert GenericVM.peek(vm) == 30
  end

  test "handle_load_subscript raises on non-subscriptable" do
    vm = make_vm([42, 0])
    assert_raise Errors.VMTypeError, fn ->
      Handlers.handle_load_subscript(vm, make_instr(), make_code())
    end
  end

  test "handle_store_subscript on list" do
    vm = make_vm([[10, 20, 30], 1, 99])
    {_output, vm} = Handlers.handle_store_subscript(vm, make_instr(), make_code())
    assert GenericVM.peek(vm) == [10, 99, 30]
  end

  test "handle_store_subscript on list with negative index" do
    vm = make_vm([[10, 20, 30], -1, 99])
    {_output, vm} = Handlers.handle_store_subscript(vm, make_instr(), make_code())
    assert GenericVM.peek(vm) == [10, 20, 99]
  end

  test "handle_store_subscript on map" do
    vm = make_vm([%{"a" => 1}, "b", 2])
    {_output, vm} = Handlers.handle_store_subscript(vm, make_instr(), make_code())
    assert GenericVM.peek(vm) == %{"a" => 1, "b" => 2}
  end

  test "handle_store_subscript raises on non-assignable" do
    vm = make_vm([42, 0, 99])
    assert_raise Errors.VMTypeError, fn ->
      Handlers.handle_store_subscript(vm, make_instr(), make_code())
    end
  end

  test "handle_load_attr on map" do
    vm = make_vm([%{"x" => 42}])
    code = make_code([], ["x"])
    {_output, vm} = Handlers.handle_load_attr(vm, make_instr(0), code)
    assert GenericVM.peek(vm) == 42
  end

  test "handle_load_attr string method upper" do
    vm = make_vm(["hello"])
    code = make_code([], ["upper"])
    {_output, vm} = Handlers.handle_load_attr(vm, make_instr(0), code)
    {:builtin, impl} = GenericVM.peek(vm)
    assert impl.([], nil) == "HELLO"
  end

  test "handle_load_attr string method lower" do
    vm = make_vm(["HELLO"])
    code = make_code([], ["lower"])
    {_output, vm} = Handlers.handle_load_attr(vm, make_instr(0), code)
    {:builtin, impl} = GenericVM.peek(vm)
    assert impl.([], nil) == "hello"
  end

  test "handle_load_attr string method strip" do
    vm = make_vm(["  hello  "])
    code = make_code([], ["strip"])
    {_output, vm} = Handlers.handle_load_attr(vm, make_instr(0), code)
    {:builtin, impl} = GenericVM.peek(vm)
    assert impl.([], nil) == "hello"
  end

  test "handle_load_attr string method lstrip" do
    vm = make_vm(["  hello  "])
    code = make_code([], ["lstrip"])
    {_output, vm} = Handlers.handle_load_attr(vm, make_instr(0), code)
    {:builtin, impl} = GenericVM.peek(vm)
    assert impl.([], nil) == "hello  "
  end

  test "handle_load_attr string method rstrip" do
    vm = make_vm(["  hello  "])
    code = make_code([], ["rstrip"])
    {_output, vm} = Handlers.handle_load_attr(vm, make_instr(0), code)
    {:builtin, impl} = GenericVM.peek(vm)
    assert impl.([], nil) == "  hello"
  end

  test "handle_load_attr string method startswith" do
    vm = make_vm(["hello"])
    code = make_code([], ["startswith"])
    {_output, vm} = Handlers.handle_load_attr(vm, make_instr(0), code)
    {:builtin, impl} = GenericVM.peek(vm)
    assert impl.(["hel"], nil) == true
    assert impl.(["xyz"], nil) == false
  end

  test "handle_load_attr string method endswith" do
    vm = make_vm(["hello"])
    code = make_code([], ["endswith"])
    {_output, vm} = Handlers.handle_load_attr(vm, make_instr(0), code)
    {:builtin, impl} = GenericVM.peek(vm)
    assert impl.(["llo"], nil) == true
    assert impl.(["xyz"], nil) == false
  end

  test "handle_load_attr string method replace" do
    vm = make_vm(["hello world"])
    code = make_code([], ["replace"])
    {_output, vm} = Handlers.handle_load_attr(vm, make_instr(0), code)
    {:builtin, impl} = GenericVM.peek(vm)
    assert impl.(["world", "elixir"], nil) == "hello elixir"
  end

  test "handle_load_attr string method split with no args" do
    vm = make_vm(["hello world"])
    code = make_code([], ["split"])
    {_output, vm} = Handlers.handle_load_attr(vm, make_instr(0), code)
    {:builtin, impl} = GenericVM.peek(vm)
    assert impl.([], nil) == ["hello", "world"]
  end

  test "handle_load_attr string method split with separator" do
    vm = make_vm(["a,b,c"])
    code = make_code([], ["split"])
    {_output, vm} = Handlers.handle_load_attr(vm, make_instr(0), code)
    {:builtin, impl} = GenericVM.peek(vm)
    assert impl.([","], nil) == ["a", "b", "c"]
  end

  test "handle_load_attr string method join" do
    vm = make_vm([", "])
    code = make_code([], ["join"])
    {_output, vm} = Handlers.handle_load_attr(vm, make_instr(0), code)
    {:builtin, impl} = GenericVM.peek(vm)
    assert impl.([["a", "b", "c"]], nil) == "a, b, c"
  end

  test "handle_load_attr string method find" do
    vm = make_vm(["hello world"])
    code = make_code([], ["find"])
    {_output, vm} = Handlers.handle_load_attr(vm, make_instr(0), code)
    {:builtin, impl} = GenericVM.peek(vm)
    assert impl.(["world"], nil) == 6
    assert impl.(["xyz"], nil) == -1
  end

  test "handle_load_attr string method count" do
    vm = make_vm(["banana"])
    code = make_code([], ["count"])
    {_output, vm} = Handlers.handle_load_attr(vm, make_instr(0), code)
    {:builtin, impl} = GenericVM.peek(vm)
    assert impl.(["an"], nil) == 2
  end

  test "handle_load_attr string method title" do
    vm = make_vm(["hello world"])
    code = make_code([], ["title"])
    {_output, vm} = Handlers.handle_load_attr(vm, make_instr(0), code)
    {:builtin, impl} = GenericVM.peek(vm)
    assert impl.([], nil) == "Hello World"
  end

  test "handle_load_attr string method isdigit" do
    vm = make_vm(["123"])
    code = make_code([], ["isdigit"])
    {_output, vm} = Handlers.handle_load_attr(vm, make_instr(0), code)
    {:builtin, impl} = GenericVM.peek(vm)
    assert impl.([], nil) == true
  end

  test "handle_load_attr string method isdigit false" do
    vm = make_vm(["abc"])
    code = make_code([], ["isdigit"])
    {_output, vm} = Handlers.handle_load_attr(vm, make_instr(0), code)
    {:builtin, impl} = GenericVM.peek(vm)
    assert impl.([], nil) == false
  end

  test "handle_load_attr string method isalpha" do
    vm = make_vm(["abc"])
    code = make_code([], ["isalpha"])
    {_output, vm} = Handlers.handle_load_attr(vm, make_instr(0), code)
    {:builtin, impl} = GenericVM.peek(vm)
    assert impl.([], nil) == true
  end

  test "handle_load_attr string method isalpha false" do
    vm = make_vm(["abc123"])
    code = make_code([], ["isalpha"])
    {_output, vm} = Handlers.handle_load_attr(vm, make_instr(0), code)
    {:builtin, impl} = GenericVM.peek(vm)
    assert impl.([], nil) == false
  end

  test "handle_load_attr string method unknown raises" do
    vm = make_vm(["hello"])
    code = make_code([], ["nonexistent"])
    assert_raise Errors.VMTypeError, ~r/str has no method/, fn ->
      Handlers.handle_load_attr(vm, make_instr(0), code)
    end
  end

  test "handle_load_attr list method append" do
    vm = make_vm([[1, 2]])
    code = make_code([], ["append"])
    {_output, vm} = Handlers.handle_load_attr(vm, make_instr(0), code)
    {:builtin, impl} = GenericVM.peek(vm)
    assert impl.([3], nil) == [1, 2, 3]
  end

  test "handle_load_attr list method extend" do
    vm = make_vm([[1, 2]])
    code = make_code([], ["extend"])
    {_output, vm} = Handlers.handle_load_attr(vm, make_instr(0), code)
    {:builtin, impl} = GenericVM.peek(vm)
    assert impl.([[3, 4]], nil) == [1, 2, 3, 4]
  end

  test "handle_load_attr list method insert" do
    vm = make_vm([[1, 3]])
    code = make_code([], ["insert"])
    {_output, vm} = Handlers.handle_load_attr(vm, make_instr(0), code)
    {:builtin, impl} = GenericVM.peek(vm)
    assert impl.([1, 2], nil) == [1, 2, 3]
  end

  test "handle_load_attr list method remove" do
    vm = make_vm([[1, 2, 3]])
    code = make_code([], ["remove"])
    {_output, vm} = Handlers.handle_load_attr(vm, make_instr(0), code)
    {:builtin, impl} = GenericVM.peek(vm)
    assert impl.([2], nil) == [1, 3]
  end

  test "handle_load_attr list method pop with no args" do
    vm = make_vm([[1, 2, 3]])
    code = make_code([], ["pop"])
    {_output, vm} = Handlers.handle_load_attr(vm, make_instr(0), code)
    {:builtin, impl} = GenericVM.peek(vm)
    assert impl.([], nil) == 3
  end

  test "handle_load_attr list method pop with index" do
    vm = make_vm([[1, 2, 3]])
    code = make_code([], ["pop"])
    {_output, vm} = Handlers.handle_load_attr(vm, make_instr(0), code)
    {:builtin, impl} = GenericVM.peek(vm)
    assert impl.([0], nil) == 1
  end

  test "handle_load_attr list method index" do
    vm = make_vm([[10, 20, 30]])
    code = make_code([], ["index"])
    {_output, vm} = Handlers.handle_load_attr(vm, make_instr(0), code)
    {:builtin, impl} = GenericVM.peek(vm)
    assert impl.([20], nil) == 1
  end

  test "handle_load_attr list method index raises for missing item" do
    vm = make_vm([[10, 20, 30]])
    code = make_code([], ["index"])
    {_output, vm} = Handlers.handle_load_attr(vm, make_instr(0), code)
    {:builtin, impl} = GenericVM.peek(vm)
    assert_raise Errors.VMTypeError, fn ->
      impl.([99], nil)
    end
  end

  test "handle_load_attr list method unknown raises" do
    vm = make_vm([[1, 2]])
    code = make_code([], ["nonexistent"])
    assert_raise Errors.VMTypeError, ~r/list has no method/, fn ->
      Handlers.handle_load_attr(vm, make_instr(0), code)
    end
  end

  test "handle_load_attr dict method keys" do
    vm = make_vm([%{"a" => 1, "b" => 2}])
    # Map without the attr key, so it falls through to dict methods
    code = make_code([], ["keys"])
    {_output, vm} = Handlers.handle_load_attr(vm, make_instr(0), code)
    {:builtin, impl} = GenericVM.peek(vm)
    result = impl.([], nil)
    assert Enum.sort(result) == ["a", "b"]
  end

  test "handle_load_attr dict method values" do
    vm = make_vm([%{"a" => 1, "b" => 2}])
    code = make_code([], ["values"])
    {_output, vm} = Handlers.handle_load_attr(vm, make_instr(0), code)
    {:builtin, impl} = GenericVM.peek(vm)
    result = impl.([], nil)
    assert Enum.sort(result) == [1, 2]
  end

  test "handle_load_attr dict method items" do
    vm = make_vm([%{"a" => 1}])
    code = make_code([], ["items"])
    {_output, vm} = Handlers.handle_load_attr(vm, make_instr(0), code)
    {:builtin, impl} = GenericVM.peek(vm)
    assert impl.([], nil) == [{"a", 1}]
  end

  test "handle_load_attr dict method get with key" do
    vm = make_vm([%{"a" => 1}])
    code = make_code([], ["get"])
    {_output, vm} = Handlers.handle_load_attr(vm, make_instr(0), code)
    {:builtin, impl} = GenericVM.peek(vm)
    assert impl.(["a"], nil) == 1
    assert impl.(["b"], nil) == nil
  end

  test "handle_load_attr dict method get with default" do
    vm = make_vm([%{"a" => 1}])
    code = make_code([], ["get"])
    {_output, vm} = Handlers.handle_load_attr(vm, make_instr(0), code)
    {:builtin, impl} = GenericVM.peek(vm)
    assert impl.(["b", 42], nil) == 42
  end

  test "handle_load_attr dict method pop" do
    vm = make_vm([%{"a" => 1}])
    code = make_code([], ["pop"])
    {_output, vm} = Handlers.handle_load_attr(vm, make_instr(0), code)
    {:builtin, impl} = GenericVM.peek(vm)
    assert impl.(["a"], nil) == 1
    assert impl.(["b", 42], nil) == 42
  end

  test "handle_load_attr dict method update" do
    vm = make_vm([%{"a" => 1}])
    code = make_code([], ["update"])
    {_output, vm} = Handlers.handle_load_attr(vm, make_instr(0), code)
    {:builtin, impl} = GenericVM.peek(vm)
    assert impl.([%{"b" => 2}], nil) == %{"a" => 1, "b" => 2}
  end

  test "handle_load_attr dict method unknown raises" do
    vm = make_vm([%{}])
    code = make_code([], ["nonexistent"])
    assert_raise Errors.VMTypeError, ~r/dict has no method/, fn ->
      Handlers.handle_load_attr(vm, make_instr(0), code)
    end
  end

  test "handle_load_attr raises on non-object" do
    vm = make_vm([42])
    code = make_code([], ["something"])
    assert_raise Errors.VMTypeError, ~r/has no attribute/, fn ->
      Handlers.handle_load_attr(vm, make_instr(0), code)
    end
  end

  test "handle_store_attr on map" do
    vm = make_vm([%{"a" => 1}, 42])
    code = make_code([], ["b"])
    {_output, vm} = Handlers.handle_store_attr(vm, make_instr(0), code)
    assert GenericVM.peek(vm) == %{"a" => 1, "b" => 42}
  end

  test "handle_store_attr raises on non-map" do
    vm = make_vm([42, 99])
    code = make_code([], ["x"])
    assert_raise Errors.VMTypeError, ~r/Cannot set attribute/, fn ->
      Handlers.handle_store_attr(vm, make_instr(0), code)
    end
  end

  # ---------------------------------------------------------------------------
  # Slice Operations
  # ---------------------------------------------------------------------------

  test "handle_load_slice on list with start and stop" do
    vm = make_vm([[10, 20, 30, 40, 50], 1, 4])
    {_output, vm} = Handlers.handle_load_slice(vm, make_instr(2), make_code())
    assert GenericVM.peek(vm) == [20, 30, 40]
  end

  test "handle_load_slice on list with start only" do
    vm = make_vm([[10, 20, 30], 1])
    {_output, vm} = Handlers.handle_load_slice(vm, make_instr(1), make_code())
    # start=1, stop=nil(default to len), step=nil
    # But with only 1 component, it's [s] -> {s, nil, nil}
    # This means slice from index 1 to end
    result = GenericVM.peek(vm)
    assert is_list(result)
  end

  test "handle_load_slice on list with negative indices" do
    vm = make_vm([[10, 20, 30, 40, 50], -3, -1])
    {_output, vm} = Handlers.handle_load_slice(vm, make_instr(2), make_code())
    assert GenericVM.peek(vm) == [30, 40]
  end

  test "handle_load_slice on string" do
    vm = make_vm(["hello", 1, 4])
    {_output, vm} = Handlers.handle_load_slice(vm, make_instr(2), make_code())
    assert GenericVM.peek(vm) == "ell"
  end

  test "handle_load_slice on string with negative index" do
    vm = make_vm(["hello", -3, -1])
    {_output, vm} = Handlers.handle_load_slice(vm, make_instr(2), make_code())
    assert GenericVM.peek(vm) == "ll"
  end

  test "handle_load_slice with zero components" do
    vm = make_vm([[1, 2, 3]])
    {_output, vm} = Handlers.handle_load_slice(vm, make_instr(0), make_code())
    assert GenericVM.peek(vm) == [1, 2, 3]
  end

  test "handle_load_slice with three components" do
    vm = make_vm([[10, 20, 30, 40, 50], 0, 5, 2])
    {_output, vm} = Handlers.handle_load_slice(vm, make_instr(3), make_code())
    # step is passed but ignored in current implementation
    result = GenericVM.peek(vm)
    assert is_list(result)
  end

  test "handle_load_slice raises on non-sliceable" do
    vm = make_vm([42, 0, 1])
    assert_raise Errors.VMTypeError, ~r/not sliceable/, fn ->
      Handlers.handle_load_slice(vm, make_instr(2), make_code())
    end
  end

  # ---------------------------------------------------------------------------
  # Iteration Operations
  # ---------------------------------------------------------------------------

  test "handle_get_iter from list" do
    vm = make_vm([[1, 2, 3]])
    {_output, vm} = Handlers.handle_get_iter(vm, make_instr(), make_code())
    %StarlarkIterator{items: items} = GenericVM.peek(vm)
    assert items == [1, 2, 3]
  end

  test "handle_get_iter from map (keys)" do
    vm = make_vm([%{"a" => 1, "b" => 2}])
    {_output, vm} = Handlers.handle_get_iter(vm, make_instr(), make_code())
    %StarlarkIterator{items: items} = GenericVM.peek(vm)
    assert Enum.sort(items) == ["a", "b"]
  end

  test "handle_get_iter from string (graphemes)" do
    vm = make_vm(["abc"])
    {_output, vm} = Handlers.handle_get_iter(vm, make_instr(), make_code())
    %StarlarkIterator{items: items} = GenericVM.peek(vm)
    assert items == ["a", "b", "c"]
  end

  test "handle_get_iter from tuple" do
    vm = make_vm([{1, 2, 3}])
    {_output, vm} = Handlers.handle_get_iter(vm, make_instr(), make_code())
    %StarlarkIterator{items: items} = GenericVM.peek(vm)
    assert items == [1, 2, 3]
  end

  test "handle_get_iter from existing iterator" do
    iter = %StarlarkIterator{items: [1, 2], index: 0}
    vm = make_vm([iter])
    {_output, vm} = Handlers.handle_get_iter(vm, make_instr(), make_code())
    %StarlarkIterator{items: items} = GenericVM.peek(vm)
    assert items == [1, 2]
  end

  test "handle_get_iter raises on non-iterable" do
    vm = make_vm([42])
    assert_raise Errors.VMTypeError, ~r/not iterable/, fn ->
      Handlers.handle_get_iter(vm, make_instr(), make_code())
    end
  end

  test "handle_for_iter with remaining items" do
    iter = %StarlarkIterator{items: [10, 20], index: 0}
    vm = make_vm([iter])
    {_output, vm} = Handlers.handle_for_iter(vm, make_instr(99), make_code())
    # Should push updated iterator and then the value
    {value, vm} = GenericVM.pop(vm)
    assert value == 10
    %StarlarkIterator{items: remaining} = GenericVM.peek(vm)
    assert remaining == [20]
  end

  test "handle_for_iter with exhausted iterator jumps" do
    iter = %StarlarkIterator{items: [], index: 2}
    vm = make_vm([iter])
    {_output, vm} = Handlers.handle_for_iter(vm, make_instr(99), make_code())
    assert vm.pc == 99
  end

  test "handle_unpack_sequence from list" do
    vm = make_vm([[1, 2, 3]])
    {_output, vm} = Handlers.handle_unpack_sequence(vm, make_instr(3), make_code())
    # Items pushed in reverse so first item is on top
    {first, vm} = GenericVM.pop(vm)
    {second, vm} = GenericVM.pop(vm)
    {third, _vm} = GenericVM.pop(vm)
    assert first == 1
    assert second == 2
    assert third == 3
  end

  test "handle_unpack_sequence from tuple" do
    vm = make_vm([{10, 20}])
    {_output, vm} = Handlers.handle_unpack_sequence(vm, make_instr(2), make_code())
    {first, vm} = GenericVM.pop(vm)
    {second, _vm} = GenericVM.pop(vm)
    assert first == 10
    assert second == 20
  end

  test "handle_unpack_sequence raises on wrong count" do
    vm = make_vm([[1, 2]])
    assert_raise Errors.VMTypeError, ~r/Not enough values/, fn ->
      Handlers.handle_unpack_sequence(vm, make_instr(3), make_code())
    end
  end

  test "handle_unpack_sequence raises on non-sequence" do
    vm = make_vm([42])
    assert_raise Errors.VMTypeError, ~r/Cannot unpack/, fn ->
      Handlers.handle_unpack_sequence(vm, make_instr(1), make_code())
    end
  end

  # ---------------------------------------------------------------------------
  # Module Operations
  # ---------------------------------------------------------------------------

  test "handle_load_module pushes empty map" do
    vm = make_vm()
    {_output, vm} = Handlers.handle_load_module(vm, make_instr(), make_code())
    assert GenericVM.peek(vm) == %{}
  end

  test "handle_import_from extracts symbol from module dict" do
    module_dict = %{"foo" => 42}
    vm = make_vm([module_dict])
    code = make_code([], ["foo"])
    {_output, vm} = Handlers.handle_import_from(vm, make_instr(0), code)
    assert GenericVM.peek(vm) == 42
  end

  test "handle_import_from returns nil for missing symbol" do
    module_dict = %{"foo" => 42}
    vm = make_vm([module_dict])
    code = make_code([], ["bar"])
    {_output, vm} = Handlers.handle_import_from(vm, make_instr(0), code)
    assert GenericVM.peek(vm) == nil
  end

  test "handle_import_from with non-map returns nil" do
    vm = make_vm([42])
    code = make_code([], ["foo"])
    {_output, vm} = Handlers.handle_import_from(vm, make_instr(0), code)
    assert GenericVM.peek(vm) == nil
  end

  # ---------------------------------------------------------------------------
  # I/O Operations
  # ---------------------------------------------------------------------------

  test "handle_print captures output" do
    vm = make_vm([42])
    {output, vm} = Handlers.handle_print(vm, make_instr(), make_code())
    assert output == "42"
    assert vm.output == ["42"]
  end

  test "handle_print with string value" do
    vm = make_vm(["hello"])
    {output, _vm} = Handlers.handle_print(vm, make_instr(), make_code())
    assert output == "hello"
  end

  test "handle_print with nil value" do
    vm = make_vm([nil])
    {output, _vm} = Handlers.handle_print(vm, make_instr(), make_code())
    assert output == "None"
  end

  # ---------------------------------------------------------------------------
  # VM Control
  # ---------------------------------------------------------------------------

  test "handle_halt sets halted flag" do
    vm = make_vm()
    {_output, vm} = Handlers.handle_halt(vm, make_instr(), make_code())
    assert vm.halted == true
  end

  # ---------------------------------------------------------------------------
  # Return Operation
  # ---------------------------------------------------------------------------

  test "handle_return at top level halts" do
    vm = make_vm([42])
    vm = %{vm | call_stack: []}
    {_output, vm} = Handlers.handle_return(vm, make_instr(), make_code())
    assert vm.halted == true
    assert GenericVM.peek(vm) == 42
  end
end
