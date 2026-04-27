# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_feature_normalization"

class FeatureNormalizationTest < Minitest::Test
  ROWS = [
    [1000.0, 3.0, 1.0],
    [1500.0, 4.0, 0.0],
    [2000.0, 5.0, 1.0]
  ].freeze

  def test_standard_scaler_centers_and_scales_columns
    scaler = CodingAdventures::FeatureNormalization.fit_standard_scaler(ROWS)
    assert_in_delta 1500.0, scaler.means[0], 1e-9
    assert_in_delta 4.0, scaler.means[1], 1e-9

    transformed = CodingAdventures::FeatureNormalization.transform_standard(ROWS, scaler)
    assert_in_delta(-1.224744871391589, transformed[0][0], 1e-9)
    assert_in_delta 0.0, transformed[1][0], 1e-9
    assert_in_delta 1.224744871391589, transformed[2][0], 1e-9
  end

  def test_min_max_scaler_maps_to_unit_range
    transformed = CodingAdventures::FeatureNormalization.transform_min_max(
      ROWS,
      CodingAdventures::FeatureNormalization.fit_min_max_scaler(ROWS)
    )

    assert_equal [[0.0, 0.0, 1.0], [0.5, 0.5, 0.0], [1.0, 1.0, 1.0]], transformed
  end

  def test_constant_columns_map_to_zero
    rows = [[1.0, 7.0], [2.0, 7.0]]

    standard = CodingAdventures::FeatureNormalization.transform_standard(
      rows,
      CodingAdventures::FeatureNormalization.fit_standard_scaler(rows)
    )
    min_max = CodingAdventures::FeatureNormalization.transform_min_max(
      rows,
      CodingAdventures::FeatureNormalization.fit_min_max_scaler(rows)
    )

    assert_equal 0.0, standard[0][1]
    assert_equal 0.0, min_max[0][1]
  end
end
