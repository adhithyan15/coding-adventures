"""Shared test fixtures for vendor API simulator tests.

Provides common GPU instructions (saxpy-style) and helper data that
all six simulator test suites can use.
"""

import pytest
from gpu_core import limm, halt


@pytest.fixture
def simple_instructions():
    """A minimal GPU program: load a constant and halt.

    This is the simplest possible kernel — it loads the value 42.0 into
    register 0 and then halts. Used for testing that dispatch works at all.
    """
    return [limm(0, 42.0), halt()]


@pytest.fixture
def nop_instructions():
    """A no-op GPU program: just halt immediately.

    Even simpler than simple_instructions — no computation at all.
    Used for testing dispatch mechanics without caring about results.
    """
    return [halt()]
