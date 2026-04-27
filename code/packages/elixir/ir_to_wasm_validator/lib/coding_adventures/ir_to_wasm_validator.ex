defmodule CodingAdventures.IrToWasmValidator.ValidationError do
  @moduledoc """
  A backend validation failure produced before lowering completes.
  """

  defstruct [:rule, :message]

  @type t :: %__MODULE__{
          rule: String.t(),
          message: String.t()
        }
end

defmodule CodingAdventures.IrToWasmValidator do
  @moduledoc """
  Validate whether the current IR can be lowered by the Elixir WASM backend.
  """

  alias CodingAdventures.CompilerIr.IrProgram
  alias CodingAdventures.IrToWasmCompiler
  alias CodingAdventures.IrToWasmCompiler.{FunctionSignature, WasmLoweringError}
  alias CodingAdventures.IrToWasmValidator.ValidationError

  @spec validate(IrProgram.t(), [FunctionSignature.t()] | nil) :: [ValidationError.t()]
  def validate(%IrProgram{} = program, function_signatures \\ nil) do
    try do
      _module = IrToWasmCompiler.compile(program, function_signatures)
      []
    rescue
      error in [WasmLoweringError] ->
        [%ValidationError{rule: "lowering", message: error.message}]
    end
  end
end
