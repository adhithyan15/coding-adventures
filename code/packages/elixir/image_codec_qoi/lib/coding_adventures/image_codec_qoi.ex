defmodule CodingAdventures.ImageCodecQoi do
  @moduledoc """
  IC03: QOI (Quite OK Image) encoder and decoder.

  ## QOI format overview

  QOI was designed by Dominic Szablewski in 2021 as a dead-simple lossless
  image format that compresses almost as well as PNG but encodes and decodes
  10–20× faster. The core insight: most pixels in natural images differ only
  slightly from their neighbours, so short delta codes beat generic compression.

  ## Encoding pipeline

  The encoder walks every pixel left-to-right, top-to-bottom and emits the
  shortest applicable op-code:

  1. **QOI_OP_RUN**   (1 byte)  — same pixel repeated 1–62 times
  2. **QOI_OP_INDEX** (1 byte)  — pixel matches a recently-seen pixel in the
                                   64-entry rolling hash table
  3. **QOI_OP_DIFF**  (1 byte)  — small deltas: dr, dg, db each in −2..+1
                                   (alpha unchanged)
  4. **QOI_OP_LUMA**  (2 bytes) — medium deltas: dg in −32..+31,
                                   dr−dg and db−dg each in −8..+7
                                   (alpha unchanged)
  5. **QOI_OP_RGB**   (4 bytes) — explicit R, G, B (alpha unchanged)
  6. **QOI_OP_RGBA**  (5 bytes) — explicit R, G, B, A

  The encoder always picks the shortest applicable op in that priority order.

  ## Hash function

  The 64-slot pixel cache uses this hash to map an RGBA pixel to a slot:

      index = rem(r*3 + g*5 + b*7 + a*11, 64)

  This is a cheap but surprisingly effective hash: the prime multipliers spread
  common colours across the 64 slots and minimise collisions.

  ## Signed deltas

  QOI encodes channel deltas as small signed integers packed into few bits.
  The wrapping arithmetic ensures the delta is computed correctly even when
  channel values wrap around 255→0 or 0→255:

      dr = wrap_delta(curr_r - prev_r)

  where `wrap_delta` maps the raw difference to the range −128..+127 by
  treating it as an unsigned byte difference first:

      wrap_delta(d) = rem((d &&& 0xFF) + 128, 256) - 128

  For example: `0 - 255 = -255` in normal arithmetic, but in the QOI spec
  channels wrap modulo 256. `(-255) &&& 0xFF = 1`, then `rem(1+128, 256)-128 = -127`,
  which correctly represents "one step forward from 255 to 0".

  ## File structure

  ```
  Header (14 bytes):
    magic:      "qoif"  (4 bytes)
    width:      uint32 big-endian
    height:     uint32 big-endian
    channels:   uint8 (3 = RGB, 4 = RGBA)
    colorspace: uint8 (0 = sRGB+linear alpha, 1 = all linear)

  Body: stream of op-code bytes (see above)

  Footer (8 bytes):
    0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x01
  ```

  ## Op-code bit layout

  ```
  QOI_OP_RGB:   11111110  r g b           (tag = 0xFE, 4 bytes total)
  QOI_OP_RGBA:  11111111  r g b a         (tag = 0xFF, 5 bytes total)
  QOI_OP_INDEX: 00xxxxxx                  (2-bit tag 00, 6-bit index)
  QOI_OP_DIFF:  01drrdggdbb               (2-bit tag 01, 2+2+2 bias-2 deltas)
  QOI_OP_LUMA:  10dddddddd  drr_dgg dbb_dgg  (2-bit tag 10, 6-bit dg bias-32,
                                              then 4+4 bias-8 deltas)
  QOI_OP_RUN:   11xxxxxx                  (2-bit tag 11, 6-bit run minus 1;
                                           values 62 and 63 are reserved for
                                           QOI_OP_RGB and QOI_OP_RGBA)
  ```

  ## Example

      iex> alias CodingAdventures.{PixelContainer, ImageCodecQoi}
      iex> c = PixelContainer.new(2, 2)
      iex> c = PixelContainer.set_pixel(c, 0, 0, 255, 0, 0, 255)
      iex> data = ImageCodecQoi.encode(c)
      iex> {:ok, c2} = ImageCodecQoi.decode(data)
      iex> PixelContainer.pixel_at(c2, 0, 0)
      {255, 0, 0, 255}

  """

  @behaviour CodingAdventures.ImageCodec

  alias CodingAdventures.PixelContainer

  import Bitwise

  # ── Constants ────────────────────────────────────────────────────────────────

  # The "qoif" magic bytes at the start of every QOI file
  @qoi_magic "qoif"

  # The 8-byte end-of-stream marker
  @qoi_footer <<0, 0, 0, 0, 0, 0, 0, 1>>

  # Op-code tags (top 2 bits for single-byte ops, full byte for 2-byte ops)
  @qoi_op_run 0xC0
  @qoi_op_index 0x00
  @qoi_op_diff 0x40
  @qoi_op_luma 0x80
  @qoi_op_rgb 0xFE
  @qoi_op_rgba 0xFF

  # The rolling hash table has 64 slots
  @hash_table_size 64

  # ── Behaviour callbacks ───────────────────────────────────────────────────────

  @impl true
  def mime_type, do: "image/qoi"

  @impl true
  def encode(%PixelContainer{} = c), do: encode_qoi(c)

  @impl true
  def decode(data) when is_binary(data), do: decode_qoi(data)

  # ── Encoder ──────────────────────────────────────────────────────────────────

  @doc """
  Encodes a `PixelContainer` as a QOI binary.

  Uses all 6 QOI op-codes, choosing the shortest applicable code for each
  pixel to minimise output size.
  """
  def encode_qoi(%PixelContainer{width: w, height: h} = c) do
    # Build the 14-byte header
    header = <<
      @qoi_magic::binary,
      w::big-32,
      h::big-32,
      # 4 channels (RGBA)
      4::8,
      # sRGB colorspace
      0::8
    >>

    # Initial encoder state:
    #   prev  — the "previous" pixel (starts as opaque black per spec)
    #   table — 64-slot rolling hash table, all initialised to transparent black
    #   run   — current run-length count
    prev = {0, 0, 0, 255}
    table = :array.new(@hash_table_size, default: {0, 0, 0, 0})

    # Collect all pixels in row-major order
    pixels =
      for y <- 0..(h - 1), x <- 0..(w - 1) do
        PixelContainer.pixel_at(c, x, y)
      end

    # Encode the pixel stream
    {body, table, prev, run} =
      Enum.reduce(pixels, {<<>>, table, prev, 0}, fn pixel, {acc, tbl, pv, run} ->
        encode_pixel(pixel, acc, tbl, pv, run)
      end)

    # Flush any outstanding run at end of stream
    body = flush_run(body, run)

    # Update table with the last pixel (required by spec, even though we don't
    # use it further — ensures the table is in a defined state)
    _table = update_table(table, prev)

    header <> body <> @qoi_footer
  end

  # Encode one pixel, choosing the shortest applicable op-code.
  defp encode_pixel(pixel, acc, table, prev, run) do
    {r, g, b, a} = pixel
    {pr, pg, pb, pa} = prev

    if pixel == prev do
      # QOI_OP_RUN: same pixel as before — increment run counter.
      # Flush at 62 (maximum run length before we must emit the byte).
      run = run + 1

      if run == 62 do
        # Emit QOI_OP_RUN for 62 pixels and reset
        acc = acc <> <<@qoi_op_run ||| (run - 1)>>
        {acc, update_table(table, pixel), pixel, 0}
      else
        {acc, table, pixel, run}
      end
    else
      # Different pixel — flush any pending run first
      acc = flush_run(acc, run)
      run = 0

      # Check QOI_OP_INDEX: is this pixel in the rolling hash table?
      idx = hash_pixel(r, g, b, a)
      cached = :array.get(idx, table)
      table = update_table(table, pixel)

      cond do
        cached == pixel ->
          # QOI_OP_INDEX: 2-bit tag 00, 6-bit index
          acc = acc <> <<@qoi_op_index ||| idx>>
          {acc, table, pixel, run}

        a == pa ->
          # Alpha unchanged — try DIFF or LUMA before falling back to RGB
          # Compute wrapped deltas for each channel
          dr = wrap_delta(r - pr)
          dg = wrap_delta(g - pg)
          db = wrap_delta(b - pb)

          cond do
            # QOI_OP_DIFF: dr, dg, db each fit in −2..+1
            # Pack with bias 2: stored value = delta + 2 (range 0..3, 2 bits)
            dr >= -2 and dr <= 1 and dg >= -2 and dg <= 1 and db >= -2 and db <= 1 ->
              byte = @qoi_op_diff ||| (dr + 2) <<< 4 ||| (dg + 2) <<< 2 ||| (db + 2)
              {acc <> <<byte>>, table, pixel, run}

            # QOI_OP_LUMA: dg fits in −32..+31, dr−dg and db−dg each in −8..+7
            # First byte: tag 10 + dg with bias 32 (stored = dg+32, 6 bits)
            # Second byte: (dr−dg+8) in top nibble, (db−dg+8) in bottom nibble
            dg >= -32 and dg <= 31 and (dr - dg) >= -8 and (dr - dg) <= 7 and
                (db - dg) >= -8 and (db - dg) <= 7 ->
              byte1 = @qoi_op_luma ||| (dg + 32)
              byte2 = (dr - dg + 8) <<< 4 ||| (db - dg + 8)
              {acc <> <<byte1, byte2>>, table, pixel, run}

            # QOI_OP_RGB: full RGB, alpha stays the same
            true ->
              {acc <> <<@qoi_op_rgb, r, g, b>>, table, pixel, run}
          end

        true ->
          # Alpha changed — must emit QOI_OP_RGBA
          {acc <> <<@qoi_op_rgba, r, g, b, a>>, table, pixel, run}
      end
    end
  end

  # Flush a pending run to the output buffer.
  # QOI_OP_RUN encodes the count as (run - 1) in the lower 6 bits.
  # For example, run=1 → lower bits = 0, run=62 → lower bits = 61.
  defp flush_run(acc, 0), do: acc

  defp flush_run(acc, run),
    do: acc <> <<@qoi_op_run ||| (run - 1)>>

  # Update the rolling hash table with the given pixel.
  defp update_table(table, {r, g, b, a} = pixel) do
    idx = hash_pixel(r, g, b, a)
    :array.set(idx, pixel, table)
  end

  # The QOI hash: spread RGBA values across 64 slots using prime multipliers.
  # The specific primes (3, 5, 7, 11) were chosen to minimise collisions for
  # typical image data — small primes that interact well with the 64-modulus.
  defp hash_pixel(r, g, b, a) do
    rem(r * 3 + g * 5 + b * 7 + a * 11, @hash_table_size)
  end

  # Compute a signed delta in the range −128..+127, wrapping around byte bounds.
  # We first mask to 8 bits (treating the difference as unsigned mod 256), then
  # shift the range to be centred on 0.
  # Examples:
  #   wrap_delta(5)    =  5  (no wrap needed)
  #   wrap_delta(-3)   = -3  (no wrap needed)
  #   wrap_delta(-255) = 1   (255 → 0, one step forward)
  #   wrap_delta(255)  = -1  (0 → 255, one step backward)
  defp wrap_delta(d), do: rem((d &&& 0xFF) + 128, 256) - 128

  # ── Decoder ──────────────────────────────────────────────────────────────────

  @doc """
  Decodes a QOI binary into a `PixelContainer`.

  Returns `{:ok, container}` or `{:error, reason}`.
  """
  def decode_qoi(data) when is_binary(data) do
    with {:ok, {w, h, rest}} <- parse_qoi_header(data) do
      # Remove the 8-byte footer from the end before decoding the body
      body_size = byte_size(rest) - 8
      if body_size < 0 do
        {:error, "QOI data too short"}
      else
        <<body::binary-size(body_size), _footer::binary>> = rest
        total_pixels = w * h
        initial_table = :array.new(@hash_table_size, default: {0, 0, 0, 0})
        prev = {0, 0, 0, 255}
        result = decode_pixels(body, total_pixels, [], initial_table, prev)

        case result do
          {:ok, pixels} ->
            container = pixels_to_container(pixels, w, h)
            {:ok, container}

          {:error, _} = err ->
            err
        end
      end
    end
  end

  # Parse the 14-byte QOI header.
  defp parse_qoi_header(data) do
    if byte_size(data) < 22 do
      {:error, "QOI data too short"}
    else
      case data do
        <<@qoi_magic::binary,
          width::big-32,
          height::big-32,
          _channels::8,
          _colorspace::8,
          rest::binary>> ->
          {:ok, {width, height, rest}}

        _ ->
          {:error, "Invalid QOI header"}
      end
    end
  end

  # Decode pixels from the body byte stream.
  # Returns {:ok, [pixel...]} in row-major order.
  defp decode_pixels(_data, 0, pixels, _table, _prev) do
    {:ok, Enum.reverse(pixels)}
  end

  defp decode_pixels(<<>>, remaining, _pixels, _table, _prev) when remaining > 0 do
    {:error, "QOI body truncated: #{remaining} pixels remaining"}
  end

  defp decode_pixels(data, remaining, pixels, table, prev) do
    case data do
      # QOI_OP_RGBA: full explicit RGBA
      <<@qoi_op_rgba, r, g, b, a, rest::binary>> ->
        pixel = {r, g, b, a}
        table = update_table(table, pixel)
        decode_pixels(rest, remaining - 1, [pixel | pixels], table, pixel)

      # QOI_OP_RGB: explicit RGB, keep previous alpha
      <<@qoi_op_rgb, r, g, b, rest::binary>> ->
        {_pr, _pg, _pb, pa} = prev
        pixel = {r, g, b, pa}
        table = update_table(table, pixel)
        decode_pixels(rest, remaining - 1, [pixel | pixels], table, pixel)

      # Single-byte ops: decode by inspecting top 2 bits
      <<byte, rest::binary>> ->
        tag2 = byte >>> 6

        case tag2 do
          # QOI_OP_RUN: lower 6 bits = (run_length - 1), run 1..62
          # The tag 0xC0 (11000000) handles runs; 0xFE and 0xFF are used for
          # RGB/RGBA so run values 62 and 63 (0xFE and 0xFF after masking)
          # are reserved — they will be caught by the RGB/RGBA clauses above.
          3 ->
            run = (byte &&& 0x3F) + 1
            # Emit `run` copies of the previous pixel
            new_pixels = List.duplicate(prev, run) ++ pixels
            table = update_table(table, prev)
            decode_pixels(rest, remaining - run, new_pixels, table, prev)

          # QOI_OP_INDEX: 6-bit index into the hash table
          0 ->
            idx = byte &&& 0x3F
            pixel = :array.get(idx, table)
            # Note: QOI_OP_INDEX does NOT update the hash table
            decode_pixels(rest, remaining - 1, [pixel | pixels], table, pixel)

          # QOI_OP_DIFF: 2-bit deltas for r, g, b (bias 2)
          # Layout: 01 dr(2) dg(2) db(2)
          1 ->
            dr = ((byte >>> 4) &&& 0x3) - 2
            dg = ((byte >>> 2) &&& 0x3) - 2
            db = (byte &&& 0x3) - 2
            {pr, pg, pb, pa} = prev
            # Apply deltas with byte wrap-around modulo 256
            pixel = {byte_wrap(pr + dr), byte_wrap(pg + dg), byte_wrap(pb + db), pa}
            table = update_table(table, pixel)
            decode_pixels(rest, remaining - 1, [pixel | pixels], table, pixel)

          # QOI_OP_LUMA: 6-bit dg, then 4+4 dr_dg and db_dg deltas
          # First byte:  10 dg(6)  — dg with bias 32
          # Second byte: dr_dg(4) db_dg(4) — each with bias 8
          2 ->
            case rest do
              <<byte2, rest2::binary>> ->
                dg = (byte &&& 0x3F) - 32
                dr_dg = (byte2 >>> 4) - 8
                db_dg = (byte2 &&& 0x0F) - 8
                dr = dr_dg + dg
                db = db_dg + dg
                {pr, pg, pb, pa} = prev
                pixel = {byte_wrap(pr + dr), byte_wrap(pg + dg), byte_wrap(pb + db), pa}
                table = update_table(table, pixel)
                decode_pixels(rest2, remaining - 1, [pixel | pixels], table, pixel)

              <<>> ->
                {:error, "QOI LUMA op missing second byte"}
            end
        end
    end
  end

  # Wrap a channel value to the 0..255 range modulo 256.
  # This matches the QOI spec's requirement that channel arithmetic wraps.
  defp byte_wrap(v), do: v &&& 0xFF

  # Build a PixelContainer from a flat list of {r,g,b,a} tuples in row-major order.
  defp pixels_to_container(pixels, w, h) do
    container = PixelContainer.new(w, h)

    pixels
    |> Enum.with_index()
    |> Enum.reduce(container, fn {{r, g, b, a}, i}, acc ->
      x = rem(i, w)
      y = div(i, w)
      PixelContainer.set_pixel(acc, x, y, r, g, b, a)
    end)
  end
end
