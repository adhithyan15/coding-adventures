"""Tests for the Module base class."""

from ml_framework_core import Parameter, Tensor

from ml_framework_torch.nn.module import Module


class DummyModule(Module):
    """A simple module for testing."""

    def __init__(self, in_f: int, out_f: int) -> None:
        super().__init__()
        self.weight = Parameter(Tensor.randn(out_f, in_f))
        self.bias = Parameter(Tensor.zeros(out_f))

    def forward(self, x: Tensor) -> Tensor:
        return x


class NestedModule(Module):
    """Module with child modules."""

    def __init__(self) -> None:
        super().__init__()
        self.layer1 = DummyModule(2, 3)
        self.layer2 = DummyModule(3, 4)

    def forward(self, x: Tensor) -> Tensor:
        return x


class TestModuleInit:
    def test_empty_module(self) -> None:
        m = Module()
        assert m.training is True
        assert len(m._parameters) == 0
        assert len(m._modules) == 0

    def test_parameter_registration(self) -> None:
        m = DummyModule(5, 3)
        assert "weight" in m._parameters
        assert "bias" in m._parameters
        assert m._parameters["weight"] is m.weight
        assert m._parameters["bias"] is m.bias

    def test_module_registration(self) -> None:
        m = NestedModule()
        assert "layer1" in m._modules
        assert "layer2" in m._modules

    def test_non_param_non_module_not_registered(self) -> None:
        m = Module()
        m.some_int = 42
        assert "some_int" not in m._parameters
        assert "some_int" not in m._modules
        assert m.some_int == 42


class TestModuleParameters:
    def test_parameters_yields_all(self) -> None:
        m = DummyModule(2, 3)
        params = list(m.parameters())
        assert len(params) == 2

    def test_parameters_recursive(self) -> None:
        m = NestedModule()
        params = list(m.parameters())
        # layer1 has 2, layer2 has 2
        assert len(params) == 4

    def test_named_parameters(self) -> None:
        m = DummyModule(2, 3)
        names = dict(m.named_parameters())
        assert "weight" in names
        assert "bias" in names

    def test_named_parameters_nested(self) -> None:
        m = NestedModule()
        names = dict(m.named_parameters())
        assert "layer1.weight" in names
        assert "layer1.bias" in names
        assert "layer2.weight" in names
        assert "layer2.bias" in names

    def test_named_modules(self) -> None:
        m = NestedModule()
        modules = dict(m.named_modules())
        assert "" in modules  # root
        assert "layer1" in modules
        assert "layer2" in modules


class TestModuleTrainEval:
    def test_train_mode(self) -> None:
        m = NestedModule()
        m.eval()
        assert m.training is False
        assert m.layer1.training is False
        m.train()
        assert m.training is True
        assert m.layer1.training is True

    def test_eval_mode(self) -> None:
        m = Module()
        result = m.eval()
        assert m.training is False
        assert result is m  # returns self


class TestModuleZeroGrad:
    def test_zero_grad(self) -> None:
        m = DummyModule(2, 3)
        # Simulate gradients
        for p in m.parameters():
            p.grad = Tensor.ones(*p.shape)
        m.zero_grad()
        for p in m.parameters():
            assert p.grad is None


class TestModuleStateDict:
    def test_state_dict(self) -> None:
        m = DummyModule(2, 3)
        state = m.state_dict()
        assert "weight" in state
        assert "bias" in state

    def test_load_state_dict(self) -> None:
        m1 = DummyModule(2, 3)
        m2 = DummyModule(2, 3)
        state = m1.state_dict()
        m2.load_state_dict(state)
        # Check data matches
        for a, b in zip(m1.weight.data, m2.weight.data):
            assert a == b

    def test_nested_state_dict(self) -> None:
        m = NestedModule()
        state = m.state_dict()
        assert "layer1.weight" in state
        assert "layer2.bias" in state


class TestModuleForward:
    def test_forward_not_implemented(self) -> None:
        m = Module()
        try:
            m(Tensor.zeros(1))
            assert False, "Should have raised"
        except NotImplementedError:
            pass

    def test_call_delegates_to_forward(self) -> None:
        m = DummyModule(2, 3)
        x = Tensor.zeros(2)
        result = m(x)
        assert result is x


class TestModuleTo:
    def test_to_device(self) -> None:
        m = DummyModule(2, 3)
        m.to("cuda")
        assert m.weight.device == "cuda"
        assert m.bias.device == "cuda"

    def test_to_device_nested(self) -> None:
        m = NestedModule()
        m.to("metal")
        for p in m.parameters():
            assert p.device == "metal"


class TestModuleRepr:
    def test_repr_leaf(self) -> None:
        m = Module()
        assert "Module()" in repr(m)

    def test_repr_nested(self) -> None:
        m = NestedModule()
        r = repr(m)
        assert "NestedModule" in r
        assert "layer1" in r
        assert "layer2" in r
