# frozen_string_literal: true

# =============================================================================
# Bit-exact conformance of Trig against the shared TRIG-REF-1 vectors
# =============================================================================
#
# code/specs/fixtures/trig-reference-v1 holds every vector as one row
#
#     op <TAB> x <TAB> y <TAB> cr <TAB> flags
#
# where x and y are IEEE binary64 BIT PATTERNS (16 lowercase hex digits), never
# decimals.  This test decodes x by a bit cast, calls the public function,
# encodes the result back to bits and compares the two INTEGERS (PHY00a 6.5).
# Printed floats are never compared: Float#to_s hides nothing in Ruby, but the
# rule is the same in every language so the test follows it.  The one
# relaxation is NaN, which matches any NaN (VER04p "same").
#
# Bit casts in Ruby:
#
#   [bits].pack("Q>").unpack1("G")   # 64-bit integer -> Float
#   [float].pack("G").unpack1("Q>")  # Float -> 64-bit integer
#
# "Q>" is an unsigned 64-bit big-endian integer and "G" a big-endian double, so
# the two are a pure reinterpretation of the same eight bytes.

require "minitest/autorun"
require "json"
require "digest"
require_relative "../lib/trig"

class TestConformance < Minitest::Test
  FIXTURE = File.expand_path("../../../../specs/fixtures/trig-reference-v1", __dir__)
  GROUPS = %w[edge sweep-binade sweep-cw sweep-prng].freeze
  QNAN_BITS = 0x7FF8000000000000
  OPS = %w[sin cos tan radians degrees sqrt atan atan2].freeze

  # Accuracy limits in representable steps (ulps) against the `cr` column.
  # PHY00a 3.8 makes NO accuracy claim for tan, so it is named here as a
  # no-claim op rather than silently skipped.
  LIMITS = {"sin" => 3, "cos" => 3, "radians" => 1, "degrees" => 1}.freeze
  NO_CLAIM = %w[tan].freeze

  def self.bits_to_float(bits) = [bits].pack("Q>").unpack1("G")

  def self.float_to_bits(value) = [value].pack("G").unpack1("Q>")

  def self.load_rows
    GROUPS.flat_map do |group|
      text = File.read(File.join(FIXTURE, "#{group}.tsv"), encoding: "UTF-8")
      raise "#{group}.tsv does not end in a newline" unless text.end_with?("\n")

      text.split("\n").map do |line|
        op, x, y, cr, flags = line.split("\t", -1)
        {group: group, op: op, x: x, y: y, cr: cr, flags: flags}
      end
    end
  end

  ROWS = load_rows.freeze

  def bits_to_float(bits) = self.class.bits_to_float(bits)

  def float_to_bits(value) = self.class.float_to_bits(value)

  def call_op(op, x)
    if op == "atan2"
      ys, xs = x.split(",")
      Trig.atan2(bits_to_float(ys.to_i(16)), bits_to_float(xs.to_i(16)))
    else
      Trig.public_send(op, bits_to_float(x.to_i(16)))
    end
  end

  # ---------------------------------------------------------------------------
  # The fixture on disk is the one the manifest describes
  # ---------------------------------------------------------------------------

  def manifest
    @manifest ||= JSON.parse(File.read(File.join(FIXTURE, "manifest.json")))
  end

  def test_manifest_hashes_and_counts
    assert_equal "TRIG-REF-1", manifest["algorithm"]
    assert_equal GROUPS, manifest["groups"].map { |g| g["name"] }
    manifest["groups"].each do |g|
      data = File.binread(File.join(FIXTURE, g["file"]))
      assert_equal g["sha256"], Digest::SHA256.hexdigest(data), g["file"]
      assert_equal g["rows"], data.count("\n"), g["file"]
    end
  end

  def test_row_syntax
    hexdigit = /\A[0-9a-f]{16}\z/
    ROWS.each do |row|
      assert_includes OPS, row[:op], "unknown op #{row[:op].inspect}"
      parts = (row[:op] == "atan2") ? row[:x].split(",") : [row[:x]]
      assert_equal((row[:op] == "atan2") ? 2 : 1, parts.length)
      fields = parts.dup
      fields << row[:y] unless row[:y] == "error"
      fields << row[:cr] unless row[:cr] == "-"
      fields.each { |f| assert_match hexdigit, f }
    end
  end

  def test_constants_in_the_manifest_match_the_module
    consts = manifest["constants"].transform_values { |v| v.to_i(16) }
    assert_equal consts["PI"], float_to_bits(Trig::PI)
    assert_equal consts["TWO_PI"], float_to_bits(Trig::TWO_PI)
    assert_equal consts["HALF_PI"], float_to_bits(Trig::HALF_PI)
    assert_equal consts["INV_PIO2"], float_to_bits(Trig::INV_PIO2)
    Trig::PIO2.each_with_index do |piece, i|
      assert_equal consts["PIO2_#{i + 1}"], float_to_bits(piece), "PIO2_#{i + 1}"
    end
    assert_equal consts["MAGIC"], float_to_bits(Trig::MAGIC)
    assert_equal consts["LIMIT"], float_to_bits(Trig::LIMIT)
    assert_equal consts["TINY"], float_to_bits(Trig::TINY)
    (1..9).each { |n| assert_equal consts["S#{n}"], float_to_bits(Trig::S[n]), "S#{n}" }
    (2..9).each { |n| assert_equal consts["C#{n}"], float_to_bits(Trig::C[n]), "C#{n}" }
    assert_equal consts["RAD_PER_DEG"], float_to_bits(Trig::RAD_PER_DEG)
    assert_equal consts["DEG_PER_RAD"], float_to_bits(Trig::DEG_PER_RAD)
  end

  # Decimal literals written in the source must parse to the pinned bits
  # (PHY00a 6.5e): "the literal parses to the nearest double" is a property of
  # the interpreter, not of this package.
  def test_literal_bits
    {
      1e-15 => 0x3CD203AF9EE75616,
      1.0e308 => 0x7FE1CCF385EBC8A0,
      1e-17 => 0x3C670EF54646D497,
      1e-300 => 0x01A56E1FC2F8F359,
      7.450580596923828e-9 => 0x3E40000000000000
    }.each { |literal, bits| assert_equal bits, float_to_bits(literal) }
  end

  def test_degree_constants_equal_the_divisions
    assert_equal Trig::PI / 180.0, Trig::RAD_PER_DEG
    assert_equal 180.0 / Trig::PI, Trig::DEG_PER_RAD
  end

  # ---------------------------------------------------------------------------
  # Every vector, bit for bit
  # ---------------------------------------------------------------------------

  GROUPS.each do |group|
    define_method("test_every_vector_matches_bit_for_bit_#{group.tr("-", "_")}") do
      failures = Hash.new(0)
      examples = []
      checked = 0
      ROWS.each do |row|
        next unless row[:group] == group

        checked += 1
        op = row[:op]
        problem = nil
        if row[:y] == "error"
          begin
            call_op(op, row[:x])
            problem = "no error raised"
          rescue ArgumentError
            next
          end
        else
          got = call_op(op, row[:x])
          if row[:y].to_i(16) == QNAN_BITS
            problem = "expected NaN, got #{format("%016x", float_to_bits(got))}" unless got.nan?
          elsif float_to_bits(got) != row[:y].to_i(16)
            problem = "expected #{row[:y]}, got #{format("%016x", float_to_bits(got))}"
          end
        end
        next unless problem

        failures[op] += 1
        examples << [op, row[:x], problem] if examples.length < 5
      end
      assert_operator checked, :>, 0
      assert_empty failures, "failing rows per op: #{failures.inspect}; first: #{examples.inspect}"
    end
  end

  # ---------------------------------------------------------------------------
  # Accuracy against the correctly rounded column
  # ---------------------------------------------------------------------------

  # Map a bit pattern onto a signed integer line on which adjacent doubles
  # differ by exactly one, so |ordered(a) - ordered(b)| is the distance in steps.
  def ordered(bits)
    mag = bits & 0x7FFFFFFFFFFFFFFF
    (bits >> 63 == 1) ? -mag : mag
  end

  def test_accuracy_against_the_correctly_rounded_column
    worst = LIMITS.keys.to_h { |op| [op, 0] }
    ROWS.each do |row|
      op = row[:op]
      next if row[:cr] == "-" || NO_CLAIM.include?(op)

      # Fail closed: an op with a cr value but no limit is an error, so a
      # future op cannot be skipped silently.
      flunk "op #{op.inspect} has a cr value but no accuracy limit" unless LIMITS.key?(op)

      got = float_to_bits(call_op(op, row[:x]))
      steps = (ordered(got) - ordered(row[:cr].to_i(16))).abs
      worst[op] = steps if steps > worst[op]
    end
    LIMITS.each { |op, bound| assert_operator worst[op], :<=, bound, "#{op} worst=#{worst[op]}" }
  end

  def test_unknown_ops_fail
    assert_raises(NoMethodError) { Trig.public_send(:no_such_op, 1.0) }
    ROWS.each { |row| assert_includes OPS, row[:op] }
  end

  # ---------------------------------------------------------------------------
  # The behaviours PHY00a 3.3 names, asserted directly
  # ---------------------------------------------------------------------------

  def test_sin_of_negative_zero_is_negative_zero
    assert_equal 0x8000000000000000, float_to_bits(Trig.sin(-0.0))
    assert_equal 0, float_to_bits(Trig.sin(0.0))
  end

  def test_cos_of_zero_is_one
    assert_equal 1.0, Trig.cos(0.0)
    assert_equal 1.0, Trig.cos(-0.0)
  end

  def test_tiny_arguments_are_returned_unchanged
    tiny = bits_to_float(0x3E3FFFFFFFFFFFFF)
    assert_equal tiny, Trig.sin(tiny)
    assert_equal 5e-324, Trig.sin(5e-324)
    assert_equal 1.0, Trig.cos(tiny)
  end

  def test_outside_the_domain_is_nan
    [Float::NAN, Float::INFINITY, -Float::INFINITY, 1e20,
      2.0**29 * (1 + 2.0**-52), Float::MAX].each do |x|
      %i[sin cos tan].each { |fn| assert Trig.public_send(fn, x).nan?, "#{fn}(#{x})" }
    end
  end

  def test_the_last_in_domain_value_is_finite
    assert_equal 0x3FD4E67C0E2622DF, float_to_bits(Trig.sin(2.0**29))
    assert_equal 0xBFEE3EDD2DFEEF90, float_to_bits(Trig.cos(2.0**29))
  end

  # An Integer argument is converted once, with Integer#to_f, and must not be
  # promoted to Integer or Rational arithmetic anywhere inside the algorithm.
  def test_integer_arguments_are_converted_once
    assert_equal Trig.sin(1.0), Trig.sin(1)
    assert_equal Trig.cos(3.0), Trig.cos(3)
    assert_equal Trig::PI, Trig.radians(180)
    assert_equal 0.0, Trig.degrees(0)
    assert_kind_of Float, Trig.sin(0)
    assert_kind_of Float, Trig.cos(0)
    assert_kind_of Float, Trig.radians(0)
    assert_kind_of Float, Trig.degrees(0)
    assert_equal 0x8000000000000000, float_to_bits(Trig.radians(-0.0))
    assert_equal 0x8000000000000000, float_to_bits(Trig.degrees(-0.0))
  end

  def test_radians_and_degrees_are_single_multiplies
    assert_equal Trig::PI, Trig.radians(180.0)
    assert_equal 180.0, Trig.degrees(Trig::PI)
    assert_equal Float::INFINITY, Trig.degrees(1.7e308)
    assert Trig.radians(Float::NAN).nan?
  end

  def test_tan_pole_sentinel
    assert_equal 1e308, Trig.tan(Trig::HALF_PI)
    assert_equal(-1e308, Trig.tan(-Trig::HALF_PI))
  end

  def test_atan2_of_nan_is_nan
    nan = Float::NAN
    assert Trig.atan2(nan, 1.0).nan?
    assert Trig.atan2(1.0, nan).nan?
    assert Trig.atan2(-1.0, nan).nan?
    assert Trig.atan2(nan, nan).nan?
  end

  def test_atan2_pinned_conventions
    assert_equal Trig::PI, Trig.atan2(-0.0, -1.0)
    assert_equal 0.0, Trig.atan2(0.0, 0.0)
    assert Trig.atan2(Float::INFINITY, Float::INFINITY).nan?
    assert_equal Trig::HALF_PI, Trig.atan2(1e300, 1e-300)
  end

  def test_atan_keeps_the_sign_of_zero
    assert_equal 0x8000000000000000, float_to_bits(Trig.atan(-0.0))
  end

  def test_negative_quadrant_arithmetic
    # k = -1, -5 map to quadrant 3: "& 3" is the floor modulus.
    assert_equal(-Trig.sin(Trig::HALF_PI), Trig.sin(-Trig::HALF_PI))
    assert_in_delta 0.0, Trig.cos(-5 * Trig::HALF_PI), 1e-15
  end
end
