defmodule CodingAdventures.TypeCheckerProtocol.TypeErrorDiagnostic do
  @enforce_keys [:message, :line, :column]
  defstruct [:message, :line, :column]

  @type t :: %__MODULE__{
          message: String.t(),
          line: pos_integer(),
          column: pos_integer()
        }
end

defmodule CodingAdventures.TypeCheckerProtocol.TypeCheckResult do
  @enforce_keys [:typed_ast, :errors, :ok]
  defstruct [:typed_ast, :errors, :ok]

  @type t :: %__MODULE__{
          typed_ast: term(),
          errors: [CodingAdventures.TypeCheckerProtocol.TypeErrorDiagnostic.t()],
          ok: boolean()
        }
end

defmodule CodingAdventures.TypeCheckerProtocol do
  alias CodingAdventures.TypeCheckerProtocol.TypeCheckResult
  alias CodingAdventures.TypeCheckerProtocol.TypeErrorDiagnostic

  @callback check(term()) :: TypeCheckResult.t()

  def new_result(typed_ast, errors \\ []) do
    %TypeCheckResult{typed_ast: typed_ast, errors: errors, ok: errors == []}
  end

  def new_diagnostic(message, line \\ 1, column \\ 1) do
    %TypeErrorDiagnostic{message: message, line: line, column: column}
  end
end

defmodule CodingAdventures.TypeCheckerProtocol.GenericTypeChecker do
  alias CodingAdventures.TypeCheckerProtocol

  defstruct hooks: %{}, errors: [], node_kind: nil, locate: nil

  def new(opts \\ []) do
    %__MODULE__{
      hooks: %{},
      errors: [],
      node_kind: Keyword.get(opts, :node_kind),
      locate: Keyword.get(opts, :locate, fn _ -> {1, 1} end)
    }
  end

  def reset(%__MODULE__{} = checker) do
    %{checker | errors: []}
  end

  def register_hook(%__MODULE__{} = checker, phase, kind, hook) do
    key_kind = if kind == "*", do: "*", else: normalize_kind(kind)
    key = "#{phase}:#{key_kind}"
    hooks = Map.update(checker.hooks, key, [hook], fn existing -> existing ++ [hook] end)
    %{checker | hooks: hooks}
  end

  def dispatch(%__MODULE__{} = checker, phase, node, args \\ []) do
    kind =
      if is_function(checker.node_kind, 1) do
        checker.node_kind.(node) |> to_string() |> normalize_kind()
      else
        ""
      end

    Enum.reduce_while(["#{phase}:#{kind}", "#{phase}:*"], nil, fn key, _acc ->
      hooks = Map.get(checker.hooks, key, [])

      result =
        Enum.reduce_while(hooks, :not_handled, fn hook, _inner ->
          value = hook.(node, args)

          if value == :not_handled do
            {:cont, :not_handled}
          else
            {:halt, value}
          end
        end)

      if result == :not_handled do
        {:cont, nil}
      else
        {:halt, result}
      end
    end)
  end

  def not_handled, do: :not_handled

  def error(%__MODULE__{} = checker, message, subject) do
    {line, column} = checker.locate.(subject)
    diagnostic = TypeCheckerProtocol.new_diagnostic(message, line, column)
    %{checker | errors: checker.errors ++ [diagnostic]}
  end

  def check(%__MODULE__{} = checker, ast, run_fun) when is_function(run_fun, 2) do
    checker = reset(checker)
    checker = run_fun.(checker, ast)
    TypeCheckerProtocol.new_result(ast, checker.errors)
  end

  defp normalize_kind(kind) do
    kind
    |> String.graphemes()
    |> Enum.reduce({"", false}, fn grapheme, {acc, last_underscore} ->
      if grapheme =~ ~r/^[[:alnum:]]$/u do
        {acc <> grapheme, false}
      else
        if last_underscore do
          {acc, true}
        else
          {acc <> "_", true}
        end
      end
    end)
    |> elem(0)
    |> String.trim("_")
  end
end
