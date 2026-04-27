import pytest
from matrix import Matrix

def test_zeros():
    z = Matrix.zeros(2, 3)
    assert z.rows == 2
    assert z.cols == 3
    assert z.data[1][2] == 0.0

def test_add_subtract():
    A = Matrix([[1.0, 2.0], [3.0, 4.0]])
    B = Matrix([[5.0, 6.0], [7.0, 8.0]])
    assert (A + B).data == [[6.0, 8.0], [10.0, 12.0]]
    assert (B - A).data == [[4.0, 4.0], [4.0, 4.0]]
    
    # Test scalar broadcasting
    assert (A + 2.0).data == [[3.0, 4.0], [5.0, 6.0]]
    assert (A - 1.0).data == [[0.0, 1.0], [2.0, 3.0]]
    
    with pytest.raises(ValueError):
        A + Matrix([[1.0]])

def test_scale():
    A = Matrix([[1.0, 2.0], [3.0, 4.0]])
    assert (A * 2.0).data == [[2.0, 4.0], [6.0, 8.0]]

def test_transpose():
    A = Matrix([[1.0, 2.0, 3.0], [4.0, 5.0, 6.0]])
    assert A.transpose().data == [[1.0, 4.0], [2.0, 5.0], [3.0, 6.0]]
    assert Matrix([]).transpose().data == []

def test_dot():
    A = Matrix([[1.0, 2.0], [3.0, 4.0]])
    B = Matrix([[5.0, 6.0], [7.0, 8.0]])
    assert A.dot(B).data == [[19.0, 22.0], [43.0, 50.0]]
    
    C = Matrix([1.0, 2.0, 3.0])
    D = Matrix([[4.0], [5.0], [6.0]])
    assert C.dot(D).data == [[32.0]]
    
    with pytest.raises(ValueError):
        A.dot(D)
