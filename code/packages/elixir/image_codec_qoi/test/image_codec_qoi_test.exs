defmodule CodingAdventures.ImageCodecQoiTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.{PixelContainer, ImageCodecQoi}

  import Bitwise

  # ─────────────────────────────────────────────────────────────────────────────
  # mime_type/0
  # ─────────────────────────────────────────────────────────────────────────────

  describe "mime_type/0" do
    test "returns image/qoi" do
      assert ImageCodecQoi.mime_type() == "image/qoi"
    end
  end

  # ─────────────────────────────────────────────────────────────────────────────
  # encode/1 — structural checks
  # ─────────────────────────────────────────────────────────────────────────────

  describe "encode/1 structure" do
    test "encoded output starts with qoif magic" do
      c = PixelContainer.new(2, 2)
      data = ImageCodecQoi.encode(c)
      assert binary_part(data, 0, 4) == "qoif"
    end

    test "header contains correct width (big-endian)" do
      c = PixelContainer.new(300, 200)
      data = ImageCodecQoi.encode(c)
      <<_magic::binary-size(4), width::big-32, _::binary>> = data
      assert width == 300
    end

    test "header contains correct height (big-endian)" do
      c = PixelContainer.new(300, 200)
      data = ImageCodecQoi.encode(c)
      <<_magic::binary-size(4), _w::big-32, height::big-32, _::binary>> = data
      assert height == 200
    end

    test "encoded output ends with QOI footer" do
      c = PixelContainer.new(2, 2)
      data = ImageCodecQoi.encode(c)
      footer = binary_part(data, byte_size(data) - 8, 8)
      assert footer == <<0, 0, 0, 0, 0, 0, 0, 1>>
    end

    test "minimum file size is 14 (header) + 1 (op) + 8 (footer) = 23 bytes" do
      # A 1x1 image with one pixel needs at least one op-code
      c = PixelContainer.new(1, 1)
      data = ImageCodecQoi.encode(c)
      assert byte_size(data) >= 23
    end
  end

  # ─────────────────────────────────────────────────────────────────────────────
  # Round-trip: encode then decode
  # ─────────────────────────────────────────────────────────────────────────────

  describe "encode/decode round-trip" do
    test "single red pixel" do
      c = PixelContainer.new(1, 1)
      c = PixelContainer.set_pixel(c, 0, 0, 255, 0, 0, 255)
      data = ImageCodecQoi.encode(c)
      assert {:ok, c2} = ImageCodecQoi.decode(data)
      assert PixelContainer.pixel_at(c2, 0, 0) == {255, 0, 0, 255}
    end

    test "single transparent pixel" do
      c = PixelContainer.new(1, 1)
      c = PixelContainer.set_pixel(c, 0, 0, 0, 128, 255, 0)
      data = ImageCodecQoi.encode(c)
      assert {:ok, c2} = ImageCodecQoi.decode(data)
      assert PixelContainer.pixel_at(c2, 0, 0) == {0, 128, 255, 0}
    end

    test "uniform fill — exercises QOI_OP_RUN heavily" do
      c = PixelContainer.new(10, 10)
      c = PixelContainer.fill_pixels(c, 200, 150, 100, 255)
      data = ImageCodecQoi.encode(c)
      assert {:ok, c2} = ImageCodecQoi.decode(data)

      for x <- 0..9, y <- 0..9 do
        assert PixelContainer.pixel_at(c2, x, y) == {200, 150, 100, 255},
               "mismatch at (#{x}, #{y})"
      end
    end

    test "gradient image — exercises QOI_OP_DIFF and QOI_OP_LUMA" do
      # A slow gradient changes by 1–2 per step, ideal for DIFF/LUMA ops
      c = PixelContainer.new(8, 8)

      c =
        for y <- 0..7, x <- 0..7, reduce: c do
          acc ->
            r = min(255, x * 30)
            g = min(255, y * 30)
            b = min(255, (x + y) * 15)
            PixelContainer.set_pixel(acc, x, y, r, g, b, 255)
        end

      data = ImageCodecQoi.encode(c)
      assert {:ok, c2} = ImageCodecQoi.decode(data)

      for x <- 0..7, y <- 0..7 do
        r = min(255, x * 30)
        g = min(255, y * 30)
        b = min(255, (x + y) * 15)
        assert PixelContainer.pixel_at(c2, x, y) == {r, g, b, 255},
               "mismatch at (#{x}, #{y})"
      end
    end

    test "checkerboard — exercises QOI_OP_INDEX (repeating pixel pairs)" do
      c = PixelContainer.new(4, 4)

      c =
        for y <- 0..3, x <- 0..3, reduce: c do
          acc ->
            if rem(x + y, 2) == 0 do
              PixelContainer.set_pixel(acc, x, y, 255, 255, 255, 255)
            else
              PixelContainer.set_pixel(acc, x, y, 0, 0, 0, 255)
            end
        end

      data = ImageCodecQoi.encode(c)
      assert {:ok, c2} = ImageCodecQoi.decode(data)

      for x <- 0..3, y <- 0..3 do
        expected =
          if rem(x + y, 2) == 0 do
            {255, 255, 255, 255}
          else
            {0, 0, 0, 255}
          end

        assert PixelContainer.pixel_at(c2, x, y) == expected,
               "mismatch at (#{x}, #{y})"
      end
    end

    test "image with varying alpha — exercises QOI_OP_RGBA" do
      c = PixelContainer.new(4, 1)
      c = PixelContainer.set_pixel(c, 0, 0, 255, 0, 0, 255)
      c = PixelContainer.set_pixel(c, 1, 0, 255, 0, 0, 128)
      c = PixelContainer.set_pixel(c, 2, 0, 255, 0, 0, 64)
      c = PixelContainer.set_pixel(c, 3, 0, 255, 0, 0, 0)
      data = ImageCodecQoi.encode(c)
      assert {:ok, c2} = ImageCodecQoi.decode(data)
      assert PixelContainer.pixel_at(c2, 0, 0) == {255, 0, 0, 255}
      assert PixelContainer.pixel_at(c2, 1, 0) == {255, 0, 0, 128}
      assert PixelContainer.pixel_at(c2, 2, 0) == {255, 0, 0, 64}
      assert PixelContainer.pixel_at(c2, 3, 0) == {255, 0, 0, 0}
    end

    test "decoded container has correct dimensions" do
      c = PixelContainer.new(13, 7)
      data = ImageCodecQoi.encode(c)
      assert {:ok, c2} = ImageCodecQoi.decode(data)
      assert c2.width == 13
      assert c2.height == 7
    end

    test "1x1 blank image round-trips" do
      c = PixelContainer.new(1, 1)
      data = ImageCodecQoi.encode(c)
      assert {:ok, c2} = ImageCodecQoi.decode(data)
      # Blank pixel is {0,0,0,0}; QOI initial prev is {0,0,0,255} so this
      # will emit QOI_OP_RGBA to change alpha to 0
      assert PixelContainer.pixel_at(c2, 0, 0) == {0, 0, 0, 0}
    end

    test "pixels that wrap around byte boundaries round-trip correctly" do
      # Channel value near boundary: 254 → 1 (delta = -253, wraps to +3)
      c = PixelContainer.new(2, 1)
      c = PixelContainer.set_pixel(c, 0, 0, 254, 254, 254, 255)
      c = PixelContainer.set_pixel(c, 1, 0, 1, 1, 1, 255)
      data = ImageCodecQoi.encode(c)
      assert {:ok, c2} = ImageCodecQoi.decode(data)
      assert PixelContainer.pixel_at(c2, 0, 0) == {254, 254, 254, 255}
      assert PixelContainer.pixel_at(c2, 1, 0) == {1, 1, 1, 255}
    end

    test "long run (>62 pixels) is encoded and decoded correctly" do
      # 10x10 uniform image = 100 pixels. Max run length is 62, so encoder
      # must split into at least two run ops.
      c = PixelContainer.new(10, 10)
      c = PixelContainer.fill_pixels(c, 77, 88, 99, 255)
      data = ImageCodecQoi.encode(c)
      assert {:ok, c2} = ImageCodecQoi.decode(data)

      for x <- 0..9, y <- 0..9 do
        assert PixelContainer.pixel_at(c2, x, y) == {77, 88, 99, 255}
      end
    end

    test "all pixel channel permutations in a 4x4 grid round-trip" do
      c = PixelContainer.new(4, 4)

      c =
        for x <- 0..3, y <- 0..3, reduce: c do
          acc ->
            r = x * 63
            g = y * 63
            b = (x + y) * 32
            a = if rem(x + y, 2) == 0, do: 255, else: 128
            PixelContainer.set_pixel(acc, x, y, r, g, b, a)
        end

      data = ImageCodecQoi.encode(c)
      assert {:ok, c2} = ImageCodecQoi.decode(data)

      for x <- 0..3, y <- 0..3 do
        r = x * 63
        g = y * 63
        b = (x + y) * 32
        a = if rem(x + y, 2) == 0, do: 255, else: 128
        assert PixelContainer.pixel_at(c2, x, y) == {r, g, b, a},
               "mismatch at (#{x}, #{y})"
      end
    end
  end

  # ─────────────────────────────────────────────────────────────────────────────
  # Compression effectiveness
  # ─────────────────────────────────────────────────────────────────────────────

  describe "compression" do
    test "uniform image compresses significantly vs raw size" do
      # 100 pixel uniform image: raw = 400 bytes, QOI body should be << that
      c = PixelContainer.new(10, 10)
      c = PixelContainer.fill_pixels(c, 100, 200, 50, 255)
      data = ImageCodecQoi.encode(c)
      # Header (14) + footer (8) + at most a few run bytes
      assert byte_size(data) < 50,
             "Expected strong compression, got #{byte_size(data)} bytes"
    end
  end

  # ─────────────────────────────────────────────────────────────────────────────
  # decode/1 — error handling
  # ─────────────────────────────────────────────────────────────────────────────

  describe "decode/1 error handling" do
    test "returns error for empty binary" do
      assert {:error, _} = ImageCodecQoi.decode(<<>>)
    end

    test "returns error for wrong magic" do
      bad = <<"NOPE", 0::big-32, 0::big-32, 4, 0>>
      assert {:error, _} = ImageCodecQoi.decode(bad)
    end

    test "returns error for truncated data" do
      assert {:error, _} = ImageCodecQoi.decode(<<"qoif", 1::big-32, 1::big-32, 4, 0>>)
    end
  end
end
