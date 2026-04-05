"""Tests for wasm-execution."""

from wasm_execution import __version__


class TestVersion:
    """Verify the package is importable and has a version."""

    def test_version_exists(self) -> None:
        assert __version__ == "0.1.0"
