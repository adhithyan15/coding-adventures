# frozen_string_literal: true

# Security regression tests: hostile headers must not drive work or memory.
#
# Each case is a few bytes of input whose header claims an enormous count or
# length.  An implementation that looped or allocated from the claim would run
# for minutes or exhaust memory; these tests assert bounded time and sane
# results, plus round trips over every input of length <= 2 and a seeded random
# sample.

require "minitest/autorun"
require "coding_adventures_lz78"

class TestHostileHeaders < Minitest::Test
  LIMIT_SECONDS = 10.0
  CODEC = CodingAdventures::LZ78

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

  def hdr(len, count)
    bin(u32(len), u32(count))
  end

  def test_max_header_only
    assert_equal "".b, quick { decompress("\xFF".b * 8) }
  end

  def test_max_length_zero_tokens
    assert_equal "".b, quick { decompress(hdr(0xFFFFFFFF, 0)) }
  end

  def test_count_larger_than_remaining_bytes_yields_present_tokens
    d = hdr(2, 0xFFFFFFFF) + bin(0, 0, 104, 0, 0, 0, 105, 0)
    assert_equal "hi".b, quick { decompress(d) }
  end

  def test_length_larger_than_tokens_supply
    d = hdr(0xFFFFFFFF, 1) + bin(0, 0, 104, 0)
    assert_equal "h".b, quick { decompress(d) }
  end

  def test_dangling_dict_index_does_not_raise
    d = hdr(10, 2) + bin(0, 0, 97, 0, 0x27, 0x0F, 98, 0)
    assert_equal "a".b, quick { decompress(d) }
  end

  def test_chain_tokens_stop_at_declared_length
    n = 20_000
    body = bin((0...n).flat_map { |i| [i >> 8, i & 255, 97, 0] })
    assert_equal 1000, quick { decompress(hdr(1000, n) + body) }.bytesize
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
