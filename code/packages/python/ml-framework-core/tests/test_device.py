"""Tests for DeviceManager: device management and backend creation."""

import pytest

from ml_framework_core.device import DeviceManager


class TestDeviceManagerDefault:
    def setup_method(self):
        DeviceManager.reset()

    def test_default_device_is_cpu(self):
        assert DeviceManager.get_default_device() == "cpu"

    def test_set_default_device(self):
        DeviceManager.set_default_device("cuda")
        assert DeviceManager.get_default_device() == "cuda"

    def test_set_then_get(self):
        DeviceManager.set_default_device("metal")
        assert DeviceManager.get_default_device() == "metal"

    def test_set_multiple_times(self):
        DeviceManager.set_default_device("cuda")
        DeviceManager.set_default_device("vulkan")
        assert DeviceManager.get_default_device() == "vulkan"


class TestDeviceManagerReset:
    def test_reset_device(self):
        DeviceManager.set_default_device("cuda")
        DeviceManager.reset()
        assert DeviceManager.get_default_device() == "cpu"

    def test_reset_clears_backends(self):
        DeviceManager.get_backend("cpu")
        DeviceManager.reset()
        assert DeviceManager._backends == {}


class TestDeviceManagerBackend:
    def setup_method(self):
        DeviceManager.reset()

    def test_get_backend_cpu(self):
        DeviceManager.get_backend("cpu")
        # Just ensure it doesn't raise

    def test_get_backend_caches(self):
        b1 = DeviceManager.get_backend("cpu")
        b2 = DeviceManager.get_backend("cpu")
        assert b1 is b2

    def test_get_backend_different_devices(self):
        DeviceManager.get_backend("cpu")
        DeviceManager.get_backend("cuda")
        assert "cpu" in DeviceManager._backends
        assert "cuda" in DeviceManager._backends

    def test_get_backend_unknown_device(self):
        with pytest.raises(RuntimeError, match="not registered"):
            DeviceManager.get_backend("nonexistent_device")
