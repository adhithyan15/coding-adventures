"""Shared fixtures for x86-64 simulator tests."""

import pytest

from x86_64_simulator import X86_64Simulator


@pytest.fixture
def sim() -> X86_64Simulator:
    """Fresh simulator instance for each test."""
    return X86_64Simulator()


def assemble(*bytes_: int) -> bytes:
    """Convenience: build a byte program, appending HLT (0xF4)."""
    return bytes(list(bytes_) + [0xF4])
