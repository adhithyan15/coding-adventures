# frozen_string_literal: true

# Security regression tests: hostile headers must not drive work or memory.
#
# Each case is a few bytes of input whose header claims an enormous count or
# length.  An implementation that looped or allocated from the claim would run
# for minutes or exhaust memory; these tests assert bounded time and sane
# results, plus round trips over every input of length <= 2 and a seeded random
# sample.

require "minitest/autorun"
require "coding_adventures_lz77"

class TestHostileHeaders < Minitest::Test
  LIMIT_SECONDS = 10.0
  CODEC = CodingAdventures::LZ77

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

  def test_max_count_header_only
    assert_equal "".b, quick { decompress(bin(255, 255, 255, 255)) }
  end

  def test_count_larger_than_remaining_bytes_yields_present_tokens
    d = bin(255, 255, 255, 255, 0, 0, 0, 104, 0, 0, 0, 105)
    assert_equal "hi".b, quick { decompress(d) }
    assert_equal "hi".b, decompress(d + bin(0, 0, 0))
  end

  def test_count_smaller_than_remaining_bytes_stops_at_count
    assert_equal "h".b, decompress(bin(0, 0, 0, 1, 0, 0, 0, 104, 0, 0, 0, 105))
  end

  def test_invalid_offsets_do_not_read_nil
    assert_equal "".b, decompress(bin(0, 0, 0, 1, 0, 5, 3, 120))
    assert_equal "".b, decompress(bin(0, 0, 0, 1, 0, 0, 3, 120))
    assert_equal "a".b, decompress(bin(0, 0, 0, 2, 0, 0, 0, 97, 0, 9, 2, 122))
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
