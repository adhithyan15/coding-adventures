defmodule CodingAdventures.JsonSerializer do
  @moduledoc """
  JSON Serializer — Convert typed JSON values or native Elixir types to JSON text.

  ## Two Modes

  1. **Compact** — minimal whitespace, smallest output. Suitable for wire
     transmission, storage, or any context where human readability is not
     important.

         serialize({:object, [{"name", {:string, "Alice"}}]})
         # => {:ok, ~s({"name":"Alice"})}

  2. **Pretty** — human-readable with configurable indentation. Suitable for
     configuration files, debugging output, or any context where a human
     will read the JSON.

         serialize_pretty({:object, [{"name", {:string, "Alice"}}]})
         # => {:ok, ~s({\\n  "name": "Alice"\\n})}

  ## Configuration (Pretty Mode)

  Pretty-printing is controlled by a keyword list of options:

      [
        indent_size: 2,           # spaces per indent level (default: 2)
        indent_char: " ",         # character for indentation (default: space)
        sort_keys: false,         # alphabetically sort object keys? (default: false)
        trailing_newline: false   # add \\n at end of output? (default: false)
      ]

  ## String Escaping (RFC 8259)

  Per RFC 8259, these characters MUST be escaped in JSON strings:

      Character        Escape     Reason
      ---------        ------     ------
      " (quote)        \\"         Delimiter
      \\ (backslash)    \\\\         Escape character
      Backspace        \\b         Control char (U+0008)
      Form feed        \\f         Control char (U+000C)
      Newline          \\n         Control char (U+000A)
      Carriage return  \\r         Control char (U+000D)
      Tab              \\t         Control char (U+0009)
      U+0000-U+001F    \\uXXXX    All other control characters

  Forward slash (/) is NOT escaped — RFC 8259 allows but does not require it.

  ## Note on Non-Finite Floats

  In many languages, you must guard against Infinity and NaN when serializing
  floats. In Elixir/Erlang, the BEAM raises `ArithmeticError` before a
  non-finite float value can be constructed, so this is not a concern.
  All float values that reach the serializer are guaranteed to be finite.
  """

  alias CodingAdventures.JsonValue

  # ---------------------------------------------------------------------------
  # Type Definitions
  # ---------------------------------------------------------------------------

  @type opts :: [
          indent_size: non_neg_integer(),
          indent_char: String.t(),
          sort_keys: boolean(),
          trailing_newline: boolean()
        ]

  # Default configuration for pretty-printing.
  @default_opts [
    indent_size: 2,
    indent_char: " ",
    sort_keys: false,
    trailing_newline: false
  ]

  # ---------------------------------------------------------------------------
  # serialize/1 — Compact JSON output from a JsonValue
  # ---------------------------------------------------------------------------
  #
  # Dispatch on the tagged tuple type:
  #   :null           -> "null"
  #   {:boolean, b}   -> "true" or "false"
  #   {:number, n}    -> string representation of n
  #   {:string, s}    -> quoted and escaped string
  #   {:array, elems} -> "[elem1,elem2,...]"
  #   {:object, pairs}-> "{"key1":val1,"key2":val2,...}"

  @doc """
  Serialize a JSON value to compact JSON text (no unnecessary whitespace).

  ## Examples

      {:ok, "null"} = serialize(:null)
      {:ok, "42"} = serialize({:number, 42})
      {:ok, ~s({"a":1})} = serialize({:object, [{"a", {:number, 1}}]})
  """
  @spec serialize(JsonValue.json_value()) :: {:ok, String.t()}
  def serialize(value) do
    {:ok, serialize_value(value)}
  end

  # ---------------------------------------------------------------------------
  # serialize_pretty/2 — Pretty-printed JSON output from a JsonValue
  # ---------------------------------------------------------------------------

  @doc """
  Serialize a JSON value to pretty-printed JSON text.

  Accepts an optional keyword list for configuration. See module docs for
  available options.

  ## Examples

      {:ok, text} = serialize_pretty({:object, [{"a", {:number, 1}}]})
      # text == ~s({\\n  "a": 1\\n})

      {:ok, text} = serialize_pretty({:object, [{"b", {:number, 2}}, {"a", {:number, 1}}]},
                                     sort_keys: true)
      # keys are sorted alphabetically
  """
  @spec serialize_pretty(JsonValue.json_value(), opts()) :: {:ok, String.t()}
  def serialize_pretty(value, user_opts \\ []) do
    merged_opts = Keyword.merge(@default_opts, user_opts)
    text = serialize_pretty_value(value, merged_opts, 0)

    final_text =
      if Keyword.get(merged_opts, :trailing_newline, false) do
        text <> "\n"
      else
        text
      end

    {:ok, final_text}
  end

  # ---------------------------------------------------------------------------
  # stringify/1 — Compact JSON from native Elixir types
  # ---------------------------------------------------------------------------

  @doc """
  Convert native Elixir types to compact JSON text.

  Equivalent to `from_native(value) |> serialize()`.

  ## Examples

      {:ok, ~s({"a":1})} = stringify(%{"a" => 1})
      {:ok, "[1,2,3]"} = stringify([1, 2, 3])
      {:ok, "null"} = stringify(nil)
  """
  @spec stringify(any()) :: {:ok, String.t()} | {:error, String.t()}
  def stringify(native_value) do
    case JsonValue.from_native(native_value) do
      {:ok, json_val} -> serialize(json_val)
      {:error, _} = err -> err
    end
  end

  # ---------------------------------------------------------------------------
  # stringify_pretty/2 — Pretty-printed JSON from native Elixir types
  # ---------------------------------------------------------------------------

  @doc """
  Convert native Elixir types to pretty-printed JSON text.

  Equivalent to `from_native(value) |> serialize_pretty(opts)`.

  ## Examples

      {:ok, text} = stringify_pretty(%{"a" => 1})
      # text includes newlines and indentation
  """
  @spec stringify_pretty(any(), opts()) :: {:ok, String.t()} | {:error, String.t()}
  def stringify_pretty(native_value, user_opts \\ []) do
    case JsonValue.from_native(native_value) do
      {:ok, json_val} -> serialize_pretty(json_val, user_opts)
      {:error, _} = err -> err
    end
  end

  # ===========================================================================
  # Private Helpers — Compact Serialization
  # ===========================================================================
  #
  # These functions always return a string. In Elixir, all floats are finite
  # (the BEAM raises ArithmeticError for overflow), so there are no error
  # cases to propagate during serialization.

  defp serialize_value(:null), do: "null"
  defp serialize_value({:boolean, true}), do: "true"
  defp serialize_value({:boolean, false}), do: "false"

  defp serialize_value({:number, num}) when is_integer(num) do
    Integer.to_string(num)
  end

  defp serialize_value({:number, num}) when is_float(num) do
    # In Elixir/Erlang, floats are always finite. The BEAM raises
    # ArithmeticError before producing Infinity or NaN, so we don't need
    # to check for non-finite values. Every float that reaches here is
    # safe to serialize.
    format_float(num)
  end

  defp serialize_value({:string, str}) when is_binary(str) do
    "\"" <> escape_json_string(str) <> "\""
  end

  defp serialize_value({:array, []}) do
    "[]"
  end

  defp serialize_value({:array, elements}) when is_list(elements) do
    parts = Enum.map(elements, &serialize_value/1)
    "[" <> Enum.join(parts, ",") <> "]"
  end

  defp serialize_value({:object, []}) do
    "{}"
  end

  defp serialize_value({:object, pairs}) when is_list(pairs) do
    parts =
      Enum.map(pairs, fn {key, val} ->
        "\"" <> escape_json_string(key) <> "\":" <> serialize_value(val)
      end)

    "{" <> Enum.join(parts, ",") <> "}"
  end

  # ===========================================================================
  # Private Helpers — Pretty Serialization
  # ===========================================================================
  #
  # Pretty printing adds newlines and indentation to make JSON human-readable.
  # The algorithm is recursive: each level of nesting increases the depth by 1.
  #
  # For primitives (null, boolean, number, string), pretty is identical to
  # compact — they have no internal structure to indent.
  #
  # For arrays and objects, we add newlines between elements and indent each
  # element to the appropriate depth.

  defp serialize_pretty_value(:null, _opts, _depth), do: "null"
  defp serialize_pretty_value({:boolean, true}, _opts, _depth), do: "true"
  defp serialize_pretty_value({:boolean, false}, _opts, _depth), do: "false"

  defp serialize_pretty_value({:number, _} = num, _opts, _depth) do
    serialize_value(num)
  end

  defp serialize_pretty_value({:string, _} = str, _opts, _depth) do
    serialize_value(str)
  end

  defp serialize_pretty_value({:array, []}, _opts, _depth), do: "[]"

  defp serialize_pretty_value({:array, elements}, merged_opts, depth) do
    indent = make_indent(merged_opts, depth + 1)
    closing_indent = make_indent(merged_opts, depth)

    lines =
      Enum.map(elements, fn elem ->
        indent <> serialize_pretty_value(elem, merged_opts, depth + 1)
      end)

    "[\n" <> Enum.join(lines, ",\n") <> "\n" <> closing_indent <> "]"
  end

  defp serialize_pretty_value({:object, []}, _opts, _depth), do: "{}"

  defp serialize_pretty_value({:object, pairs}, merged_opts, depth) do
    indent = make_indent(merged_opts, depth + 1)
    closing_indent = make_indent(merged_opts, depth)

    # Optionally sort keys alphabetically.
    # When sort_keys is false (default), we preserve insertion order — the
    # same order the keys appeared in the original JSON or the order they
    # were added to the map.
    sorted_pairs =
      if Keyword.get(merged_opts, :sort_keys, false) do
        Enum.sort_by(pairs, fn {key, _val} -> key end)
      else
        pairs
      end

    lines =
      Enum.map(sorted_pairs, fn {key, val} ->
        val_str = serialize_pretty_value(val, merged_opts, depth + 1)
        indent <> "\"" <> escape_json_string(key) <> "\": " <> val_str
      end)

    "{\n" <> Enum.join(lines, ",\n") <> "\n" <> closing_indent <> "}"
  end

  # Build the indentation string for a given depth.
  #
  # Example with indent_size=2 and indent_char=" ":
  #   depth 0 -> ""
  #   depth 1 -> "  "
  #   depth 2 -> "    "
  defp make_indent(merged_opts, depth) do
    indent_char = Keyword.get(merged_opts, :indent_char, " ")
    indent_size = Keyword.get(merged_opts, :indent_size, 2)
    String.duplicate(indent_char, indent_size * depth)
  end

  # ===========================================================================
  # String Escaping
  # ===========================================================================
  #
  # JSON string escaping follows RFC 8259. We process the string one codepoint
  # at a time, replacing special characters with their escape sequences.
  #
  # The escape table:
  #   "    -> \"
  #   \    -> \\
  #   \b   -> \b   (backspace, U+0008)
  #   \f   -> \f   (form feed, U+000C)
  #   \n   -> \n   (newline, U+000A)
  #   \r   -> \r   (carriage return, U+000D)
  #   \t   -> \t   (tab, U+0009)
  #   U+0000-U+001F (other control chars) -> \uXXXX
  #
  # All other characters pass through unchanged, including non-ASCII Unicode
  # characters (we output UTF-8 directly, not \uXXXX escapes for non-control
  # characters).

  defp escape_json_string(str) do
    str
    |> String.to_charlist()
    |> Enum.map(&escape_char/1)
    |> IO.iodata_to_binary()
  end

  # Each clause handles one character. The order matters — we check specific
  # characters first, then the control character range, then pass everything
  # else through unchanged.
  defp escape_char(?"), do: ~c"\\\""
  defp escape_char(?\\), do: ~c"\\\\"
  defp escape_char(?\b), do: ~c"\\b"
  defp escape_char(?\f), do: ~c"\\f"
  defp escape_char(?\n), do: ~c"\\n"
  defp escape_char(?\r), do: ~c"\\r"
  defp escape_char(?\t), do: ~c"\\t"

  # Control characters (U+0000 to U+001F) not covered above get \uXXXX escaping.
  # We produce lowercase hex digits for consistency.
  defp escape_char(cp) when cp >= 0x00 and cp <= 0x1F do
    hex = cp |> Integer.to_string(16) |> String.downcase() |> String.pad_leading(4, "0")
    String.to_charlist("\\u" <> hex)
  end

  # All other characters pass through unchanged (including non-ASCII Unicode).
  defp escape_char(cp), do: [cp]

  # ===========================================================================
  # Float Formatting
  # ===========================================================================
  #
  # We use :erlang.float_to_binary with the :short option, which produces the
  # shortest decimal representation that round-trips correctly. This gives us
  # output like "3.14" for 3.14, "1.0" for 1.0, etc.

  defp format_float(num) do
    :erlang.float_to_binary(num, [:short])
  end
end
