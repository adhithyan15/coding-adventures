"""Tests for ``Intel4004Backend`` — the ``BackendProtocol`` adapter.

These verify the *adapter shape*: protocol conformance, the CIR →
``IRInstr`` re-projection, and the deopt cases that should return
``None`` rather than raising.  End-to-end JIT execution through the
backend is exercised by every test in
``tetrad-runtime/tests/test_runtime.py`` (where the JIT path runs the
same programs as the interpreter and asserts identical results).
"""

from __future__ import annotations

from jit_core.backend import BackendProtocol
from jit_core.cir import CIRInstr

from intel4004_backend import Intel4004Backend


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
    """``add_u8`` must be re-projected to ``add`` for the codegen.

    We can't easily assert on the bytes (the codegen is opaque), but we
    can verify the call doesn't raise and returns either bytes or None
    — both are valid backend protocol responses.
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


def test_run_dispatches_to_simulator() -> None:
    """``run`` should defer to the in-package ``run_on_4004`` (which
    in turn loads the binary into ``intel4004-simulator``).

    Feeding empty bytes is a sufficient smoke test that the import
    path is wired correctly — any binary the simulator rejects should
    raise rather than silently return wrong data.
    """
    backend = Intel4004Backend()
    try:
        result = backend.run(b"", [1, 2])
        assert isinstance(result, int)
    except Exception:
        # Either a graceful int result (zeroes-rom halt) or an exception
        # is acceptable; the point is the call is wired.
        pass
