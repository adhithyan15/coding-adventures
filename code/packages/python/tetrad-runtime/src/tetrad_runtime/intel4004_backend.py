"""``Intel4004Backend`` — re-export from the ``intel4004-backend`` package.

This module was originally the in-tree home of the Intel 4004 codegen.
The codegen and the ``Intel4004Backend`` class have moved to their own
package, ``intel4004-backend``, so they can be reused by frontends
other than Tetrad — and so future native backends (``intel8008-backend``,
``mos6502-backend``, ``riscv32-backend``, …) follow the same
``<arch>-backend`` package pattern.

This module remains as a thin re-export so callers that reach
``tetrad_runtime.Intel4004Backend`` keep working.  New code should
import from ``intel4004_backend`` directly.
"""

from __future__ import annotations

from intel4004_backend import Intel4004Backend

__all__ = ["Intel4004Backend"]
