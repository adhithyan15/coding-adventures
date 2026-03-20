# frozen_string_literal: true

require_relative "test_helper"

# ================================================================
# Tests for BLAS data types: Vector, Matrix, and enumerations.
# ================================================================

class TestVector < Minitest::Test
  include CodingAdventures::BlasLibrary

  def test_vector_creation
    v = Vector.new(data: [1.0, 2.0, 3.0], size: 3)
    assert_equal 3, v.size
    assert_equal [1.0, 2.0, 3.0], v.data
  end

  def test_vector_single_element
    v = Vector.new(data: [42.0], size: 1)
    assert_equal 1, v.size
    assert_equal 42.0, v.data[0]
  end

  def test_vector_size_mismatch_raises
    assert_raises(ArgumentError) do
      Vector.new(data: [1.0, 2.0], size: 3)
    end
  end

  def test_vector_empty_raises
    assert_raises(ArgumentError) do
      Vector.new(data: [], size: 1)
    end
  end

  def test_vector_zero_size
    v = Vector.new(data: [], size: 0)
    assert_equal 0, v.size
    assert_empty v.data
  end

  def test_vector_negative_values
    v = Vector.new(data: [-1.0, -2.5, 0.0], size: 3)
    assert_equal [-1.0, -2.5, 0.0], v.data
  end

  def test_vector_large
    data = Array.new(100) { |i| i.to_f }
    v = Vector.new(data: data, size: 100)
    assert_equal 100, v.size
    assert_equal 99.0, v.data[99]
  end
end

class TestMatrix < Minitest::Test
  include CodingAdventures::BlasLibrary

  def test_matrix_creation
    m = Matrix.new(data: [1.0, 2.0, 3.0, 4.0, 5.0, 6.0], rows: 2, cols: 3)
    assert_equal 2, m.rows
    assert_equal 3, m.cols
    assert_equal 6, m.data.length
  end

  def test_matrix_default_order
    m = Matrix.new(data: [1.0, 2.0, 3.0, 4.0], rows: 2, cols: 2)
    assert_equal StorageOrder::ROW_MAJOR, m.order
  end

  def test_matrix_column_major
    m = Matrix.new(
      data: [1.0, 2.0, 3.0, 4.0],
      rows: 2,
      cols: 2,
      order: StorageOrder::COLUMN_MAJOR
    )
    assert_equal StorageOrder::COLUMN_MAJOR, m.order
  end

  def test_matrix_size_mismatch_raises
    assert_raises(ArgumentError) do
      Matrix.new(data: [1.0, 2.0, 3.0], rows: 2, cols: 2)
    end
  end

  def test_matrix_1x1
    m = Matrix.new(data: [5.0], rows: 1, cols: 1)
    assert_equal 1, m.rows
    assert_equal 1, m.cols
    assert_equal 5.0, m.data[0]
  end

  def test_matrix_square
    m = Matrix.new(data: [1.0, 0.0, 0.0, 1.0], rows: 2, cols: 2)
    assert_equal 2, m.rows
    assert_equal 2, m.cols
  end

  def test_matrix_wide
    m = Matrix.new(data: [1.0, 2.0, 3.0, 4.0, 5.0, 6.0], rows: 1, cols: 6)
    assert_equal 1, m.rows
    assert_equal 6, m.cols
  end

  def test_matrix_tall
    m = Matrix.new(data: [1.0, 2.0, 3.0, 4.0, 5.0, 6.0], rows: 6, cols: 1)
    assert_equal 6, m.rows
    assert_equal 1, m.cols
  end
end

class TestEnumerations < Minitest::Test
  include CodingAdventures::BlasLibrary

  def test_storage_order_values
    assert_equal "row_major", StorageOrder::ROW_MAJOR
    assert_equal "column_major", StorageOrder::COLUMN_MAJOR
  end

  def test_transpose_values
    assert_equal "no_trans", Transpose::NO_TRANS
    assert_equal "trans", Transpose::TRANS
  end

  def test_side_values
    assert_equal "left", Side::LEFT
    assert_equal "right", Side::RIGHT
  end
end
