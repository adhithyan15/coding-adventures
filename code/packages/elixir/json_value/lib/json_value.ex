defmodule CodingAdventures.JsonValue do
  @moduledoc """
  JSON Value — Convert parser ASTs into typed JSON representations.

  ## The Problem

  The json-parser package gives us an AST (abstract syntax tree) — a generic
  tree of `ASTNode` and `Token` structs. That's great for representing
  structure, but it doesn't tell us "this is a JSON object with keys and
  values." We need a typed intermediate representation.

  ## The Solution: Tagged Tuples

  Elixir is a functional language, so we use tagged tuples instead of classes:

      {:object, [{key, value}, ...]}   # ordered list of key-value pairs
      {:array, [json_value, ...]}      # list of JSON values
      {:string, binary}                # a string
      {:number, integer | float}       # a number (int or float)
      {:boolean, boolean}              # true or false
      :null                            # JSON null

  This is idiomatic Elixir — pattern matching on tagged tuples is natural
  and efficient.

  ## Why Ordered Pairs for Objects?

  RFC 8259 says JSON objects are "unordered," but in practice insertion order
  matters for:
  - Human readability (keys in a sensible order)
  - Round-trip fidelity (parse then serialize should preserve order)
  - Deterministic output (same input = same output)

  We store object pairs as a list of `{key, value}` tuples, preserving the
  order they appeared in the source JSON.

  ## Usage

      # Parse JSON text into a typed value
      {:ok, value} = CodingAdventures.JsonValue.parse(~s({"name": "Alice"}))
      # => {:ok, {:object, [{"name", {:string, "Alice"}}]}}

      # Convert to native Elixir types
      native = CodingAdventures.JsonValue.to_native(value)
      # => %{"name" => "Alice"}

      # Convert native Elixir types back to JsonValue
      {:ok, value} = CodingAdventures.JsonValue.from_native(%{"name" => "Alice"})
      # => {:ok, {:object, [{"name", {:string, "Alice"}}]}}
  """

  alias CodingAdventures.JsonParser
  alias CodingAdventures.Parser.ASTNode
  alias CodingAdventures.Lexer.Token

  # ---------------------------------------------------------------------------
  # Type Definitions
  # ---------------------------------------------------------------------------
  #
  # These typespecs document the six JSON value types as Elixir tagged tuples.
  # The `json_value` type is a union of all six — any function that accepts or
  # returns a JSON value uses this type.

  @type json_value ::
          {:object, [{String.t(), json_value}]}
          | {:array, [json_value]}
          | {:string, String.t()}
          | {:number, integer | float}
          | {:boolean, boolean}
          | :null

  # ---------------------------------------------------------------------------
  # from_ast/1 — Convert an ASTNode tree into a json_value
  # ---------------------------------------------------------------------------
  #
  # This is the core tree walk. The json-parser produces an AST with these
  # rule names:
  #
  #   "value"  — wraps exactly one meaningful child (object, array, or token)
  #   "object" — LBRACE, then pair nodes, then RBRACE
  #   "pair"   — STRING token, COLON token, value node
  #   "array"  — LBRACKET, then value nodes, then RBRACKET
  #
  # Tokens are leaf nodes with types like "STRING", "NUMBER", "TRUE", etc.
  #
  # We dispatch on the node type:
  #   - Token → convert directly (string, number, boolean, null)
  #   - ASTNode → dispatch on rule_name

  @doc """
  Convert a json-parser AST node into a typed JSON value.

  The AST is produced by `CodingAdventures.JsonParser.parse/1`. This function
  walks the tree recursively, converting each node into the appropriate tagged
  tuple representation.

  ## Examples

      {:ok, ast} = CodingAdventures.JsonParser.parse("42")
      {:ok, {:number, 42}} = CodingAdventures.JsonValue.from_ast(ast)

      {:ok, ast} = CodingAdventures.JsonParser.parse(~s({"key": "val"}))
      {:ok, {:object, [{"key", {:string, "val"}}]}} = CodingAdventures.JsonValue.from_ast(ast)
  """
  @spec from_ast(ASTNode.t() | Token.t()) :: {:ok, json_value} | {:error, String.t()}
  def from_ast(%ASTNode{} = node) do
    case convert_node(node) do
      {:error, _} = err -> err
      val -> {:ok, val}
    end
  end

  def from_ast(%Token{} = token) do
    case convert_token(token) do
      {:error, _} = err -> err
      val -> {:ok, val}
    end
  end

  def from_ast(_other) do
    {:error, "Expected an ASTNode or Token"}
  end

  # ---------------------------------------------------------------------------
  # to_native/1 — Convert a json_value to native Elixir types
  # ---------------------------------------------------------------------------
  #
  # The mapping is straightforward:
  #
  #   {:object, pairs}   → %{key => native_value}   (Elixir map)
  #   {:array, elements} → [native_value, ...]       (Elixir list)
  #   {:string, s}       → s                         (binary/string)
  #   {:number, n}       → n                         (integer or float)
  #   {:boolean, b}      → b                         (true or false)
  #   :null              → nil
  #
  # The conversion is recursive — nested JSON values are also converted.

  @doc """
  Convert a JSON value to native Elixir types.

  ## Examples

      CodingAdventures.JsonValue.to_native({:string, "hello"})
      # => "hello"

      CodingAdventures.JsonValue.to_native({:object, [{"a", {:number, 1}}]})
      # => %{"a" => 1}

      CodingAdventures.JsonValue.to_native(:null)
      # => nil
  """
  @spec to_native(json_value) :: map | list | String.t() | number | boolean | nil
  def to_native({:object, pairs}) do
    # Convert ordered pairs to an Elixir map.
    # Note: Elixir maps do not guarantee insertion order, but for small maps
    # (< 32 keys), the internal representation preserves order in practice.
    Map.new(pairs, fn {key, val} -> {key, to_native(val)} end)
  end

  def to_native({:array, elements}) do
    Enum.map(elements, &to_native/1)
  end

  def to_native({:string, str}), do: str
  def to_native({:number, num}), do: num
  def to_native({:boolean, bool_val}), do: bool_val
  def to_native(:null), do: nil

  # ---------------------------------------------------------------------------
  # from_native/1 — Convert native Elixir types to a json_value
  # ---------------------------------------------------------------------------
  #
  # This is the inverse of to_native/1. It accepts:
  #   - map with string keys  → {:object, pairs}
  #   - list                  → {:array, elements}
  #   - binary (string)       → {:string, s}
  #   - integer               → {:number, n}
  #   - float                 → {:number, n}
  #   - boolean               → {:boolean, b}
  #   - nil                   → :null
  #
  # Non-JSON-compatible types (atoms other than nil/true/false, tuples,
  # functions, etc.) produce an error.

  @doc """
  Convert native Elixir types to a JSON value.

  ## Examples

      {:ok, {:string, "hello"}} = CodingAdventures.JsonValue.from_native("hello")
      {:ok, {:number, 42}} = CodingAdventures.JsonValue.from_native(42)
      {:ok, :null} = CodingAdventures.JsonValue.from_native(nil)

  Non-string map keys produce an error:

      {:error, _} = CodingAdventures.JsonValue.from_native(%{1 => "val"})
  """
  @spec from_native(any) :: {:ok, json_value} | {:error, String.t()}
  def from_native(nil), do: {:ok, :null}
  def from_native(val) when is_boolean(val), do: {:ok, {:boolean, val}}
  def from_native(val) when is_integer(val), do: {:ok, {:number, val}}
  def from_native(val) when is_float(val), do: {:ok, {:number, val}}
  def from_native(val) when is_binary(val), do: {:ok, {:string, val}}

  def from_native(val) when is_list(val) do
    # Convert each element; bail on the first error.
    convert_list_elements(val, [])
  end

  def from_native(val) when is_map(val) do
    # All keys must be strings. Convert to ordered pairs.
    convert_map_pairs(Map.to_list(val), [])
  end

  def from_native(val) do
    {:error, "Cannot convert #{inspect(val)} to a JSON value — unsupported type"}
  end

  # ---------------------------------------------------------------------------
  # parse/1 — Parse JSON text into a json_value
  # ---------------------------------------------------------------------------

  @doc """
  Parse JSON text into a typed JSON value.

  This is a convenience function that combines lexing, parsing, and AST
  conversion into a single call.

  ## Examples

      {:ok, {:number, 42}} = CodingAdventures.JsonValue.parse("42")
      {:ok, {:string, "hi"}} = CodingAdventures.JsonValue.parse(~s("hi"))
      {:error, _} = CodingAdventures.JsonValue.parse("not json")
  """
  @spec parse(String.t()) :: {:ok, json_value} | {:error, String.t()}
  def parse(text) do
    case JsonParser.parse(text) do
      {:ok, ast} -> from_ast(ast)
      {:error, msg} -> {:error, msg}
    end
  end

  # ---------------------------------------------------------------------------
  # parse_native/1 — Parse JSON text into native Elixir types
  # ---------------------------------------------------------------------------

  @doc """
  Parse JSON text directly into native Elixir types.

  Equivalent to `parse(text) |> to_native()`. This is the most common use
  case — "give me a map/list from this JSON string."

  ## Examples

      {:ok, %{"name" => "Alice"}} = CodingAdventures.JsonValue.parse_native(~s({"name": "Alice"}))
      {:ok, [1, 2, 3]} = CodingAdventures.JsonValue.parse_native("[1, 2, 3]")
  """
  @spec parse_native(String.t()) :: {:ok, any} | {:error, String.t()}
  def parse_native(text) do
    case parse(text) do
      {:ok, val} -> {:ok, to_native(val)}
      {:error, msg} -> {:error, msg}
    end
  end

  # ===========================================================================
  # Private Helpers
  # ===========================================================================

  # ---------------------------------------------------------------------------
  # AST Conversion Internals
  # ---------------------------------------------------------------------------
  #
  # The AST produced by json-parser has this shape:
  #
  #   ASTNode(rule="value", children=[...])
  #     The top-level wrapper. Children include the actual value node/token
  #     plus potentially structural tokens (which we skip).
  #
  #   ASTNode(rule="object", children=[LBRACE, pair*, RBRACE])
  #     Structural tokens (LBRACE, RBRACE, COMMA) are in the children list.
  #     We only care about children that are ASTNode with rule="pair".
  #
  #   ASTNode(rule="pair", children=[STRING, COLON, value])
  #     The STRING token is the key, the value is an ASTNode wrapping the val.
  #
  #   ASTNode(rule="array", children=[LBRACKET, value*, RBRACKET])
  #     We only care about children that are ASTNode with rule="value".

  defp convert_node(%ASTNode{rule_name: "value", children: children}) do
    # The "value" rule wraps exactly one meaningful child.
    # Find the first child that is either:
    #   a) An ASTNode (object or array)
    #   b) A Token with a value type (STRING, NUMBER, TRUE, FALSE, NULL)
    meaningful_child = find_meaningful_child(children)

    case meaningful_child do
      nil -> {:error, "Empty value node"}
      %ASTNode{} = child_node -> convert_node(child_node)
      %Token{} = token -> convert_token(token)
    end
  end

  defp convert_node(%ASTNode{rule_name: "object", children: children}) do
    # Extract all "pair" children, skipping structural tokens (LBRACE, etc.)
    pairs =
      children
      |> Enum.filter(fn
        %ASTNode{rule_name: "pair"} -> true
        _ -> false
      end)
      |> Enum.map(&convert_pair/1)

    # Check for errors in any pair
    first_error = Enum.find(pairs, fn
      {:error, _} -> true
      _ -> false
    end)

    case first_error do
      nil -> {:object, pairs}
      err -> err
    end
  end

  defp convert_node(%ASTNode{rule_name: "array", children: children}) do
    # Extract all "value" children, skipping structural tokens (LBRACKET, etc.)
    # Array elements can be ASTNode with rule="value" OR direct tokens.
    elements =
      children
      |> Enum.filter(fn
        %ASTNode{rule_name: "value"} -> true
        _ -> false
      end)
      |> Enum.map(&convert_node/1)

    # Check for errors
    first_error = Enum.find(elements, fn
      {:error, _} -> true
      _ -> false
    end)

    case first_error do
      nil -> {:array, elements}
      err -> err
    end
  end

  defp convert_node(%ASTNode{rule_name: rule_name}) do
    {:error, "Unexpected rule: #{rule_name}"}
  end

  # Convert a "pair" node into a {key, value} tuple.
  # The pair has children: [STRING_token, COLON_token, value_ASTNode]
  defp convert_pair(%ASTNode{rule_name: "pair", children: children}) do
    # Find the STRING token (the key)
    key_token = Enum.find(children, fn
      %Token{type: "STRING"} -> true
      _ -> false
    end)

    # Find the value ASTNode
    value_node = Enum.find(children, fn
      %ASTNode{rule_name: "value"} -> true
      _ -> false
    end)

    case {key_token, value_node} do
      {nil, _} -> {:error, "Pair missing key"}
      {_, nil} -> {:error, "Pair missing value"}
      {%Token{value: key_str}, val_ast} ->
        case convert_node(val_ast) do
          {:error, _} = err -> err
          val -> {key_str, val}
        end
    end
  end

  # Find the first "meaningful" child in a value node's children.
  # Meaningful means: an ASTNode (object/array), or a value-carrying token.
  # Structural tokens (LBRACE, RBRACE, COMMA, COLON, etc.) are skipped.
  @value_token_types ~w(STRING NUMBER TRUE FALSE NULL)

  defp find_meaningful_child(children) do
    Enum.find(children, fn
      %ASTNode{} -> true
      %Token{type: token_type} -> token_type in @value_token_types
      _ -> false
    end)
  end

  # Convert a single token to a json_value.
  #
  # Token types and their conversions:
  #   "STRING"  → {:string, value}      (value is already unescaped by lexer)
  #   "NUMBER"  → {:number, int|float}  (parse the string to a number)
  #   "TRUE"    → {:boolean, true}
  #   "FALSE"   → {:boolean, false}
  #   "NULL"    → :null
  #
  # For numbers, we distinguish integers from floats:
  #   - "42" → 42 (integer) — no decimal point or exponent
  #   - "3.14" → 3.14 (float) — has a decimal point
  #   - "1e10" → 1.0e10 (float) — has an exponent
  defp convert_token(%Token{type: "STRING", value: str}), do: {:string, str}

  defp convert_token(%Token{type: "NUMBER", value: num_str}) do
    # If the number string contains a decimal point or exponent, parse as float.
    # Otherwise, parse as integer.
    if String.contains?(num_str, ".") or String.contains?(num_str, "e") or
         String.contains?(num_str, "E") do
      {:number, String.to_float(normalize_float_string(num_str))}
    else
      {:number, String.to_integer(num_str)}
    end
  end

  defp convert_token(%Token{type: "TRUE"}), do: {:boolean, true}
  defp convert_token(%Token{type: "FALSE"}), do: {:boolean, false}
  defp convert_token(%Token{type: "NULL"}), do: :null

  defp convert_token(%Token{type: token_type}) do
    {:error, "Unexpected token type: #{token_type}"}
  end

  # Normalize a float string so Elixir's String.to_float can parse it.
  #
  # Elixir's String.to_float requires:
  #   - A decimal point (so "1e10" must become "1.0e10")
  #   - At least one digit before the decimal point
  #
  # JSON allows numbers like "1e10" (no decimal point) which we need to
  # handle by inserting ".0" before the exponent.
  defp normalize_float_string(num_str) do
    if String.contains?(num_str, ".") do
      num_str
    else
      # No decimal point but has exponent (e.g., "1e10" → "1.0e10")
      case String.split(num_str, ~r/[eE]/, parts: 2) do
        [mantissa, exponent] ->
          separator = if String.contains?(num_str, "E"), do: "E", else: "e"
          mantissa <> ".0" <> separator <> exponent

        _ ->
          num_str
      end
    end
  end

  # ---------------------------------------------------------------------------
  # from_native helpers
  # ---------------------------------------------------------------------------

  # Convert a list of native values to json_value elements, accumulating
  # results in reverse order for efficiency (then reversing at the end).
  defp convert_list_elements([], acc), do: {:ok, {:array, Enum.reverse(acc)}}

  defp convert_list_elements([head | tail], acc) do
    case from_native(head) do
      {:ok, val} -> convert_list_elements(tail, [val | acc])
      {:error, _} = err -> err
    end
  end

  # Convert a list of map entries to ordered pairs, validating that all
  # keys are strings.
  defp convert_map_pairs([], acc), do: {:ok, {:object, Enum.reverse(acc)}}

  defp convert_map_pairs([{key, val} | remaining], acc) when is_binary(key) do
    case from_native(val) do
      {:ok, json_val} -> convert_map_pairs(remaining, [{key, json_val} | acc])
      {:error, _} = err -> err
    end
  end

  defp convert_map_pairs([{key, _val} | _remaining], _acc) do
    {:error, "JSON object keys must be strings, got: #{inspect(key)}"}
  end
end
