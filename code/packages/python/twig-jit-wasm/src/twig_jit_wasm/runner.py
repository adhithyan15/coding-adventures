"""``TwigJITRunner`` — Twig source → JIT-specialised WebAssembly.

Architecture
============

This module is *glue*.  All the heavy lifting lives in pre-existing
packages:

* ``twig.compile_program`` — Twig source → ``IIRModule`` (already
  exposes a fully-typed enough IR for the JIT path).
* ``twig.vm.TwigVM`` — sets up a ``VMCore`` and registers every
  Twig builtin (heap ops, closure machinery, IO).  We mirror its
  builtin-registration to make sure the JIT-driven runs see the
  same world the interpreter does.
* ``jit_core.JITCore`` — the generic JIT controller.  Given a
  ``VMCore`` and a ``BackendProtocol``, it watches for hot
  functions and specialises them via the backend.
* ``wasm_backend.WASMBackend`` — the ``BackendProtocol``
  implementation that lowers ``CIRInstr`` → WebAssembly bytes
  and runs them through ``wasm-runtime``.

What this package adds is **the wiring between Twig and JITCore**.
That's it.  No new IR transforms, no new optimization passes — those
all already live in their respective packages and the JIT path
inherits them for free.

Why it's a separate package
===========================

The ``twig`` package's pyproject.toml deliberately doesn't depend on
``jit-core`` or ``wasm-backend``.  Educational users running through
the simple interpreter path shouldn't need a WASM toolchain on disk.
``twig-jit-wasm`` is the opt-in package: install this when you want
the JIT track.  Mirrors the way ``twig-jvm-compiler`` and
``twig-beam-compiler`` are opt-in for their respective real-runtime
targets.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any

from interpreter_ir import IIRModule
from jit_core import JITCore
from twig import (
    extract_program,
    parse_twig,
)
from twig.compiler import compile_program
from twig.vm import TwigVM
from vm_core import VMCore


def compile_to_iir(source: str) -> IIRModule:
    """Compile Twig source to an :class:`IIRModule`.

    Thin wrapper over ``twig.compile_program`` exposed here so
    callers don't have to import from three different places to
    drive the JIT pipeline.
    """
    return compile_program(extract_program(parse_twig(source)))


@dataclass
class TwigJITRunner:
    """Run a Twig program with JIT specialisation.

    Each ``run`` call gets a fresh execution environment (heap,
    globals, output buffer) — no state leaks between programs.

    Parameters
    ----------
    backend
        The :class:`BackendProtocol` instance to give ``JITCore``.
        ``None`` means "use ``wasm_backend.WASMBackend()`` as the
        default" — we lazy-import the WASM backend so importing
        this module without the WASM toolchain installed doesn't
        explode at import time.
    threshold_partial
        Forwarded to ``JITCore`` — number of interpreter-side
        observations of a partially-typed function before the JIT
        promotes it.  Default 10 (matches jit-core default).
    threshold_untyped
        Forwarded to ``JITCore`` — same idea for untyped functions.
        Default 100.
    """

    backend: Any | None = None
    threshold_partial: int = 10
    threshold_untyped: int = 100
    _twig_vm: TwigVM | None = None

    def __post_init__(self) -> None:
        # Lazily build a TwigVM we'll borrow for builtin
        # registration; the actual VMCore lives per-run because
        # TwigVM creates a fresh one each ``execute_module``.
        self._twig_vm = TwigVM()

    # ------------------------------------------------------------------

    def run(self, source: str) -> Any:
        """Compile, run via ``JITCore``, return the program result."""
        module = compile_to_iir(source)
        return self.run_module(module)

    def run_module(self, module: IIRModule) -> Any:
        """Run an already-compiled :class:`IIRModule` through JIT."""
        backend = self.backend if self.backend is not None else _default_backend()

        # Set up the VMCore + Twig builtins exactly the way TwigVM
        # does — but instead of calling ``vm.execute(module, fn="main")``
        # ourselves, hand the configured VM to JITCore so it can
        # specialise hot functions before / during the run.
        vm = self._build_configured_vm(module)
        jit = JITCore(
            vm,
            backend,
            threshold_partial=self.threshold_partial,
            threshold_untyped=self.threshold_untyped,
        )
        return jit.execute_with_jit(module, fn="main")

    # ------------------------------------------------------------------
    # Internal: borrow TwigVM's builtin-registration to set up VMCore
    # ------------------------------------------------------------------

    def _build_configured_vm(self, module: IIRModule) -> VMCore:
        """Build a ``VMCore`` configured with Twig's builtins.

        ``TwigVM`` does this internally when you call
        ``execute_module``, but it doesn't expose the configured
        VMCore separately.  We replicate the setup here so JITCore
        can drive the *same* environment the interpreter would.

        Implementation note: rather than duplicating the builtin-
        wiring code (which is non-trivial — ~150 lines covering
        heap, closure, I/O, and arithmetic ops), we rely on a small
        amount of TwigVM internals via the public ``_register_builtins``
        method.  If ``twig`` ever marks that helper private-only,
        we'll have to factor the builtin block out into a shared
        ``twig_runtime`` module.  For now the dependency is
        intentional and documented.
        """
        # Re-use TwigVM's per-run state (heap, globals, output).
        # We mirror execute_module's first 5 statements.
        import io  # noqa: PLC0415 — local import keeps top-level lean

        from twig.heap import Heap  # noqa: PLC0415

        from twig.vm import _make_jmp_if  # noqa: PLC0415 — internal

        heap = Heap()
        globals_table: dict[str, Any] = {}
        output = io.StringIO()

        vm = VMCore(
            max_frames=256,
            profiler_enabled=True,  # JIT needs the profiler
            opcodes={
                "jmp_if_true": _make_jmp_if(truthy=True),
                "jmp_if_false": _make_jmp_if(truthy=False),
            },
        )
        # Re-use TwigVM's builtin-registration directly.  It's a
        # public-ish method on the class; staying out of its
        # implementation keeps this glue thin.
        assert self._twig_vm is not None
        self._twig_vm._register_builtins(  # noqa: SLF001
            vm, heap, globals_table, output, module
        )
        return vm


def run_with_jit(
    source: str,
    *,
    backend: Any | None = None,
) -> Any:
    """Convenience wrapper: instantiate a ``TwigJITRunner`` and run.

    >>> from twig_jit_wasm import run_with_jit
    >>> run_with_jit("(+ 1 2)")
    3
    """
    return TwigJITRunner(backend=backend).run(source)


# ---------------------------------------------------------------------------
# Lazy backend default
# ---------------------------------------------------------------------------


def _default_backend() -> Any:
    """Return a fresh ``WASMBackend()``.

    Imported lazily so callers who pass an explicit ``backend=`` or
    only test the runner against a stub backend don't have to have
    the WASM toolchain installed at import time.
    """
    from wasm_backend import WASMBackend  # noqa: PLC0415

    return WASMBackend()
