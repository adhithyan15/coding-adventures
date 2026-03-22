"""End-to-end integration tests: build, train, and verify."""

from ml_framework_core import Tensor

import ml_framework_torch as torch
from ml_framework_torch.nn import Linear, MSELoss, ReLU, Sequential
from ml_framework_torch.optim import SGD, Adam


class TestTopLevelAPI:
    """Test the torch.* top-level functions."""

    def test_tensor_creation(self) -> None:
        x = torch.tensor([[1.0, 2.0], [3.0, 4.0]])
        assert x.shape == (2, 2)

    def test_zeros(self) -> None:
        x = torch.zeros(3, 4)
        assert x.shape == (3, 4)
        assert all(v == 0.0 for v in x.data)

    def test_ones(self) -> None:
        x = torch.ones(2, 2)
        assert all(v == 1.0 for v in x.data)

    def test_randn(self) -> None:
        x = torch.randn(5, 3)
        assert x.shape == (5, 3)

    def test_eye(self) -> None:
        x = torch.eye(3)
        assert x.shape == (3, 3)
        assert x.data[0] == 1.0
        assert x.data[1] == 0.0

    def test_arange(self) -> None:
        x = torch.arange(0, 5)
        assert x.shape == (5,)

    def test_full(self) -> None:
        x = torch.full((2, 3), 7.0)
        assert all(v == 7.0 for v in x.data)

    def test_no_grad(self) -> None:
        with torch.no_grad():
            assert not torch.is_grad_enabled()
        assert torch.is_grad_enabled()


class TestForwardPass:
    """Test that models produce correct-shape outputs."""

    def test_single_linear(self) -> None:
        model = Linear(4, 2)
        x = Tensor.randn(3, 4)
        y = model(x)
        assert y.shape == (3, 2)

    def test_mlp(self) -> None:
        model = Sequential(
            Linear(4, 8),
            ReLU(),
            Linear(8, 2),
        )
        x = Tensor.randn(5, 4)
        y = model(x)
        assert y.shape == (5, 2)

    def test_deep_network(self) -> None:
        model = Sequential(
            Linear(3, 16),
            ReLU(),
            Linear(16, 16),
            ReLU(),
            Linear(16, 1),
        )
        x = Tensor.randn(2, 3)
        y = model(x)
        assert y.shape == (2, 1)


class TestGradientFlow:
    """Verify gradients are computed correctly through layers."""

    def test_linear_gradients(self) -> None:
        model = Linear(2, 1)
        x = Tensor.from_list([[1.0, 2.0]], requires_grad=True)
        y = model(x)
        loss = y.sum()
        loss.backward()

        # All parameters should have gradients
        for p in model.parameters():
            assert p.grad is not None

    def test_sequential_gradients(self) -> None:
        model = Sequential(Linear(2, 4), ReLU(), Linear(4, 1))
        x = Tensor.from_list([[1.0, 2.0]], requires_grad=True)
        y = model(x)
        loss = y.sum()
        loss.backward()

        for p in model.parameters():
            assert p.grad is not None

    def test_mse_loss_gradient(self) -> None:
        model = Linear(2, 1)
        loss_fn = MSELoss()

        x = Tensor.from_list([[1.0, 2.0]], requires_grad=True)
        target = Tensor.from_list([[3.0]])

        pred = model(x)
        loss = loss_fn(pred, target)
        loss.backward()

        assert model.weight.grad is not None


class TestTrainingLoop:
    """Test that loss decreases over multiple training steps."""

    def test_sgd_training(self) -> None:
        """Train a simple model to learn y = 2*x1 + 3*x2."""
        model = Linear(2, 1)
        optimizer = SGD(model.parameters(), lr=0.01)
        loss_fn = MSELoss()

        # Training data: y = 2*x1 + 3*x2
        x = Tensor.from_list([[1.0, 0.0], [0.0, 1.0], [1.0, 1.0]])
        target = Tensor.from_list([[2.0], [3.0], [5.0]])

        initial_loss = None
        final_loss = None

        for step in range(50):
            optimizer.zero_grad()
            pred = model(x)
            loss = loss_fn(pred, target)

            if step == 0:
                initial_loss = loss.data[0]

            loss.backward()
            optimizer.step()

            if step == 49:
                final_loss = loss.data[0]

        # Loss should decrease
        assert final_loss < initial_loss

    def test_adam_training(self) -> None:
        """Adam should converge faster than SGD for this simple task."""
        model = Linear(2, 1)
        optimizer = Adam(model.parameters(), lr=0.01)
        loss_fn = MSELoss()

        x = Tensor.from_list([[1.0, 0.0], [0.0, 1.0]])
        target = Tensor.from_list([[1.0], [1.0]])

        initial_loss = None
        for step in range(30):
            optimizer.zero_grad()
            pred = model(x)
            loss = loss_fn(pred, target)

            if step == 0:
                initial_loss = loss.data[0]

            loss.backward()
            optimizer.step()

        final_pred = model(x)
        final_loss = loss_fn(final_pred, target)
        assert final_loss.data[0] < initial_loss


class TestModeSwitch:
    """Test train/eval mode switching."""

    def test_dropout_behavior_changes(self) -> None:
        from ml_framework_torch.nn.dropout import Dropout

        model = Sequential(Linear(2, 4), Dropout(0.5), Linear(4, 1))

        model.train()
        assert model._modules["1"].training is True

        model.eval()
        assert model._modules["1"].training is False


class TestStateDict:
    """Test saving and loading model state."""

    def test_save_load(self) -> None:
        model1 = Sequential(Linear(2, 3), Linear(3, 1))
        state = model1.state_dict()

        model2 = Sequential(Linear(2, 3), Linear(3, 1))
        model2.load_state_dict(state)

        # Parameters should match
        for (_, p1), (_, p2) in zip(
            model1.named_parameters(), model2.named_parameters()
        ):
            for a, b in zip(p1.data, p2.data):
                assert abs(a - b) < 1e-6


class TestZeroGrad:
    """Test gradient zeroing."""

    def test_model_zero_grad(self) -> None:
        model = Linear(2, 1)
        x = Tensor.from_list([[1.0, 2.0]], requires_grad=True)
        y = model(x)
        y.sum().backward()

        # Gradients exist
        for p in model.parameters():
            assert p.grad is not None

        model.zero_grad()

        # Gradients cleared
        for p in model.parameters():
            assert p.grad is None

    def test_optimizer_zero_grad(self) -> None:
        model = Linear(2, 1)
        optimizer = SGD(model.parameters(), lr=0.01)

        x = Tensor.from_list([[1.0, 2.0]], requires_grad=True)
        y = model(x)
        y.sum().backward()

        optimizer.zero_grad()
        for p in model.parameters():
            assert p.grad is None


class TestEmbedding:
    """Test embedding layer in integration context."""

    def test_embedding_lookup(self) -> None:
        from ml_framework_torch.nn.embedding import Embedding

        embed = Embedding(10, 4)
        indices = Tensor.from_list([0.0, 5.0, 9.0])
        result = embed(indices)
        assert result.shape == (3, 4)


class TestFlatten:
    """Test flatten in a model pipeline."""

    def test_flatten_in_sequential(self) -> None:
        from ml_framework_torch.nn.flatten import Flatten

        model = Sequential(Flatten())
        x = Tensor.randn(2, 3, 4)
        y = model(x)
        assert y.shape == (2, 12)
