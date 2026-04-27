"""simulator-protocol — generic interface all architecture simulators implement.

This package defines the ``Simulator[StateT]`` Protocol along with the
``ExecutionResult`` and ``StepTrace`` types that form the return values.

All architecture simulators in this repo (Intel 4004, Intel 8008, ARM1, ...)
implement this protocol so that the compiler pipeline, end-to-end tests, and
visualization tools can treat every simulator uniformly.

Usage
-----
    from simulator_protocol import Simulator, ExecutionResult, StepTrace

See the README and protocol.py for full documentation and usage examples.
"""

from __future__ import annotations

from simulator_protocol.protocol import ExecutionResult, Simulator, StepTrace

__all__ = ["Simulator", "ExecutionResult", "StepTrace"]
