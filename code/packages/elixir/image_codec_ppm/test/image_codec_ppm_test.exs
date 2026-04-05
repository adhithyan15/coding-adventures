defmodule CodingAdventures.ImageCodecPpmTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.{PixelContainer, ImageCodecPpm}

  # ─────────────────────────────────────────────────────────────────────────────
  # mime_type/0
  # ─────────────────────────────────────────────────────────────────────────────

  describe "mime_type/0" do
    test "returns image/x-portable-pixmap" do
      assert ImageCodecPpm.mime_type() == "image/x-portable-pixmap"
    end
  end

  # ─────────────────────────────────────────────────────────────────────────────
  # encode/1 — header structure
  # ─────────────────────────────────────────────────────────────────────────────

  describe "encode/1 header structure" do
    test "output starts with P6 magic line" do
      c = PixelContainer.new(4, 4)
      data = ImageCodecPpm.encode(c)
      assert String.starts_with?(data, "P6\n")
    end

    test "header contains width and height on second line" do
      c = PixelContainer.new(13, 7)
      data = ImageCodecPpm.encode(c)
      lines = String.split(data, "\n", parts: 4)
      assert Enum.at(lines, 1) == "13 7"
    end

    test "header contains maxval 255 on third line" do
      c = PixelContainer.new(2, 2)
      data = ImageCodecPpm.encode(c)
      lines = String.split(data, "\n", parts: 4)
      assert Enum.at(lines, 2) == "255"
    end

    test "pixel data section is width * height * 3 bytes" do
      w = 5
      h = 4
      c = PixelContainer.new(w, h)
      data = ImageCodecPpm.encode(c)
      # Header = "P6\n5 4\n255\n" = 12 bytes
      header = "P6\n#{w} #{h}\n255\n"
      pixel_data = binary_part(data, byte_size(header), byte_size(data) - byte_size(header))
      assert byte_size(pixel_data) == w * h * 3
    end
  end

  # ─────────────────────────────────────────────────────────────────────────────
  # encode + decode round-trip
  # ─────────────────────────────────────────────────────────────────────────────

  describe "encode/decode round-trip" do
    test "red pixel round-trips (alpha dropped, restored to 255)" do
      c = PixelContainer.new(1, 1)
      c = PixelContainer.set_pixel(c, 0, 0, 255, 0, 0, 200)
      data = ImageCodecPpm.encode(c)
      assert {:ok, c2} = ImageCodecPpm.decode(data)
      # Alpha is not stored in PPM; decode always sets it to 255
      assert PixelContainer.pixel_at(c2, 0, 0) == {255, 0, 0, 255}
    end

    test "green pixel round-trips" do
      c = PixelContainer.new(1, 1)
      c = PixelContainer.set_pixel(c, 0, 0, 0, 200, 0, 255)
      data = ImageCodecPpm.encode(c)
      assert {:ok, c2} = ImageCodecPpm.decode(data)
      assert PixelContainer.pixel_at(c2, 0, 0) == {0, 200, 0, 255}
    end

    test "blue pixel round-trips" do
      c = PixelContainer.new(1, 1)
      c = PixelContainer.set_pixel(c, 0, 0, 0, 0, 180, 255)
      data = ImageCodecPpm.encode(c)
      assert {:ok, c2} = ImageCodecPpm.decode(data)
      assert PixelContainer.pixel_at(c2, 0, 0) == {0, 0, 180, 255}
    end

    test "all pixels in 4x4 grid round-trip correctly" do
      c = PixelContainer.new(4, 4)

      c =
        for x <- 0..3, y <- 0..3, reduce: c do
          acc ->
            r = x * 60
            g = y * 60
            b = (x + y) * 30
            PixelContainer.set_pixel(acc, x, y, r, g, b, 255)
        end

      data = ImageCodecPpm.encode(c)
      assert {:ok, c2} = ImageCodecPpm.decode(data)

      for x <- 0..3, y <- 0..3 do
        r = x * 60
        g = y * 60
        b = (x + y) * 30
        assert PixelContainer.pixel_at(c2, x, y) == {r, g, b, 255},
               "mismatch at (#{x}, #{y})"
      end
    end

    test "decoded container has correct dimensions" do
      c = PixelContainer.new(15, 9)
      data = ImageCodecPpm.encode(c)
      assert {:ok, c2} = ImageCodecPpm.decode(data)
      assert c2.width == 15
      assert c2.height == 9
    end

    test "filled image round-trips correctly" do
      c = PixelContainer.new(5, 5)
      c = PixelContainer.fill_pixels(c, 100, 150, 200, 255)
      data = ImageCodecPpm.encode(c)
      assert {:ok, c2} = ImageCodecPpm.decode(data)

      for x <- 0..4, y <- 0..4 do
        assert PixelContainer.pixel_at(c2, x, y) == {100, 150, 200, 255}
      end
    end

    test "black image round-trips correctly" do
      c = PixelContainer.new(3, 3)
      data = ImageCodecPpm.encode(c)
      assert {:ok, c2} = ImageCodecPpm.decode(data)

      for x <- 0..2, y <- 0..2 do
        # Transparent black encodes as black (R=0, G=0, B=0), decoded as opaque black
        assert PixelContainer.pixel_at(c2, x, y) == {0, 0, 0, 255}
      end
    end
  end

  # ─────────────────────────────────────────────────────────────────────────────
  # decode/1 — comment line handling
  # ─────────────────────────────────────────────────────────────────────────────

  describe "decode/1 comment handling" do
    test "skips comment lines before dimensions" do
      ppm = "P6\n# This is a comment\n2 2\n255\n#{String.duplicate(<<0>>, 12)}"
      assert {:ok, c} = ImageCodecPpm.decode(ppm)
      assert c.width == 2
      assert c.height == 2
    end

    test "skips multiple comment lines" do
      ppm = "P6\n# comment 1\n# comment 2\n# comment 3\n1 1\n255\n#{<<128, 64, 32>>}"
      assert {:ok, c} = ImageCodecPpm.decode(ppm)
      assert PixelContainer.pixel_at(c, 0, 0) == {128, 64, 32, 255}
    end

    test "skips comment between dimensions and maxval" do
      ppm = "P6\n3 3\n# a comment here\n255\n#{String.duplicate(<<0>>, 27)}"
      assert {:ok, c} = ImageCodecPpm.decode(ppm)
      assert c.width == 3
    end
  end

  # ─────────────────────────────────────────────────────────────────────────────
  # decode/1 — error handling
  # ─────────────────────────────────────────────────────────────────────────────

  describe "decode/1 error handling" do
    test "returns error for empty binary" do
      assert {:error, _} = ImageCodecPpm.decode(<<>>)
    end

    test "returns error for wrong magic number" do
      assert {:error, _} = ImageCodecPpm.decode("P3\n1 1\n255\n#{<<255, 0, 0>>}")
    end

    test "returns error for truncated pixel data" do
      # 2x2 needs 12 bytes, we only provide 6
      ppm = "P6\n2 2\n255\n#{String.duplicate(<<0>>, 6)}"
      assert {:error, _} = ImageCodecPpm.decode(ppm)
    end

    test "returns error when only magic line is present" do
      assert {:error, _} = ImageCodecPpm.decode("P6\n")
    end
  end
end
