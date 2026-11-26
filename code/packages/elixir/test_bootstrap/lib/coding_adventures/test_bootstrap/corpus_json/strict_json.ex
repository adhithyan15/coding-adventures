defmodule CodingAdventures.TestBootstrap.CorpusJson.StrictJson do
  @moduledoc """
  A small, STRICT JSON reader, hand-written because a conformance fixture is
  DATA and a lenient parser quietly papers over exactly the corruption a
  conformance check exists to notice.  (OTP's `:json` keeps the last of two
  duplicate keys and turns numbers into floats unless its callbacks are
  replaced, which is most of a parser.)  Refused:

    * duplicate object keys (compared as bytes), trailing data after the document;
    * nesting deeper than 64 containers (refused before recursing);
    * documents over 8 MiB or more than 200,000 values;
    * invalid UTF-8 and a byte-order mark;
    * raw control characters in strings, lone or reversed surrogate escapes,
      leading zeros, `+1`, `NaN`, comments, trailing commas.

  Values become maps (string keys), lists, binaries, `true`, `false`, `nil` and
  `{:number, text}`: a JSON number kept as its literal text, so a 30-digit
  integer cannot lose its low digits.
  """

  import Bitwise
  alias CodingAdventures.TestBootstrap.CorpusJson.Clip

  defmodule JsonError do
    defexception [:message]
  end

  @max_depth 64
  @max_bytes 8 * 1024 * 1024
  @max_nodes 200_000

  def max_depth, do: @max_depth
  def max_bytes, do: @max_bytes
  def max_nodes, do: @max_nodes

  # Parser state travels as `{rest, nodes}`; the document length gives byte offsets.
  @spec parse(binary()) :: term()
  def parse(data) when is_binary(data) do
    if byte_size(data) > @max_bytes,
      do: fail("document of #{byte_size(data)} bytes exceeds #{@max_bytes}")

    unless String.valid?(data), do: fail("document is not valid UTF-8")

    total = byte_size(data)
    {value, rest, _nodes} = value(skip_ws(data), 0, 0, total)
    rest = skip_ws(rest)
    if rest != "", do: fail_at("trailing data after the document", total, rest)
    value
  end

  def parse(_), do: fail("JSON input must be a binary")

  defp fail(message), do: raise(JsonError, message: message)

  defp fail_at(message, total, rest),
    do: raise(JsonError, message: "#{message} at byte #{total - byte_size(rest)}")

  defp skip_ws(<<c, rest::binary>>) when c in [0x20, 0x09, 0x0A, 0x0D], do: skip_ws(rest)
  defp skip_ws(rest), do: rest

  defp count(nodes, total, rest) do
    nodes = nodes + 1
    if nodes > @max_nodes, do: fail_at("more than #{@max_nodes} values", total, rest)
    nodes
  end

  defp value(rest, depth, nodes, total) do
    nodes = count(nodes, total, rest)

    case rest do
      <<?{, r::binary>> -> object(r, rest, depth, nodes, total)
      <<?[, r::binary>> -> array(r, rest, depth, nodes, total)
      <<?", r::binary>> -> string_value(r, nodes, total)
      <<"true", r::binary>> -> {true, r, nodes}
      <<"false", r::binary>> -> {false, r, nodes}
      <<"null", r::binary>> -> {nil, r, nodes}
      <<c, _::binary>> when c == ?- or c in ?0..?9 -> number(rest, nodes, total)
      <<>> -> fail_at("unexpected end of input", total, rest)
      <<c, _::binary>> when c in [?t, ?f, ?n] -> fail_at("invalid literal", total, rest)
      <<c, _::binary>> -> fail_at("unexpected byte 0x#{Integer.to_string(c, 16)}", total, rest)
    end
  end

  defp string_value(rest, nodes, total) do
    {s, r} = string(rest, [], total)
    {s, r, nodes}
  end

  defp array(r, at, depth, nodes, total) do
    if depth + 1 > @max_depth, do: fail_at("nesting deeper than #{@max_depth}", total, at)
    r = skip_ws(r)

    case r do
      <<?], r2::binary>> -> {[], r2, nodes}
      _ -> array_items(r, depth, nodes, total, [])
    end
  end

  defp array_items(r, depth, nodes, total, acc) do
    {v, r, nodes} = value(skip_ws(r), depth + 1, nodes, total)
    acc = [v | acc]

    case skip_ws(r) do
      <<?,, r2::binary>> -> array_items(r2, depth, nodes, total, acc)
      <<?], r2::binary>> -> {Enum.reverse(acc), r2, nodes}
      other -> fail_at("expected , or ]", total, other)
    end
  end

  defp object(r, at, depth, nodes, total) do
    if depth + 1 > @max_depth, do: fail_at("nesting deeper than #{@max_depth}", total, at)
    r = skip_ws(r)

    case r do
      <<?}, r2::binary>> -> {%{}, r2, nodes}
      _ -> object_items(r, depth, nodes, total, %{})
    end
  end

  defp object_items(r, depth, nodes, total, acc) do
    r = skip_ws(r)

    case r do
      <<?", r1::binary>> ->
        nodes = count(nodes, total, r)
        {key, r2} = string(r1, [], total)

        if Map.has_key?(acc, key),
          do: fail_at("duplicate key #{Clip.clip(key, 40)}", total, r2)

        case skip_ws(r2) do
          <<?:, r3::binary>> ->
            {v, r4, nodes} = value(skip_ws(r3), depth + 1, nodes, total)
            acc = Map.put(acc, key, v)

            case skip_ws(r4) do
              <<?,, r5::binary>> -> object_items(r5, depth, nodes, total, acc)
              <<?}, r5::binary>> -> {acc, r5, nodes}
              other -> fail_at("expected , or }", total, other)
            end

          other ->
            fail_at("expected :", total, other)
        end

      other ->
        fail_at("expected a string key", total, other)
    end
  end

  # -- numbers (kept as text) --------------------------------------------------------

  defp number(rest, nodes, total) do
    {text, r} = scan_number(rest, total)
    {{:number, text}, r, nodes}
  end

  defp scan_number(rest, total) do
    r0 = rest
    r = if match?(<<?-, _::binary>>, r0), do: binary_part(r0, 1, byte_size(r0) - 1), else: r0

    r =
      case r do
        <<?0, r2::binary>> ->
          if match?(<<d, _::binary>> when d in ?0..?9, r2), do: fail_at("leading zero", total, r2)
          r2

        <<d, _::binary>> when d in ?1..?9 ->
          digits(r)

        _ ->
          fail_at("invalid number", total, r)
      end

    r =
      case r do
        <<?., r2::binary>> ->
          unless match?(<<d, _::binary>> when d in ?0..?9, r2),
            do: fail_at("digits required after .", total, r2)

          digits(r2)

        _ ->
          r
      end

    r =
      case r do
        <<e, r2::binary>> when e in [?e, ?E] ->
          r3 =
            case r2 do
              <<s, r4::binary>> when s in [?+, ?-] -> r4
              _ -> r2
            end

          unless match?(<<d, _::binary>> when d in ?0..?9, r3),
            do: fail_at("digits required in exponent", total, r3)

          digits(r3)

        _ ->
          r
      end

    {binary_part(rest, 0, byte_size(rest) - byte_size(r)), r}
  end

  defp digits(<<d, rest::binary>>) when d in ?0..?9, do: digits(rest)
  defp digits(rest), do: rest

  # -- strings ----------------------------------------------------------------------------

  defp string(rest, acc, total) do
    plain = plain_len(rest, 0)
    <<chunk::binary-size(plain), r::binary>> = rest
    acc = if plain > 0, do: [chunk | acc], else: acc

    case r do
      <<?", r2::binary>> ->
        {acc |> Enum.reverse() |> IO.iodata_to_binary(), r2}

      <<?\\, r2::binary>> ->
        {piece, r3} = escape(r2, total)
        string(r3, [piece | acc], total)

      <<>> ->
        fail_at("unterminated string", total, r)

      _control ->
        fail_at("raw control character in string", total, r)
    end
  end

  defp plain_len(<<c, rest::binary>>, n) when c != ?" and c != ?\\ and c >= 0x20,
    do: plain_len(rest, n + 1)

  defp plain_len(_, n), do: n

  @simple %{?" => "\"", ?\\ => "\\", ?/ => "/", ?b => "\b", ?f => "\f", ?n => "\n", ?r => "\r", ?t => "\t"}

  defp escape(<<c, r::binary>>, _total) when is_map_key(@simple, c), do: {Map.fetch!(@simple, c), r}

  defp escape(<<?u, r::binary>>, total) do
    {unit, r2} = hex4(r, total)

    cond do
      unit in 0xD800..0xDBFF ->
        case r2 do
          <<?\\, ?u, r3::binary>> ->
            {low, r4} = hex4(r3, total)

            unless low in 0xDC00..0xDFFF, do: fail_at("invalid surrogate pair", total, r4)
            cp = 0x10000 + ((unit - 0xD800) <<< 10) + (low - 0xDC00)
            {<<cp::utf8>>, r4}

          _ ->
            fail_at("lone high surrogate", total, r2)
        end

      unit in 0xDC00..0xDFFF ->
        fail_at("lone low surrogate", total, r2)

      true ->
        {<<unit::utf8>>, r2}
    end
  end

  defp escape(<<>>, total), do: fail_at("unterminated escape", total, <<>>)
  defp escape(r, total), do: fail_at("invalid escape", total, r)

  defp hex4(<<a, b, c, d, r::binary>> = all, total) do
    if Enum.all?([a, b, c, d], &hex?/1) do
      {String.to_integer(<<a, b, c, d>>, 16), r}
    else
      fail_at("invalid \\u escape", total, all)
    end
  end

  defp hex4(r, total), do: fail_at("invalid \\u escape", total, r)

  defp hex?(c), do: c in ?0..?9 or c in ?a..?f or c in ?A..?F

end
