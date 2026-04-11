defmodule CodingAdventures.JsonRpc.JsonCodec do
  @moduledoc """
  Minimal JSON encoder/decoder for JSON-RPC messages.

  ## Why not Jason or Poison?

  The `json-rpc` package is explicitly stdlib-only — it must not depend on any
  external Hex package. This keeps it usable as a foundation for LSP servers
  without pulling in transitive dependencies.

  ## OTP 27 `:json` module

  OTP 27 ships a built-in `:json` module with `encode/1` and `decode/1`. We
  detect its availability at compile time via `Code.ensure_loaded?(:json)`.
  When present, we delegate to it — it is fast, standards-compliant, and
  maintained by the Erlang/OTP team.

  ## Fallback encoder/decoder

  When `:json` is not available (OTP < 27), we use a hand-written encoder and
  decoder sufficient for JSON-RPC messages. JSON-RPC payloads are maps with
  string keys; values are strings, numbers, booleans, null, nested maps, and
  arrays — exactly the subset our codec handles.

  ## Output format

  `encode/1` converts a native Elixir map/list/scalar to a JSON binary string.
  `decode/1` converts a JSON binary string to a native Elixir map/list/scalar,
  using string keys (not atom keys) for safety (atom table is finite).

  ## Examples

      {:ok, json} = JsonCodec.encode(%{"jsonrpc" => "2.0", "id" => 1, "method" => "ping"})
      # json is a binary string like ~s({"jsonrpc":"2.0","id":1,"method":"ping"})

      {:ok, map} = JsonCodec.decode(json)
      # map is %{"jsonrpc" => "2.0", "id" => 1, "method" => "ping"}
  """

  # ---------------------------------------------------------------------------
  # Public API
  # ---------------------------------------------------------------------------

  @doc """
  Encode a native Elixir value to a JSON binary string.

  Accepts maps (with string or atom keys), lists, strings, integers, floats,
  booleans, and nil.

  Returns `{:ok, json_binary}` on success or `{:error, reason}` on failure.
  """
  @spec encode(any()) :: {:ok, binary()} | {:error, String.t()}
  def encode(value) do
    if otp27_json_available?() do
      try do
        {:ok, :json.encode(to_encodable(value))}
      rescue
        e -> {:error, "encode failed: #{inspect(e)}"}
      end
    else
      try do
        {:ok, encode_value(value)}
      rescue
        e -> {:error, "encode failed: #{inspect(e)}"}
      end
    end
  end

  @doc """
  Decode a JSON binary string to a native Elixir value.

  Object keys are always returned as strings (binary), never atoms. This is
  deliberate — the atom table in the BEAM is limited (1,048,576 entries by
  default), and accepting untrusted JSON with arbitrary keys could exhaust it.

  Returns `{:ok, value}` on success or `{:error, reason}` on failure.
  """
  @spec decode(binary()) :: {:ok, any()} | {:error, String.t()}
  def decode(json) when is_binary(json) do
    if otp27_json_available?() do
      try do
        result = :json.decode(json)
        {:ok, result}
      rescue
        e -> {:error, "decode failed: #{inspect(e)}"}
      catch
        :error, reason -> {:error, "decode failed: #{inspect(reason)}"}
      end
    else
      case decode_value(String.trim(json), 0) do
        {:ok, value, _rest} -> {:ok, value}
        {:error, _} = err -> err
      end
    end
  end

  def decode(json) do
    {:error, "expected binary, got: #{inspect(json)}"}
  end

  # ---------------------------------------------------------------------------
  # OTP 27 detection
  # ---------------------------------------------------------------------------
  #
  # We check at runtime (not compile time) so the module compiles cleanly on
  # any OTP version. The check is a simple module_info call wrapped in a
  # Code.ensure_loaded? equivalent.

  defp otp27_json_available?() do
    case :code.ensure_loaded(:json) do
      {:module, :json} -> true
      _ -> false
    end
  end

  # ---------------------------------------------------------------------------
  # Fallback: hand-written encoder
  # ---------------------------------------------------------------------------
  #
  # Encodes Elixir native values to JSON text.
  # Rules:
  #   nil           -> "null"
  #   true/false    -> "true"/"false"
  #   integer n     -> decimal representation
  #   float f       -> shortest decimal representation
  #   string s      -> quoted, with RFC 8259 escaping
  #   list l        -> "[elem, ...]"
  #   map m         -> "{\"key\": value, ...}"

  defp to_encodable(nil), do: nil
  defp to_encodable(true), do: true
  defp to_encodable(false), do: false
  defp to_encodable(n) when is_integer(n), do: n
  defp to_encodable(f) when is_float(f), do: f
  defp to_encodable(s) when is_binary(s), do: s
  defp to_encodable(a) when is_atom(a), do: Atom.to_string(a)

  defp to_encodable(list) when is_list(list) do
    Enum.map(list, &to_encodable/1)
  end

  defp to_encodable(map) when is_map(map) do
    Map.new(map, fn {k, v} ->
      key = if is_atom(k), do: Atom.to_string(k), else: k
      {key, to_encodable(v)}
    end)
  end

  defp encode_value(nil), do: "null"
  defp encode_value(true), do: "true"
  defp encode_value(false), do: "false"
  defp encode_value(n) when is_integer(n), do: Integer.to_string(n)
  defp encode_value(f) when is_float(f), do: :erlang.float_to_binary(f, [:short])

  defp encode_value(s) when is_binary(s) do
    escaped =
      s
      |> String.to_charlist()
      |> Enum.map(&encode_char/1)
      |> IO.iodata_to_binary()

    "\"" <> escaped <> "\""
  end

  defp encode_value(a) when is_atom(a) do
    encode_value(Atom.to_string(a))
  end

  defp encode_value(list) when is_list(list) do
    parts = Enum.map(list, &encode_value/1)
    "[" <> Enum.join(parts, ",") <> "]"
  end

  defp encode_value(map) when is_map(map) do
    parts =
      Enum.map(map, fn {k, v} ->
        key_str = if is_atom(k), do: Atom.to_string(k), else: k
        encode_value(key_str) <> ":" <> encode_value(v)
      end)

    "{" <> Enum.join(parts, ",") <> "}"
  end

  # RFC 8259 character escaping for the fallback encoder.
  defp encode_char(?"), do: ~c"\\\""
  defp encode_char(?\\), do: ~c"\\\\"
  defp encode_char(?\b), do: ~c"\\b"
  defp encode_char(?\f), do: ~c"\\f"
  defp encode_char(?\n), do: ~c"\\n"
  defp encode_char(?\r), do: ~c"\\r"
  defp encode_char(?\t), do: ~c"\\t"

  defp encode_char(cp) when cp >= 0x00 and cp <= 0x1F do
    hex = cp |> Integer.to_string(16) |> String.downcase() |> String.pad_leading(4, "0")
    String.to_charlist("\\u" <> hex)
  end

  defp encode_char(cp), do: [cp]

  # ---------------------------------------------------------------------------
  # Fallback: hand-written decoder
  # ---------------------------------------------------------------------------
  #
  # A recursive-descent JSON decoder. Returns {:ok, value, remaining_bytes}
  # or {:error, reason}.
  #
  # The grammar (simplified BNF):
  #   value  := null | true | false | number | string | array | object
  #   array  := "[" (value ("," value)*)? "]"
  #   object := "{" (pair ("," pair)*)? "}"
  #   pair   := string ":" value
  #
  # We work on the JSON string as a whole binary and track position via
  # pattern matching, passing back the unconsumed suffix.

  defp decode_value(<<"null", rest::binary>>, _pos), do: {:ok, nil, rest}
  defp decode_value(<<"true", rest::binary>>, _pos), do: {:ok, true, rest}
  defp decode_value(<<"false", rest::binary>>, _pos), do: {:ok, false, rest}

  defp decode_value(<<"\"", _::binary>> = bin, pos) do
    decode_string(bin, pos)
  end

  defp decode_value(<<"[", rest::binary>>, pos) do
    decode_array(String.trim_leading(rest), pos + 1, [])
  end

  defp decode_value(<<"{", rest::binary>>, pos) do
    decode_object(String.trim_leading(rest), pos + 1, [])
  end

  defp decode_value(bin, pos) do
    # Try number
    case decode_number(bin, pos) do
      {:ok, n, rest} -> {:ok, n, rest}
      _ -> {:error, "unexpected token at: #{String.slice(bin, 0, 20)}"}
    end
  end

  # String decoder — consumes from the opening " to the closing ".
  # Handles \", \\, \/, \b, \f, \n, \r, \t, \uXXXX escapes.
  defp decode_string(<<"\"", rest::binary>>, _pos) do
    decode_string_chars(rest, [])
  end

  defp decode_string_chars(<<"\"", rest::binary>>, acc) do
    {:ok, IO.iodata_to_binary(Enum.reverse(acc)), rest}
  end

  defp decode_string_chars(<<"\\\"", rest::binary>>, acc) do
    decode_string_chars(rest, [?" | acc])
  end

  defp decode_string_chars(<<"\\\\", rest::binary>>, acc) do
    decode_string_chars(rest, [?\\ | acc])
  end

  defp decode_string_chars(<<"\\/", rest::binary>>, acc) do
    decode_string_chars(rest, [?/ | acc])
  end

  defp decode_string_chars(<<"\\b", rest::binary>>, acc) do
    decode_string_chars(rest, [?\b | acc])
  end

  defp decode_string_chars(<<"\\f", rest::binary>>, acc) do
    decode_string_chars(rest, [?\f | acc])
  end

  defp decode_string_chars(<<"\\n", rest::binary>>, acc) do
    decode_string_chars(rest, [?\n | acc])
  end

  defp decode_string_chars(<<"\\r", rest::binary>>, acc) do
    decode_string_chars(rest, [?\r | acc])
  end

  defp decode_string_chars(<<"\\t", rest::binary>>, acc) do
    decode_string_chars(rest, [?\t | acc])
  end

  defp decode_string_chars(<<"\\u", h1, h2, h3, h4, rest::binary>>, acc) do
    hex = <<h1, h2, h3, h4>>
    codepoint = String.to_integer(hex, 16)
    char = <<codepoint::utf8>>
    decode_string_chars(rest, [char | acc])
  end

  defp decode_string_chars(<<byte::utf8, rest::binary>>, acc) do
    decode_string_chars(rest, [<<byte::utf8>> | acc])
  end

  defp decode_string_chars(_, _acc) do
    {:error, "unterminated string"}
  end

  # Number decoder — reads an optional sign, integer part, optional fractional
  # part, and optional exponent.
  defp decode_number(bin, _pos) do
    {num_str, rest} = consume_number_chars(bin, "")

    if num_str == "" do
      {:error, "not a number"}
    else
      cond do
        String.contains?(num_str, ".") or String.contains?(num_str, "e") or
            String.contains?(num_str, "E") ->
          case Float.parse(num_str) do
            {f, ""} -> {:ok, f, rest}
            {f, _} -> {:ok, f, rest}
            :error -> {:error, "invalid float: #{num_str}"}
          end

        true ->
          case Integer.parse(num_str) do
            {n, ""} -> {:ok, n, rest}
            {n, _} -> {:ok, n, rest}
            :error -> {:error, "invalid integer: #{num_str}"}
          end
      end
    end
  end

  defp consume_number_chars(<<c, rest::binary>>, acc)
       when c in [?-, ?+, ?0, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?., ?e, ?E] do
    consume_number_chars(rest, acc <> <<c>>)
  end

  defp consume_number_chars(rest, acc), do: {acc, rest}

  # Array decoder — accumulates elements separated by commas.
  defp decode_array(<<"]", rest::binary>>, _pos, acc) do
    {:ok, Enum.reverse(acc), rest}
  end

  defp decode_array(bin, pos, acc) do
    trimmed = String.trim_leading(bin)

    case decode_value(trimmed, pos) do
      {:ok, val, after_val} ->
        after_ws = String.trim_leading(after_val)

        case after_ws do
          <<",", more::binary>> ->
            decode_array(String.trim_leading(more), pos, [val | acc])

          <<"]", rest::binary>> ->
            {:ok, Enum.reverse([val | acc]), rest}

          _ ->
            {:error, "expected ',' or ']' in array"}
        end

      {:error, _} = err ->
        err
    end
  end

  # Object decoder — accumulates key-value pairs separated by commas.
  defp decode_object(<<"}", rest::binary>>, _pos, acc) do
    {:ok, Map.new(acc), rest}
  end

  defp decode_object(bin, pos, acc) do
    trimmed = String.trim_leading(bin)

    # Handle empty object or trailing comma before '}'.
    case trimmed do
      <<"}", rest::binary>> ->
        {:ok, Map.new(acc), rest}

      <<"\"", _::binary>> ->
        # Decode the key (must be a JSON string starting with ").
        with {:ok, key, after_key} <- decode_string(trimmed, pos),
             after_colon_ws = String.trim_leading(after_key),
             <<":", value_part::binary>> <- after_colon_ws,
             after_colon2 = String.trim_leading(value_part),
             {:ok, val, after_val} <- decode_value(after_colon2, pos) do
          after_ws = String.trim_leading(after_val)
          new_acc = [{key, val} | acc]

          case after_ws do
            <<",", more::binary>> ->
              decode_object(String.trim_leading(more), pos, new_acc)

            <<"}", rest::binary>> ->
              {:ok, Map.new(new_acc), rest}

            _ ->
              {:error, "expected ',' or '}' in object"}
          end
        else
          {:error, _} = err -> err
          other -> {:error, "expected ':' after key, got: #{inspect(other)}"}
        end

      _ ->
        {:error, "expected string key in object, got: #{String.slice(trimmed, 0, 20)}"}
    end
  end
end
