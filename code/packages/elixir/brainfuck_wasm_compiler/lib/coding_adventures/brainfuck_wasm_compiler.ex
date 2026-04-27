defmodule CodingAdventures.BrainfuckWasmCompiler.PackageResult do
  @moduledoc """
  Output of a successful Brainfuck to WASM compilation.
  """

  defstruct source: "",
            filename: "program.bf",
            ast: nil,
            raw_ir: nil,
            optimized_ir: nil,
            module: nil,
            validated_module: nil,
            binary: <<>>,
            wasm_path: nil

  @type t :: %__MODULE__{
          source: String.t(),
          filename: String.t(),
          ast: term(),
          raw_ir: term(),
          optimized_ir: term(),
          module: term(),
          validated_module: term(),
          binary: binary(),
          wasm_path: Path.t() | nil
        }
end

defmodule CodingAdventures.BrainfuckWasmCompiler.PackageError do
  defexception [:stage, :message, :cause]

  @impl true
  def message(%__MODULE__{stage: stage, message: message}) do
    "[#{stage}] #{message}"
  end
end

defmodule CodingAdventures.BrainfuckWasmCompiler do
  @moduledoc """
  Compile Brainfuck source code into WebAssembly bytes.
  """

  alias CodingAdventures.Brainfuck
  alias CodingAdventures.BrainfuckIrCompiler
  alias CodingAdventures.BrainfuckIrCompiler.BuildConfig
  alias CodingAdventures.BrainfuckWasmCompiler.{PackageError, PackageResult}
  alias CodingAdventures.IrToWasmCompiler
  alias CodingAdventures.IrToWasmCompiler.FunctionSignature
  alias CodingAdventures.IrToWasmValidator
  alias CodingAdventures.WasmModuleEncoder
  alias CodingAdventures.WasmValidator

  @spec compile_source(String.t(), keyword()) ::
          {:ok, PackageResult.t()} | {:error, PackageError.t()}
  def compile_source(source, opts \\ []) when is_binary(source) do
    filename = Keyword.get(opts, :filename, "program.bf")
    build_config = Keyword.get(opts, :build_config, BuildConfig.release_config())
    signatures = [FunctionSignature.exported("_start", 0)]

    with {:ok, ast} <- normalize_result(Brainfuck.parse(source), "parse"),
         {:ok, ir_result} <-
           normalize_result(
             BrainfuckIrCompiler.compile(ast, filename, build_config),
             "ir-compile"
           ),
         :ok <- ensure_lowerable(ir_result.program, signatures) do
      try do
        module = IrToWasmCompiler.compile(ir_result.program, signatures)

        with {:ok, validated_module} <-
               normalize_result(WasmValidator.validate(module), "validate-wasm") do
          binary = WasmModuleEncoder.encode_module(module)

          {:ok,
           %PackageResult{
             source: source,
             filename: filename,
             ast: ast,
             raw_ir: ir_result.program,
             optimized_ir: ir_result.program,
             module: module,
             validated_module: validated_module,
             binary: binary
           }}
        end
      rescue
        error ->
          {:error, package_error("compile", Exception.message(error), error)}
      end
    end
  end

  @spec pack_source(String.t(), keyword()) ::
          {:ok, PackageResult.t()} | {:error, PackageError.t()}
  def pack_source(source, opts \\ []), do: compile_source(source, opts)

  @spec write_wasm_file(String.t(), Path.t(), keyword()) ::
          {:ok, PackageResult.t()} | {:error, PackageError.t()}
  def write_wasm_file(source, output_path, opts \\ []) do
    with {:ok, result} <- compile_source(source, opts),
         :ok <- write_binary(output_path, result.binary) do
      {:ok, %{result | wasm_path: output_path}}
    end
  end

  defp ensure_lowerable(program, signatures) do
    case IrToWasmValidator.validate(program, signatures) do
      [] -> :ok
      [%{message: message} | _] -> {:error, package_error("validate-ir", message)}
    end
  end

  defp normalize_result({:ok, value}, _stage), do: {:ok, value}
  defp normalize_result({:error, reason}, stage), do: {:error, package_error(stage, reason)}
  defp normalize_result(value, _stage), do: {:ok, value}

  defp write_binary(output_path, binary) do
    try do
      output_path
      |> Path.dirname()
      |> File.mkdir_p!()

      File.write!(output_path, binary)
      :ok
    rescue
      error ->
        {:error, package_error("write", Exception.message(error), error)}
    end
  end

  defp package_error(stage, message, cause \\ nil) do
    %PackageError{stage: stage, message: to_string(message), cause: cause}
  end
end
