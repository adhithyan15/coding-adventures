defmodule CodingAdventures.ImageCodecPng.Filter do
  @moduledoc """
  RFC 2083 Section 6: Scanline prediction filters and reconstruction.
  """

  import Bitwise
  alias CodingAdventures.ImageCodecPng.Constants
  alias CodingAdventures.ImageCodecPng.PngError

  @filter_none Constants.filter_none()
  @filter_sub Constants.filter_sub()
  @filter_up Constants.filter_up()
  @filter_average Constants.filter_average()
  @filter_paeth Constants.filter_paeth()
  @filter_count Constants.filter_count()
  @average_divisor Constants.average_divisor()

  @doc """
  Paeth predictor (RFC 2083 Section 6.6).
  Normative tie-breaking order: a, then b, then c.
  """
  def paeth(a, b, c) do
    # RFC 2083 §6.6 invariant: Compute distances to initial estimate p = a + b - c,
    # then break ties in favor of a first, then b.
    p = a + b - c
    pa = abs(p - a)
    pb = abs(p - b)
    pc = abs(p - c)

    cond do
      pa <= pb and pa <= pc -> a
      pb <= pc -> b
      true -> c
    end
  end

  @doc """
  Applies a single filter to a raw row against the reconstructed prior row.
  """
  def apply_filter(filter_id, raw, prior, bpp)
      when is_integer(filter_id) and is_binary(raw) and is_binary(prior) and is_integer(bpp) do
    len = byte_size(raw)
    do_apply(0, len, filter_id, raw, prior, bpp, <<>>)
  end

  defp do_apply(i, len, _filter_id, _raw, _prior, _bpp, acc) when i >= len, do: acc

  defp do_apply(i, len, filter_id, raw, prior, bpp, acc) do
    x = :binary.at(raw, i)
    left = if i >= bpp, do: :binary.at(raw, i - bpp), else: 0
    above = :binary.at(prior, i)
    above_left = if i >= bpp, do: :binary.at(prior, i - bpp), else: 0

    predicted =
      case filter_id do
        @filter_none -> 0
        @filter_sub -> left
        @filter_up -> above
        @filter_average -> div(left + above, @average_divisor)
        @filter_paeth -> paeth(left, above, above_left)
      end

    val = x - predicted &&& Constants.byte_mask()
    do_apply(i + 1, len, filter_id, raw, prior, bpp, <<acc::binary, val>>)
  end

  @doc """
  Evaluates the heuristic score for a filtered row: sum of absolute signed differences.
  """
  def filter_score(filtered) when is_binary(filtered) do
    do_score(filtered, 0)
  end

  defp do_score(<<>>, acc), do: acc

  defp do_score(<<val, rest::binary>>, acc) do
    score =
      if val < Constants.byte_midpoint() do
        val
      else
        Constants.byte_modulus() - val
      end

    do_score(rest, acc + score)
  end

  @doc """
  Chooses the best filter for a row using the sum of signed differences heuristic.
  Returns `{best_filter_id, best_filtered_binary}`.
  Ties break towards the lowest numbered filter type.
  """
  def choose_filter(raw, prior, bpp)
      when is_binary(raw) and is_binary(prior) and is_integer(bpp) do
    do_choose(@filter_none, raw, prior, bpp, nil, nil, nil)
  end

  defp do_choose(filter_id, _raw, _prior, _bpp, _best_score, best_filter, best_filtered)
       when filter_id >= @filter_count do
    {best_filter, best_filtered}
  end

  defp do_choose(filter_id, raw, prior, bpp, best_score, best_filter, best_filtered) do
    filtered = apply_filter(filter_id, raw, prior, bpp)
    score = filter_score(filtered)

    if is_nil(best_score) or score < best_score do
      do_choose(filter_id + 1, raw, prior, bpp, score, filter_id, filtered)
    else
      do_choose(filter_id + 1, raw, prior, bpp, best_score, best_filter, best_filtered)
    end
  end

  @doc """
  Inverses a scanline filter in place using already-reconstructed samples.
  """
  def undo_filter(filter_id, filtered, prior, bpp)
      when is_integer(filter_id) and is_binary(filtered) and is_binary(prior) and is_integer(bpp) do
    # RFC 2083 invariant: un-filtering Sub, Average, and Paeth reads the
    # previously-reconstructed 'left' sample from the accumulated reconstructed row (acc),
    # NOT from raw filtered bytes.
    len = byte_size(filtered)

    case filter_id do
      @filter_none ->
        filtered

      @filter_sub ->
        do_undo_sub(0, len, filtered, bpp, <<>>)

      @filter_up ->
        do_undo_up(0, len, filtered, prior, <<>>)

      @filter_average ->
        do_undo_average(0, len, filtered, prior, bpp, <<>>)

      @filter_paeth ->
        do_undo_paeth(0, len, filtered, prior, bpp, <<>>)

      _ ->
        raise PngError.exception(:invalid_filter)
    end
  end

  defp do_undo_sub(i, len, _filtered, _bpp, acc) when i >= len, do: acc

  defp do_undo_sub(i, len, filtered, bpp, acc) do
    x = :binary.at(filtered, i)
    left = if i >= bpp, do: :binary.at(acc, i - bpp), else: 0
    val = x + left &&& Constants.byte_mask()
    do_undo_sub(i + 1, len, filtered, bpp, <<acc::binary, val>>)
  end

  defp do_undo_up(i, len, _filtered, _prior, acc) when i >= len, do: acc

  defp do_undo_up(i, len, filtered, prior, acc) do
    x = :binary.at(filtered, i)
    above = :binary.at(prior, i)
    val = x + above &&& Constants.byte_mask()
    do_undo_up(i + 1, len, filtered, prior, <<acc::binary, val>>)
  end

  defp do_undo_average(i, len, _filtered, _prior, _bpp, acc) when i >= len, do: acc

  defp do_undo_average(i, len, filtered, prior, bpp, acc) do
    x = :binary.at(filtered, i)
    left = if i >= bpp, do: :binary.at(acc, i - bpp), else: 0
    above = :binary.at(prior, i)
    val = x + div(left + above, @average_divisor) &&& Constants.byte_mask()
    do_undo_average(i + 1, len, filtered, prior, bpp, <<acc::binary, val>>)
  end

  defp do_undo_paeth(i, len, _filtered, _prior, _bpp, acc) when i >= len, do: acc

  defp do_undo_paeth(i, len, filtered, prior, bpp, acc) do
    x = :binary.at(filtered, i)
    left = if i >= bpp, do: :binary.at(acc, i - bpp), else: 0
    above = :binary.at(prior, i)
    above_left = if i >= bpp, do: :binary.at(prior, i - bpp), else: 0
    pred = paeth(left, above, above_left)
    val = x + pred &&& Constants.byte_mask()
    do_undo_paeth(i + 1, len, filtered, prior, bpp, <<acc::binary, val>>)
  end
end
