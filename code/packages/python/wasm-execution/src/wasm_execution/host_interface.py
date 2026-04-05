"""host_interface.py --- Host function interface and TrapError for WASM execution.

===========================================================================
WHAT IS THE HOST INTERFACE?
===========================================================================

WebAssembly modules interact with the outside world through *imports* ---
functions, globals, memories, and tables that the host environment provides.
The HostInterface is the contract any host environment must implement to
provide these imported values.

===========================================================================
WHAT IS A TRAP?
===========================================================================

A *trap* is an unrecoverable WASM runtime error. Division by zero,
out-of-bounds memory access, and unreachable instructions all cause traps.
We model them as TrapError exceptions so the host can catch them.
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Any, Protocol

if TYPE_CHECKING:
    from wasm_types import FuncType

    from wasm_execution.linear_memory import LinearMemory
    from wasm_execution.table import Table
    from wasm_execution.values import WasmValue


# ===========================================================================
# TrapError --- unrecoverable WASM runtime error
# ===========================================================================


class TrapError(Exception):
    """A WASM trap --- an unrecoverable runtime error.

    Common causes:
      - Out-of-bounds memory access
      - Division by zero (integer only; float yields NaN/Inf)
      - Integer overflow in i32.div_s(INT32_MIN, -1)
      - Unreachable instruction executed
      - Indirect call type mismatch
    """

    def __init__(self, message: str) -> None:
        super().__init__(message)
        self.name = "TrapError"


# ===========================================================================
# HostFunction --- a callable provided by the host
# ===========================================================================


class HostFunction(Protocol):
    """Protocol for host-provided functions importable by WASM modules.

    Attributes:
        type: The function's type signature (params and results).
    """

    @property
    def type(self) -> FuncType: ...

    def call(self, args: list[WasmValue]) -> list[WasmValue]:
        """Invoke the host function with typed WASM arguments."""
        ...


# ===========================================================================
# HostInterface --- the contract for resolving WASM imports
# ===========================================================================


class HostInterface(Protocol):
    """Protocol for resolving WASM module imports from the host environment.

    Each resolve method takes a two-level namespace (module_name, name) and
    returns the definition or None if not found.
    """

    def resolve_function(
        self, module_name: str, name: str
    ) -> Any | None:
        """Resolve an imported function."""
        ...

    def resolve_global(
        self, module_name: str, name: str
    ) -> dict[str, Any] | None:
        """Resolve an imported global. Returns {'type': GlobalType, 'value': WasmValue}."""
        ...

    def resolve_memory(
        self, module_name: str, name: str
    ) -> Any | None:
        """Resolve an imported linear memory."""
        ...

    def resolve_table(
        self, module_name: str, name: str
    ) -> Any | None:
        """Resolve an imported table."""
        ...
