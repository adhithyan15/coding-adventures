require 'minitest/autorun'
require_relative '../lib/matrix_ml'

class MatrixTest < Minitest::Test
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
end
