defmodule CodingAdventures.WasmRuntime.Engine do
  @moduledoc """
  WASM Execution Engine -- bridges parsed/validated modules to the GenericVM.

  The engine is responsible for:

  1. **Instantiation** -- taking a validated WASM module and creating
     an execution context with memory, tables, globals, and function bodies.

  2. **Function dispatch** -- looking up exported functions by name,
     setting up the call frame (locals, typed stack), and running
     the GenericVM's context-aware execution loop.

  3. **Call handling** -- when a function calls another function (via
     the `call` instruction), the engine intercepts the pending_call
     signal in the context, saves the current state, and dispatches
     the callee.

  ## Architecture

      ┌────────────────────────────────────────────────┐
      │                    Engine                       │
      │                                                 │
      │  ValidatedModule ──► ExecutionContext            │
      │                        ├── memory               │
      │                        ├── tables               │
      │                        ├── globals              │
      │                        ├── typed_locals         │
      │                        ├── label_stack          │
      │                        ├── control_flow_map     │
      │                        └── func_bodies          │
      │                                                 │
      │  GenericVM + Dispatch.register_all              │
      │  ──► execute_with_context(vm, code, ctx)        │
      └────────────────────────────────────────────────┘
  """

  alias CodingAdventures.VirtualMachine.GenericVM
  alias CodingAdventures.VirtualMachine.Types.CodeObject

  alias CodingAdventures.WasmExecution.{
    Decoder,
    Values,
    LinearMemory,
    ConstExpr,
    TrapError
  }

  alias CodingAdventures.WasmExecution.Instructions.Dispatch
  alias CodingAdventures.WasmValidator.ValidatedModule
  alias CodingAdventures.WasmTypes.WasmModule

  # ===========================================================================
  # Execution Context
  # ===========================================================================

  @typedoc """
  The execution context carries all WASM-specific mutable state through
  the GenericVM's execution loop.

  - `memory` -- the linear memory (or nil if none)
  - `tables` -- list of Table structs
  - `globals` -- list of typed WASM values
  - `typed_locals` -- list of typed WASM values for the current function
  - `label_stack` -- stack of block/loop/if labels for control flow
  - `control_flow_map` -- maps block/loop/if instruction indices to end/else PCs
  - `func_bodies` -- list of decoded function bodies (list of {code_object, control_flow_map})
  - `func_types` -- list of FuncType structs
  - `num_imported_funcs` -- how many entries in func_types are imports
  - `pending_call` -- set by the call instruction, read by the engine
  - `host_functions` -- map of {module_name, func_name} => handler_fn
  """
  @type execution_context :: %{
          memory: LinearMemory.t() | nil,
          tables: [any()],
          globals: [Values.wasm_value()],
          typed_locals: [Values.wasm_value()],
          label_stack: [map()],
          control_flow_map: map(),
          func_bodies: [any()],
          func_types: [any()],
          num_imported_funcs: non_neg_integer(),
          pending_call: non_neg_integer() | nil,
          host_functions: map()
        }

  # ===========================================================================
  # Public API
  # ===========================================================================

  @doc """
  Instantiate a validated WASM module into an execution context.

  This sets up memory, tables, and globals according to the module's
  declarations, initializes data and element segments, and decodes
  all function bodies.

  Returns `{vm, context}` where `vm` is a GenericVM with all WASM handlers
  registered and `context` is the execution context.
  """
  @spec instantiate(ValidatedModule.t(), map()) :: {GenericVM.t(), execution_context()}
  def instantiate(%ValidatedModule{} = validated, host_functions \\ %{}) do
    wasm_mod = validated.module

    # 1. Create the GenericVM with all WASM instruction handlers
    vm = GenericVM.new() |> Dispatch.register_all()

    # 2. Initialize linear memory (WASM 1.0 allows at most one)
    imported_mem = find_imported_memory(wasm_mod)
    declared_mem = List.first(wasm_mod.memories)

    mem_spec = imported_mem || declared_mem

    memory =
      if mem_spec do
        min_pages = mem_spec.limits.min
        max_pages = mem_spec.limits.max
        LinearMemory.new(min_pages, max_pages)
      else
        nil
      end

    # 3. Initialize globals
    globals = initialize_globals(wasm_mod)

    # 4. Initialize data segments (copy static data into memory)
    memory = initialize_data_segments(memory, wasm_mod.data, globals)

    # 5. Decode all function bodies
    func_bodies =
      Enum.map(wasm_mod.code, fn body ->
        decoded = Decoder.decode_function_body(body.code)
        cf_map = Decoder.build_control_flow_map(decoded)
        vm_instructions = Decoder.to_vm_instructions(decoded)
        code_object = %CodeObject{instructions: vm_instructions}
        {code_object, cf_map, body.locals}
      end)

    # 6. Build the execution context
    ctx = %{
      memory: memory,
      tables: [],
      globals: globals,
      typed_locals: [],
      label_stack: [],
      control_flow_map: %{},
      func_bodies: func_bodies,
      func_types: validated.func_types,
      num_imported_funcs: validated.num_imported_funcs,
      pending_call: nil,
      host_functions: host_functions
    }

    {vm, ctx}
  end

  @doc """
  Call an exported function by name with the given arguments.

  Looks up the function in the module's export table, sets up the call
  frame (arguments + zero-initialized locals), and runs the GenericVM.

  Returns the result values as a list of WasmValues.

  ## Example

      {vm, ctx} = Engine.instantiate(validated)
      results = Engine.call_function(vm, ctx, "square", [Values.i32(5)])
      # => [%{type: 0x7F, value: 25}]
  """
  @spec call_function(GenericVM.t(), execution_context(), String.t(), [Values.wasm_value()]) ::
          [Values.wasm_value()]
  def call_function(vm, ctx, func_name, args) do
    # Find the export
    wasm_mod = find_module_from_ctx(ctx)
    func_idx = find_exported_function(wasm_mod, func_name)

    if func_idx == nil do
      raise TrapError, "Function '#{func_name}' not exported"
    end

    {results, _ctx} = invoke_function(vm, ctx, func_idx, args)
    results
  end

  @doc """
  Invoke a function by index with the given arguments.

  This is the core dispatch function used by both `call_function` and
  the internal `call` instruction handler.
  """
  @spec invoke_function(GenericVM.t(), execution_context(), non_neg_integer(), [
          Values.wasm_value()
        ]) ::
          [Values.wasm_value()]
  def invoke_function(vm, ctx, func_idx, args) do
    if func_idx < ctx.num_imported_funcs do
      # Imported function -- dispatch to host
      invoke_host_function(vm, ctx, func_idx, args)
    else
      # Module-defined function
      body_idx = func_idx - ctx.num_imported_funcs
      invoke_module_function(vm, ctx, func_idx, body_idx, args)
    end
  end

  # ===========================================================================
  # Private: Module Function Invocation
  # ===========================================================================

  defp invoke_module_function(vm, ctx, func_idx, body_idx, args) do
    {code_object, cf_map, body_locals} = Enum.at(ctx.func_bodies, body_idx)
    func_type = Enum.at(ctx.func_types, func_idx)

    # Build locals: params (from args) + zero-initialized body locals
    param_locals = args

    zero_locals =
      Enum.map(body_locals, fn type_atom ->
        Values.default_value(type_atom)
      end)

    all_locals = param_locals ++ zero_locals

    # Determine result arity
    result_arity = length(func_type.results)

    # Set up fresh execution state
    vm = %{vm | pc: 0, halted: false, typed_stack: []}

    # Set up context for this function
    func_ctx = %{
      ctx
      | typed_locals: all_locals,
        label_stack: [],
        control_flow_map: cf_map,
        pending_call: nil
    }

    # Run the execution loop, handling nested calls
    execute_loop(vm, code_object, func_ctx, result_arity)
  end

  defp execute_loop(vm, code_object, ctx, result_arity) do
    if vm.halted or vm.pc >= length(code_object.instructions) do
      # Execution complete -- collect results
      {collect_results(vm, result_arity), ctx}
    else
      # Execute one step
      {_trace, vm, ctx} = GenericVM.step_with_context(vm, code_object, ctx)

      # Check for pending calls
      case Map.get(ctx, :pending_call) do
        nil ->
          execute_loop(vm, code_object, ctx, result_arity)

        target_func_idx ->
          # Handle the call
          ctx = Map.put(ctx, :pending_call, nil)
          {vm, ctx} = handle_call(vm, ctx, target_func_idx)
          execute_loop(vm, code_object, ctx, result_arity)
      end
    end
  end

  defp handle_call(vm, ctx, target_func_idx) do
    callee_type = Enum.at(ctx.func_types, target_func_idx)
    num_params = length(callee_type.params)

    # Pop arguments from the caller's stack
    {call_args, vm} = pop_n_typed(vm, num_params)

    # Invoke the callee
    {callee_results, callee_ctx} = invoke_function(vm, ctx, target_func_idx, call_args)

    ctx = %{
      ctx
      | memory: callee_ctx.memory,
        tables: callee_ctx.tables,
        globals: callee_ctx.globals
    }

    # Push results back onto the caller's stack
    vm =
      Enum.reduce(callee_results, vm, fn val, acc_vm ->
        GenericVM.push_typed(acc_vm, val)
      end)

    {vm, ctx}
  end

  defp invoke_host_function(_vm, ctx, func_idx, args) do
    # Look up the import by index
    wasm_mod = find_module_from_ctx(ctx)

    func_imports =
      wasm_mod.imports
      |> Enum.filter(fn imp -> imp.kind == :function end)

    imp = Enum.at(func_imports, func_idx)

    if imp == nil do
      raise TrapError, "Host function at index #{func_idx} not found"
    end

    handler_key = {imp.module_name, imp.name}
    handler = Map.get(ctx.host_functions, handler_key)

    if handler == nil do
      raise TrapError, "No host function registered for #{imp.module_name}.#{imp.name}"
    end

    result =
      try do
        handler.({args, ctx.memory})
      rescue
        FunctionClauseError -> handler.(args)
      end

    case result do
      {results, updated_memory} ->
        {results, %{ctx | memory: updated_memory}}

      results when is_list(results) ->
        {results, ctx}
    end
  end

  # ===========================================================================
  # Private: Helpers
  # ===========================================================================

  defp collect_results(vm, arity) do
    {results, _vm} = pop_n_typed(vm, arity)
    results
  end

  defp pop_n_typed(vm, 0), do: {[], vm}

  defp pop_n_typed(vm, n) do
    {values, final_vm} =
      Enum.reduce(1..n, {[], vm}, fn _, {acc, acc_vm} ->
        {val, acc_vm} = GenericVM.pop_typed(acc_vm)
        {[val | acc], acc_vm}
      end)

    # Values come out in reverse order (last popped = first param)
    # We already accumulate with [val | acc], so they end up reversed.
    # For function results, the first popped is the last result.
    # We want results in declaration order, so reverse.
    {values, final_vm}
  end

  defp find_exported_function(wasm_mod, func_name) do
    export =
      Enum.find(wasm_mod.exports, fn exp ->
        exp.kind == :function and exp.name == func_name
      end)

    if export, do: export.index, else: nil
  end

  defp find_module_from_ctx(ctx) do
    # We need access to the original WasmModule. Store it in the context
    # during instantiation. If not available, reconstruct from func_types.
    Map.get(ctx, :wasm_module)
  end

  defp find_imported_memory(%WasmModule{imports: imports}) do
    imp = Enum.find(imports, fn imp -> imp.kind == :memory end)

    if imp do
      case imp.type_info do
        {:memory, mem_type} -> mem_type
        _ -> nil
      end
    else
      nil
    end
  end

  defp initialize_globals(%WasmModule{globals: globals}) do
    {result, _} =
      Enum.reduce(globals, {[], []}, fn global, {acc, initialized} ->
        val = ConstExpr.evaluate(global.init_expr, initialized)
        {acc ++ [val], initialized ++ [val]}
      end)

    result
  end

  defp initialize_data_segments(nil, _segments, _globals), do: nil

  defp initialize_data_segments(memory, segments, globals) do
    Enum.reduce(segments, memory, fn seg, mem ->
      offset_val = ConstExpr.evaluate(seg.offset_expr, globals)
      mem_offset = Values.as_i32(offset_val)
      LinearMemory.write_raw_bytes(mem, mem_offset, seg.data)
    end)
  end

  @doc """
  Instantiate with the WasmModule stored in context for export lookups.
  """
  @spec instantiate_full(ValidatedModule.t(), map()) :: {GenericVM.t(), execution_context()}
  def instantiate_full(%ValidatedModule{} = validated, host_functions \\ %{}) do
    {vm, ctx} = instantiate(validated, host_functions)
    ctx = Map.put(ctx, :wasm_module, validated.module)
    {vm, ctx}
  end
end
