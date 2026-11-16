defmodule CodingAdventures.ImageCodecPng do
  @moduledoc """
  Deterministic IC18 PNG encoder and decoder in pure Elixir.
  Implements the CodingAdventures.ImageCodec behaviour.
  """

  @behaviour CodingAdventures.ImageCodec

  import Bitwise
  alias CodingAdventures.ImageCodecPng.Adler32
  alias CodingAdventures.ImageCodecPng.Constants
  alias CodingAdventures.ImageCodecPng.Filter
  alias CodingAdventures.ImageCodecPng.Messages
  alias CodingAdventures.ImageCodecPng.PngError
  alias CodingAdventures.PixelContainer

  @colour_type_grey Constants.colour_type_grey()
  @colour_type_rgb Constants.colour_type_rgb()
  @colour_type_rgba Constants.colour_type_rgba()

  @channels_grey Constants.channels_grey()
  @channels_grey_alpha Constants.channels_grey_alpha()
  @channels_rgb Constants.channels_rgb()
  @channels_rgba Constants.channels_rgba()

  @ancillary_bit_mask Constants.ancillary_bit_mask()
  @zlib_cinfo_shift Constants.zlib_cinfo_shift()
  @png_max_pixels Constants.png_max_pixels()

  # ---------------------------------------------------------------------------
  # ImageCodec Behaviour Callbacks
  # ---------------------------------------------------------------------------

  @impl true
  def mime_type, do: Messages.mime_type()

  @impl true
  def encode(%PixelContainer{} = pixels) do
    encode_png(pixels)
  end

  @impl true
  def decode(data) do
    case decode_png(data) do
      %PixelContainer{} = container -> {:ok, container}
    end
  rescue
    e in PngError ->
      {:error, e.code}
  end

  # ---------------------------------------------------------------------------
  # Public API
  # ---------------------------------------------------------------------------

  @doc "Exposes the default maximum dimension per edge (16,384)."
  def max_dimension, do: Constants.png_max_dimension()

  @doc "Exposes the default maximum pixel limit (33,554,432)."
  def max_pixels, do: Constants.default_max_pixels()

  @doc "Returns the list of all 29 closed IC18 error codes."
  def error_codes, do: Messages.error_codes()

  @doc "Computes the RFC 1950 Adler-32 checksum of binary data."
  def adler32(data) when is_binary(data) do
    Adler32.compute(data)
  end

  # ---------------------------------------------------------------------------
  # Encoder
  # ---------------------------------------------------------------------------

  @doc """
  Encodes an RGBA8 PixelContainer into a deterministic PNG binary.
  """
  def encode_png(%PixelContainer{width: width, height: height, data: data}) do
    validate_encoder_dimensions!(width, height)
    validate_encoder_data!(width, height, data)

    bd = Constants.supported_bit_depth()
    ct = Constants.colour_type_rgba()
    cm = Constants.compression_method_deflate()
    fm = Constants.filter_method_adaptive()
    im = Constants.interlace_method_none()

    ihdr_payload = <<width::32, height::32, bd, ct, cm, fm, im>>

    filtered = filter_scanlines(data, width, height)
    adler = Adler32.compute(filtered)
    deflated = CodingAdventures.Zip.raw_deflate(filtered)

    zlib_stream =
      <<Constants.zlib_cmf_deflate(), Constants.zlib_flg_default(), deflated::binary, adler::32>>

    ihdr_chunk = format_chunk(Constants.chunk_ihdr(), ihdr_payload)
    idat_chunk = format_chunk(Constants.chunk_idat(), zlib_stream)
    iend_chunk = format_chunk(Constants.chunk_iend(), <<>>)

    <<Constants.png_signature()::binary, ihdr_chunk::binary, idat_chunk::binary,
      iend_chunk::binary>>
  end

  def encode_png(_) do
    raise PngError.exception(:invalid_image_dimensions)
  end

  defp validate_encoder_dimensions!(width, height) do
    unless is_integer(width) and is_integer(height) and width > 0 and height > 0 do
      raise PngError.exception(:invalid_image_dimensions)
    end

    if width > Constants.png_max_dimension() or height > Constants.png_max_dimension() do
      raise PngError.exception(:dimension_limit)
    end

    if width * height > Constants.png_max_pixels() do
      raise PngError.exception(:pixel_limit)
    end
  end

  defp validate_encoder_data!(width, height, data) do
    expected_bytes = width * height * Constants.bytes_per_rgba_pixel()

    unless is_binary(data) and byte_size(data) == expected_bytes do
      raise PngError.exception(:invalid_pixel_data_length)
    end
  end

  defp filter_scanlines(raw_data, width, height) do
    bpp = Constants.bytes_per_rgba_pixel()
    stride = width * bpp
    zero_prior = :binary.copy(<<0>>, stride)
    do_filter_scanlines(0, height, raw_data, stride, bpp, zero_prior, [])
  end

  defp do_filter_scanlines(y, height, _raw_data, _stride, _bpp, _prior, acc) when y >= height do
    IO.iodata_to_binary(Enum.reverse(acc))
  end

  defp do_filter_scanlines(y, height, raw_data, stride, bpp, prior, acc) do
    row = :binary.part(raw_data, y * stride, stride)
    {filter_id, filtered_row} = Filter.choose_filter(row, prior, bpp)
    chunk = [filter_id, filtered_row]
    do_filter_scanlines(y + 1, height, raw_data, stride, bpp, row, [chunk | acc])
  end

  defp format_chunk(type, payload) do
    len = byte_size(payload)
    chunk_type_size = Constants.chunk_type_size()
    crc_input = <<type::binary-size(chunk_type_size), payload::binary>>
    crc = CodingAdventures.Zip.crc32(crc_input)
    <<len::32, type::binary-size(chunk_type_size), payload::binary, crc::32>>
  end

  # ---------------------------------------------------------------------------
  # Decoder
  # ---------------------------------------------------------------------------

  @doc """
  Decodes a PNG binary into an RGBA8 PixelContainer.
  Options:
    - `:max_pixels` - maximum allowed total pixel count (positive integer <= 33,554,432).
  """
  def decode_png(data, opts \\ []) do
    max_pixels = validate_max_pixels!(opts)
    validate_file_length!(data)
    validate_signature!(data)

    sig_len = Constants.signature_length()
    <<_sig::binary-size(sig_len), chunk_bytes::binary>> = data

    chunks_state =
      parse_chunks(
        chunk_bytes,
        sig_len,
        byte_size(data),
        max_pixels,
        init_decoder_state()
      )

    zlib_data = IO.iodata_to_binary(Enum.reverse(chunks_state.idat_parts))

    uncompressed =
      decompress_idat(
        zlib_data,
        chunks_state.width,
        chunks_state.height,
        chunks_state.colour_type
      )

    reconstruct_image(
      uncompressed,
      chunks_state.width,
      chunks_state.height,
      chunks_state.colour_type,
      chunks_state.transparent_grey,
      chunks_state.transparent_rgb
    )
  end

  defp validate_max_pixels!(opts) do
    if Keyword.keyword?(opts) do
      case Keyword.fetch(opts, :max_pixels) do
        :error ->
          Constants.default_max_pixels()

        {:ok, limit}
        when is_integer(limit) and limit > 0 and limit <= @png_max_pixels ->
          limit

        _ ->
          raise PngError.exception(:invalid_max_pixels)
      end
    else
      raise PngError.exception(:invalid_max_pixels)
    end
  end

  defp validate_file_length!(data) do
    unless is_binary(data) and byte_size(data) >= Constants.signature_length() do
      raise PngError.exception(:file_too_short)
    end
  end

  defp validate_signature!(data) do
    sig_len = Constants.signature_length()
    expected_sig = Constants.png_signature()

    case data do
      <<^expected_sig::binary-size(sig_len), _rest::binary>> -> :ok
      _ -> raise PngError.exception(:invalid_signature)
    end
  end

  defp init_decoder_state do
    %{
      saw_ihdr: false,
      saw_iend: false,
      saw_plte: false,
      saw_trns: false,
      saw_idat: false,
      in_idat: false,
      idat_ended: false,
      width: 0,
      height: 0,
      colour_type: 0,
      transparent_grey: nil,
      transparent_rgb: nil,
      idat_parts: []
    }
  end

  defp parse_chunks(<<>>, _pos, _total_len, _limit, state) do
    unless state.saw_ihdr and state.saw_iend and state.saw_idat do
      raise PngError.exception(:missing_required_chunk)
    end

    state
  end

  defp parse_chunks(remaining, pos, total_len, limit, state) do
    if byte_size(remaining) < Constants.chunk_header_size() do
      raise PngError.exception(:truncated_chunk)
    end

    chunk_type_size = Constants.chunk_type_size()
    <<length::32, chunk_type::binary-size(chunk_type_size), rest::binary>> = remaining

    # CVE-2018-14550: Verify declared chunk length does not exceed remaining file bytes before allocation.
    if length > total_len - pos - Constants.chunk_overhead() do
      raise PngError.exception(:truncated_chunk)
    end

    unless valid_chunk_type?(chunk_type) do
      raise PngError.exception(:invalid_chunk_type)
    end

    if byte_size(rest) < length + Constants.chunk_crc_size() do
      raise PngError.exception(:truncated_chunk)
    end

    <<payload::binary-size(length), declared_crc::32, next_chunks::binary>> = rest

    # Compute and verify CRC-32 over chunk type and payload before parsing chunk payload fields.
    actual_crc = CodingAdventures.Zip.crc32(<<chunk_type::binary, payload::binary>>)

    if actual_crc != declared_crc do
      raise PngError.exception(:chunk_crc_mismatch)
    end

    unless state.saw_ihdr or chunk_type == Constants.chunk_ihdr() do
      raise PngError.exception(:chunk_before_ihdr)
    end

    new_state = process_chunk(chunk_type, length, payload, limit, next_chunks, state)

    next_state =
      if chunk_type != Constants.chunk_idat() and new_state.in_idat do
        %{new_state | in_idat: false, idat_ended: true}
      else
        new_state
      end

    if next_state.saw_iend do
      unless next_state.saw_ihdr and next_state.saw_idat do
        raise PngError.exception(:missing_required_chunk)
      end

      next_state
    else
      new_pos = pos + Constants.chunk_overhead() + length
      parse_chunks(next_chunks, new_pos, total_len, limit, next_state)
    end
  end

  defp valid_chunk_type?(<<b1, b2, b3, b4>>) do
    is_ascii_letter?(b1) and is_ascii_letter?(b2) and is_ascii_upper?(b3) and
      is_ascii_letter?(b4)
  end

  defp valid_chunk_type?(_), do: false

  defp is_ascii_letter?(b), do: (b >= ?a and b <= ?z) or (b >= ?A and b <= ?Z)
  defp is_ascii_upper?(b), do: b >= ?A and b <= ?Z

  defp process_chunk(type, length, payload, limit, next_chunks, state) do
    cond do
      type == Constants.chunk_ihdr() ->
        process_ihdr(length, payload, limit, state)

      type == Constants.chunk_plte() ->
        process_plte(length, state)

      type == Constants.chunk_trns() ->
        process_trns(length, payload, state)

      type == Constants.chunk_idat() ->
        process_idat(payload, state)

      type == Constants.chunk_iend() ->
        process_iend(length, next_chunks, state)

      type in Constants.apng_chunks() ->
        # APNG chunk refusal: acTL, fcTL, fdAT change animated image semantics; fail closed.
        raise PngError.exception(:unsupported_feature)

      true ->
        process_unknown_chunk(type, state)
    end
  end

  defp process_ihdr(length, payload, limit, state) do
    if state.saw_ihdr do
      raise PngError.exception(:duplicate_ihdr)
    end

    if length != Constants.ihdr_length() do
      raise PngError.exception(:invalid_ihdr_length)
    end

    <<w::32, h::32, bit_depth, col_type, compression, filter_method, interlace>> = payload

    if w == 0 or h == 0 do
      raise PngError.exception(:invalid_dimensions)
    end

    if w > Constants.png_max_dimension() or h > Constants.png_max_dimension() do
      raise PngError.exception(:dimension_limit)
    end

    if w * h > limit do
      raise PngError.exception(:pixel_limit)
    end

    if compression != Constants.compression_method_deflate() or
         filter_method != Constants.filter_method_adaptive() or
         interlace != Constants.interlace_method_none() do
      raise PngError.exception(:unsupported_feature)
    end

    if bit_depth != Constants.supported_bit_depth() or
         col_type not in Constants.supported_colour_types() do
      raise PngError.exception(:unsupported_feature)
    end

    %{state | saw_ihdr: true, width: w, height: h, colour_type: col_type}
  end

  defp process_plte(length, state) do
    if state.saw_plte or state.saw_idat or state.saw_trns or
         (state.colour_type != @colour_type_rgb and
            state.colour_type != @colour_type_rgba) or
         length < Constants.plte_min_length() or length > Constants.plte_max_length() or
         rem(length, Constants.plte_entry_size()) != 0 do
      raise PngError.exception(:invalid_plte)
    end

    %{state | saw_plte: true}
  end

  defp process_trns(length, payload, state) do
    if state.saw_trns or state.saw_idat do
      raise PngError.exception(:invalid_trns)
    end

    case state.colour_type do
      @colour_type_grey ->
        if length != Constants.trns_grey_length() do
          raise PngError.exception(:invalid_trns)
        end

        <<sample::16>> = payload

        if sample > Constants.max_sample_value() do
          raise PngError.exception(:invalid_trns)
        end

        %{state | saw_trns: true, transparent_grey: sample}

      @colour_type_rgb ->
        if length != Constants.trns_rgb_length() do
          raise PngError.exception(:invalid_trns)
        end

        <<r::16, g::16, b::16>> = payload

        if r > Constants.max_sample_value() or g > Constants.max_sample_value() or
             b > Constants.max_sample_value() do
          raise PngError.exception(:invalid_trns)
        end

        %{state | saw_trns: true, transparent_rgb: {r, g, b}}

      _ ->
        raise PngError.exception(:invalid_trns)
    end
  end

  defp process_idat(payload, state) do
    if state.idat_ended do
      raise PngError.exception(:nonconsecutive_idat)
    end

    %{
      state
      | saw_idat: true,
        in_idat: true,
        idat_parts: [payload | state.idat_parts]
    }
  end

  defp process_iend(length, next_chunks, state) do
    if length != 0 do
      raise PngError.exception(:invalid_iend)
    end

    if byte_size(next_chunks) != 0 do
      raise PngError.exception(:trailing_data)
    end

    %{state | saw_iend: true}
  end

  defp process_unknown_chunk(<<first_byte, _::binary>>, state) do
    if (first_byte &&& @ancillary_bit_mask) == 0 do
      raise PngError.exception(:unknown_critical_chunk)
    else
      state
    end
  end

  defp decompress_idat(zlib_data, width, height, colour_type) do
    if byte_size(zlib_data) < Constants.zlib_min_stream_size() do
      raise PngError.exception(:invalid_zlib_header)
    end

    <<cmf, flg, rest_zlib::binary>> = zlib_data

    if (cmf &&& Constants.zlib_cmf_method_mask()) != Constants.zlib_method_deflate() or
         cmf >>> @zlib_cinfo_shift > Constants.zlib_cinfo_max() or
         rem(cmf <<< Constants.bits_per_byte() ||| flg, Constants.zlib_fcheck_modulus()) != 0 do
      raise PngError.exception(:invalid_zlib_header)
    end

    if (flg &&& Constants.zlib_fdict_mask()) != 0 do
      raise PngError.exception(:preset_dictionary)
    end

    channels = Constants.channels_for_colour_type(colour_type)
    stride = width * channels

    # Decompression bomb defence: bound uncompressed output to exact scanline size height * (stride + 1).
    expected_length = height * (stride + 1)

    deflate_len = byte_size(rest_zlib) - Constants.zlib_trailer_size()
    <<deflate_data::binary-size(deflate_len), declared_adler::32>> = rest_zlib

    inflated =
      try do
        CodingAdventures.Zip.raw_inflate_counted(deflate_data, expected_length)
      rescue
        e in CodingAdventures.Zip.RawInflateError ->
          if e.code == CodingAdventures.Zip.Messages.inflate_error_code(:output_limit_exceeded) do
            raise PngError.exception(:inflated_length_mismatch)
          else
            raise PngError.exception(:inflate_failed)
          end
      end

    if byte_size(inflated.output) != expected_length do
      raise PngError.exception(:inflated_length_mismatch)
    end

    # IDAT cavity defence: ensure inflate consumed every byte of deflate stream before trailer.
    if inflated.bytes_consumed != deflate_len do
      raise PngError.exception(:idat_cavity)
    end

    actual_adler = Adler32.compute(inflated.output)

    if actual_adler != declared_adler do
      raise PngError.exception(:adler_mismatch)
    end

    validate_scanline_filters!(inflated.output, height, stride + 1)

    inflated.output
  end

  defp validate_scanline_filters!(uncompressed, height, row_size) do
    do_validate_filters!(0, height, uncompressed, row_size)
  end

  defp do_validate_filters!(y, height, _uncompressed, _row_size) when y >= height, do: :ok

  defp do_validate_filters!(y, height, uncompressed, row_size) do
    filter_byte = :binary.at(uncompressed, y * row_size)

    if filter_byte > Constants.max_filter_type() do
      raise PngError.exception(:invalid_filter)
    end

    do_validate_filters!(y + 1, height, uncompressed, row_size)
  end

  defp reconstruct_image(uncompressed, width, height, colour_type, trans_grey, trans_rgb) do
    channels = Constants.channels_for_colour_type(colour_type)
    stride = width * channels
    row_size = stride + 1
    zero_prior = :binary.copy(<<0>>, stride)

    rgba_data =
      do_reconstruct_rows(
        0,
        height,
        uncompressed,
        row_size,
        stride,
        channels,
        trans_grey,
        trans_rgb,
        zero_prior,
        []
      )

    %PixelContainer{
      width: width,
      height: height,
      data: rgba_data
    }
  end

  defp do_reconstruct_rows(
         y,
         height,
         _uncompressed,
         _row_size,
         _stride,
         _channels,
         _trans_grey,
         _trans_rgb,
         _prior,
         acc
       )
       when y >= height do
    IO.iodata_to_binary(Enum.reverse(acc))
  end

  defp do_reconstruct_rows(
         y,
         height,
         uncompressed,
         row_size,
         stride,
         channels,
         trans_grey,
         trans_rgb,
         prior,
         acc
       ) do
    offset = y * row_size
    filter_type = :binary.at(uncompressed, offset)
    filtered_row = :binary.part(uncompressed, offset + 1, stride)
    reconstructed = Filter.undo_filter(filter_type, filtered_row, prior, channels)

    widened =
      case channels do
        @channels_grey ->
          widen_grey(reconstructed, trans_grey, <<>>)

        @channels_grey_alpha ->
          widen_grey_alpha(reconstructed, <<>>)

        @channels_rgb ->
          widen_rgb(reconstructed, trans_rgb, <<>>)

        @channels_rgba ->
          reconstructed
      end

    do_reconstruct_rows(
      y + 1,
      height,
      uncompressed,
      row_size,
      stride,
      channels,
      trans_grey,
      trans_rgb,
      reconstructed,
      [widened | acc]
    )
  end

  defp widen_grey(<<>>, _trans, acc), do: acc

  defp widen_grey(<<v, rest::binary>>, trans, acc) do
    alpha =
      if trans == v do
        Constants.transparent_alpha()
      else
        Constants.opaque_alpha()
      end

    widen_grey(rest, trans, <<acc::binary, v, v, v, alpha>>)
  end

  defp widen_grey_alpha(<<>>, acc), do: acc

  defp widen_grey_alpha(<<v, a, rest::binary>>, acc) do
    widen_grey_alpha(rest, <<acc::binary, v, v, v, a>>)
  end

  defp widen_rgb(<<>>, _trans, acc), do: acc

  defp widen_rgb(<<r, g, b, rest::binary>>, trans, acc) do
    alpha =
      if trans == {r, g, b} do
        Constants.transparent_alpha()
      else
        Constants.opaque_alpha()
      end

    widen_rgb(rest, trans, <<acc::binary, r, g, b, alpha>>)
  end
end
