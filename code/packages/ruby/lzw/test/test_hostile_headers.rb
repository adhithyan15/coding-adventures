# frozen_string_literal: true

# Security regression tests: hostile headers must not drive work or memory.
#
# Each case is a few bytes of input whose header claims an enormous count or
# length.  An implementation that looped or allocated from the claim would run
# for minutes or exhaust memory; these tests assert bounded time and sane
# results, plus round trips over every input of length <= 2 and a seeded random
# sample.

require "minitest/autorun"
require_relative "../lib/coding_adventures/lzw"

class TestHostileHeaders < Minitest::Test
  LIMIT_SECONDS = 10.0
  CODEC = CodingAdventures::LZW

  def quick
    start = Process.clock_gettime(Process::CLOCK_MONOTONIC)
    result = yield
    elapsed = Process.clock_gettime(Process::CLOCK_MONOTONIC) - start
    assert_operator elapsed, :<, LIMIT_SECONDS, "hostile input took #{elapsed}s"
    result
  end

  def bin(*bytes)
    bytes.flatten.pack("C*")
  end

  def u32(n)
    [n].pack("N").bytes
  end

  def decompress(data)
    CODEC.decompress(data).b
  end

  def test_max_header_only
    quick { decompress("\xFF".b * 4) }
    quick { decompress("\xFF".b * 64) }
  end

  def test_max_declared_length_on_small_stream
    c = CODEC.compress("hello hello hello").b
    c[0, 4] = bin(u32(0xFFFFFFFF))
    assert_equal "hello hello hello".b, quick { decompress(c) }
  end

  def test_declared_length_truncates_output
    c = CODEC.compress("hello hello hello").b
    c[0, 4] = bin(u32(5))
    assert_equal "hello".b, quick { decompress(c) }
  end


  def test_round_trip_all_inputs_up_to_length_2
    inputs = [""]
    256.times do |a|
      inputs << bin(a)
      256.times { |b| inputs << bin(a, b) }
    end
    inputs.each do |d|
      assert_equal d.b, decompress(CODEC.compress(d)), "round trip failed for #{d.bytes.inspect}"
    end
  end

  def test_round_trip_seeded_random_inputs
    rng = Random.new(0x5EED)
    alphabets = [2, 4, 16, 256]
    300.times do |i|
      d = bin(Array.new(3 + rng.rand(597)) { rng.rand(alphabets[i % 4]) })
      assert_equal d, decompress(CODEC.compress(d))
    end
  end
end
