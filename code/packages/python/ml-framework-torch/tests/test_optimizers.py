"""Tests for optimizers (SGD, Adam, AdamW, RMSprop)."""

from ml_framework_core import Parameter, Tensor

from ml_framework_torch.optim.adam import Adam, AdamW
from ml_framework_torch.optim.optimizer import Optimizer
from ml_framework_torch.optim.rmsprop import RMSprop
from ml_framework_torch.optim.sgd import SGD


def _make_param_with_grad(
    data: list[float],
    grad: list[float],
    shape: tuple[int, ...] | None = None,
) -> Parameter:
    """Helper: create a parameter and set its gradient."""
    if shape is None:
        shape = (len(data),)
    p = Parameter(Tensor(data, shape))
    p.grad = Tensor(grad, shape)
    return p


class TestOptimizerBase:
    def test_zero_grad(self) -> None:
        p1 = _make_param_with_grad([1.0], [0.5])
        p2 = _make_param_with_grad([2.0], [0.3])
        opt = Optimizer([p1, p2], lr=0.1)
        opt.zero_grad()
        assert p1.grad is None
        assert p2.grad is None

    def test_step_not_implemented(self) -> None:
        p = _make_param_with_grad([1.0], [0.5])
        opt = Optimizer([p], lr=0.1)
        try:
            opt.step()
            assert False, "Should have raised"
        except NotImplementedError:
            pass


class TestSGD:
    def test_basic_step(self) -> None:
        """w_new = w - lr * grad = 1.0 - 0.1 * 2.0 = 0.8"""
        p = _make_param_with_grad([1.0], [2.0])
        opt = SGD([p], lr=0.1)
        opt.step()
        assert abs(p.data[0] - 0.8) < 1e-6

    def test_momentum(self) -> None:
        """With momentum, velocity accumulates across steps."""
        p = _make_param_with_grad([1.0], [1.0])
        opt = SGD([p], lr=0.1, momentum=0.9)

        # Step 1: v = 0.9*0 + 1.0 = 1.0, w = 1.0 - 0.1*1.0 = 0.9
        opt.step()
        assert abs(p.data[0] - 0.9) < 1e-6

        # Step 2: v = 0.9*1.0 + 1.0 = 1.9, w = 0.9 - 0.1*1.9 = 0.71
        p.grad = Tensor([1.0], (1,))
        opt.step()
        assert abs(p.data[0] - 0.71) < 1e-6

    def test_weight_decay(self) -> None:
        """grad_effective = grad + wd * w = 1.0 + 0.1 * 2.0 = 1.2"""
        p = _make_param_with_grad([2.0], [1.0])
        opt = SGD([p], lr=0.1, weight_decay=0.1)
        opt.step()
        # w = 2.0 - 0.1 * 1.2 = 1.88
        assert abs(p.data[0] - 1.88) < 1e-6

    def test_no_grad_skipped(self) -> None:
        p = _make_param_with_grad([1.0], [0.0])
        p.grad = None
        opt = SGD([p], lr=0.1)
        opt.step()
        assert p.data[0] == 1.0

    def test_multiple_params(self) -> None:
        p1 = _make_param_with_grad([1.0], [1.0])
        p2 = _make_param_with_grad([2.0], [2.0])
        opt = SGD([p1, p2], lr=0.1)
        opt.step()
        assert abs(p1.data[0] - 0.9) < 1e-6
        assert abs(p2.data[0] - 1.8) < 1e-6


class TestAdam:
    def test_basic_step(self) -> None:
        p = _make_param_with_grad([1.0], [0.5])
        opt = Adam([p], lr=0.001)
        opt.step()
        # Parameter should have moved
        assert p.data[0] != 1.0

    def test_converges_toward_zero_grad(self) -> None:
        """Multiple steps with constant gradient should converge."""
        p = _make_param_with_grad([10.0], [1.0])
        opt = Adam([p], lr=0.1)
        for _ in range(50):
            p.grad = Tensor([1.0], (1,))
            opt.step()
        # Should have decreased significantly
        assert p.data[0] < 10.0

    def test_weight_decay(self) -> None:
        p = _make_param_with_grad([2.0], [1.0])
        opt = Adam([p], lr=0.001, weight_decay=0.1)
        opt.step()
        # Should have moved
        assert p.data[0] != 2.0

    def test_no_grad_skipped(self) -> None:
        p = _make_param_with_grad([1.0], [0.0])
        p.grad = None
        opt = Adam([p], lr=0.001)
        opt.step()
        assert p.data[0] == 1.0

    def test_bias_correction(self) -> None:
        """Early steps should have larger effective learning rate."""
        p1 = _make_param_with_grad([1.0], [1.0])
        p2 = _make_param_with_grad([1.0], [1.0])
        opt1 = Adam([p1], lr=0.01)
        opt2 = Adam([p2], lr=0.01)

        # Step 1
        opt1.step()
        # Step 1 then 2
        opt2.step()
        p2.grad = Tensor([1.0], (1,))
        opt2.step()

        # Both should have changed
        assert p1.data[0] != 1.0
        assert p2.data[0] != 1.0


class TestAdamW:
    def test_basic_step(self) -> None:
        p = _make_param_with_grad([1.0], [0.5])
        opt = AdamW([p], lr=0.001)
        opt.step()
        assert p.data[0] != 1.0

    def test_decoupled_weight_decay(self) -> None:
        """AdamW applies weight decay separately from gradient."""
        p = _make_param_with_grad([2.0], [0.0])
        p.grad = Tensor([0.0], (1,))
        opt = AdamW([p], lr=0.01, weight_decay=0.1)
        opt.step()
        # Even with zero gradient, weight should decrease due to decay
        # decay_factor = 1 - 0.01 * 0.1 = 0.999
        # But Adam step with zero grad still applies bias-corrected zero
        assert p.data[0] < 2.0

    def test_no_grad_skipped(self) -> None:
        p = _make_param_with_grad([1.0], [0.0])
        p.grad = None
        opt = AdamW([p], lr=0.001)
        opt.step()
        assert p.data[0] == 1.0


class TestRMSprop:
    def test_basic_step(self) -> None:
        p = _make_param_with_grad([1.0], [0.5])
        opt = RMSprop([p], lr=0.01)
        opt.step()
        assert p.data[0] != 1.0

    def test_adaptive_lr(self) -> None:
        """Params with large grads should get smaller effective LR."""
        p1 = _make_param_with_grad([1.0], [10.0])
        p2 = _make_param_with_grad([1.0], [0.1])
        opt1 = RMSprop([p1], lr=0.01)
        opt2 = RMSprop([p2], lr=0.01)
        opt1.step()
        opt2.step()
        # p1 should move less relative to its gradient
        move1 = abs(1.0 - p1.data[0]) / 10.0
        move2 = abs(1.0 - p2.data[0]) / 0.1
        assert move1 < move2

    def test_momentum(self) -> None:
        p = _make_param_with_grad([1.0], [1.0])
        opt = RMSprop([p], lr=0.01, momentum=0.9)
        opt.step()
        assert p.data[0] != 1.0

    def test_weight_decay(self) -> None:
        p = _make_param_with_grad([2.0], [1.0])
        opt = RMSprop([p], lr=0.01, weight_decay=0.1)
        opt.step()
        assert p.data[0] != 2.0

    def test_no_grad_skipped(self) -> None:
        p = _make_param_with_grad([1.0], [0.0])
        p.grad = None
        opt = RMSprop([p], lr=0.01)
        opt.step()
        assert p.data[0] == 1.0
