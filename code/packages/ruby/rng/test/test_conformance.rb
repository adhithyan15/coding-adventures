# frozen_string_literal: true

# test_conformance.rb — the shared rng corpus (spec rng.md, "Conformance
# corpus").
#
# code/specs/fixtures/rng-v1/cases.json pins exact outputs every language's
# rng package must reproduce; its results come from the independent reference
# code/scripts/rng_conformance.py. Integers in the corpus are decimal strings,
# so they reach Ruby's Integer exactly.
require_relative "test_helper"
require "json"

class TestRngConformance < Minitest::Test
  CORPUS = File.expand_path("../../../../specs/fixtures/rng-v1/cases.json", __dir__)
  GENERATORS = {
    "lcg" => CodingAdventures::Rng::LCG,
    "xorshift64" => CodingAdventures::Rng::Xorshift64,
    "pcg32" => CodingAdventures::Rng::PCG32
  }.freeze
  I64_MIN = -(1 << 63)
  I64_MAX = (1 << 63) - 1

  def float_bits(x)
    [x].pack("G").unpack1("Q>").to_s(16).rjust(16, "0")
  end

  def test_every_corpus_case
    cases = JSON.parse(File.read(CORPUS))["cases"]
    assert_operator cases.length, :>=, 27
    cases.each do |c|
      g = GENERATORS.fetch(c["generator"]).new(Integer(c["seed"]))
      c["calls"].each_with_index do |call, index|
        at = "#{c["id"]} call #{index} (#{call["op"]})"
        case call["op"]
        when "next_u32" then assert_equal Integer(call["result"]), g.next_u32, at
        when "next_u64" then assert_equal Integer(call["result"]), g.next_u64, at
        when "next_float" then assert_equal call["result_bits"], float_bits(g.next_float), at
        when "next_int_in_range"
          min = Integer(call["min"])
          max = Integer(call["max"])
          if call.key?("error")
            # Raised before any draw, so the sequence carries on.
            assert_raises(ArgumentError, at) { g.next_int_in_range(min, max) }
          else
            assert_equal Integer(call["result"]), g.next_int_in_range(min, max), at
          end
        else flunk "#{at}: unknown op"
        end
      end
    end
  end

  # ── Regressions for the v1 range bug ───────────────────────────────────────

  def test_wide_ranges_reach_beyond_two_to_the_32
    g = CodingAdventures::Rng::PCG32.new(3)
    assert(64.times.any? { g.next_int_in_range(0, 1 << 40) >= 1 << 32 })
  end

  def test_the_whole_signed_64_bit_domain
    g = CodingAdventures::Rng::LCG.new(3)
    values = Array.new(64) { g.next_int_in_range(I64_MIN, I64_MAX) }
    assert values.all? { |v| v.between?(I64_MIN, I64_MAX) }
    assert(values.any?(&:negative?) && values.any?(&:positive?))
  end

  def test_invalid_bounds_raise_and_draw_nothing
    [[1, 0], [I64_MIN - 1, 0], [0, I64_MAX + 1], [0.5, 2]].each do |min, max|
      g = CodingAdventures::Rng::Xorshift64.new(9)
      fresh = CodingAdventures::Rng::Xorshift64.new(9)
      assert_raises(ArgumentError) { g.next_int_in_range(min, max) }
      assert_equal fresh.next_u32, g.next_u32
    end
  end
end
