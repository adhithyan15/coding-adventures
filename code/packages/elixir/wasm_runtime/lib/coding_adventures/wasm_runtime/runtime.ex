defmodule CodingAdventures.WasmRuntime.Runtime do
  @moduledoc """
  High-level WASM runtime that orchestrates the full pipeline:
  parse -> validate -> instantiate -> execute.

  This is the main entry point for running WASM modules in Elixir.
  It composes the parser, validator, and execution engine into a
  simple API.

  ## Pipeline

      ┌──────────┐     ┌──────────┐     ┌──────────┐     ┌──────────┐
      │  Binary   │ ──► │  Parse   │ ──► │ Validate │ ──► │Instance  │
      │  (.wasm)  │     │ (parser) │     │(validator)│    │(execute) │
      └──────────┘     └──────────┘     └──────────┘     └──────────┘

  ## Example

      {:ok, instance} = Runtime.instantiate_bytes(wasm_bytes)
      results = Runtime.call(instance, "square", [Values.i32(5)])
      # => [%{type: 0x7F, value: 25}]
  """

  alias CodingAdventures.WasmModuleParser
  alias CodingAdventures.WasmValidator
  alias CodingAdventures.WasmRuntime.Instance
  alias CodingAdventures.WasmRuntime.WasiStub
  alias CodingAdventures.WasmExecution.Values

  @doc """
  Parse, validate, and instantiate a WASM module from raw bytes.

  Optionally provide host functions. If none are provided and the module
  imports WASI functions, WASI stubs are automatically registered.

  Returns `{:ok, instance}` or `{:error, reason}`.

  ## Example

      {:ok, instance} = Runtime.instantiate_bytes(File.read!("module.wasm"))
  """
  @spec instantiate_bytes(binary(), map() | nil) :: {:ok, Instance.t()} | {:error, String.t()}
  def instantiate_bytes(wasm_bytes, host_functions \\ nil) when is_binary(wasm_bytes) do
    with {:ok, wasm_module} <- parse_module(wasm_bytes),
         {:ok, validated} <- WasmValidator.validate(wasm_module) do
      # Auto-register WASI stubs if needed and no explicit host functions provided
      final_host_fns = host_functions || auto_host_functions(wasm_module)

      Instance.from_validated(validated, final_host_fns)
    end
  end

  @doc """
  Call an exported function on an instance.

  Convenience wrapper around `Instance.call/3`.
  """
  @spec call(Instance.t(), String.t(), [Values.wasm_value()]) :: [Values.wasm_value()]
  def call(%Instance{} = instance, func_name, args \\ []) do
    Instance.call(instance, func_name, args)
  end

  # ===========================================================================
  # Private Helpers
  # ===========================================================================

  defp parse_module(wasm_bytes) do
    # The parser may return {:ok, module} or raise
    result = WasmModuleParser.parse(wasm_bytes)

    case result do
      {:ok, wasm_module} -> {:ok, wasm_module}
      {:error, _} = err -> err
      %CodingAdventures.WasmTypes.WasmModule{} = wasm_module -> {:ok, wasm_module}
    end
  rescue
    ex -> {:error, Exception.message(ex)}
  end

  defp auto_host_functions(wasm_module) do
    has_wasi =
      Enum.any?(wasm_module.imports, fn imp ->
        imp.module_name == "wasi_snapshot_preview1"
      end)

    if has_wasi, do: WasiStub.host_functions(), else: %{}
  end
end
