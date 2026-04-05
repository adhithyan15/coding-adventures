defmodule MatrixTest do
  use ExUnit.Case

  # ── Base operations ──────────────────────────────────────────────

  test "zeros extraction bounds" do
    z = Matrix.zeros(2, 3)
    assert z.rows == 2
    assert z.cols == 3
  end

  test "add and subtract arrays" do
    a = Matrix.new([[1.0, 2.0], [3.0, 4.0]])
    b = Matrix.new([[5.0, 6.0], [7.0, 8.0]])
    c = Matrix.add(a, b)
    assert c.data == [[6.0, 8.0], [10.0, 12.0]]
    d = Matrix.subtract(b, a)
    assert d.data == [[4.0, 4.0], [4.0, 4.0]]
    e = Matrix.add_scalar(a, 2.0)
    assert e.data == [[3.0, 4.0], [5.0, 6.0]]
  end

  test "transpose arrays" do
    a = Matrix.new([[1.0, 2.0, 3.0], [4.0, 5.0, 6.0]])
    c = Matrix.transpose(a)
    assert c.data == [[1.0, 4.0], [2.0, 5.0], [3.0, 6.0]]
  end

  test "dot product mappings natively handled" do
    a = Matrix.new([[1.0, 2.0], [3.0, 4.0]])
    b = Matrix.new([[5.0, 6.0], [7.0, 8.0]])
    c = Matrix.dot(a, b)
    assert c.data == [[19.0, 22.0], [43.0, 50.0]]

    d = Matrix.new([1.0, 2.0, 3.0])
    e = Matrix.new([[4.0], [5.0], [6.0]])
    f = Matrix.dot(d, e)
    assert f.data == [[32.0]]
  end

  # ── Factory methods ─────────────────────────────────────────────

  test "identity matrix" do
    i3 = Matrix.identity(3)
    assert i3.rows == 3
    assert i3.cols == 3
    assert i3.data == [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
  end

  test "identity(n).dot(M) == M" do
    m = Matrix.new([[1.0, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.0]])
    i3 = Matrix.identity(3)
    result = Matrix.dot(i3, m)
    assert Matrix.equals(result, m)
  end

  test "from_diagonal" do
    d = Matrix.from_diagonal([2, 3])
    assert d.data == [[2.0, 0.0], [0.0, 3.0]]
  end

  test "identity(1) is [[1.0]]" do
    i1 = Matrix.identity(1)
    assert i1.data == [[1.0]]
  end

  # ── Element access ──────────────────────────────────────────────

  test "get element" do
    m = Matrix.new([[1.0, 2.0], [3.0, 4.0]])
    assert Matrix.get(m, 0, 0) == 1.0
    assert Matrix.get(m, 1, 1) == 4.0
  end

  test "get out of bounds raises" do
    m = Matrix.new([[1.0, 2.0], [3.0, 4.0]])
    assert_raise RuntimeError, fn -> Matrix.get(m, 2, 0) end
    assert_raise RuntimeError, fn -> Matrix.get(m, 0, 2) end
  end

  test "set returns new matrix" do
    m = Matrix.new([[1.0, 2.0], [3.0, 4.0]])
    m2 = Matrix.set(m, 0, 0, 99.0)
    assert Matrix.get(m2, 0, 0) == 99.0
    assert Matrix.get(m, 0, 0) == 1.0
  end

  test "set out of bounds raises" do
    m = Matrix.new([[1.0, 2.0], [3.0, 4.0]])
    assert_raise RuntimeError, fn -> Matrix.set(m, 5, 0, 1.0) end
  end

  # ── Reductions ──────────────────────────────────────────────────

  test "sum of [[1,2],[3,4]] is 10" do
    m = Matrix.new([[1.0, 2.0], [3.0, 4.0]])
    assert Matrix.sum(m) == 10.0
  end

  test "mean of [[1,2],[3,4]] is 2.5" do
    m = Matrix.new([[1.0, 2.0], [3.0, 4.0]])
    assert Matrix.mean(m) == 2.5
  end

  test "sum_rows of [[1,2],[3,4]] is [[3],[7]]" do
    m = Matrix.new([[1.0, 2.0], [3.0, 4.0]])
    sr = Matrix.sum_rows(m)
    assert sr.data == [[3.0], [7.0]]
    assert sr.rows == 2
    assert sr.cols == 1
  end

  test "sum_cols of [[1,2],[3,4]] is [[4,6]]" do
    m = Matrix.new([[1.0, 2.0], [3.0, 4.0]])
    sc = Matrix.sum_cols(m)
    assert sc.data == [[4.0, 6.0]]
    assert sc.rows == 1
    assert sc.cols == 2
  end

  test "min and max" do
    m = Matrix.new([[1.0, 2.0], [3.0, 4.0]])
    assert Matrix.min_val(m) == 1.0
    assert Matrix.max_val(m) == 4.0
  end

  test "argmin and argmax" do
    m = Matrix.new([[1.0, 2.0], [3.0, 4.0]])
    assert Matrix.argmin(m) == {0, 0}
    assert Matrix.argmax(m) == {1, 1}
  end

  test "argmin/argmax ties return first occurrence" do
    t = Matrix.new([[5.0, 5.0], [5.0, 5.0]])
    assert Matrix.argmin(t) == {0, 0}
    assert Matrix.argmax(t) == {0, 0}
  end

  test "reductions on larger matrix" do
    m = Matrix.new([[1.0, 2.0, 3.0], [4.0, 5.0, 6.0]])
    assert Matrix.sum(m) == 21.0
    assert Matrix.mean(m) == 3.5
    assert Matrix.sum_rows(m).data == [[6.0], [15.0]]
    assert Matrix.sum_cols(m).data == [[5.0, 7.0, 9.0]]
  end

  # ── Element-wise math ──────────────────────────────────────────

  test "map_elements doubles elements" do
    m = Matrix.new([[1.0, 2.0], [3.0, 4.0]])
    doubled = Matrix.map_elements(m, &(&1 * 2))
    assert doubled.data == [[2.0, 4.0], [6.0, 8.0]]
  end

  test "sqrt of perfect squares" do
    m = Matrix.new([[1.0, 4.0], [9.0, 16.0]])
    s = Matrix.matrix_sqrt(m)
    assert s.data == [[1.0, 2.0], [3.0, 4.0]]
  end

  test "abs of negative values" do
    m = Matrix.new([[-1.0, 2.0], [-3.0, 4.0]])
    a = Matrix.matrix_abs(m)
    assert a.data == [[1.0, 2.0], [3.0, 4.0]]
  end

  test "pow squares elements" do
    m = Matrix.new([[1.0, 2.0], [3.0, 4.0]])
    p = Matrix.matrix_pow(m, 2)
    assert p.data == [[1.0, 4.0], [9.0, 16.0]]
  end

  test "M.close(M.sqrt().pow(2), 1e-9) is true" do
    m = Matrix.new([[1.0, 2.0], [3.0, 4.0]])
    roundtrip = Matrix.matrix_sqrt(m) |> Matrix.matrix_pow(2)
    assert Matrix.close(m, roundtrip, 1.0e-9)
  end

  # ── Shape operations ───────────────────────────────────────────

  test "flatten produces row vector" do
    m = Matrix.new([[1.0, 2.0], [3.0, 4.0]])
    f = Matrix.flatten(m)
    assert f.rows == 1
    assert f.cols == 4
    assert f.data == [[1.0, 2.0, 3.0, 4.0]]
  end

  test "flatten then reshape roundtrip" do
    m = Matrix.new([[1.0, 2.0], [3.0, 4.0]])
    roundtrip = Matrix.flatten(m) |> Matrix.reshape(m.rows, m.cols)
    assert Matrix.equals(roundtrip, m)
  end

  test "reshape changes dimensions" do
    flat = Matrix.new([[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]])
    reshaped = Matrix.reshape(flat, 2, 3)
    assert reshaped.data == [[1.0, 2.0, 3.0], [4.0, 5.0, 6.0]]
    assert reshaped.rows == 2
    assert reshaped.cols == 3
  end

  test "reshape with invalid dimensions raises" do
    m = Matrix.new([[1.0, 2.0], [3.0, 4.0]])
    assert_raise RuntimeError, fn -> Matrix.reshape(m, 3, 3) end
  end

  test "get_row extracts single row" do
    m = Matrix.new([[1.0, 2.0], [3.0, 4.0]])
    assert Matrix.get_row(m, 0).data == [[1.0, 2.0]]
    assert Matrix.get_row(m, 1).data == [[3.0, 4.0]]
  end

  test "get_row out of bounds raises" do
    m = Matrix.new([[1.0, 2.0], [3.0, 4.0]])
    assert_raise RuntimeError, fn -> Matrix.get_row(m, 2) end
  end

  test "get_col extracts single column" do
    m = Matrix.new([[1.0, 2.0], [3.0, 4.0]])
    assert Matrix.get_col(m, 0).data == [[1.0], [3.0]]
    assert Matrix.get_col(m, 1).data == [[2.0], [4.0]]
  end

  test "get_col out of bounds raises" do
    m = Matrix.new([[1.0, 2.0], [3.0, 4.0]])
    assert_raise RuntimeError, fn -> Matrix.get_col(m, 2) end
  end

  test "slice extracts sub-matrix" do
    m = Matrix.new([[1.0, 2.0], [3.0, 4.0]])
    s = Matrix.matrix_slice(m, 0, 2, 0, 1)
    assert s.data == [[1.0], [3.0]]
  end

  test "slice on larger matrix" do
    m = Matrix.new([[1.0, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.0]])
    s = Matrix.matrix_slice(m, 0, 2, 1, 3)
    assert s.data == [[2.0, 3.0], [5.0, 6.0]]
  end

  test "slice with invalid bounds raises" do
    m = Matrix.new([[1.0, 2.0], [3.0, 4.0]])
    assert_raise RuntimeError, fn -> Matrix.matrix_slice(m, 0, 3, 0, 1) end
    assert_raise RuntimeError, fn -> Matrix.matrix_slice(m, 1, 0, 0, 1) end
  end

  # ── Equality ────────────────────────────────────────────────────

  test "equals returns true for identical matrices" do
    a = Matrix.new([[1.0, 2.0], [3.0, 4.0]])
    b = Matrix.new([[1.0, 2.0], [3.0, 4.0]])
    assert Matrix.equals(a, b)
  end

  test "equals returns false for different values" do
    a = Matrix.new([[1.0, 2.0], [3.0, 4.0]])
    b = Matrix.new([[1.0, 2.0], [3.0, 5.0]])
    refute Matrix.equals(a, b)
  end

  test "equals returns false for different shapes" do
    a = Matrix.new([[1.0, 2.0], [3.0, 4.0]])
    b = Matrix.new([[1.0, 2.0, 3.0]])
    refute Matrix.equals(a, b)
  end

  test "close with tolerance handles floating point" do
    a = Matrix.new([[1.0000000001]])
    b = Matrix.new([[1.0]])
    assert Matrix.close(a, b, 1.0e-9)
  end

  test "close returns false when outside tolerance" do
    a = Matrix.new([[1.1]])
    b = Matrix.new([[1.0]])
    refute Matrix.close(a, b, 0.01)
  end

  test "close returns false for different shapes" do
    a = Matrix.new([[1.0]])
    b = Matrix.new([[1.0, 2.0]])
    refute Matrix.close(a, b)
  end
end
