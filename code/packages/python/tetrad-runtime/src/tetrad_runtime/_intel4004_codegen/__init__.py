"""Internal Intel 4004 codegen — adopted from the legacy ``tetrad-jit``.

This subpackage is the bytecode → 4004-abstract-assembly → binary
pipeline that ``Intel4004Backend`` uses.  It moved here as part of
deprecating ``tetrad-jit`` (and ``tetrad-vm``) — the only consumer of
the codegen was ``Intel4004Backend``, so co-locating the code with its
sole user simplifies the dependency graph and lets the legacy package
be deleted.

The leading underscore marks this as **internal**.  External callers
should go through ``tetrad_runtime.Intel4004Backend``; the codegen
shape is not a stable public API.

Modules
-------

- :mod:`ir` — ``IRInstr`` dataclass, the SSA-by-name shape that
  ``Intel4004Backend`` re-projects ``CIRInstr`` into.
- :mod:`codegen` — ``codegen(ir)`` translates an ``IRInstr`` list to a
  4004 binary; ``run_on_4004(binary, args)`` executes that binary on
  the ``intel4004-simulator``.

When the day comes for a CIR-native 4004 backend (the planned
``intel4004-backend`` package), this whole subpackage gets retired.
For now it bridges CIR back to the original codegen so we don't lose
the working Tetrad → 4004 pipeline.
"""

from __future__ import annotations

from tetrad_runtime._intel4004_codegen.codegen import (
    DeoptimizerError,
    codegen,
    run_on_4004,
)
from tetrad_runtime._intel4004_codegen.ir import IRInstr

__all__ = ["DeoptimizerError", "IRInstr", "codegen", "run_on_4004"]
