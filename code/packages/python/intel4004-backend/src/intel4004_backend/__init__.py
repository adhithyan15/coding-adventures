"""``intel4004-backend`` — ``jit_core.BackendProtocol`` for Intel 4004.

This package is a *native code-generation backend* for the LANG
pipeline.  It implements ``jit_core.backend.BackendProtocol`` for the
Intel 4004 (the world's first commercial microprocessor, 1971), which
makes it the canonical example of how a native backend is packaged
for the LANG pipeline.

What backends are
-----------------

A backend is the seam between the language-agnostic JIT/AOT engine
(``jit-core`` / ``aot-core``) and a specific target ISA.  Every
backend implements two methods:

- ``compile(cir: list[CIRInstr]) -> bytes | None`` — translate a
  list of post-specialise / post-optimise CIR instructions to a
  native binary for this target.  Return ``None`` to deopt
  (``jit-core`` then falls back to interpretation).
- ``run(binary: bytes, args: list[Any]) -> Any`` — execute the
  compiled binary on this target's runtime (a simulator, a real
  device, an emulated CPU, etc.).

All backends live in their own package named ``<arch>-backend``.
Future siblings will follow the same pattern: ``intel8008-backend``,
``intel8080-backend``, ``mos6502-backend``, ``z80-backend``,
``riscv32-backend``, ``x86_64-backend``, ``arm64-backend``,
``wasm32-backend``, …

This separation lets a Lisp / Scheme / ML / JavaScript runtime pick
its target by importing the right backend package — no Tetrad-specific
machinery in the way, no monolithic "backends" package that pulls in
every codegen toolchain.

Why a CIR re-projection step?
-----------------------------

``Intel4004Backend.compile`` re-projects ``CIRInstr`` (jit-core's
typed-SSA shape) into a small ``IRInstr`` form the codegen consumes
directly.  Historically this codegen was written against ``IRInstr``
in the (now-retired) ``tetrad-jit`` package, and we kept that shape
when we lifted the codegen out.  A future cleanup will rewrite the
codegen to consume ``CIRInstr`` directly and remove the re-projection
— at that point, ``compile`` becomes a one-line forwarder.

Public API
----------

- :class:`Intel4004Backend` — implements ``BackendProtocol``.
- :class:`IRInstr` — internal IR shape (re-exported for backends and
  tests that want to feed the codegen directly).
- :func:`codegen` — IR → 4004 binary (returns ``bytes | None``).
- :func:`run_on_4004` — run a 4004 binary on the simulator.
- :func:`evaluate_op` — abstract-evaluation helper used by constant
  folding.
- :class:`DeoptimizerError` — raised by the codegen for unrecoverable
  encoding failures.
"""

from __future__ import annotations

from intel4004_backend.backend import Intel4004Backend
from intel4004_backend.codegen import DeoptimizerError, codegen, run_on_4004
from intel4004_backend.ir import IRInstr, evaluate_op

__all__ = [
    "Intel4004Backend",
    "IRInstr",
    "codegen",
    "evaluate_op",
    "run_on_4004",
    "DeoptimizerError",
]
