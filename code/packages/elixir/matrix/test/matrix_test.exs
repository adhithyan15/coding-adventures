defmodule MatrixTest do
  use ExUnit.Case

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
end
