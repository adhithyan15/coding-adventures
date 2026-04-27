defmodule CodingAdventures.WasmRuntime.Instance do
  @moduledoc """
  A WASM module instance -- a validated, instantiated module ready to execute.

  An instance holds:
  - The GenericVM with all instruction handlers registered
  - The execution context (memory, globals, function bodies, etc.)
  - The validated module reference

  ## Lifecycle

      parse -> validate -> instantiate -> call functions
      (wasm_module_parser) (wasm_validator) (instance) (instance)

  ## Example

      {:ok, instance} = Instance.from_validated(validated_module)
      results = Instance.call(instance, "square", [Values.i32(5)])
      # => [%{type: 0x7F, value: 25}]
  """

  alias CodingAdventures.WasmRuntime.Engine
  alias CodingAdventures.WasmExecution.Values
  alias CodingAdventures.WasmValidator.ValidatedModule

  defstruct [:vm, :context, :validated_module]

  @type t :: %__MODULE__{
          vm: any(),
          context: Engine.execution_context(),
          validated_module: ValidatedModule.t()
        }

  @doc """
  Create a WASM instance from a validated module.

  Optionally provide host functions as a map of
  `{module_name, func_name} => handler_fn`.

  ## Example

      {:ok, instance} = Instance.from_validated(validated)
      {:ok, instance} = Instance.from_validated(validated, WasiStub.host_functions())
  """
  @spec from_validated(ValidatedModule.t(), map()) :: {:ok, t()}
  def from_validated(%ValidatedModule{} = validated, host_functions \\ %{}) do
    {vm, ctx} = Engine.instantiate_full(validated, host_functions)

    instance = %__MODULE__{
      vm: vm,
      context: ctx,
      validated_module: validated
    }

    {:ok, instance}
  end

  @doc """
  Call an exported function on this instance.

  Arguments must be WasmValue maps (e.g., `Values.i32(5)`).
  Returns a list of WasmValue results.

  ## Example

      results = Instance.call(instance, "add", [Values.i32(3), Values.i32(4)])
      [%{type: 0x7F, value: 7}] = results
  """
  @spec call(t(), String.t(), [Values.wasm_value()]) :: [Values.wasm_value()]
  def call(%__MODULE__{} = instance, func_name, args \\ []) do
    Engine.call_function(instance.vm, instance.context, func_name, args)
  end

  @doc """
  Get the current linear memory from the instance (or nil).
  """
  @spec memory(t()) :: any()
  def memory(%__MODULE__{context: ctx}), do: ctx.memory

  @doc """
  Get the current globals from the instance.
  """
  @spec globals(t()) :: [Values.wasm_value()]
  def globals(%__MODULE__{context: ctx}), do: ctx.globals
end
