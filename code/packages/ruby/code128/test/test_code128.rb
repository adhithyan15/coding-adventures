# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_code128"

class TestCode128 < Minitest::Test
  def test_version_exists
    refute_nil CodingAdventures::Code128::VERSION
  end

  def test_compute_checksum_matches_reference
    values = "Code 128".chars.map { |char| CodingAdventures::Code128.value_for_code128_b_char(char) }
    assert_equal 64, CodingAdventures::Code128.compute_code128_checksum(values)
  end

  def test_normalize_rejects_non_printable_characters
    assert_raises(ArgumentError) { CodingAdventures::Code128.normalize_code128_b("HELLO\n") }
  end

  def test_encode_code128_includes_start_checksum_and_stop
    encoded = CodingAdventures::Code128.encode_code128_b("Code 128")
    assert_equal "start", encoded.first[:role]
    assert_equal "check", encoded[-2][:role]
    assert_equal "stop", encoded.last[:role]
  end

  def test_paint_scene
    scene = CodingAdventures::Code128.draw_code128("Code 128")
    assert_equal "code128", scene.metadata[:symbology]
    assert_equal "B", scene.metadata[:code_set]
    assert_operator scene.width, :>, 0
    assert_equal 120, scene.height
  end
end
