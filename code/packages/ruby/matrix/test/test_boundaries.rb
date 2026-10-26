# frozen_string_literal: true

require_relative "test_helper"

# These tests preserve the current arithmetic/ownership contract; they do not
# turn this small foundation into a validated neural model boundary.
class MatrixBoundariesTest < Minitest::Test
  def test_constructor_forms_and_exposed_array_ownership
    assert_equal [[2.5]], Matrix.new(2.5).data
    assert_equal [[1.0, 2.0]], Matrix.new([1, 2]).data
    assert_equal [], Matrix.new(Object.new).data
    cells = [[1.0]]
    matrix = Matrix.new(cells)
    cells[0][0] = 7.0
    assert_equal 7.0, matrix.get(0, 0) # Deliberately retained existing alias.
  end

  def test_empty_reductions_and_transpose
    empty = Matrix.new([])
    assert_equal [], empty.transpose.data
    assert_equal 0.0, empty.sum
    %i[mean min max argmin argmax].each do |operation|
      assert_raises(ArgumentError) { empty.public_send(operation) }
    end
  end

  def test_scalar_subtraction_and_dimension_errors
    matrix = Matrix.new([[1.0, 2.0]])
    assert_equal [[0.0, 1.0]], (matrix - 1).data
    incompatible = Matrix.new([[1.0], [2.0]])
    assert_raises(ArgumentError) { matrix + incompatible }
    assert_raises(ArgumentError) { matrix - incompatible }
    assert_raises(ArgumentError) { matrix.dot(matrix) }
    refute matrix.close(incompatible)
  end

  def test_index_and_slice_boundaries
    matrix = Matrix.new([[1.0, 2.0], [3.0, 4.0]])
    assert_raises(IndexError) { matrix.get(0, -1) }
    assert_raises(IndexError) { matrix.set(-1, 0, 1) }
    assert_raises(IndexError) { matrix.row(-1) }
    assert_raises(IndexError) { matrix.col(-1) }
    assert_raises(IndexError) { matrix.slice(-1, 1, 0, 1) }
    assert_raises(IndexError) { matrix.slice(0, 1, 0, 3) }
    assert_raises(ArgumentError) { matrix.slice(1, 1, 0, 1) }
    assert_raises(ArgumentError) { matrix.slice(0, 1, 1, 0) }
  end

  def test_argmin_tie_and_negative_square_root
    assert_equal [0, 0], Matrix.new([[1, 2], [3, 1]]).argmin
    assert_raises(Math::DomainError) { Matrix.new([[-1.0]]).sqrt }
  end
end
