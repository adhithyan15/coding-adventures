"""testbench-framework: Pythonic harness around HardwareVM."""

from testbench_framework.runner import (
    DUTHandle,
    SignalHandle,
    TestCase,
    TestReport,
    clear_registry,
    discover,
    exhaustive,
    random_stimulus,
    run,
    test,
)

__version__ = "0.1.0"

__all__ = [
    "DUTHandle",
    "SignalHandle",
    "TestCase",
    "TestReport",
    "__version__",
    "clear_registry",
    "discover",
    "exhaustive",
    "random_stimulus",
    "run",
    "test",
]
