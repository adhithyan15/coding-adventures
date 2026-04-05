require "minitest/autorun"
require_relative "../lib/matrix_ml"

class MatrixTest < Minitest::Test
  # ====================================================================
  # Existing base tests
  # ====================================================================

  def test_zeros
    z = Matrix.zeros(2, 3)
    assert_equal 2, z.rows
    assert_equal 3, z.cols
    assert_equal 0.0, z.data[1][2]
  end

  def test_add_subtract
    a = Matrix.new([[1.0, 2.0], [3.0, 4.0]])
    b = Matrix.new([[5.0, 6.0], [7.0, 8.0]])
    assert_equal [[6.0, 8.0], [10.0, 12.0]], (a + b).data
    assert_equal [[4.0, 4.0], [4.0, 4.0]], (b - a).data
    assert_equal [[3.0, 4.0], [5.0, 6.0]], (a + 2.0).data
  end

  def test_scale
    a = Matrix.new([[1.0, 2.0], [3.0, 4.0]])
    assert_equal [[2.0, 4.0], [6.0, 8.0]], (a * 2.0).data
  end

  def test_transpose
    a = Matrix.new([[1.0, 2.0, 3.0], [4.0, 5.0, 6.0]])
    assert_equal [[1.0, 4.0], [2.0, 5.0], [3.0, 6.0]], a.transpose.data
  end

  def test_dot
    a = Matrix.new([[1.0, 2.0], [3.0, 4.0]])
    b = Matrix.new([[5.0, 6.0], [7.0, 8.0]])
    assert_equal [[19.0, 22.0], [43.0, 50.0]], a.dot(b).data

    c = Matrix.new([1.0, 2.0, 3.0])
    d = Matrix.new([[4.0], [5.0], [6.0]])
    assert_equal [[32.0]], c.dot(d).data
  end

  # ====================================================================
  # Element access tests
  # ====================================================================

  def test_get
    m = Matrix.new([[1.0, 2.0], [3.0, 4.0]])
    assert_equal 1.0, m.get(0, 0)
    assert_equal 2.0, m.get(0, 1)
    assert_equal 3.0, m.get(1, 0)
    assert_equal 4.0, m.get(1, 1)
  end

  def test_get_out_of_bounds
    m = Matrix.new([[1.0, 2.0], [3.0, 4.0]])
    assert_raises(IndexError) { m.get(2, 0) }
    assert_raises(IndexError) { m.get(0, 2) }
    assert_raises(IndexError) { m.get(-1, 0) }
  end

  def test_set
    m = Matrix.new([[1.0, 2.0], [3.0, 4.0]])
    n = m.set(0, 0, 99.0)
    # Original unchanged (immutability)
    assert_equal 1.0, m.get(0, 0)
    assert_equal 99.0, n.get(0, 0)
    assert_equal 4.0, n.get(1, 1)
  end

  def test_set_out_of_bounds
    m = Matrix.new([[1.0, 2.0], [3.0, 4.0]])
    assert_raises(IndexError) { m.set(2, 0, 5.0) }
  end

  # ====================================================================
  # Reduction tests
  # ====================================================================

  def test_sum
    m = Matrix.new([[1.0, 2.0], [3.0, 4.0]])
    assert_equal 10.0, m.sum
  end

  def test_sum_rows
    m = Matrix.new([[1.0, 2.0], [3.0, 4.0]])
    result = m.sum_rows
    assert_equal [[3.0], [7.0]], result.data
    assert_equal 2, result.rows
    assert_equal 1, result.cols
  end

  def test_sum_cols
    m = Matrix.new([[1.0, 2.0], [3.0, 4.0]])
    result = m.sum_cols
    assert_equal [[4.0, 6.0]], result.data
    assert_equal 1, result.rows
    assert_equal 2, result.cols
  end

  def test_mean
    m = Matrix.new([[1.0, 2.0], [3.0, 4.0]])
    assert_equal 2.5, m.mean
  end

  def test_min_max
    m = Matrix.new([[3.0, 1.0], [4.0, 2.0]])
    assert_equal 1.0, m.min
    assert_equal 4.0, m.max
  end

  def test_min_max_negative
    m = Matrix.new([[-5.0, 3.0], [0.0, -1.0]])
    assert_equal(-5.0, m.min)
    assert_equal 3.0, m.max
  end

  def test_argmin
    m = Matrix.new([[3.0, 1.0], [4.0, 2.0]])
    assert_equal [0, 1], m.argmin
  end

  def test_argmax
    m = Matrix.new([[1.0, 2.0], [3.0, 4.0]])
    assert_equal [1, 1], m.argmax
  end

  def test_argmax_first_occurrence
    m = Matrix.new([[4.0, 2.0], [3.0, 4.0]])
    assert_equal [0, 0], m.argmax
  end

  # ====================================================================
  # Element-wise math tests
  # ====================================================================

  def test_map_elements
    m = Matrix.new([[1.0, 4.0], [9.0, 16.0]])
    result = m.map_elements { |v| Math.sqrt(v) }
    assert_equal [[1.0, 2.0], [3.0, 4.0]], result.data
  end

  def test_sqrt
    m = Matrix.new([[1.0, 4.0], [9.0, 16.0]])
    assert_equal [[1.0, 2.0], [3.0, 4.0]], m.sqrt.data
  end

  def test_abs
    m = Matrix.new([[-1.0, 2.0], [-3.0, 4.0]])
    assert_equal [[1.0, 2.0], [3.0, 4.0]], m.abs.data
  end

  def test_pow
    m = Matrix.new([[1.0, 2.0], [3.0, 4.0]])
    assert_equal [[1.0, 4.0], [9.0, 16.0]], m.pow(2).data
  end

  def test_sqrt_pow_roundtrip
    m = Matrix.new([[1.0, 2.0], [3.0, 4.0]])
    assert m.close(m.sqrt.pow(2.0), 1e-9)
  end

  # ====================================================================
  # Shape operation tests
  # ====================================================================

  def test_flatten
    m = Matrix.new([[1.0, 2.0], [3.0, 4.0]])
    flat = m.flatten
    assert_equal [[1.0, 2.0, 3.0, 4.0]], flat.data
    assert_equal 1, flat.rows
    assert_equal 4, flat.cols
  end

  def test_reshape
    m = Matrix.new([[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]])
    result = m.reshape(2, 3)
    assert_equal [[1.0, 2.0, 3.0], [4.0, 5.0, 6.0]], result.data
  end

  def test_reshape_invalid
    m = Matrix.new([[1.0, 2.0], [3.0, 4.0]])
    assert_raises(ArgumentError) { m.reshape(3, 3) }
  end

  def test_flatten_reshape_roundtrip
    m = Matrix.new([[1.0, 2.0], [3.0, 4.0]])
    assert m.flatten.reshape(m.rows, m.cols).equals(m)
  end

  def test_row
    m = Matrix.new([[1.0, 2.0], [3.0, 4.0]])
    assert_equal [[1.0, 2.0]], m.row(0).data
    assert_equal [[3.0, 4.0]], m.row(1).data
  end

  def test_row_out_of_bounds
    m = Matrix.new([[1.0, 2.0], [3.0, 4.0]])
    assert_raises(IndexError) { m.row(2) }
  end

  def test_col
    m = Matrix.new([[1.0, 2.0], [3.0, 4.0]])
    assert_equal [[1.0], [3.0]], m.col(0).data
    assert_equal [[2.0], [4.0]], m.col(1).data
  end

  def test_col_out_of_bounds
    m = Matrix.new([[1.0, 2.0], [3.0, 4.0]])
    assert_raises(IndexError) { m.col(2) }
  end

  def test_slice
    m = Matrix.new([[1.0, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.0]])
    result = m.slice(0, 2, 1, 3)
    assert_equal [[2.0, 3.0], [5.0, 6.0]], result.data
  end

  def test_slice_single_column
    m = Matrix.new([[1.0, 2.0], [3.0, 4.0]])
    result = m.slice(0, 2, 0, 1)
    assert_equal [[1.0], [3.0]], result.data
  end

  def test_slice_out_of_bounds
    m = Matrix.new([[1.0, 2.0], [3.0, 4.0]])
    assert_raises(IndexError) { m.slice(0, 3, 0, 1) }
  end

  # ====================================================================
  # Equality tests
  # ====================================================================

  def test_equals
    a = Matrix.new([[1.0, 2.0], [3.0, 4.0]])
    b = Matrix.new([[1.0, 2.0], [3.0, 4.0]])
    c = Matrix.new([[1.0, 2.0], [3.0, 5.0]])
    assert a.equals(b)
    refute a.equals(c)
  end

  def test_equals_shape_mismatch
    a = Matrix.new([[1.0, 2.0]])
    b = Matrix.new([[1.0], [2.0]])
    refute a.equals(b)
  end

  def test_close
    a = Matrix.new([[1.0, 2.0]])
    b = Matrix.new([[1.0 + 1e-10, 2.0 - 1e-10]])
    assert a.close(b, 1e-9)
  end

  def test_close_fails
    a = Matrix.new([[1.0, 2.0]])
    b = Matrix.new([[1.1, 2.0]])
    refute a.close(b, 1e-9)
  end

  # ====================================================================
  # Factory method tests
  # ====================================================================

  def test_identity
    i = Matrix.identity(3)
    assert_equal 3, i.rows
    assert_equal 3, i.cols
    assert_equal [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]], i.data
  end

  def test_identity_dot
    m = Matrix.new([[1.0, 2.0], [3.0, 4.0], [5.0, 6.0]])
    i = Matrix.identity(3)
    assert i.dot(m).equals(m)
  end

  def test_from_diagonal
    d = Matrix.from_diagonal([2.0, 3.0])
    assert_equal [[2.0, 0.0], [0.0, 3.0]], d.data
  end

  def test_from_diagonal_single
    d = Matrix.from_diagonal([5.0])
    assert_equal [[5.0]], d.data
  end

  # ====================================================================
  # Parity test vectors (cross-language consistency)
  # ====================================================================

  def test_parity_sum_mean
    m = Matrix.new([[1.0, 2.0], [3.0, 4.0]])
    assert_equal 10.0, m.sum
    assert_equal 2.5, m.mean
  end

  def test_parity_sum_rows_cols
    m = Matrix.new([[1.0, 2.0], [3.0, 4.0]])
    assert_equal [[3.0], [7.0]], m.sum_rows.data
    assert_equal [[4.0, 6.0]], m.sum_cols.data
  end

  def test_parity_identity_dot
    m = Matrix.new([[1.0, 2.0], [3.0, 4.0], [5.0, 6.0]])
    assert Matrix.identity(3).dot(m).equals(m)
  end

  def test_parity_flatten_reshape
    m = Matrix.new([[1.0, 2.0], [3.0, 4.0]])
    assert m.flatten.reshape(m.rows, m.cols).equals(m)
  end

  def test_parity_close_sqrt_pow
    m = Matrix.new([[1.0, 2.0], [3.0, 4.0]])
    assert m.close(m.sqrt.pow(2.0), 1e-9)
  end
end
