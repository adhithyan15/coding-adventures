# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_barcode_layout_1d"

class TestBarcodeLayout1D < Minitest::Test
  def test_runs_from_binary_pattern
    runs = CodingAdventures::BarcodeLayout1D.runs_from_binary_pattern("111001")
    assert_equal %w[bar space bar], runs.map { |run| run[:color] }
    assert_equal [3, 2, 1], runs.map { |run| run[:modules] }
  end

  def test_layout_barcode_1d
    runs = CodingAdventures::BarcodeLayout1D.runs_from_width_pattern(
      "WNW",
      %w[bar space bar],
      source_char: "A",
      source_index: 0,
    )
    scene = CodingAdventures::BarcodeLayout1D.layout_barcode_1d(runs)
    assert_equal 27, scene.width / 4
    assert_equal 120, scene.height
    assert_equal 2, scene.instructions.length
  end
end
