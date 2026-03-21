"""Tests for the Sequential container."""

from ml_framework_core import Tensor

from ml_framework_torch.nn.activation import ReLU
from ml_framework_torch.nn.linear import Linear
from ml_framework_torch.nn.sequential import Sequential


class TestSequentialInit:
    def test_empty(self) -> None:
        s = Sequential()
        assert len(s) == 0

    def test_with_layers(self) -> None:
        s = Sequential(Linear(2, 3), ReLU(), Linear(3, 1))
        assert len(s) == 3

    def test_modules_registered(self) -> None:
        s = Sequential(Linear(2, 3), Linear(3, 1))
        assert "0" in s._modules
        assert "1" in s._modules


class TestSequentialForward:
    def test_forward_chain(self) -> None:
        s = Sequential(Linear(2, 3), ReLU(), Linear(3, 1))
        x = Tensor.randn(1, 2)
        y = s(x)
        assert y.shape == (1, 1)

    def test_single_layer(self) -> None:
        s = Sequential(ReLU())
        x = Tensor.from_list([-1.0, 0.0, 1.0])
        y = s(x)
        assert y.data == [0.0, 0.0, 1.0]


class TestSequentialParameters:
    def test_parameters(self) -> None:
        s = Sequential(Linear(2, 3), Linear(3, 1))
        params = list(s.parameters())
        # Linear(2,3): weight + bias, Linear(3,1): weight + bias
        assert len(params) == 4

    def test_named_parameters(self) -> None:
        s = Sequential(Linear(2, 3), Linear(3, 1))
        names = dict(s.named_parameters())
        assert "0.weight" in names
        assert "0.bias" in names
        assert "1.weight" in names
        assert "1.bias" in names


class TestSequentialAccessors:
    def test_getitem(self) -> None:
        layer1 = Linear(2, 3)
        layer2 = ReLU()
        s = Sequential(layer1, layer2)
        assert s[0] is layer1
        assert s[1] is layer2

    def test_len(self) -> None:
        s = Sequential(Linear(2, 3), Linear(3, 1))
        assert len(s) == 2


class TestSequentialRepr:
    def test_repr(self) -> None:
        s = Sequential(Linear(2, 3), ReLU())
        r = repr(s)
        assert "Sequential" in r
        assert "Linear" in r
        assert "ReLU" in r


class TestSequentialTrainEval:
    def test_train_eval_propagates(self) -> None:
        s = Sequential(Linear(2, 3), Linear(3, 1))
        s.eval()
        for module in s._modules.values():
            assert module.training is False
        s.train()
        for module in s._modules.values():
            assert module.training is True
