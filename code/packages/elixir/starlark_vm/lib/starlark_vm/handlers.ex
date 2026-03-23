defmodule CodingAdventures.StarlarkVm.Handlers do
  @moduledoc """
  Starlark VM Opcode Handlers — The execution semantics for Starlark bytecode.

  ## Chapter 1: What Opcode Handlers Do

  Each handler is a function that implements one Starlark bytecode instruction.
  The GenericVM's eval loop calls the handler whenever it encounters the
  corresponding opcode. The handler mutates the VM state (stack, PC, variables)
  and optionally returns output text.

  All handlers follow the same signature:

      fn(vm, instruction, code) -> {output_or_nil, updated_vm}

  - `vm` — The GenericVM state. Use `GenericVM.push/2`, `GenericVM.pop/1`, etc.
  - `instruction` — The `%Instruction{}` being executed (opcode + optional operand).
  - `code` — The `%CodeObject{}` being run (for constant/name pool access).

  Returns `{output_string, vm}` if the handler produces output, else `{nil, vm}`.

  ## Chapter 2: Starlark Type Semantics

  Starlark has a small, well-defined type system. Each handler must respect:

  - **int + int -> int**, **float + float -> float**, **int + float -> float**
  - **str + str -> str** (concatenation), **str * int -> str** (repetition)
  - **list + list -> list** (concatenation), **list * int -> list** (repetition)
  - Division always produces **float** (even `4 / 2 -> 2.0`)
  - Floor division `//` produces **int** (for int operands)
  - Truthiness: `0`, `0.0`, `""`, `[]`, `{}`, `()`, `nil`, `false` are falsy
  """

  alias CodingAdventures.VirtualMachine.GenericVM
  alias CodingAdventures.VirtualMachine.Errors

  # ===========================================================================
  # Starlark Types
  # ===========================================================================

  defmodule StarlarkFunction do
    @moduledoc """
    A compiled Starlark function object.

    When the compiler encounters a `def` statement, it compiles the function
    body into a separate `CodeObject` and wraps it in a `StarlarkFunction`.
    This struct stores everything needed to call the function later.

    ## Fields

    - `name` — the function's name (for error messages and debugging)
    - `code` — the compiled `CodeObject` for the function body
    - `params` — list of parameter names (strings)
    - `defaults` — list of default values for trailing parameters
    - `closure_cells` — list of captured values from enclosing scopes
    """

    defstruct name: "<lambda>",
              code: nil,
              params: [],
              defaults: [],
              closure_cells: []
  end

  defmodule StarlarkIterator do
    @moduledoc """
    An iterator wrapper for Starlark for-loops.

    When the VM executes GET_ITER, it wraps an iterable (list, dict, string)
    in a StarlarkIterator. The FOR_ITER instruction then advances the iterator
    one step at a time.

    ## Fields

    - `items` — the remaining items to iterate over
    - `index` — current position (for debugging)
    """

    defstruct items: [], index: 0
  end

  defmodule StarlarkResult do
    @moduledoc """
    The result of executing a Starlark program.

    Contains all the information about the execution:
    - `variables` — the final state of all named variables
    - `output` — captured print output
    - `traces` — step-by-step execution trace (for debugging)
    """

    defstruct variables: %{}, output: [], traces: []
  end

  # ===========================================================================
  # Truthiness
  # ===========================================================================
  #
  # Starlark truthiness follows Python's rules:
  # Falsy: nil, false, 0, 0.0, "", [], {}, {}(empty tuple)
  # Everything else is truthy.

  @doc "Determine if a Starlark value is truthy."
  def truthy?(nil), do: false
  def truthy?(false), do: false
  def truthy?(0), do: false
  def truthy?(val) when is_float(val) and val == 0.0, do: false
  def truthy?(""), do: false
  def truthy?(list) when is_list(list) and length(list) == 0, do: false
  def truthy?(map) when is_map(map) and map_size(map) == 0, do: false
  def truthy?(tuple) when is_tuple(tuple) and tuple_size(tuple) == 0, do: false
  def truthy?(_), do: true

  # ===========================================================================
  # Stack Operations (0x0_)
  # ===========================================================================

  @doc "LOAD_CONST: Push a constant from the pool onto the stack."
  def handle_load_const(vm, instr, code) do
    value = Enum.at(code.constants, instr.operand)
    vm = GenericVM.push(vm, value)
    vm = GenericVM.advance_pc(vm)
    {nil, vm}
  end

  @doc "POP: Discard the top value from the stack."
  def handle_pop(vm, _instr, _code) do
    {_value, vm} = GenericVM.pop(vm)
    vm = GenericVM.advance_pc(vm)
    {nil, vm}
  end

  @doc "DUP: Duplicate the top value on the stack."
  def handle_dup(vm, _instr, _code) do
    value = GenericVM.peek(vm)
    vm = GenericVM.push(vm, value)
    vm = GenericVM.advance_pc(vm)
    {nil, vm}
  end

  @doc "LOAD_NONE: Push nil (Starlark's None) onto the stack."
  def handle_load_none(vm, _instr, _code) do
    vm = GenericVM.push(vm, nil)
    vm = GenericVM.advance_pc(vm)
    {nil, vm}
  end

  @doc "LOAD_TRUE: Push true onto the stack."
  def handle_load_true(vm, _instr, _code) do
    vm = GenericVM.push(vm, true)
    vm = GenericVM.advance_pc(vm)
    {nil, vm}
  end

  @doc "LOAD_FALSE: Push false onto the stack."
  def handle_load_false(vm, _instr, _code) do
    vm = GenericVM.push(vm, false)
    vm = GenericVM.advance_pc(vm)
    {nil, vm}
  end

  # ===========================================================================
  # Variable Operations (0x1_)
  # ===========================================================================

  @doc "STORE_NAME: Pop value and store in named variable."
  def handle_store_name(vm, instr, code) do
    name = Enum.at(code.names, instr.operand)
    {value, vm} = GenericVM.pop(vm)
    vm = %{vm | variables: Map.put(vm.variables, name, value)}
    vm = GenericVM.advance_pc(vm)
    {nil, vm}
  end

  @doc "LOAD_NAME: Push named variable's value onto the stack."
  def handle_load_name(vm, instr, code) do
    name = Enum.at(code.names, instr.operand)

    value = cond do
      Map.has_key?(vm.variables, name) ->
        Map.get(vm.variables, name)

      Map.has_key?(vm.builtins, name) ->
        builtin = Map.get(vm.builtins, name)
        {:builtin, builtin.implementation}

      true ->
        raise Errors.UndefinedNameError, "Undefined name: '#{name}'"
    end

    vm = GenericVM.push(vm, value)
    vm = GenericVM.advance_pc(vm)
    {nil, vm}
  end

  @doc "STORE_LOCAL: Pop value and store in local slot."
  def handle_store_local(vm, instr, _code) do
    {value, vm} = GenericVM.pop(vm)
    locals = ensure_locals_size(vm.locals, instr.operand + 1)
    locals = List.replace_at(locals, instr.operand, value)
    vm = %{vm | locals: locals}
    vm = GenericVM.advance_pc(vm)
    {nil, vm}
  end

  @doc "LOAD_LOCAL: Push local slot's value onto the stack."
  def handle_load_local(vm, instr, _code) do
    value = Enum.at(vm.locals, instr.operand)
    vm = GenericVM.push(vm, value)
    vm = GenericVM.advance_pc(vm)
    {nil, vm}
  end

  @doc "STORE_CLOSURE: Pop value and store in closure cell."
  def handle_store_closure(vm, instr, _code) do
    {value, vm} = GenericVM.pop(vm)
    cells = GenericVM.get_extra(vm, :closure_cells, [])
    cells = ensure_locals_size(cells, instr.operand + 1)
    cells = List.replace_at(cells, instr.operand, value)
    vm = GenericVM.put_extra(vm, :closure_cells, cells)
    vm = GenericVM.advance_pc(vm)
    {nil, vm}
  end

  @doc "LOAD_CLOSURE: Push closure cell's value onto the stack."
  def handle_load_closure(vm, instr, _code) do
    cells = GenericVM.get_extra(vm, :closure_cells, [])
    value = Enum.at(cells, instr.operand)
    vm = GenericVM.push(vm, value)
    vm = GenericVM.advance_pc(vm)
    {nil, vm}
  end

  # ===========================================================================
  # Arithmetic Operations (0x2_)
  # ===========================================================================

  @doc "ADD: Pop two values, push a + b."
  def handle_add(vm, _instr, _code) do
    {b, vm} = GenericVM.pop(vm)
    {a, vm} = GenericVM.pop(vm)

    result = cond do
      is_binary(a) and is_binary(b) -> a <> b
      is_list(a) and is_list(b) -> a ++ b
      is_number(a) and is_number(b) -> a + b
      true -> raise Errors.VMTypeError, "Cannot add #{inspect(a)} and #{inspect(b)}"
    end

    vm = GenericVM.push(vm, result)
    vm = GenericVM.advance_pc(vm)
    {nil, vm}
  end

  @doc "SUB: Pop two values, push a - b."
  def handle_sub(vm, _instr, _code) do
    {b, vm} = GenericVM.pop(vm)
    {a, vm} = GenericVM.pop(vm)
    result = a - b
    vm = GenericVM.push(vm, result)
    vm = GenericVM.advance_pc(vm)
    {nil, vm}
  end

  @doc "MUL: Pop two values, push a * b."
  def handle_mul(vm, _instr, _code) do
    {b, vm} = GenericVM.pop(vm)
    {a, vm} = GenericVM.pop(vm)

    result = cond do
      is_binary(a) and is_integer(b) -> String.duplicate(a, b)
      is_integer(a) and is_binary(b) -> String.duplicate(b, a)
      is_list(a) and is_integer(b) -> List.flatten(List.duplicate(a, b))
      is_integer(a) and is_list(b) -> List.flatten(List.duplicate(b, a))
      is_number(a) and is_number(b) -> a * b
      true -> raise Errors.VMTypeError, "Cannot multiply #{inspect(a)} and #{inspect(b)}"
    end

    vm = GenericVM.push(vm, result)
    vm = GenericVM.advance_pc(vm)
    {nil, vm}
  end

  @doc "DIV: Pop two values, push a / b (always float)."
  def handle_div(vm, _instr, _code) do
    {b, vm} = GenericVM.pop(vm)
    {a, vm} = GenericVM.pop(vm)

    if b == 0 or b == 0.0 do
      raise Errors.DivisionByZeroError, "Division by zero"
    end

    result = a / b
    vm = GenericVM.push(vm, result)
    vm = GenericVM.advance_pc(vm)
    {nil, vm}
  end

  @doc "FLOOR_DIV: Pop two values, push a // b."
  def handle_floor_div(vm, _instr, _code) do
    {b, vm} = GenericVM.pop(vm)
    {a, vm} = GenericVM.pop(vm)

    if b == 0 or b == 0.0 do
      raise Errors.DivisionByZeroError, "Floor division by zero"
    end

    result = if is_integer(a) and is_integer(b) do
      Integer.floor_div(a, b)
    else
      Float.floor(a / b) |> trunc()
    end

    vm = GenericVM.push(vm, result)
    vm = GenericVM.advance_pc(vm)
    {nil, vm}
  end

  @doc "MOD: Pop two values, push a % b."
  def handle_mod(vm, _instr, _code) do
    {b, vm} = GenericVM.pop(vm)
    {a, vm} = GenericVM.pop(vm)

    if b == 0 or b == 0.0 do
      raise Errors.DivisionByZeroError, "Modulo by zero"
    end

    result = if is_binary(a) do
      # String formatting: "hello %s" % ("world",)
      a
    else
      rem(a, b)
    end

    vm = GenericVM.push(vm, result)
    vm = GenericVM.advance_pc(vm)
    {nil, vm}
  end

  @doc "POWER: Pop two values, push a ** b."
  def handle_power(vm, _instr, _code) do
    {b, vm} = GenericVM.pop(vm)
    {a, vm} = GenericVM.pop(vm)

    result = if is_integer(a) and is_integer(b) and b >= 0 do
      integer_pow(a, b)
    else
      :math.pow(a, b)
    end

    vm = GenericVM.push(vm, result)
    vm = GenericVM.advance_pc(vm)
    {nil, vm}
  end

  @doc "NEGATE: Pop one value, push -a."
  def handle_negate(vm, _instr, _code) do
    {a, vm} = GenericVM.pop(vm)
    vm = GenericVM.push(vm, -a)
    vm = GenericVM.advance_pc(vm)
    {nil, vm}
  end

  @doc "BIT_AND: Pop two values, push a & b."
  def handle_bit_and(vm, _instr, _code) do
    {b, vm} = GenericVM.pop(vm)
    {a, vm} = GenericVM.pop(vm)
    result = Bitwise.band(a, b)
    vm = GenericVM.push(vm, result)
    vm = GenericVM.advance_pc(vm)
    {nil, vm}
  end

  @doc "BIT_OR: Pop two values, push a | b."
  def handle_bit_or(vm, _instr, _code) do
    {b, vm} = GenericVM.pop(vm)
    {a, vm} = GenericVM.pop(vm)
    result = Bitwise.bor(a, b)
    vm = GenericVM.push(vm, result)
    vm = GenericVM.advance_pc(vm)
    {nil, vm}
  end

  @doc "BIT_XOR: Pop two values, push a ^ b."
  def handle_bit_xor(vm, _instr, _code) do
    {b, vm} = GenericVM.pop(vm)
    {a, vm} = GenericVM.pop(vm)
    result = Bitwise.bxor(a, b)
    vm = GenericVM.push(vm, result)
    vm = GenericVM.advance_pc(vm)
    {nil, vm}
  end

  @doc "BIT_NOT: Pop one value, push ~a."
  def handle_bit_not(vm, _instr, _code) do
    {a, vm} = GenericVM.pop(vm)
    result = Bitwise.bnot(a)
    vm = GenericVM.push(vm, result)
    vm = GenericVM.advance_pc(vm)
    {nil, vm}
  end

  @doc "LSHIFT: Pop two values, push a << b."
  def handle_lshift(vm, _instr, _code) do
    {b, vm} = GenericVM.pop(vm)
    {a, vm} = GenericVM.pop(vm)
    result = Bitwise.bsl(a, b)
    vm = GenericVM.push(vm, result)
    vm = GenericVM.advance_pc(vm)
    {nil, vm}
  end

  @doc "RSHIFT: Pop two values, push a >> b."
  def handle_rshift(vm, _instr, _code) do
    {b, vm} = GenericVM.pop(vm)
    {a, vm} = GenericVM.pop(vm)
    result = Bitwise.bsr(a, b)
    vm = GenericVM.push(vm, result)
    vm = GenericVM.advance_pc(vm)
    {nil, vm}
  end

  # ===========================================================================
  # Comparison Operations (0x3_)
  # ===========================================================================

  @doc "CMP_EQ: Pop two values, push a == b."
  def handle_cmp_eq(vm, _instr, _code) do
    {b, vm} = GenericVM.pop(vm)
    {a, vm} = GenericVM.pop(vm)
    vm = GenericVM.push(vm, a == b)
    vm = GenericVM.advance_pc(vm)
    {nil, vm}
  end

  @doc "CMP_NE: Pop two values, push a != b."
  def handle_cmp_ne(vm, _instr, _code) do
    {b, vm} = GenericVM.pop(vm)
    {a, vm} = GenericVM.pop(vm)
    vm = GenericVM.push(vm, a != b)
    vm = GenericVM.advance_pc(vm)
    {nil, vm}
  end

  @doc "CMP_LT: Pop two values, push a < b."
  def handle_cmp_lt(vm, _instr, _code) do
    {b, vm} = GenericVM.pop(vm)
    {a, vm} = GenericVM.pop(vm)
    vm = GenericVM.push(vm, a < b)
    vm = GenericVM.advance_pc(vm)
    {nil, vm}
  end

  @doc "CMP_GT: Pop two values, push a > b."
  def handle_cmp_gt(vm, _instr, _code) do
    {b, vm} = GenericVM.pop(vm)
    {a, vm} = GenericVM.pop(vm)
    vm = GenericVM.push(vm, a > b)
    vm = GenericVM.advance_pc(vm)
    {nil, vm}
  end

  @doc "CMP_LE: Pop two values, push a <= b."
  def handle_cmp_le(vm, _instr, _code) do
    {b, vm} = GenericVM.pop(vm)
    {a, vm} = GenericVM.pop(vm)
    vm = GenericVM.push(vm, a <= b)
    vm = GenericVM.advance_pc(vm)
    {nil, vm}
  end

  @doc "CMP_GE: Pop two values, push a >= b."
  def handle_cmp_ge(vm, _instr, _code) do
    {b, vm} = GenericVM.pop(vm)
    {a, vm} = GenericVM.pop(vm)
    vm = GenericVM.push(vm, a >= b)
    vm = GenericVM.advance_pc(vm)
    {nil, vm}
  end

  @doc "CMP_IN: Pop two values, push a in b."
  def handle_cmp_in(vm, _instr, _code) do
    {b, vm} = GenericVM.pop(vm)
    {a, vm} = GenericVM.pop(vm)

    result = cond do
      is_list(b) -> Enum.member?(b, a)
      is_map(b) -> Map.has_key?(b, a)
      is_binary(b) and is_binary(a) -> String.contains?(b, a)
      is_tuple(b) -> a in Tuple.to_list(b)
      true -> false
    end

    vm = GenericVM.push(vm, result)
    vm = GenericVM.advance_pc(vm)
    {nil, vm}
  end

  @doc "CMP_NOT_IN: Pop two values, push a not in b."
  def handle_cmp_not_in(vm, _instr, _code) do
    {b, vm} = GenericVM.pop(vm)
    {a, vm} = GenericVM.pop(vm)

    result = cond do
      is_list(b) -> not Enum.member?(b, a)
      is_map(b) -> not Map.has_key?(b, a)
      is_binary(b) and is_binary(a) -> not String.contains?(b, a)
      is_tuple(b) -> a not in Tuple.to_list(b)
      true -> true
    end

    vm = GenericVM.push(vm, result)
    vm = GenericVM.advance_pc(vm)
    {nil, vm}
  end

  # ===========================================================================
  # Boolean Operations (0x38)
  # ===========================================================================

  @doc "NOT: Pop one value, push logical not."
  def handle_not(vm, _instr, _code) do
    {a, vm} = GenericVM.pop(vm)
    vm = GenericVM.push(vm, not truthy?(a))
    vm = GenericVM.advance_pc(vm)
    {nil, vm}
  end

  # ===========================================================================
  # Control Flow (0x4_)
  # ===========================================================================

  @doc "JUMP: Unconditional jump to target."
  def handle_jump(vm, instr, _code) do
    vm = GenericVM.jump_to(vm, instr.operand)
    {nil, vm}
  end

  @doc "JUMP_IF_FALSE: Pop value, jump if falsy."
  def handle_jump_if_false(vm, instr, _code) do
    {value, vm} = GenericVM.pop(vm)

    vm = if truthy?(value) do
      GenericVM.advance_pc(vm)
    else
      GenericVM.jump_to(vm, instr.operand)
    end

    {nil, vm}
  end

  @doc "JUMP_IF_TRUE: Pop value, jump if truthy."
  def handle_jump_if_true(vm, instr, _code) do
    {value, vm} = GenericVM.pop(vm)

    vm = if truthy?(value) do
      GenericVM.jump_to(vm, instr.operand)
    else
      GenericVM.advance_pc(vm)
    end

    {nil, vm}
  end

  @doc "JUMP_IF_FALSE_OR_POP: For `and` short-circuit. If falsy, jump; else pop."
  def handle_jump_if_false_or_pop(vm, instr, _code) do
    value = GenericVM.peek(vm)

    if truthy?(value) do
      # Truthy: pop and continue
      {_value, vm} = GenericVM.pop(vm)
      vm = GenericVM.advance_pc(vm)
      {nil, vm}
    else
      # Falsy: keep value, jump
      vm = GenericVM.jump_to(vm, instr.operand)
      {nil, vm}
    end
  end

  @doc "JUMP_IF_TRUE_OR_POP: For `or` short-circuit. If truthy, jump; else pop."
  def handle_jump_if_true_or_pop(vm, instr, _code) do
    value = GenericVM.peek(vm)

    if truthy?(value) do
      # Truthy: keep value, jump
      vm = GenericVM.jump_to(vm, instr.operand)
      {nil, vm}
    else
      # Falsy: pop and continue
      {_value, vm} = GenericVM.pop(vm)
      vm = GenericVM.advance_pc(vm)
      {nil, vm}
    end
  end

  # ===========================================================================
  # Function Operations (0x5_)
  # ===========================================================================

  @doc "MAKE_FUNCTION: Create a function object from code and params on the stack."
  def handle_make_function(vm, instr, _code) do
    flags = instr.operand || 0

    # Pop parameter names tuple
    {param_names_tuple, vm} = GenericVM.pop(vm)
    params = if is_tuple(param_names_tuple), do: Tuple.to_list(param_names_tuple), else: []

    # Pop the code object
    {func_code, vm} = GenericVM.pop(vm)

    # Pop defaults if present (bit 0 of flags)
    {defaults, vm} = if Bitwise.band(flags, 1) == 1 do
      {defaults_tuple, vm} = GenericVM.pop(vm)
      defaults = if is_tuple(defaults_tuple), do: Tuple.to_list(defaults_tuple), else: []
      {defaults, vm}
    else
      {[], vm}
    end

    func = %StarlarkFunction{
      code: func_code,
      params: params,
      defaults: defaults
    }

    vm = GenericVM.push(vm, func)
    vm = GenericVM.advance_pc(vm)
    {nil, vm}
  end

  @doc "CALL_FUNCTION: Call function with N positional args."
  def handle_call_function(vm, instr, code) do
    arg_count = instr.operand

    # Pop arguments (in reverse order)
    {args, vm} = pop_n(vm, arg_count)

    # Pop the callable
    {callable, vm} = GenericVM.pop(vm)

    call_function(vm, callable, args, code)
  end

  @doc "CALL_FUNCTION_KW: Call function with keyword args."
  def handle_call_function_kw(vm, instr, code) do
    total_args = instr.operand

    # Pop keyword names tuple
    {kw_names_tuple, vm} = GenericVM.pop(vm)
    kw_names = if is_tuple(kw_names_tuple), do: Tuple.to_list(kw_names_tuple), else: []

    # Pop all arguments
    {all_args, vm} = pop_n(vm, total_args)

    # Split into positional and keyword args
    kw_count = length(kw_names)
    pos_count = length(all_args) - kw_count
    pos_args = Enum.take(all_args, pos_count)
    kw_values = Enum.drop(all_args, pos_count)
    kwargs = Enum.zip(kw_names, kw_values) |> Map.new()

    # Pop the callable
    {callable, vm} = GenericVM.pop(vm)

    # For keyword calls, we need to bind args correctly
    case callable do
      %StarlarkFunction{} = func ->
        bound_args = bind_args_with_kwargs(func, pos_args, kwargs)
        call_starlark_function(vm, func, bound_args, code)

      {:builtin, impl} ->
        # For builtins, just pass positional + keyword as positional
        all = pos_args ++ kw_values
        result = impl.(all, vm)
        {vm, result_val} = extract_builtin_result(vm, result)
        vm = GenericVM.push(vm, result_val)
        vm = GenericVM.advance_pc(vm)
        {nil, vm}

      _ ->
        raise Errors.VMTypeError, "Object is not callable: #{inspect(callable)}"
    end
  end

  @doc "RETURN: Return from function."
  def handle_return(vm, _instr, _code) do
    {return_value, vm} = GenericVM.pop(vm)

    if vm.call_stack == [] do
      # Top-level return — halt
      vm = GenericVM.push(vm, return_value)
      vm = %{vm | halted: true}
      {nil, vm}
    else
      {frame, vm} = GenericVM.pop_frame(vm)
      vm = %{vm | variables: frame.saved_variables, locals: frame.saved_locals || []}
      vm = GenericVM.push(vm, return_value)
      vm = GenericVM.jump_to(vm, frame.return_address)
      {nil, vm}
    end
  end

  # ===========================================================================
  # Collection Operations (0x6_)
  # ===========================================================================

  @doc "BUILD_LIST: Create list from N stack items."
  def handle_build_list(vm, instr, _code) do
    count = instr.operand
    {items, vm} = pop_n(vm, count)
    vm = GenericVM.push(vm, items)
    vm = GenericVM.advance_pc(vm)
    {nil, vm}
  end

  @doc "BUILD_DICT: Create dict from N key-value pairs."
  def handle_build_dict(vm, instr, _code) do
    count = instr.operand
    {pairs, vm} = pop_n(vm, count * 2)

    dict = pairs
    |> Enum.chunk_every(2)
    |> Enum.reduce(%{}, fn [k, v], acc -> Map.put(acc, k, v) end)

    vm = GenericVM.push(vm, dict)
    vm = GenericVM.advance_pc(vm)
    {nil, vm}
  end

  @doc "BUILD_TUPLE: Create tuple from N stack items."
  def handle_build_tuple(vm, instr, _code) do
    count = instr.operand
    {items, vm} = pop_n(vm, count)
    vm = GenericVM.push(vm, List.to_tuple(items))
    vm = GenericVM.advance_pc(vm)
    {nil, vm}
  end

  @doc "LIST_APPEND: Append value to list (for comprehensions)."
  def handle_list_append(vm, _instr, _code) do
    {value, vm} = GenericVM.pop(vm)
    # The list is deeper in the stack — we need to find it
    # For comprehensions, the list is just below the iterator
    {iter, vm} = GenericVM.pop(vm)
    {the_list, vm} = GenericVM.pop(vm)
    the_list = the_list ++ [value]
    vm = GenericVM.push(vm, the_list)
    vm = GenericVM.push(vm, iter)
    vm = GenericVM.advance_pc(vm)
    {nil, vm}
  end

  @doc "DICT_SET: Set dict entry (for comprehensions)."
  def handle_dict_set(vm, _instr, _code) do
    {value, vm} = GenericVM.pop(vm)
    {key, vm} = GenericVM.pop(vm)
    {iter, vm} = GenericVM.pop(vm)
    {the_dict, vm} = GenericVM.pop(vm)
    the_dict = Map.put(the_dict, key, value)
    vm = GenericVM.push(vm, the_dict)
    vm = GenericVM.push(vm, iter)
    vm = GenericVM.advance_pc(vm)
    {nil, vm}
  end

  # ===========================================================================
  # Subscript & Attribute Operations (0x7_)
  # ===========================================================================

  @doc "LOAD_SUBSCRIPT: obj[key]."
  def handle_load_subscript(vm, _instr, _code) do
    {key, vm} = GenericVM.pop(vm)
    {obj, vm} = GenericVM.pop(vm)

    value = cond do
      is_list(obj) ->
        idx = if key < 0, do: length(obj) + key, else: key
        Enum.at(obj, idx)

      is_map(obj) ->
        Map.get(obj, key)

      is_binary(obj) ->
        idx = if key < 0, do: String.length(obj) + key, else: key
        String.at(obj, idx)

      is_tuple(obj) ->
        idx = if key < 0, do: tuple_size(obj) + key, else: key
        elem(obj, idx)

      true ->
        raise Errors.VMTypeError, "Object is not subscriptable: #{inspect(obj)}"
    end

    vm = GenericVM.push(vm, value)
    vm = GenericVM.advance_pc(vm)
    {nil, vm}
  end

  @doc "STORE_SUBSCRIPT: obj[key] = value."
  def handle_store_subscript(vm, _instr, _code) do
    {value, vm} = GenericVM.pop(vm)
    {key, vm} = GenericVM.pop(vm)
    {obj, vm} = GenericVM.pop(vm)

    new_obj = cond do
      is_list(obj) ->
        idx = if key < 0, do: length(obj) + key, else: key
        List.replace_at(obj, idx, value)

      is_map(obj) ->
        Map.put(obj, key, value)

      true ->
        raise Errors.VMTypeError, "Object does not support item assignment: #{inspect(obj)}"
    end

    vm = GenericVM.push(vm, new_obj)
    vm = GenericVM.advance_pc(vm)
    {nil, vm}
  end

  @doc "LOAD_ATTR: obj.attr."
  def handle_load_attr(vm, instr, code) do
    attr_name = Enum.at(code.names, instr.operand)
    {obj, vm} = GenericVM.pop(vm)

    value = cond do
      is_map(obj) and Map.has_key?(obj, attr_name) ->
        Map.get(obj, attr_name)

      # String methods
      is_binary(obj) ->
        get_string_method(obj, attr_name)

      # List methods
      is_list(obj) ->
        get_list_method(obj, attr_name)

      # Dict methods
      is_map(obj) ->
        get_dict_method(obj, attr_name)

      true ->
        raise Errors.VMTypeError, "Object has no attribute '#{attr_name}': #{inspect(obj)}"
    end

    vm = GenericVM.push(vm, value)
    vm = GenericVM.advance_pc(vm)
    {nil, vm}
  end

  @doc "STORE_ATTR: obj.attr = value."
  def handle_store_attr(vm, instr, code) do
    attr_name = Enum.at(code.names, instr.operand)
    {value, vm} = GenericVM.pop(vm)
    {obj, vm} = GenericVM.pop(vm)

    new_obj = if is_map(obj) do
      Map.put(obj, attr_name, value)
    else
      raise Errors.VMTypeError, "Cannot set attribute on #{inspect(obj)}"
    end

    vm = GenericVM.push(vm, new_obj)
    vm = GenericVM.advance_pc(vm)
    {nil, vm}
  end

  @doc "LOAD_SLICE: obj[start:stop:step]."
  def handle_load_slice(vm, instr, _code) do
    count = instr.operand || 0

    {components, vm} = pop_n(vm, count)
    {obj, vm} = GenericVM.pop(vm)

    {start_val, stop_val, step_val} = case components do
      [s] -> {s, nil, nil}
      [s, e] -> {s, e, nil}
      [s, e, st] -> {s, e, st}
      _ -> {nil, nil, nil}
    end

    result = slice_object(obj, start_val, stop_val, step_val)

    vm = GenericVM.push(vm, result)
    vm = GenericVM.advance_pc(vm)
    {nil, vm}
  end

  # ===========================================================================
  # Iteration Operations (0x8_)
  # ===========================================================================

  @doc "GET_ITER: Get iterator from iterable."
  def handle_get_iter(vm, _instr, _code) do
    {iterable, vm} = GenericVM.pop(vm)

    items = case iterable do
      %StarlarkIterator{items: iter_items} -> iter_items
      _ ->
        cond do
          is_list(iterable) -> iterable
          is_map(iterable) -> Map.keys(iterable)
          is_binary(iterable) -> String.graphemes(iterable)
          is_tuple(iterable) -> Tuple.to_list(iterable)
          true -> raise Errors.VMTypeError, "Object is not iterable: #{inspect(iterable)}"
        end
    end

    iterator = %StarlarkIterator{items: items, index: 0}
    vm = GenericVM.push(vm, iterator)
    vm = GenericVM.advance_pc(vm)
    {nil, vm}
  end

  @doc "FOR_ITER: Get next from iterator, or jump to end."
  def handle_for_iter(vm, instr, _code) do
    {iterator, vm} = GenericVM.pop(vm)

    case iterator.items do
      [] ->
        # Exhausted — jump to target
        vm = GenericVM.jump_to(vm, instr.operand)
        {nil, vm}

      [next_val | remaining] ->
        # Push updated iterator back, then push the value
        new_iter = %StarlarkIterator{items: remaining, index: iterator.index + 1}
        vm = GenericVM.push(vm, new_iter)
        vm = GenericVM.push(vm, next_val)
        vm = GenericVM.advance_pc(vm)
        {nil, vm}
    end
  end

  @doc "UNPACK_SEQUENCE: Unpack N items from sequence."
  def handle_unpack_sequence(vm, instr, _code) do
    count = instr.operand
    {seq, vm} = GenericVM.pop(vm)

    items = cond do
      is_list(seq) -> seq
      is_tuple(seq) -> Tuple.to_list(seq)
      true -> raise Errors.VMTypeError, "Cannot unpack #{inspect(seq)}"
    end

    if length(items) != count do
      raise Errors.VMTypeError,
            "Not enough values to unpack (expected #{count}, got #{length(items)})"
    end

    # Push in reverse order so first item ends up on top
    vm = Enum.reduce(Enum.reverse(items), vm, fn item, acc_vm ->
      GenericVM.push(acc_vm, item)
    end)

    vm = GenericVM.advance_pc(vm)
    {nil, vm}
  end

  # ===========================================================================
  # Module Operations (0x9_)
  # ===========================================================================

  @doc "LOAD_MODULE: Load a module (stub — overridden by interpreter)."
  def handle_load_module(vm, _instr, _code) do
    # This is a stub. The actual implementation is provided by the
    # interpreter package which has access to the file resolver.
    vm = GenericVM.push(vm, %{})
    vm = GenericVM.advance_pc(vm)
    {nil, vm}
  end

  @doc "IMPORT_FROM: Extract symbol from module dict."
  def handle_import_from(vm, instr, code) do
    sym_name = Enum.at(code.names, instr.operand)
    module_dict = GenericVM.peek(vm)

    value = if is_map(module_dict) do
      Map.get(module_dict, sym_name)
    else
      nil
    end

    vm = GenericVM.push(vm, value)
    vm = GenericVM.advance_pc(vm)
    {nil, vm}
  end

  # ===========================================================================
  # I/O Operations (0xA_)
  # ===========================================================================

  @doc "PRINT: Pop and print value, capture in output."
  def handle_print(vm, _instr, _code) do
    {value, vm} = GenericVM.pop(vm)
    output_str = starlark_repr(value)
    vm = %{vm | output: vm.output ++ [output_str]}
    vm = GenericVM.advance_pc(vm)
    {output_str, vm}
  end

  # ===========================================================================
  # VM Control (0xF_)
  # ===========================================================================

  @doc "HALT: Stop execution."
  def handle_halt(vm, _instr, _code) do
    vm = %{vm | halted: true}
    {nil, vm}
  end

  # ===========================================================================
  # Helper Functions
  # ===========================================================================

  @doc false
  def starlark_repr(nil), do: "None"
  def starlark_repr(true), do: "True"
  def starlark_repr(false), do: "False"
  def starlark_repr(value) when is_binary(value), do: value
  def starlark_repr(value) when is_integer(value), do: Integer.to_string(value)
  def starlark_repr(value) when is_float(value), do: Float.to_string(value)
  def starlark_repr(value) when is_list(value) do
    items = Enum.map(value, &starlark_repr_quoted/1) |> Enum.join(", ")
    "[#{items}]"
  end
  def starlark_repr(value) when is_tuple(value) do
    items = value |> Tuple.to_list() |> Enum.map(&starlark_repr_quoted/1) |> Enum.join(", ")
    "(#{items})"
  end
  def starlark_repr(value) when is_map(value) do
    items = Enum.map(value, fn {k, v} ->
      "#{starlark_repr_quoted(k)}: #{starlark_repr_quoted(v)}"
    end) |> Enum.join(", ")
    "{#{items}}"
  end
  def starlark_repr(value), do: inspect(value)

  defp starlark_repr_quoted(value) when is_binary(value), do: "\"#{value}\""
  defp starlark_repr_quoted(value), do: starlark_repr(value)

  defp pop_n(vm, 0), do: {[], vm}
  defp pop_n(vm, count) do
    Enum.reduce(1..count, {[], vm}, fn _i, {acc, vm_acc} ->
      {val, vm_acc} = GenericVM.pop(vm_acc)
      {[val | acc], vm_acc}
    end)
  end

  defp ensure_locals_size(locals, min_size) when length(locals) >= min_size, do: locals
  defp ensure_locals_size(locals, min_size) do
    locals ++ List.duplicate(nil, min_size - length(locals))
  end

  defp integer_pow(_base, 0), do: 1
  defp integer_pow(base_val, exp) when exp > 0 do
    base_val * integer_pow(base_val, exp - 1)
  end

  defp call_function(vm, callable, args, code) do
    case callable do
      %StarlarkFunction{} = func ->
        call_starlark_function(vm, func, args, code)

      {:builtin, impl} ->
        result = impl.(args, vm)
        {vm, result_val} = extract_builtin_result(vm, result)
        vm = GenericVM.push(vm, result_val)
        vm = GenericVM.advance_pc(vm)
        {nil, vm}

      _ ->
        raise Errors.VMTypeError, "Object is not callable: #{inspect(callable)}"
    end
  end

  defp call_starlark_function(vm, func, args, _code) do
    # Bind arguments to parameters.
    # If args is already a map (from keyword call binding), use it directly.
    # Otherwise bind positional args to parameter names.
    bound = if is_map(args), do: args, else: bind_args(func, args)

    # Save the caller's entire state so we can restore it after the call.
    # We run the function in a fresh execution context via GenericVM.execute,
    # which starts its own eval loop over the function's CodeObject.
    saved_pc = vm.pc
    saved_vars = vm.variables
    saved_locals = vm.locals
    saved_stack = vm.stack
    saved_call_stack = vm.call_stack
    saved_halted = vm.halted

    # Set up function scope: merge params into variables, reset PC/stack/halted
    # so GenericVM.execute starts from instruction 0 of the function's code.
    vm = %{vm |
      variables: Map.merge(vm.variables, bound),
      locals: [],
      pc: 0,
      halted: false,
      stack: [],
      call_stack: []
    }

    # Execute the function's code object
    {_traces, vm} = GenericVM.execute(vm, func.code)

    # Get the return value — the RETURN handler pushes it onto the stack
    # before setting halted=true (for top-level returns).
    return_value = if vm.stack != [] do
      GenericVM.peek(vm)
    else
      nil
    end

    # Restore the caller's state completely
    vm = %{vm |
      pc: saved_pc,
      variables: saved_vars,
      locals: saved_locals,
      stack: saved_stack,
      call_stack: saved_call_stack,
      halted: saved_halted
    }

    # Push the return value and advance past the CALL_FUNCTION instruction
    vm = GenericVM.push(vm, return_value)
    vm = GenericVM.advance_pc(vm)
    {nil, vm}
  end

  defp bind_args(func, args) do
    param_count = length(func.params)
    default_count = length(func.defaults)
    required_count = param_count - default_count

    # Start with required args
    bindings = Enum.zip(func.params, args) |> Map.new()

    # Fill in defaults for missing args
    if length(args) < param_count do
      missing_start = length(args)

      Enum.reduce(missing_start..(param_count - 1), bindings, fn i, acc ->
        default_idx = i - required_count
        default_val = if default_idx >= 0 and default_idx < default_count do
          Enum.at(func.defaults, default_idx)
        else
          nil
        end
        Map.put(acc, Enum.at(func.params, i), default_val)
      end)
    else
      bindings
    end
  end

  defp bind_args_with_kwargs(func, pos_args, kwargs) do
    bindings = Enum.zip(func.params, pos_args) |> Map.new()
    Map.merge(bindings, kwargs)
  end

  defp extract_builtin_result(vm, result) do
    case result do
      {output, %{__struct__: _} = new_vm} -> {new_vm, output}
      {nil, %{__struct__: _} = new_vm} -> {new_vm, nil}
      value -> {vm, value}
    end
  end

  # String methods
  defp get_string_method(str, "upper"), do: {:builtin, fn _args, _vm -> String.upcase(str) end}
  defp get_string_method(str, "lower"), do: {:builtin, fn _args, _vm -> String.downcase(str) end}
  defp get_string_method(str, "strip"), do: {:builtin, fn _args, _vm -> String.trim(str) end}
  defp get_string_method(str, "lstrip"), do: {:builtin, fn _args, _vm -> String.trim_leading(str) end}
  defp get_string_method(str, "rstrip"), do: {:builtin, fn _args, _vm -> String.trim_trailing(str) end}
  defp get_string_method(str, "startswith"), do: {:builtin, fn [prefix], _vm -> String.starts_with?(str, prefix) end}
  defp get_string_method(str, "endswith"), do: {:builtin, fn [suffix], _vm -> String.ends_with?(str, suffix) end}
  defp get_string_method(str, "replace"), do: {:builtin, fn [old, new_str], _vm -> String.replace(str, old, new_str) end}
  defp get_string_method(str, "split"), do: {:builtin, fn args, _vm ->
    case args do
      [] -> String.split(str)
      [sep] -> String.split(str, sep)
    end
  end}
  defp get_string_method(str, "join"), do: {:builtin, fn [items], _vm -> Enum.join(items, str) end}
  defp get_string_method(str, "find"), do: {:builtin, fn [sub], _vm ->
    case :binary.match(str, sub) do
      {pos, _len} -> pos
      :nomatch -> -1
    end
  end}
  defp get_string_method(str, "count"), do: {:builtin, fn [sub], _vm ->
    (String.length(str) - String.length(String.replace(str, sub, ""))) |> div(String.length(sub))
  end}
  defp get_string_method(str, "title"), do: {:builtin, fn _args, _vm ->
    str |> String.split() |> Enum.map(fn w ->
      String.capitalize(w)
    end) |> Enum.join(" ")
  end}
  defp get_string_method(str, "isdigit"), do: {:builtin, fn _args, _vm -> String.match?(str, ~r/^\d+$/) end}
  defp get_string_method(str, "isalpha"), do: {:builtin, fn _args, _vm -> String.match?(str, ~r/^[a-zA-Z]+$/) end}
  defp get_string_method(_str, name), do: raise(Errors.VMTypeError, "str has no method '#{name}'")

  # List methods
  defp get_list_method(the_list, "append"), do: {:builtin, fn [item], _vm -> the_list ++ [item] end}
  defp get_list_method(the_list, "extend"), do: {:builtin, fn [items], _vm -> the_list ++ items end}
  defp get_list_method(the_list, "insert"), do: {:builtin, fn [idx, item], _vm -> List.insert_at(the_list, idx, item) end}
  defp get_list_method(the_list, "remove"), do: {:builtin, fn [item], _vm -> List.delete(the_list, item) end}
  defp get_list_method(the_list, "pop"), do: {:builtin, fn args, _vm ->
    idx = case args do
      [] -> -1
      [i] -> i
    end
    actual_idx = if idx < 0, do: length(the_list) + idx, else: idx
    Enum.at(the_list, actual_idx)
  end}
  defp get_list_method(the_list, "index"), do: {:builtin, fn [item], _vm ->
    case Enum.find_index(the_list, fn x -> x == item end) do
      nil -> raise Errors.VMTypeError, "#{inspect(item)} is not in list"
      idx -> idx
    end
  end}
  defp get_list_method(_list, name), do: raise(Errors.VMTypeError, "list has no method '#{name}'")

  # Dict methods
  defp get_dict_method(the_dict, "keys"), do: {:builtin, fn _args, _vm -> Map.keys(the_dict) end}
  defp get_dict_method(the_dict, "values"), do: {:builtin, fn _args, _vm -> Map.values(the_dict) end}
  defp get_dict_method(the_dict, "items"), do: {:builtin, fn _args, _vm ->
    Enum.map(the_dict, fn {k, v} -> {k, v} end)
  end}
  defp get_dict_method(the_dict, "get"), do: {:builtin, fn args, _vm ->
    case args do
      [key] -> Map.get(the_dict, key)
      [key, default] -> Map.get(the_dict, key, default)
    end
  end}
  defp get_dict_method(the_dict, "pop"), do: {:builtin, fn args, _vm ->
    case args do
      [key] -> Map.get(the_dict, key)
      [key, default] -> Map.get(the_dict, key, default)
    end
  end}
  defp get_dict_method(the_dict, "update"), do: {:builtin, fn [other], _vm -> Map.merge(the_dict, other) end}
  defp get_dict_method(_dict, name), do: raise(Errors.VMTypeError, "dict has no method '#{name}'")

  defp slice_object(obj, start_val, stop_val, _step_val) when is_list(obj) do
    len = length(obj)
    s = normalize_index(start_val, len, 0)
    e = normalize_index(stop_val, len, len)
    Enum.slice(obj, s, max(e - s, 0))
  end

  defp slice_object(obj, start_val, stop_val, _step_val) when is_binary(obj) do
    len = String.length(obj)
    s = normalize_index(start_val, len, 0)
    e = normalize_index(stop_val, len, len)
    String.slice(obj, s, max(e - s, 0))
  end

  defp slice_object(obj, _start_val, _stop_val, _step_val) do
    raise Errors.VMTypeError, "Object is not sliceable: #{inspect(obj)}"
  end

  defp normalize_index(nil, _len, default), do: default
  defp normalize_index(idx, len, _default) when idx < 0, do: max(len + idx, 0)
  defp normalize_index(idx, len, _default), do: min(idx, len)
end
