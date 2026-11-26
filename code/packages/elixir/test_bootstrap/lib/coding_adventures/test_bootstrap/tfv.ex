defmodule CodingAdventures.TestBootstrap.Tfv do
  @moduledoc """
  Strict validator for TFV v1 text, the line format that carries test corpora
  between languages (TF01 section 2).  This is a VALIDATOR, not a reader: its one
  job here is to let the corpus runner check the pbt-v1 `corpus_reject` cases
  with a real parser instead of skipping them.

      tfv 1
      # comment
      <id> R<n> K<field> <value> K<field> <value> ...

  A value is one of

      N  T  F                 null, true, false
      I<int>                  canonical decimal integer, at most 1000 digits
      D<16 hex>               IEEE-754 bit pattern, lowercase
      X<hex>  S<hex>          bytes / UTF-8 text as even-length lowercase hex
      L<n> v1 ... vn          list
      M<n> k1 v1 ... kn vn    map; no two keys deep-equal

  Tokens are separated by exactly one ASCII space.  Rejected: uppercase or
  odd-length hex, `I-0`, `I007`, a count that disagrees with the tokens, trailing
  tokens, doubled spaces, a duplicate field or record id, invalid UTF-8 inside S,
  a carriage return in a record line, nesting deeper than 64 containers and
  integers of more than 1000 digits.

  Worked example: "tfv 1\\nr R1 Kop S706274 Kdata XFF\\n" fails (XFF is uppercase
  and odd length); "bad header\\n" fails (the first line must be exactly "tfv 1").
  """
  import Bitwise
  alias CodingAdventures.TestBootstrap.CorpusJson.Canon

  @header "tfv 1"
  @max_depth 64
  @max_int_digits 1000
  @max_input 1 <<< 20

  defmodule InvalidTfvError do
    defexception [:message]
  end

  defp fail(message), do: raise(InvalidTfvError, message: "invalid TFV: " <> message)

  def valid?(text) do
    validate(text)
    true
  rescue
    InvalidTfvError -> false
  end

  defp q(text) do
    shown = if String.length(text) > 64, do: String.slice(text, 0, 64) <> "...", else: text
    "\"" <> Canon.escape(shown, quote: true) <> "\""
  end

  @doc "Returns `:ok` for a well-formed document, raises `InvalidTfvError` otherwise."
  def validate(text) when is_binary(text) do
    if byte_size(text) > @max_input,
      do: fail("input of #{byte_size(text)} bytes exceeds #{@max_input}")

    unless String.valid?(text), do: fail("document is not valid UTF-8")

    lines = String.split(text, "\n")
    # the final newline is optional
    lines = if List.last(lines) == "", do: Enum.drop(lines, -1), else: lines

    if lines == [] or hd(lines) != @header,
      do: fail("the first line must be exactly #{q(@header)}")

    lines
    |> Enum.with_index(1)
    |> Enum.drop(1)
    |> Enum.reduce(%{}, fn {line, n}, seen ->
      if line == "" or String.starts_with?(line, "#") do
        seen
      else
        id =
          try do
            validate_record(line)
          rescue
            e in InvalidTfvError ->
              fail("line #{n}: " <> String.replace_prefix(e.message, "invalid TFV: ", ""))
          end

        if Map.has_key?(seen, id), do: fail("line #{n}: duplicate record id #{q(id)}")
        Map.put(seen, id, true)
      end
    end)

    :ok
  end

  def validate(_), do: fail("input must be a binary")

  defp valid_id?(id), do: Regex.match?(~r{\A[A-Za-z0-9._/-]+\z}, id)
  defp valid_field_name?(name), do: Regex.match?(~r/\A[a-z][a-z0-9_]*\z/, name)

  defp lower_hex?(text), do: rem(byte_size(text), 2) == 0 and Regex.match?(~r/\A[0-9a-f]*\z/, text)

  defp canonical_nat(text) do
    cond do
      text == "" -> fail("empty count")
      not Regex.match?(~r/\A[0-9]+\z/, text) -> fail("non-digit in count #{q(text)}")
      byte_size(text) > 1 and String.starts_with?(text, "0") -> fail("leading zero in count #{q(text)}")
      byte_size(text) > 9 -> fail("count #{q(text)} too large")
      true -> String.to_integer(text)
    end
  end

  defp valid_int!(body) do
    {neg, digits} =
      case body do
        "-" <> d -> {true, d}
        d -> {false, d}
      end

    cond do
      digits == "" -> fail("empty integer")
      not Regex.match?(~r/\A[0-9]+\z/, digits) -> fail("non-digit in integer #{q(body)}")
      String.starts_with?(digits, "0") and (byte_size(digits) > 1 or neg) -> fail("non-canonical integer #{q(body)}")
      byte_size(digits) > @max_int_digits -> fail("integer of #{byte_size(digits)} digits exceeds #{@max_int_digits}")
      true -> :ok
    end
  end

  defp validate_record(line) do
    if String.contains?(line, "\r"), do: fail("carriage return in a record line")

    tokens = String.split(line, " ")
    if Enum.any?(tokens, &(&1 == "")), do: fail("leading, trailing or doubled space")
    if length(tokens) < 2, do: fail("a record needs an id and R<n>")

    [id, rcount | rest] = tokens
    unless valid_id?(id), do: fail("bad record id #{q(id)}")
    unless String.starts_with?(rcount, "R"), do: fail("expected R<n>, got #{q(rcount)}")

    count = canonical_nat(binary_part(rcount, 1, byte_size(rcount) - 1))

    {rest, _seen} =
      Enum.reduce(1..count//1, {rest, %{}}, fn _, {toks, seen} ->
        {key_tok, toks} = next_token(toks)

        unless String.starts_with?(key_tok, "K") and valid_field_name?(binary_part(key_tok, 1, byte_size(key_tok) - 1)),
          do: fail("bad field key #{q(key_tok)}")

        if Map.has_key?(seen, key_tok), do: fail("duplicate field #{q(binary_part(key_tok, 1, byte_size(key_tok) - 1))}")
        {_key, toks} = read_value(toks, 0)
        {toks, Map.put(seen, key_tok, true)}
      end)

    if rest != [], do: fail("trailing tokens after the last field")
    id
  end

  defp next_token([]), do: fail("record ends early")
  defp next_token([t | rest]), do: {t, rest}

  # Reads one value; returns {deep-equality key, remaining tokens}.  The key is
  # equal for two values exactly when TF01 3.1 calls them deep-equal.
  defp read_value(tokens, depth) do
    {tok, rest} = next_token(tokens)
    <<tag, body::binary>> = tok

    case tag do
      t when t in [?N, ?T, ?F] ->
        if body != "", do: fail("unexpected body in #{q(tok)}")
        {<<t>>, rest}

      ?I ->
        valid_int!(body)
        {tok, rest}

      ?D ->
        {float_key(tok, body), rest}

      ?X ->
        unless lower_hex?(body), do: fail("bytes need even-length lowercase hex: #{q(tok)}")
        {tok, rest}

      ?S ->
        unless lower_hex?(body), do: fail("text needs even-length lowercase hex: #{q(tok)}")

        unless String.valid?(Base.decode16!(body, case: :lower)),
          do: fail("text is not strict UTF-8: #{q(tok)}")

        {tok, rest}

      t when t in [?L, ?M] ->
        container(t, body, depth, rest)

      _ ->
        fail("unknown tag in #{q(tok)}")
    end
  end

  # Every NaN equals every NaN; -0 and +0 stay distinct.
  defp float_key(tok, body) do
    unless byte_size(body) == 16 and lower_hex?(body),
      do: fail("float needs exactly 16 lowercase hex digits: #{q(tok)}")

    <<_sign::1, exp::11, mantissa::52>> = Base.decode16!(body, case: :lower)
    if exp == 0x7FF and mantissa != 0, do: "Dnan", else: tok
  end

  defp container(tag, body, depth, rest) do
    if depth + 1 > @max_depth, do: fail("nesting deeper than #{@max_depth}")
    count = canonical_nat(body)

    if tag == ?L do
      # never reserve memory from a count
      {items, rest} =
        Enum.reduce(1..count//1, {[], rest}, fn _, {acc, toks} ->
          {k, toks} = read_value(toks, depth + 1)
          {[k | acc], toks}
        end)

      {"L[" <> Enum.join(Enum.reverse(items), ",") <> ",]", rest}
    else
      {entries, _keys, rest} =
        Enum.reduce(1..count//1, {[], %{}, rest}, fn _, {acc, keys, toks} ->
          {k, toks} = read_value(toks, depth + 1)
          {v, toks} = read_value(toks, depth + 1)
          if Map.has_key?(keys, k), do: fail("duplicate map key")
          {["(#{k}=>#{v})" | acc], Map.put(keys, k, true), toks}
        end)

      {"M{" <> Enum.join(Enum.sort(entries)) <> "}", rest}
    end
  end

end
