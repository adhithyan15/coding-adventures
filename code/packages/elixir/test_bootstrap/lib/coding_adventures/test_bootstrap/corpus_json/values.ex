defmodule CodingAdventures.TestBootstrap.CorpusJson.Values do
  @moduledoc """
  Strict decoding of corpus values and kind-sensitive equality.

  Encoding (JSON):

      {"kind":"int","value":"-12"}     an integer (decimal text, any width)
      {"kind":"bytes","hex":"00ff"}    a binary
      true / false                     a boolean
      [ ... ]                          a list of the above
      "text"                           text, decoded as {:text, binary}

  Text is tagged so equality can tell bytes from text.
  """
  alias CodingAdventures.TestBootstrap.CorpusJson.{Canon, Clip}

  defmodule CaseError do
    @moduledoc "A malformed case (not an engine mismatch)."
    defexception [:message]
  end

  @max_size 10_000
  @max_int_digits 1_000
  @max_depth 64
  @max_fragment_len 200
  @max_problem_len 400

  def max_size, do: @max_size
  def max_fragment_len, do: @max_fragment_len
  def max_problem_len, do: @max_problem_len

  defp fail(message), do: raise(CaseError, message: message)

  @doc "Decimal digits only, optional leading \"-\", no leading zeros, no \"-0\"."
  def canonical_int?(text), do: Regex.match?(~r/\A(0|-?[1-9][0-9]*)\z/, text)

  def lower_hex?(text), do: rem(byte_size(text), 2) == 0 and Regex.match?(~r/\A[0-9a-f]*\z/, text)

  def decode(node, depth \\ 0)
  def decode(_node, depth) when depth > @max_depth, do: fail("value nesting deeper than #{@max_depth}")
  def decode(node, _depth) when is_boolean(node), do: node
  def decode(node, _depth) when is_binary(node), do: {:text, node}

  def decode(node, depth) when is_list(node) do
    if length(node) > @max_size, do: fail("list exceeds #{@max_size} elements")
    Enum.map(node, &decode(&1, depth + 1))
  end

  def decode(node, _depth) when is_map(node), do: decode_object(node)
  def decode(node, _depth), do: fail("unsupported JSON value #{show(node)}")

  defp decode_object(%{"kind" => "int", "value" => text} = node) when map_size(node) == 2 do
    unless is_binary(text) and canonical_int?(text), do: fail("non-canonical int #{show(text)}")
    if byte_size(text) > @max_int_digits, do: fail("int exceeds #{@max_int_digits} digits")
    String.to_integer(text)
  end

  defp decode_object(%{"kind" => "bytes", "hex" => text} = node) when map_size(node) == 2 do
    unless is_binary(text) and lower_hex?(text),
      do: fail("bytes hex must be even-length lowercase: #{show(text)}")

    if byte_size(text) > 2 * @max_size, do: fail("bytes exceed #{@max_size} bytes")
    Base.decode16!(text, case: :lower)
  end

  defp decode_object(node), do: fail("unrecognised value node #{show(node)}")

  @doc """
  Equality that does not conflate true with 1, bytes with text, or lists with
  anything else.
  """
  def same?(a, b, depth \\ 0)
  def same?(_a, _b, depth) when depth > @max_depth, do: false

  def same?(a, b, depth) do
    kind(a) == kind(b) and
      case a do
        l when is_list(l) ->
          length(a) == length(b) and
            Enum.all?(Enum.zip(a, b), fn {x, y} -> same?(x, y, depth + 1) end)

        _ ->
          a === b
      end
  end

  defp kind(true), do: :yes
  defp kind(false), do: :no
  defp kind(v) when is_integer(v), do: :int
  defp kind(v) when is_list(v), do: :list
  defp kind({:text, _}), do: :text
  defp kind(v) when is_binary(v), do: :bytes
  defp kind(_), do: :other

  @doc "A short, bounded rendering for problem lines."
  def show(value), do: Clip.clip(Canon.render(value), @max_fragment_len)
end
