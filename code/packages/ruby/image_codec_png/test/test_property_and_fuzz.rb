# frozen_string_literal: true

require_relative "test_helper"

class TestPropertyAndFuzz < Minitest::Test
  PRNG_SEED = 0x5EED_C0DE

  def test_prng_round_trip_properties
    rng = Random.new(PRNG_SEED)
    dimensions = [
      [1, 1],
      [1, 7],
      [7, 1],
      [2, 3],
      [13, 17],
      [25, 25],
      [64, 4]
    ]

    dimensions.each do |width, height|
      # Test pattern 1: Solid color
      container1 = create_container(width, height) { [200, 100, 50, 255] }
      assert_round_trip(container1)

      # Test pattern 2: Horizontal & vertical gradients (exercises Sub, Up, Average filters)
      container2 = create_container(width, height) do |x, y|
        r = (x * 255 / [width - 1, 1].max) & 0xFF
        g = (y * 255 / [height - 1, 1].max) & 0xFF
        b = ((x + y) * 128 / [width + height - 2, 1].max) & 0xFF
        [r, g, b, 255]
      end
      assert_round_trip(container2)

      # Test pattern 3: Diagonal checkerboard (exercises Paeth filter)
      container3 = create_container(width, height) do |x, y|
        v = ((x + y) % 2 == 0) ? 255 : 0
        [v, v, v, 255]
      end
      assert_round_trip(container3)

      # Test pattern 4: Pseudo-random noise and varying alpha
      container4 = create_container(width, height) do
        [rng.rand(256), rng.rand(256), rng.rand(256), rng.rand(256)]
      end
      assert_round_trip(container4)
    end
  end

  def test_adversarial_mutation_fuzz_loop
    rng = Random.new(PRNG_SEED)

    # Base seed PNGs
    seed1 = make_png(1, 1, [100, 150, 200, 255])
    seed2 = make_png(4, 4, [10, 20, 30, 40])
    seed3 = make_png(8, 2, [255, 0, 128, 255])
    seeds = [seed1, seed2, seed3]

    valid_codes = ImageCodecPng::PNG_ERROR_CODES.to_set

    10_000.times do |iter|
      base = seeds[iter % seeds.length]
      mutated = mutate(base, rng)

      begin
        result = ImageCodecPng.decode_png(mutated)
        assert_instance_of CodingAdventures::PixelContainer::Container, result
        assert_operator result.width, :>, 0
        assert_operator result.height, :>, 0
        assert_equal result.width * result.height * 4, result.data.bytesize
        assert_equal result.data, ImageCodecPng.decode_png(ImageCodecPng.encode_png(result)).data
      rescue ImageCodecPng::PngError => e
        assert_includes valid_codes, e.code, "Iteration #{iter} raised unknown code: #{e.code}"
      rescue => e
        flunk "Iteration #{iter} crashed with unexpected exception #{e.class}: #{e.message}\n#{e.backtrace.first(5).join("\n")}"
      end
    end
  end

  private

  def assert_round_trip(container)
    encoded = ImageCodecPng.encode_png(container)
    decoded = ImageCodecPng.decode_png(encoded)
    assert_equal container.width, decoded.width
    assert_equal container.height, decoded.height
    assert_equal container.data, decoded.data
  end

  def create_container(width, height)
    container = CodingAdventures::PixelContainer.create(width, height)
    height.times do |y|
      width.times do |x|
        r, g, b, a = yield(x, y)
        dest = (y * width + x) * 4
        container.data.setbyte(dest, r)
        container.data.setbyte(dest + 1, g)
        container.data.setbyte(dest + 2, b)
        container.data.setbyte(dest + 3, a)
      end
    end
    container
  end

  def make_png(width, height, rgba)
    container = create_container(width, height) { rgba }
    ImageCodecPng.encode_png(container)
  end

  def mutate(data, rng)
    bytes = data.b.dup
    len = bytes.bytesize
    mutation_type = rng.rand(8)

    case mutation_type
    when 0
      # Truncate at random position
      cut = rng.rand(len)
      bytes.byteslice(0, cut) || "".b
    when 1
      # Bit flip
      pos = rng.rand(len)
      bit = 1 << rng.rand(8)
      bytes.setbyte(pos, bytes.getbyte(pos) ^ bit)
      bytes
    when 2
      # Byte overwrite
      pos = rng.rand(len)
      bytes.setbyte(pos, rng.rand(256))
      bytes
    when 3
      # Byte insertion
      pos = rng.rand(len + 1)
      count = rng.rand(1..16)
      ins = rng.bytes(count)
      bytes[pos, 0] = ins
      bytes
    when 4
      # Byte deletion
      return bytes if len <= 1
      pos = rng.rand(len)
      count = [rng.rand(1..16), len - pos].min
      bytes.slice!(pos, count)
      bytes
    when 5
      # Length tampering in chunk header
      pos = rng.rand(len - 4)
      tampered_len = [0x7FFF_FFFF, 0xFFFF_FFFF, 0, 1, 0x1000].sample(random: rng)
      bytes[pos, 4] = [tampered_len].pack("N")
      bytes
    when 6
      # Type tampering
      pos = [8 + 4, 8 + 4 + 13 + 12 + 4].sample(random: rng)
      if pos + 4 <= len
        bytes[pos, 4] = rng.bytes(4)
      end
      bytes
    when 7
      # Random noise chunk insertion
      pos = rng.rand(len)
      chunk = [4].pack("N") + "tEXt".b + "fuzz".b + [0x12345678].pack("N")
      bytes[pos, 0] = chunk
      bytes
    end
  end
end
