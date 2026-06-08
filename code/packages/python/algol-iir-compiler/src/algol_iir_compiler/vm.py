"""ALGOL-configured wrapper around ``vm-core``."""

from __future__ import annotations

from interpreter_ir import IIRModule
from vm_core import VMCore, VMMetrics

from algol_iir_compiler.compiler import compile_source


class AlgolVM:
    """Compile and execute the current ALGOL scalar-IIR slice."""

    def __init__(self) -> None:
        self._last_metrics: VMMetrics | None = None
        self._vm: VMCore | None = None

    def compile(self, source: str, *, module_name: str = "algol60") -> IIRModule:
        """Compile source to an ``IIRModule`` without executing it."""
        return compile_source(source, module_name=module_name)

    def run(self, source: str, *, module_name: str = "algol60") -> object:
        """Compile and execute source, returning ``main``'s result."""
        module = self.compile(source, module_name=module_name)
        return self.execute_module(module)

    def execute_module(self, module: IIRModule) -> object:
        """Execute an already compiled ALGOL IIR module."""
        vm = VMCore()
        self._vm = vm
        try:
            return vm.execute(module, fn=module.entry_point or "main")
        finally:
            self._last_metrics = vm.metrics()

    @property
    def last_metrics(self) -> VMMetrics | None:
        """Metrics from the most recent run, if any."""
        return self._last_metrics
