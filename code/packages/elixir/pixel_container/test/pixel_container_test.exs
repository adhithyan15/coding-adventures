defmodule CodingAdventures.PixelContainerTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.PixelContainer, as: PC

  # ─────────────────────────────────────────────────────────────────────────────
  # new/2
  # ─────────────────────────────────────────────────────────────────────────────

  describe "new/2" do
    test "creates a container with the correct width and height" do
      c = PC.new(10, 20)
      assert c.width == 10
      assert c.height == 20
    end

    test "buffer has correct byte size: width * height * 4" do
      c = PC.new(8, 6)
      assert byte_size(c.data) == 8 * 6 * 4
    end

    test "all bytes are zero initially (transparent black)" do
      c = PC.new(4, 4)
      # A binary of all zeros equals a binary filled with zero bytes
      assert c.data == :binary.copy(<<0, 0, 0, 0>>, 4 * 4)
    end

    test "1x1 container has 4 bytes" do
      c = PC.new(1, 1)
      assert byte_size(c.data) == 4
    end

    test "0x0 container has 0 bytes (degenerate but valid)" do
      c = PC.new(0, 0)
      assert byte_size(c.data) == 0
    end

    test "large container has correct size" do
      c = PC.new(1920, 1080)
      assert byte_size(c.data) == 1920 * 1080 * 4
    end
  end

  # ─────────────────────────────────────────────────────────────────────────────
  # pixel_at/3
  # ─────────────────────────────────────────────────────────────────────────────

  describe "pixel_at/3" do
    test "all pixels in a new container are {0, 0, 0, 0}" do
      c = PC.new(5, 5)

      for x <- 0..4, y <- 0..4 do
        assert PC.pixel_at(c, x, y) == {0, 0, 0, 0},
               "expected {0,0,0,0} at (#{x}, #{y})"
      end
    end

    test "returns {0, 0, 0, 0} for negative x" do
      c = PC.new(5, 5)
      assert PC.pixel_at(c, -1, 0) == {0, 0, 0, 0}
    end

    test "returns {0, 0, 0, 0} for negative y" do
      c = PC.new(5, 5)
      assert PC.pixel_at(c, 0, -1) == {0, 0, 0, 0}
    end

    test "returns {0, 0, 0, 0} for x equal to width (off-by-one boundary)" do
      c = PC.new(5, 5)
      assert PC.pixel_at(c, 5, 0) == {0, 0, 0, 0}
    end

    test "returns {0, 0, 0, 0} for y equal to height (off-by-one boundary)" do
      c = PC.new(5, 5)
      assert PC.pixel_at(c, 0, 5) == {0, 0, 0, 0}
    end

    test "returns {0, 0, 0, 0} for x far out of bounds" do
      c = PC.new(5, 5)
      assert PC.pixel_at(c, 100, 100) == {0, 0, 0, 0}
    end
  end

  # ─────────────────────────────────────────────────────────────────────────────
  # set_pixel/7
  # ─────────────────────────────────────────────────────────────────────────────

  describe "set_pixel/7" do
    test "sets pixel at (0, 0) correctly" do
      c = PC.new(5, 5)
      c = PC.set_pixel(c, 0, 0, 255, 128, 64, 255)
      assert PC.pixel_at(c, 0, 0) == {255, 128, 64, 255}
    end

    test "sets pixel at bottom-right corner correctly" do
      c = PC.new(5, 5)
      c = PC.set_pixel(c, 4, 4, 1, 2, 3, 4)
      assert PC.pixel_at(c, 4, 4) == {1, 2, 3, 4}
    end

    test "setting one pixel does not affect neighbors" do
      c = PC.new(5, 5)
      c = PC.set_pixel(c, 2, 2, 200, 100, 50, 255)
      # Immediate neighbors should remain untouched
      assert PC.pixel_at(c, 1, 2) == {0, 0, 0, 0}
      assert PC.pixel_at(c, 3, 2) == {0, 0, 0, 0}
      assert PC.pixel_at(c, 2, 1) == {0, 0, 0, 0}
      assert PC.pixel_at(c, 2, 3) == {0, 0, 0, 0}
    end

    test "can overwrite a pixel multiple times" do
      c = PC.new(3, 3)
      c = PC.set_pixel(c, 1, 1, 10, 20, 30, 40)
      assert PC.pixel_at(c, 1, 1) == {10, 20, 30, 40}
      c = PC.set_pixel(c, 1, 1, 99, 88, 77, 66)
      assert PC.pixel_at(c, 1, 1) == {99, 88, 77, 66}
    end

    test "out-of-bounds set returns original container unchanged" do
      c = PC.new(3, 3)
      c2 = PC.set_pixel(c, 100, 100, 255, 0, 0, 255)
      assert c2 == c
    end

    test "negative x is no-op" do
      c = PC.new(3, 3)
      c2 = PC.set_pixel(c, -1, 0, 255, 0, 0, 255)
      assert c2 == c
    end

    test "x == width is no-op (off-by-one guard)" do
      c = PC.new(3, 3)
      c2 = PC.set_pixel(c, 3, 0, 255, 0, 0, 255)
      assert c2 == c
    end

    test "can set all 4 channels to max (255)" do
      c = PC.new(2, 2)
      c = PC.set_pixel(c, 0, 0, 255, 255, 255, 255)
      assert PC.pixel_at(c, 0, 0) == {255, 255, 255, 255}
    end

    test "can set all 4 channels to zero (transparent black)" do
      c = PC.new(2, 2)
      c = PC.set_pixel(c, 0, 0, 255, 255, 255, 255)
      c = PC.set_pixel(c, 0, 0, 0, 0, 0, 0)
      assert PC.pixel_at(c, 0, 0) == {0, 0, 0, 0}
    end
  end

  # ─────────────────────────────────────────────────────────────────────────────
  # fill_pixels/5
  # ─────────────────────────────────────────────────────────────────────────────

  describe "fill_pixels/5" do
    test "fills all pixels with the given color" do
      c = PC.new(4, 4)
      c = PC.fill_pixels(c, 255, 0, 128, 200)

      for x <- 0..3, y <- 0..3 do
        assert PC.pixel_at(c, x, y) == {255, 0, 128, 200},
               "expected fill color at (#{x}, #{y})"
      end
    end

    test "fill with white produces all-255 data" do
      c = PC.new(3, 3)
      c = PC.fill_pixels(c, 255, 255, 255, 255)
      assert c.data == :binary.copy(<<255, 255, 255, 255>>, 9)
    end

    test "fill with transparent black produces all-zero data" do
      c = PC.new(3, 3)
      c = PC.fill_pixels(c, 0, 0, 0, 0)
      assert c.data == :binary.copy(<<0, 0, 0, 0>>, 9)
    end

    test "fill preserves width and height" do
      c = PC.new(7, 5)
      c = PC.fill_pixels(c, 100, 100, 100, 255)
      assert c.width == 7
      assert c.height == 5
    end

    test "fill overwrites previously set pixels" do
      c = PC.new(3, 3)
      c = PC.set_pixel(c, 1, 1, 255, 0, 0, 255)
      c = PC.fill_pixels(c, 0, 255, 0, 255)
      assert PC.pixel_at(c, 1, 1) == {0, 255, 0, 255}
    end
  end

  # ─────────────────────────────────────────────────────────────────────────────
  # byte_size/1
  # ─────────────────────────────────────────────────────────────────────────────

  describe "byte_size/1" do
    test "returns width * height * 4" do
      c = PC.new(6, 7)
      assert PC.byte_size(c) == 6 * 7 * 4
    end

    test "matches byte_size of the data binary directly" do
      c = PC.new(10, 10)
      assert PC.byte_size(c) == byte_size(c.data)
    end
  end

  # ─────────────────────────────────────────────────────────────────────────────
  # Round-trip: set then read many pixels
  # ─────────────────────────────────────────────────────────────────────────────

  describe "round-trip set/get" do
    test "can independently set and read every pixel in a 4x4 grid" do
      c = PC.new(4, 4)

      # Use (x + y * 16) as a unique value per pixel for each channel offset
      c =
        for x <- 0..3, y <- 0..3, reduce: c do
          acc ->
            r = rem(x * 17 + y * 31, 256)
            g = rem(x * 53 + y * 7, 256)
            b = rem(x * 97 + y * 13, 256)
            a = 255
            PC.set_pixel(acc, x, y, r, g, b, a)
        end

      for x <- 0..3, y <- 0..3 do
        r = rem(x * 17 + y * 31, 256)
        g = rem(x * 53 + y * 7, 256)
        b = rem(x * 97 + y * 13, 256)
        assert PC.pixel_at(c, x, y) == {r, g, b, 255}
      end
    end
  end
end
