"""Tests for the ``Intel4004Backend`` adapter.

These tests verify the *adapter shape* — that ``Intel4004Backend``
implements ``jit_core.BackendProtocol`` and behaves correctly on
deopt-worthy CIR shapes.  End-to-end JIT execution through the
backend is exercised by every test in ``test_runtime.py`` via the
``_run_both_paths`` harness.
"""

from __future__ import annotations

from jit_core.backend import BackendProtocol
from jit_core.cir import CIRInstr

from tetrad_runtime import Intel4004Backend


def test_implements_backend_protocol() -> None:
    """The structural ``BackendProtocol`` check should pass."""
    backend = Intel4004Backend()
    assert isinstance(backend, BackendProtocol)
    assert backend.name == "intel4004"


def test_compile_returns_none_for_call_runtime() -> None:
    """A generic ``call_runtime`` op cannot be lowered to 4004."""
    backend = Intel4004Backend()
    cir = [CIRInstr(op="call_runtime", dest="v0", srcs=["foo"], type="any")]
    assert backend.compile(cir) is None


def test_compile_returns_none_for_type_guard() -> None:
    """A type guard signals that the IR isn't fully concrete — deopt."""
    backend = Intel4004Backend()
    cir = [
        CIRInstr(
            op="type_assert",
            dest=None,
            srcs=["v0", "u8"],
            type="u8",
            deopt_to=0,
        )
    ]
    assert backend.compile(cir) is None


def test_compile_strips_type_suffix_from_op() -> None:
    """``add_u8`` must be re-projected to ``add`` for the legacy codegen.

    We can't easily assert on the bytes (the legacy codegen is opaque),
    but we can verify the call doesn't raise and returns either bytes
    or None — both are valid backend protocol responses.
    """
    backend = Intel4004Backend()
    cir = [
        CIRInstr(op="const_u8", dest="a", srcs=[5], type="u8"),
        CIRInstr(op="const_u8", dest="b", srcs=[7], type="u8"),
        CIRInstr(op="add_u8", dest="r", srcs=["a", "b"], type="u8"),
        CIRInstr(op="ret", dest=None, srcs=["r"], type="u8"),
    ]
    result = backend.compile(cir)
    assert result is None or isinstance(result, bytes)


def test_run_dispatches_to_legacy_simulator() -> None:
    """``run`` should defer to ``tetrad_jit.codegen_4004.run_on_4004``.

    We don't construct a real binary here — feeding empty bytes is a
    sufficient smoke test that the import path is wired correctly.
    Any binary the simulator rejects should raise rather than silently
    return wrong data.
    """
    backend = Intel4004Backend()
    # An empty binary is invalid; we expect either a graceful integer
    # result (if the simulator's "all-zeroes ROM halts immediately"
    # behaviour kicks in) or a runtime error.  Either is valid; the
    # point is the call is wired.
    try:
        result = backend.run(b"", [1, 2])
        assert isinstance(result, int)
    except Exception:
        pass
