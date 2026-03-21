"""Tests for tf.Variable — mutable tensor with name and trainability."""

from ml_framework_tf.variable import Variable
from ml_framework_core import Tensor


class TestVariableCreation:
    """Test Variable construction from various input types."""

    def test_from_tensor(self):
        t = Tensor.from_list([1.0, 2.0, 3.0])
        v = Variable(t)
        assert v.data == [1.0, 2.0, 3.0]
        assert v.shape == (3,)

    def test_from_list(self):
        v = Variable([4.0, 5.0, 6.0])
        assert v.data == [4.0, 5.0, 6.0]
        assert v.shape == (3,)

    def test_from_nested_list(self):
        v = Variable([[1.0, 2.0], [3.0, 4.0]])
        assert v.shape == (2, 2)
        assert v.data == [1.0, 2.0, 3.0, 4.0]

    def test_from_scalar(self):
        v = Variable(42.0)
        assert v.data == [42.0]
        assert v.shape == (1,)

    def test_trainable_default(self):
        v = Variable([1.0])
        assert v.trainable is True
        assert v.requires_grad is True

    def test_non_trainable(self):
        v = Variable([1.0], trainable=False)
        assert v.trainable is False
        assert v.requires_grad is False

    def test_custom_name(self):
        v = Variable([1.0], name="my_weight")
        assert v.name == "my_weight"

    def test_auto_name(self):
        v = Variable([1.0])
        assert v.name.startswith("Variable:")

    def test_repr(self):
        v = Variable([1.0, 2.0], name="w")
        r = repr(v)
        assert "tf.Variable" in r
        assert "'w'" in r


class TestVariableMutation:
    """Test in-place mutation methods."""

    def test_assign_tensor(self):
        v = Variable([1.0, 2.0])
        v.assign(Tensor.from_list([10.0, 20.0]))
        assert v.data == [10.0, 20.0]

    def test_assign_list(self):
        v = Variable([1.0, 2.0])
        v.assign([10.0, 20.0])
        assert v.data == [10.0, 20.0]

    def test_assign_scalar(self):
        v = Variable([1.0])
        v.assign(5.0)
        assert v.data == [5.0]

    def test_assign_add_tensor(self):
        v = Variable([1.0, 2.0, 3.0])
        v.assign_add(Tensor.from_list([0.1, 0.2, 0.3]))
        assert abs(v.data[0] - 1.1) < 1e-6
        assert abs(v.data[1] - 2.2) < 1e-6
        assert abs(v.data[2] - 3.3) < 1e-6

    def test_assign_add_list(self):
        v = Variable([1.0, 2.0])
        v.assign_add([10.0, 10.0])
        assert v.data == [11.0, 12.0]

    def test_assign_add_scalar(self):
        v = Variable([1.0, 2.0])
        v.assign_add(5.0)
        assert v.data == [6.0, 7.0]

    def test_assign_sub_tensor(self):
        v = Variable([10.0, 20.0])
        v.assign_sub(Tensor.from_list([1.0, 2.0]))
        assert v.data == [9.0, 18.0]

    def test_assign_sub_list(self):
        v = Variable([10.0, 20.0])
        v.assign_sub([1.0, 2.0])
        assert v.data == [9.0, 18.0]

    def test_assign_sub_scalar(self):
        v = Variable([10.0, 20.0])
        v.assign_sub(5.0)
        assert v.data == [5.0, 15.0]


class TestVariableArithmetic:
    """Test that Variables work with tensor arithmetic."""

    def test_add(self):
        v = Variable([1.0, 2.0, 3.0])
        result = v + 1.0
        assert abs(result.data[0] - 2.0) < 1e-6

    def test_mul(self):
        v = Variable([2.0, 3.0])
        result = v * 2.0
        assert abs(result.data[0] - 4.0) < 1e-6
        assert abs(result.data[1] - 6.0) < 1e-6
