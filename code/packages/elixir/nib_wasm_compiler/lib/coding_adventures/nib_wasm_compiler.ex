defmodule CodingAdventures.NibWasmCompiler.PackageResult do
  defstruct [
    :source,
    :ast,
    :typed_ast,
    :raw_ir,
    :optimized_ir,
    :module,
    :validated_module,
    :binary,
    :wasm_path
  ]
end

defmodule CodingAdventures.NibWasmCompiler.PackageError do
  defexception [:stage, :message, :cause]

  @impl true
  def message(%__MODULE__{stage: stage, message: message}), do: "[#{stage}] #{message}"
end

defmodule CodingAdventures.NibWasmCompiler do
  alias CodingAdventures.IrToWasmCompiler
  alias CodingAdventures.IrToWasmCompiler.FunctionSignature
  alias CodingAdventures.IrToWasmValidator
  alias CodingAdventures.NibIrCompiler
  alias CodingAdventures.NibParser
  alias CodingAdventures.NibTypeChecker
  alias CodingAdventures.NibWasmCompiler.{PackageError, PackageResult}
  alias CodingAdventures.Parser.ASTNode
  alias CodingAdventures.Lexer.Token
  alias CodingAdventures.WasmModuleEncoder
  alias CodingAdventures.WasmValidator

  @spec compile_source(String.t()) :: {:ok, PackageResult.t()} | {:error, PackageError.t()}
  def compile_source(source) do
    with {:ok, ast} <- normalize_result(NibParser.parse_nib(source), "parse"),
         type_result <- NibTypeChecker.check(ast),
         :ok <- ensure_type_ok(type_result),
         ir_result <- NibIrCompiler.compile_nib(type_result.typed_ast),
         signatures <- extract_signatures(type_result.typed_ast.root),
         :ok <- ensure_lowerable(ir_result.program, signatures) do
      try do
        module = IrToWasmCompiler.compile(ir_result.program, signatures)

        with {:ok, validated_module} <- normalize_result(WasmValidator.validate(module), "validate-wasm") do
          binary = WasmModuleEncoder.encode_module(module)

          {:ok,
           %PackageResult{
             source: source,
             ast: ast,
             typed_ast: type_result.typed_ast,
             raw_ir: ir_result.program,
             optimized_ir: ir_result.program,
             module: module,
             validated_module: validated_module,
             binary: binary
           }}
        end
      rescue
        error ->
          {:error, %PackageError{stage: "compile", message: Exception.message(error), cause: error}}
      end
    end
  end

  def pack_source(source), do: compile_source(source)

  def write_wasm_file(source, output_path) do
    with {:ok, result} <- compile_source(source),
         :ok <- write_binary(output_path, result.binary) do
      {:ok, %{result | wasm_path: output_path}}
    end
  end

  defp ensure_type_ok(%{ok: true}), do: :ok

  defp ensure_type_ok(type_result) do
    message =
      type_result.errors
      |> Enum.map(&"Line #{&1.line}, Col #{&1.column}: #{&1.message}")
      |> Enum.join("\n")

    {:error, %PackageError{stage: "type-check", message: message}}
  end

  defp ensure_lowerable(program, signatures) do
    case IrToWasmValidator.validate(program, signatures) do
      [] -> :ok
      [%{message: message} | _] -> {:error, %PackageError{stage: "validate-ir", message: message}}
    end
  end

  defp normalize_result({:ok, value}, _stage), do: {:ok, value}
  defp normalize_result({:error, reason}, stage), do: {:error, %PackageError{stage: stage, message: to_string(reason)}}
  defp normalize_result(value, _stage), do: {:ok, value}

  defp write_binary(output_path, binary) do
    try do
      output_path |> Path.dirname() |> File.mkdir_p!()
      File.write!(output_path, binary)
      :ok
    rescue
      error ->
        {:error, %PackageError{stage: "write", message: Exception.message(error), cause: error}}
    end
  end

  defp extract_signatures(root) do
    base = [%FunctionSignature{label: "_start", param_count: 0, export_name: "_start"}]

    extra =
    root
    |> child_nodes()
    |> Enum.filter(fn
      %ASTNode{rule_name: "top_decl"} -> true
      %ASTNode{rule_name: "fn_decl"} -> true
      _ -> false
    end)
    |> Enum.map(fn
      %ASTNode{rule_name: "top_decl"} = node -> List.first(child_nodes(node))
      node -> node
    end)
      |> Enum.filter(&(&1.rule_name == "fn_decl"))
      |> Enum.map(fn decl ->
        param_count =
          decl
          |> child_nodes()
          |> Enum.find(&(&1.rule_name == "param_list"))
          |> then(fn
            nil -> 0
            param_list -> Enum.count(child_nodes(param_list), &(&1.rule_name == "param"))
          end)

        %FunctionSignature{
          label: "_fn_#{first_name(decl)}",
          param_count: param_count,
          export_name: first_name(decl)
        }
      end)

    base ++ extra
  end

  defp child_nodes(%ASTNode{} = node), do: Enum.filter(node.children, &match?(%ASTNode{}, &1))
  defp child_nodes(_), do: []

  defp first_name(%ASTNode{} = node) do
    node.children
    |> Enum.find_value(fn
      %ASTNode{} = child -> first_name(child)
      %Token{type: "NAME", value: value} -> value
      %Token{} -> nil
    end)
  end
end
