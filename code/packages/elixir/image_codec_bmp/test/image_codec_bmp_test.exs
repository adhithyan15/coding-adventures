defmodule CodingAdventures.ImageCodecBmpTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.{PixelContainer, ImageCodecBmp}

  # ─────────────────────────────────────────────────────────────────────────────
  # mime_type/0
  # ─────────────────────────────────────────────────────────────────────────────

  describe "mime_type/0" do
    test "returns image/bmp" do
      assert ImageCodecBmp.mime_type() == "image/bmp"
    end
  end

  # ─────────────────────────────────────────────────────────────────────────────
  # encode/1 — structural checks
  # ─────────────────────────────────────────────────────────────────────────────

  describe "encode/1 structure" do
    test "encoded output starts with BM signature" do
      c = PixelContainer.new(4, 4)
      data = ImageCodecBmp.encode(c)
      assert binary_part(data, 0, 2) == "BM"
    end

    test "encoded file size field matches actual byte count" do
      c = PixelContainer.new(3, 5)
      data = ImageCodecBmp.encode(c)
      <<_sig::binary-size(2), file_size::little-32, _::binary>> = data
      assert file_size == byte_size(data)
    end

    test "pixel data offset is 66 (14 + 40 + 12)" do
      c = PixelContainer.new(2, 2)
      data = ImageCodecBmp.encode(c)
      <<_::binary-size(10), pixel_offset::little-32, _::binary>> = data
      assert pixel_offset == 66
    end

    test "DIB header reports correct width" do
      c = PixelContainer.new(17, 9)
      data = ImageCodecBmp.encode(c)
      # Width is at offset 18
      <<_::binary-size(18), width::little-32, _::binary>> = data
      assert width == 17
    end

    test "DIB header reports negative height (top-down) with correct magnitude" do
      c = PixelContainer.new(10, 7)
      data = ImageCodecBmp.encode(c)
      # Height (signed) is at offset 22
      <<_::binary-size(22), height::little-signed-32, _::binary>> = data
      assert height == -7
    end

    test "bits per pixel field is 32" do
      c = PixelContainer.new(4, 4)
      data = ImageCodecBmp.encode(c)
      # bpp is at offset 28
      <<_::binary-size(28), bpp::little-16, _::binary>> = data
      assert bpp == 32
    end

    test "pixel data section is width * height * 4 bytes" do
      w = 6
      h = 5
      c = PixelContainer.new(w, h)
      data = ImageCodecBmp.encode(c)
      pixel_data = binary_part(data, 66, byte_size(data) - 66)
      assert byte_size(pixel_data) == w * h * 4
    end
  end

  # ─────────────────────────────────────────────────────────────────────────────
  # encode/decode round-trip
  # ─────────────────────────────────────────────────────────────────────────────

  describe "encode/decode round-trip" do
    test "single red pixel round-trips correctly" do
      c = PixelContainer.new(1, 1)
      c = PixelContainer.set_pixel(c, 0, 0, 255, 0, 0, 255)
      data = ImageCodecBmp.encode(c)
      assert {:ok, c2} = ImageCodecBmp.decode(data)
      assert PixelContainer.pixel_at(c2, 0, 0) == {255, 0, 0, 255}
    end

    test "single green pixel round-trips correctly" do
      c = PixelContainer.new(1, 1)
      c = PixelContainer.set_pixel(c, 0, 0, 0, 255, 0, 255)
      data = ImageCodecBmp.encode(c)
      assert {:ok, c2} = ImageCodecBmp.decode(data)
      assert PixelContainer.pixel_at(c2, 0, 0) == {0, 255, 0, 255}
    end

    test "single blue pixel round-trips correctly" do
      c = PixelContainer.new(1, 1)
      c = PixelContainer.set_pixel(c, 0, 0, 0, 0, 255, 255)
      data = ImageCodecBmp.encode(c)
      assert {:ok, c2} = ImageCodecBmp.decode(data)
      assert PixelContainer.pixel_at(c2, 0, 0) == {0, 0, 255, 255}
    end

    test "semi-transparent pixel preserves alpha" do
      c = PixelContainer.new(1, 1)
      c = PixelContainer.set_pixel(c, 0, 0, 128, 64, 32, 100)
      data = ImageCodecBmp.encode(c)
      assert {:ok, c2} = ImageCodecBmp.decode(data)
      assert PixelContainer.pixel_at(c2, 0, 0) == {128, 64, 32, 100}
    end

    test "3x3 image with distinct pixels all round-trip correctly" do
      c = PixelContainer.new(3, 3)

      # Fill each pixel with a unique color derived from its position
      c =
        for x <- 0..2, y <- 0..2, reduce: c do
          acc ->
            r = x * 80
            g = y * 80
            b = (x + y) * 40
            PixelContainer.set_pixel(acc, x, y, r, g, b, 255)
        end

      data = ImageCodecBmp.encode(c)
      assert {:ok, c2} = ImageCodecBmp.decode(data)

      for x <- 0..2, y <- 0..2 do
        r = x * 80
        g = y * 80
        b = (x + y) * 40
        assert PixelContainer.pixel_at(c2, x, y) == {r, g, b, 255},
               "mismatch at (#{x}, #{y})"
      end
    end

    test "fill + round-trip preserves fill color" do
      c = PixelContainer.new(8, 6)
      c = PixelContainer.fill_pixels(c, 123, 45, 67, 200)
      data = ImageCodecBmp.encode(c)
      assert {:ok, c2} = ImageCodecBmp.decode(data)

      for x <- 0..7, y <- 0..5 do
        assert PixelContainer.pixel_at(c2, x, y) == {123, 45, 67, 200}
      end
    end

    test "decoded container has correct width and height" do
      c = PixelContainer.new(11, 7)
      data = ImageCodecBmp.encode(c)
      assert {:ok, c2} = ImageCodecBmp.decode(data)
      assert c2.width == 11
      assert c2.height == 7
    end

    test "transparent black image round-trips (all zeros)" do
      c = PixelContainer.new(4, 4)
      data = ImageCodecBmp.encode(c)
      assert {:ok, c2} = ImageCodecBmp.decode(data)

      for x <- 0..3, y <- 0..3 do
        assert PixelContainer.pixel_at(c2, x, y) == {0, 0, 0, 0}
      end
    end
  end

  # ─────────────────────────────────────────────────────────────────────────────
  # decode/1 — error handling
  # ─────────────────────────────────────────────────────────────────────────────

  describe "decode/1 error handling" do
    test "returns error for empty binary" do
      assert {:error, _} = ImageCodecBmp.decode(<<>>)
    end

    test "returns error for data that is too short" do
      assert {:error, _} = ImageCodecBmp.decode(<<0, 1, 2, 3>>)
    end

    test "returns error for data without BM signature" do
      bad = String.duplicate(<<0>>, 100)
      assert {:error, _} = ImageCodecBmp.decode(bad)
    end
  end
end
