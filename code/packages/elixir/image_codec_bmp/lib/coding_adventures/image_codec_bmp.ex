defmodule CodingAdventures.ImageCodecBmp do
  @moduledoc """
  IC01: BMP (Windows Bitmap) encoder and decoder.

  ## BMP format overview

  BMP is one of the oldest and simplest raster image formats. Every BMP file
  starts with two fixed-size headers followed by raw pixel data.

  ### File Header (14 bytes)

  ```
  Offset  Size  Field
  0       2     Signature: always "BM" (0x42, 0x4D)
  2       4     File size in bytes (little-endian uint32)
  6       2     Reserved 1 (must be 0)
  8       2     Reserved 2 (must be 0)
  10      4     Pixel data offset from start of file (little-endian uint32)
  ```

  ### DIB Header — BITMAPINFOHEADER (40 bytes)

  ```
  Offset  Size  Field
  14      4     Header size = 40 (little-endian uint32)
  18      4     Width in pixels (little-endian int32, positive = left-to-right)
  22      4     Height in pixels (little-endian int32, negative = top-to-bottom)
  26      2     Color planes = 1
  28      2     Bits per pixel = 32 (for BGRA8)
  30      4     Compression = 3 (BI_BITFIELDS, used for BGRA)
  34      4     Raw pixel data size in bytes
  38      4     Horizontal resolution (pixels/meter) — we use 2835 = 72 DPI
  42      4     Vertical resolution (pixels/meter) — same
  46      4     Colors in palette = 0
  50      4     Important colors = 0
  ```

  Then for BI_BITFIELDS compression, 12 bytes of channel masks follow:

  ```
  54      4     Red mask   = 0x00FF0000
  58      4     Green mask = 0x0000FF00
  62      4     Blue mask  = 0x000000FF
  ```

  Total header = 14 + 40 + 12 = 66 bytes (pixel data starts at offset 66).

  ### Pixel data

  With negative height (top-to-bottom), pixels are stored row by row starting
  at row 0. Each pixel is 4 bytes in BGRA order (Blue, Green, Red, Alpha).
  Note the byte swap: BMP stores Blue first, not Red!

  Rows are padded to a 4-byte boundary. With 32 bits-per-pixel the row stride
  is always a multiple of 4, so no padding is ever needed.

  ## Example

      iex> alias CodingAdventures.{PixelContainer, ImageCodecBmp}
      iex> c = PixelContainer.new(2, 2)
      iex> c = PixelContainer.set_pixel(c, 0, 0, 255, 0, 0, 255)
      iex> data = ImageCodecBmp.encode(c)
      iex> {:ok, c2} = ImageCodecBmp.decode(data)
      iex> PixelContainer.pixel_at(c2, 0, 0)
      {255, 0, 0, 255}

  """

  @behaviour CodingAdventures.ImageCodec

  alias CodingAdventures.PixelContainer

  # ── BMP constants ────────────────────────────────────────────────────────────

  # Maximum allowed dimension to prevent oversized image allocation
  @max_dimension 16384

  # "BM" signature
  @bmp_signature <<0x42, 0x4D>>

  # BITMAPINFOHEADER is always 40 bytes
  @dib_header_size 40

  # 14 (file header) + 40 (DIB header) + 12 (channel masks) = 66
  @pixel_data_offset 66

  # BI_BITFIELDS compression type — required when specifying channel masks
  @bi_bitfields 3

  # 32-bit BGRA: 4 bytes per pixel, so row stride = width * 4 (always aligned)
  @bits_per_pixel 32

  # 72 DPI ≈ 2835 pixels per metre (standard screen resolution)
  @pixels_per_metre 2835

  # Channel masks for BGRA32 layout
  @red_mask 0x00FF0000
  @green_mask 0x0000FF00
  @blue_mask 0x000000FF

  # ── Behaviour callbacks ───────────────────────────────────────────────────────

  @impl true
  def mime_type, do: "image/bmp"

  @impl true
  def encode(%PixelContainer{} = c), do: encode_bmp(c)

  @impl true
  def decode(data) when is_binary(data), do: decode_bmp(data)

  # ── Encoder ──────────────────────────────────────────────────────────────────

  @doc """
  Encodes a `PixelContainer` as a BMP binary.

  Produces a valid BMP file with:
  - 32 bits per pixel (BGRA channel order)
  - BI_BITFIELDS compression with explicit channel masks
  - Negative height so row 0 is at the top (top-to-bottom scan order)

  ## Why BGRA and not RGBA?

  BMP was designed by Microsoft for Windows in the 1980s. The x86 architecture
  stores multi-byte integers in little-endian order (least significant byte
  first). For a 32-bit RGBA pixel `0xRRGGBBAA`, little-endian storage reverses
  the byte order to `AA BB GG RR` — which looks like BGRA when read as bytes.
  This is a historical quirk that every BMP reader must handle.
  """
  def encode_bmp(%PixelContainer{width: w, height: h} = c) do
    pixel_data = encode_pixels(c)
    pixel_data_size = byte_size(pixel_data)
    file_size = @pixel_data_offset + pixel_data_size

    # Build file header (14 bytes)
    file_header =
      <<@bmp_signature::binary,
        # total file size
        file_size::little-32,
        # reserved1
        0::little-16,
        # reserved2
        0::little-16,
        # offset to pixel data
        @pixel_data_offset::little-32>>

    # Build DIB header (40 bytes)
    # Negative height signals top-to-bottom row order (most natural for us).
    # Positive height would mean bottom-to-top (the BMP default), which would
    # require flipping rows during encode/decode.
    dib_header =
      <<@dib_header_size::little-32,
        w::little-32,
        # negative = top-down
        (-h)::little-signed-32,
        # color planes (always 1)
        1::little-16,
        @bits_per_pixel::little-16,
        @bi_bitfields::little-32,
        pixel_data_size::little-32,
        @pixels_per_metre::little-32,
        @pixels_per_metre::little-32,
        # colors in table
        0::little-32,
        # important colors
        0::little-32>>

    # Channel masks (12 bytes) — tell readers which bits are R, G, B
    channel_masks =
      <<@red_mask::little-32,
        @green_mask::little-32,
        @blue_mask::little-32>>

    file_header <> dib_header <> channel_masks <> pixel_data
  end

  # Convert each RGBA pixel to BGRA order for the BMP file.
  # BMP stores bytes as B, G, R, A — the Red and Blue channels are swapped
  # compared to our PixelContainer which stores R, G, B, A.
  defp encode_pixels(%PixelContainer{width: w, height: h} = c) do
    for y <- 0..(h - 1), x <- 0..(w - 1), into: <<>> do
      {r, g, b, a} = PixelContainer.pixel_at(c, x, y)
      # Swap: output B first, then G, R, A
      <<b, g, r, a>>
    end
  end

  # ── Decoder ──────────────────────────────────────────────────────────────────

  @doc """
  Decodes a BMP binary into a `PixelContainer`.

  Supports:
  - 24-bit BGR (no alpha channel — alpha is set to 255)
  - 32-bit BGRA with BI_BITFIELDS compression

  Returns `{:ok, container}` or `{:error, reason}`.
  """
  def decode_bmp(data) when is_binary(data) do
    with {:ok, {w, h, bits_per_pixel, compression, pixel_offset}} <- parse_headers(data),
         {:ok, pixels} <- extract_pixels(data, w, h, bits_per_pixel, compression, pixel_offset) do
      {:ok, pixels}
    end
  end

  # Parse the file header and DIB header to extract image metadata.
  defp parse_headers(data) do
    # Minimum size: 14 (file header) + 40 (DIB header) = 54 bytes
    if byte_size(data) < 54 do
      {:error, "BMP data too short: #{byte_size(data)} bytes"}
    else
      case data do
        <<@bmp_signature::binary,
          _file_size::little-32,
          _reserved1::little-16,
          _reserved2::little-16,
          pixel_offset::little-32,
          @dib_header_size::little-32,
          width::little-32,
          height::little-signed-32,
          _planes::little-16,
          bpp::little-16,
          compression::little-32,
          _pixel_data_size::little-32,
          _rest::binary>> ->
          if width > @max_dimension or abs(height) > @max_dimension do
            {:error, "BMP: image dimensions too large"}
          else
            {:ok, {width, abs(height), bpp, compression, pixel_offset}}
          end

        _ ->
          {:error, "Invalid BMP header"}
      end
    end
  end

  # Extract pixels from the raw pixel data section of the BMP.
  defp extract_pixels(data, w, h, bpp, compression, pixel_offset)
       when bpp in [24, 32] and compression in [0, 3] do
    # Guard against a pixel_offset that exceeds the file length
    if pixel_offset > byte_size(data) do
      {:error, "BMP: pixel offset exceeds file size"}
    else
    # Skip to the pixel data
    <<_header::binary-size(pixel_offset), pixel_data::binary>> = data

    # Row stride must be rounded up to the nearest 4-byte boundary.
    # For 32bpp: stride = w*4 (always aligned).
    # For 24bpp: stride = ceil(w*3 / 4) * 4.
    bytes_per_pixel = div(bpp, 8)
    row_bytes = w * bytes_per_pixel
    # BMP pads rows to 4-byte alignment
    row_stride = row_bytes + rem(4 - rem(row_bytes, 4), 4)

    container = PixelContainer.new(w, h)

    result =
      Enum.reduce_while(0..(h - 1), container, fn y, acc ->
        row_offset = y * row_stride
        row_end = row_offset + row_bytes

        if byte_size(pixel_data) < row_end do
          {:halt, {:error, "BMP pixel data truncated at row #{y}"}}
        else
          row = :binary.part(pixel_data, row_offset, row_bytes)
          updated = decode_row(acc, row, y, w, bpp)
          {:cont, updated}
        end
      end)

    case result do
      {:error, _} = err -> err
      container -> {:ok, container}
    end
    end
  end

  defp extract_pixels(_data, _w, _h, bpp, compression, _offset) do
    {:error, "Unsupported BMP format: #{bpp}bpp compression=#{compression}"}
  end

  # Decode one row of pixels.
  # 32bpp: each pixel is B, G, R, A bytes in that order.
  # 24bpp: each pixel is B, G, R bytes; we set A = 255 (fully opaque).
  defp decode_row(container, row_data, y, w, bpp) do
    Enum.reduce(0..(w - 1), container, fn x, acc ->
      offset = x * div(bpp, 8)

      {r, g, b, a} =
        case bpp do
          32 ->
            # 32bpp BGRA: read B G R A and swap B/R back to RGBA
            <<_::binary-size(offset), b, g, r, a, _::binary>> = row_data
            {r, g, b, a}

          24 ->
            # 24bpp BGR: read B G R, alpha defaults to 255 (fully opaque)
            <<_::binary-size(offset), b, g, r, _::binary>> = row_data
            {r, g, b, 255}
        end

      PixelContainer.set_pixel(acc, x, y, r, g, b, a)
    end)
  end
end
