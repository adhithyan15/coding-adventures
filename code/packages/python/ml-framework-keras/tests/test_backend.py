"""Tests for the backend module."""

import pytest

from ml_framework_keras.backend import get_backend, set_backend


class TestGetBackend:
    def test_default_backend(self):
        assert get_backend() == "ml_framework_core"

    def test_returns_string(self):
        assert isinstance(get_backend(), str)


class TestSetBackend:
    def test_set_ml_framework_core(self):
        set_backend("ml_framework_core")
        assert get_backend() == "ml_framework_core"

    def test_set_unknown_backend_raises(self):
        with pytest.raises(ValueError, match="Unknown backend"):
            set_backend("nonexistent")

    def test_set_torch_not_supported(self):
        with pytest.raises(ValueError, match="not supported"):
            set_backend("torch")

    def test_set_tensorflow_not_supported(self):
        with pytest.raises(ValueError, match="not supported"):
            set_backend("tensorflow")

    def test_set_jax_not_supported(self):
        with pytest.raises(ValueError, match="not supported"):
            set_backend("jax")
