defmodule CodingAdventures.PixelContainer do
  @moduledoc """
  IC00: Fixed RGBA8 pixel buffer.

  A pixel container is a flat binary buffer storing pixels in row-major order.
  "Row-major" means all pixels in row 0 come first, then row 1, and so on —
  just like reading text: left-to-right, top-to-bottom.

  Each pixel occupies exactly 4 bytes:
    byte 0 = Red   (0–255)
    byte 1 = Green (0–255)
    byte 2 = Blue  (0–255)
    byte 3 = Alpha (0–255, where 255 = fully opaque, 0 = fully transparent)

  This is the RGBA8 format (8 bits per channel, 4 channels).

  ## Byte layout

  For a W×H image, total bytes = W * H * 4.

  Pixel at column x, row y lives at byte offset:

      (y * W + x) * 4

  Think of it like a 2-D array flattened into 1-D:
  row y starts at byte y*W*4, and column x adds x*4 bytes within that row.

  ## Example

      iex> c = CodingAdventures.PixelContainer.new(2, 2)
      iex> c = CodingAdventures.PixelContainer.set_pixel(c, 0, 0, 255, 0, 0, 255)
      iex> CodingAdventures.PixelContainer.pixel_at(c, 0, 0)
      {255, 0, 0, 255}

  """

  defstruct [:width, :height, :data]

  @type t :: %__MODULE__{
          width: non_neg_integer(),
          height: non_neg_integer(),
          data: binary()
        }

  @doc """
  Creates a zeroed RGBA8 buffer of the given dimensions.

  All pixels start as transparent black: {0, 0, 0, 0}.

  ## Examples

      iex> c = CodingAdventures.PixelContainer.new(4, 3)
      iex> c.width
      4
      iex> c.height
      3
      iex> byte_size(c.data)
      48

  """
  def new(width, height) do
    # Total bytes = width * height pixels, each 4 bytes wide.
    # <<0::size(n)>> creates a binary of n bits, all zero.
    # We need n = width * height * 4 * 8 bits.
    size = width * height * 4
    %__MODULE__{width: width, height: height, data: <<0::size(size * 8)>>}
  end

  @doc """
  Returns `{r, g, b, a}` at pixel position `(x, y)`.

  Returns `{0, 0, 0, 0}` if the coordinates are out of bounds, rather than
  raising — this makes compositing logic easier: reading outside the canvas
  simply yields transparent black.

  ## Examples

      iex> c = CodingAdventures.PixelContainer.new(10, 10)
      iex> CodingAdventures.PixelContainer.pixel_at(c, 0, 0)
      {0, 0, 0, 0}
      iex> CodingAdventures.PixelContainer.pixel_at(c, -1, 0)
      {0, 0, 0, 0}
      iex> CodingAdventures.PixelContainer.pixel_at(c, 10, 0)
      {0, 0, 0, 0}

  """
  def pixel_at(%__MODULE__{} = c, x, y) do
    if x >= 0 and y >= 0 and x < c.width and y < c.height do
      # Jump to byte (y*width + x)*4 in the binary, then read 4 bytes.
      # `_::binary-size(offset)` skips `offset` bytes.
      # Then r, g, b, a each match one byte.
      # `_::binary` consumes the rest (required to make the pattern exhaustive).
      offset = (y * c.width + x) * 4
      <<_::binary-size(offset), r, g, b, a, _::binary>> = c.data
      {r, g, b, a}
    else
      {0, 0, 0, 0}
    end
  end

  @doc """
  Sets the pixel at `(x, y)` to the given RGBA values.

  Returns the updated container. If `(x, y)` is out of bounds, returns the
  container unchanged — safe no-op semantics.

  ## Examples

      iex> c = CodingAdventures.PixelContainer.new(5, 5)
      iex> c = CodingAdventures.PixelContainer.set_pixel(c, 2, 3, 128, 64, 32, 255)
      iex> CodingAdventures.PixelContainer.pixel_at(c, 2, 3)
      {128, 64, 32, 255}

  """
  def set_pixel(%__MODULE__{} = c, x, y, r, g, b, a) do
    if x >= 0 and y >= 0 and x < c.width and y < c.height do
      offset = (y * c.width + x) * 4
      # Split the binary around the 4 bytes we want to replace:
      #   `before` = everything before the pixel
      #   `_r, _g, _b, _a` = the old pixel bytes (discarded)
      #   `rest` = everything after the pixel
      # Reassemble with new r, g, b, a spliced in.
      <<before::binary-size(offset), _r, _g, _b, _a, rest::binary>> = c.data
      %{c | data: <<before::binary, r, g, b, a, rest::binary>>}
    else
      c
    end
  end

  @doc """
  Fills the entire buffer with a single RGBA value.

  Uses `:binary.copy/2` to efficiently repeat the 4-byte pixel pattern
  across the entire buffer — much faster than iterating pixel by pixel.

  ## Examples

      iex> c = CodingAdventures.PixelContainer.new(3, 3)
      iex> c = CodingAdventures.PixelContainer.fill_pixels(c, 255, 255, 255, 255)
      iex> CodingAdventures.PixelContainer.pixel_at(c, 1, 1)
      {255, 255, 255, 255}

  """
  def fill_pixels(%__MODULE__{width: w, height: h} = c, r, g, b, a) do
    # Build a single 4-byte pixel pattern, then tile it w*h times.
    # :binary.copy/2 is an Erlang BIF (Built-In Function) that repeats a
    # binary n times, producing a single contiguous binary.
    pixel = <<r, g, b, a>>
    data = :binary.copy(pixel, w * h)
    %{c | data: data}
  end

  @doc """
  Returns the total number of bytes in the pixel buffer.

  Should always equal `width * height * 4`.
  """
  def byte_size(%__MODULE__{data: data}), do: :erlang.byte_size(data)
end
