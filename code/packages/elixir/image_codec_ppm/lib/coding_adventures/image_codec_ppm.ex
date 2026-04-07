defmodule CodingAdventures.ImageCodecPpm do
  @moduledoc """
  IC02: PPM (Portable PixMap) encoder and decoder.

  ## PPM format overview

  PPM is the "Portable PixMap" format, part of the Netpbm family invented by
  Jef Poskanzer in 1988. It is intentionally trivial — a human-readable ASCII
  header followed by raw RGB bytes — making it perfect for learning and
  debugging image pipelines.

  ### File structure

  ```
  P6\\n
  # optional comment lines starting with '#'\\n
  <width> <height>\\n
  <maxval>\\n
  <raw RGB bytes>
  ```

  - **Magic number**: `P6` for binary PPM (as opposed to `P3` for ASCII PPM).
  - **Dimensions**: width and height as decimal ASCII integers, space-separated.
  - **Maxval**: the maximum channel value, almost always `255`.
  - **Pixel data**: raw bytes, 3 bytes per pixel in R, G, B order. No alpha.
    Each row is `width * 3` bytes, with no padding.

  ### Alpha handling

  PPM has no alpha channel. On encode, the alpha byte is dropped. On decode,
  alpha is set to 255 (fully opaque) for every pixel.

  ### Comments

  Lines beginning with `#` anywhere in the header (before the binary section)
  are comments and are skipped during parsing.

  ## Example

      iex> alias CodingAdventures.{PixelContainer, ImageCodecPpm}
      iex> c = PixelContainer.new(2, 1)
      iex> c = PixelContainer.set_pixel(c, 0, 0, 255, 0, 0, 255)
      iex> c = PixelContainer.set_pixel(c, 1, 0, 0, 0, 255, 128)
      iex> data = ImageCodecPpm.encode(c)
      iex> {:ok, c2} = ImageCodecPpm.decode(data)
      iex> PixelContainer.pixel_at(c2, 0, 0)
      {255, 0, 0, 255}
      iex> PixelContainer.pixel_at(c2, 1, 0)
      {0, 0, 255, 255}

  """

  @behaviour CodingAdventures.ImageCodec

  alias CodingAdventures.PixelContainer

  # ── Behaviour callbacks ───────────────────────────────────────────────────────

  @impl true
  def mime_type, do: "image/x-portable-pixmap"

  @impl true
  def encode(%PixelContainer{} = c), do: encode_ppm(c)

  @impl true
  def decode(data) when is_binary(data), do: decode_ppm(data)

  # ── Encoder ──────────────────────────────────────────────────────────────────

  @doc """
  Encodes a `PixelContainer` as a binary PPM (P6) file.

  Alpha is discarded — PPM stores only RGB. The maxval is always 255.
  """
  def encode_ppm(%PixelContainer{width: w, height: h} = c) do
    # Build the ASCII header: magic, dimensions, maxval, each separated by newlines.
    # A single space between width and height is conventional.
    header = "P6\n#{w} #{h}\n255\n"

    # Build the pixel data: 3 bytes per pixel (R, G, B), dropping alpha.
    pixel_data =
      for y <- 0..(h - 1), x <- 0..(w - 1), into: <<>> do
        {r, g, b, _a} = PixelContainer.pixel_at(c, x, y)
        # PPM stores R, G, B in that order — no byte swapping needed.
        <<r, g, b>>
      end

    header <> pixel_data
  end

  # ── Decoder ──────────────────────────────────────────────────────────────────

  @doc """
  Decodes a binary PPM (P6) file into a `PixelContainer`.

  Skips comment lines (starting with `#`). Sets alpha = 255 for all pixels.

  Returns `{:ok, container}` or `{:error, reason}`.
  """
  def decode_ppm(data) when is_binary(data) do
    with {:ok, {magic, rest}} <- read_token(data),
         :ok <- check_magic(magic),
         {:ok, {w_str, rest}} <- skip_comments_and_read(rest),
         {:ok, {h_str, rest}} <- skip_comments_and_read(rest),
         {:ok, {maxval_str, rest}} <- skip_comments_and_read(rest),
         {:ok, width} <- parse_int(w_str, "width"),
         {:ok, height} <- parse_int(h_str, "height"),
         {:ok, _maxval} <- parse_int(maxval_str, "maxval"),
         :ok <- check_pixel_data_size(rest, width, height) do
      container = build_container(rest, width, height)
      {:ok, container}
    end
  end

  # Validate the P6 magic number.
  defp check_magic("P6"), do: :ok
  defp check_magic(got), do: {:error, "Expected PPM magic 'P6', got '#{got}'"}

  # Read one whitespace-delimited token from the binary.
  # Skips leading whitespace (spaces, tabs, \r, \n).
  defp read_token(<<>>), do: {:error, "Unexpected end of PPM data"}

  defp read_token(data) do
    # Skip leading whitespace
    data = skip_whitespace(data)
    # Collect non-whitespace bytes into the token
    collect_token(data, "")
  end

  defp collect_token(<<>>, token) when token != "", do: {:ok, {token, <<>>}}
  defp collect_token(<<>>, _), do: {:error, "Unexpected end of PPM data"}

  defp collect_token(<<c, rest::binary>>, token) do
    if c in [?\s, ?\t, ?\r, ?\n] do
      if token == "" do
        # Keep skipping whitespace
        collect_token(rest, "")
      else
        {:ok, {token, rest}}
      end
    else
      collect_token(rest, token <> <<c>>)
    end
  end

  # Skip comment lines (starting with #) and then read the next token.
  defp skip_comments_and_read(data) do
    data = skip_whitespace(data)

    case data do
      <<"#", rest::binary>> ->
        # Skip until end of line, then try again
        rest = skip_to_eol(rest)
        skip_comments_and_read(rest)

      _ ->
        read_token(data)
    end
  end

  defp skip_whitespace(<<c, rest::binary>>) when c in [?\s, ?\t, ?\r, ?\n],
    do: skip_whitespace(rest)

  defp skip_whitespace(data), do: data

  defp skip_to_eol(<<?\n, rest::binary>>), do: rest
  defp skip_to_eol(<<_, rest::binary>>), do: skip_to_eol(rest)
  defp skip_to_eol(<<>>), do: <<>>

  defp parse_int(str, field) do
    case Integer.parse(str) do
      {n, ""} when n > 0 -> {:ok, n}
      {n, ""} when n == 0 -> {:error, "PPM #{field} must be > 0"}
      _ -> {:error, "Invalid PPM #{field}: '#{str}'"}
    end
  end

  defp check_pixel_data_size(data, w, h) do
    expected = w * h * 3
    actual = byte_size(data)

    if actual >= expected do
      :ok
    else
      {:error, "PPM pixel data too short: expected #{expected} bytes, got #{actual}"}
    end
  end

  # Build a PixelContainer from raw RGB bytes.
  # Each pixel is 3 bytes: R, G, B. Alpha is set to 255.
  defp build_container(pixel_data, width, height) do
    container = PixelContainer.new(width, height)

    Enum.reduce(0..(height - 1), container, fn y, acc ->
      Enum.reduce(0..(width - 1), acc, fn x, acc2 ->
        offset = (y * width + x) * 3
        <<_::binary-size(offset), r, g, b, _::binary>> = pixel_data
        PixelContainer.set_pixel(acc2, x, y, r, g, b, 255)
      end)
    end)
  end
end
