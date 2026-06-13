"""Tests for the ``load("name")`` runtime directive (Track M1).

Acceptance contract from ``macsyma-truly-finish-plan.md`` §M1:

- Without ``load``, orthopoly heads round-trip unevaluated.
- After ``load("orthopoly")``, the closed-form polynomial is computed.
- Unknown names raise :class:`MacsymaUserError`.
- Re-loading is idempotent.
- Loaded state is *per session*.

Each test below maps to one bullet.  The tests use the VM directly
(no REPL layer) because the loader is a backend-level concern.
"""

from __future__ import annotations

import pytest
from symbolic_ir import (
    IRApply,
    IRInteger,
    IRString,
    IRSymbol,
)
from symbolic_vm import VM

from macsyma_runtime import MacsymaBackend, MacsymaUserError

# ---------------------------------------------------------------------------
# Fixtures and helpers
# ---------------------------------------------------------------------------


def _fresh_session() -> tuple[VM, MacsymaBackend]:
    """Return a fresh VM bound to a fresh MacsymaBackend."""
    backend = MacsymaBackend()
    return VM(backend), backend


def _load(vm: VM, name: str) -> object:
    """Invoke ``load("name")`` through the VM and return the result."""
    return vm.eval(IRApply(IRSymbol("Load"), (IRString(name),)))


def _legendre_p(n: int, x_name: str) -> IRApply:
    return IRApply(
        IRSymbol("LegendreP"),
        (IRInteger(n), IRSymbol(x_name)),
    )


# ---------------------------------------------------------------------------
# 1. Without load, orthopoly heads stay unevaluated.
# ---------------------------------------------------------------------------


def test_unloaded_legendre_p_returns_unevaluated() -> None:
    """``legendre_p(3, x)`` round-trips through the VM unchanged."""
    vm, _ = _fresh_session()
    expr = _legendre_p(3, "x")
    result = vm.eval(expr)
    # The whole IRApply is identity — no handler installed → unevaluated.
    assert isinstance(result, IRApply)
    assert isinstance(result.head, IRSymbol)
    assert result.head.name == "LegendreP"
    assert result.args == (IRInteger(3), IRSymbol("x"))


def test_unloaded_chebyshev_t_returns_unevaluated() -> None:
    vm, _ = _fresh_session()
    expr = IRApply(IRSymbol("ChebyshevT"), (IRInteger(4), IRSymbol("x")))
    result = vm.eval(expr)
    assert isinstance(result, IRApply)
    assert isinstance(result.head, IRSymbol)
    assert result.head.name == "ChebyshevT"


def test_unloaded_hermite_h_returns_unevaluated() -> None:
    vm, _ = _fresh_session()
    expr = IRApply(IRSymbol("HermiteH"), (IRInteger(2), IRSymbol("x")))
    result = vm.eval(expr)
    assert isinstance(result, IRApply)
    assert isinstance(result.head, IRSymbol)
    assert result.head.name == "HermiteH"


# ---------------------------------------------------------------------------
# 2. After load("orthopoly"), closed-form reductions kick in.
# ---------------------------------------------------------------------------


def test_loaded_legendre_p_3_reduces_to_polynomial() -> None:
    """``LegendreP(3, x)`` → ``(5x³ − 3x)/2``.

    We verify by substituting a concrete ``x`` and comparing numeric values
    against the closed form.  At ``x = 2``: P_3(2) = (5·8 − 3·2)/2 = 34/2 = 17.
    """
    vm, _ = _fresh_session()
    _load(vm, "orthopoly")

    # Substitute x with a concrete integer so we get a numeric answer.
    # subst(2, x, legendre_p(3, x)) routes through cas-substitution.
    expr = IRApply(
        IRSymbol("Subst"),
        (IRInteger(2), IRSymbol("x"), _legendre_p(3, "x")),
    )
    result = vm.eval(expr)
    assert result == IRInteger(17)


def test_loaded_legendre_p_0_and_1_are_seed_values() -> None:
    """The Bonnet seed cases: ``P_0 = 1``, ``P_1 = x``."""
    vm, _ = _fresh_session()
    _load(vm, "orthopoly")

    p0 = vm.eval(_legendre_p(0, "x"))
    assert p0 == IRInteger(1)

    p1 = vm.eval(_legendre_p(1, "x"))
    assert p1 == IRSymbol("x")


def test_loaded_chebyshev_t_4_at_x_one_is_one() -> None:
    """``T_4(1) = 1`` (Chebyshev T at 1 is always 1)."""
    vm, _ = _fresh_session()
    _load(vm, "orthopoly")

    expr = IRApply(
        IRSymbol("Subst"),
        (
            IRInteger(1),
            IRSymbol("x"),
            IRApply(IRSymbol("ChebyshevT"), (IRInteger(4), IRSymbol("x"))),
        ),
    )
    result = vm.eval(expr)
    assert result == IRInteger(1)


def test_loaded_chebyshev_u_3_at_x_one_is_four() -> None:
    """``U_3(1) = 4`` (U_n(1) = n + 1 for the second kind)."""
    vm, _ = _fresh_session()
    _load(vm, "orthopoly")

    expr = IRApply(
        IRSymbol("Subst"),
        (
            IRInteger(1),
            IRSymbol("x"),
            IRApply(IRSymbol("ChebyshevU"), (IRInteger(3), IRSymbol("x"))),
        ),
    )
    result = vm.eval(expr)
    assert result == IRInteger(4)


def test_loaded_hermite_h_3_at_x_one_is_negative_four() -> None:
    """``H_3(1) = 8 − 12 = −4`` (physicists' convention)."""
    vm, _ = _fresh_session()
    _load(vm, "orthopoly")

    expr = IRApply(
        IRSymbol("Subst"),
        (
            IRInteger(1),
            IRSymbol("x"),
            IRApply(IRSymbol("HermiteH"), (IRInteger(3), IRSymbol("x"))),
        ),
    )
    result = vm.eval(expr)
    assert result == IRInteger(-4)


# ---------------------------------------------------------------------------
# 3. Passthrough heads — symbols known after load, no reduction performed.
# ---------------------------------------------------------------------------


def test_loaded_bessel_j_returns_unevaluated_with_known_head() -> None:
    """After load, ``BesselJ(0, x)`` is still unevaluated (no closed form)."""
    vm, _ = _fresh_session()
    _load(vm, "orthopoly")

    expr = IRApply(IRSymbol("BesselJ"), (IRInteger(0), IRSymbol("x")))
    result = vm.eval(expr)
    assert isinstance(result, IRApply)
    assert isinstance(result.head, IRSymbol)
    assert result.head.name == "BesselJ"


def test_loaded_legendre_q_returns_unevaluated() -> None:
    vm, _ = _fresh_session()
    _load(vm, "orthopoly")

    expr = IRApply(IRSymbol("LegendreQ"), (IRInteger(2), IRSymbol("x")))
    result = vm.eval(expr)
    assert isinstance(result, IRApply)
    assert isinstance(result.head, IRSymbol)
    assert result.head.name == "LegendreQ"


# ---------------------------------------------------------------------------
# 4. Allowlist enforcement.
# ---------------------------------------------------------------------------


def test_load_unknown_package_raises_user_error() -> None:
    vm, _ = _fresh_session()
    with pytest.raises(MacsymaUserError) as excinfo:
        _load(vm, "nonexistent")
    assert "unknown package" in str(excinfo.value)
    assert "'nonexistent'" in str(excinfo.value)
    # The error advertises what *is* available so users self-correct.
    assert "orthopoly" in str(excinfo.value)


def test_load_with_non_string_non_symbol_raises() -> None:
    """An integer arg is not a valid package name."""
    vm, _ = _fresh_session()
    expr = IRApply(IRSymbol("Load"), (IRInteger(42),))
    with pytest.raises(MacsymaUserError) as excinfo:
        vm.eval(expr)
    assert "string or symbol" in str(excinfo.value)


def test_load_with_wrong_arity_raises() -> None:
    vm, _ = _fresh_session()
    expr = IRApply(IRSymbol("Load"), ())
    with pytest.raises(MacsymaUserError):
        vm.eval(expr)


def test_load_path_traversal_string_is_rejected() -> None:
    """A string with ``..`` or ``/`` separators is treated like any other
    unknown name — the allowlist match is by string equality, so it just
    isn't on the list.  This nails down that there's no path resolution.
    """
    vm, _ = _fresh_session()
    for hostile in ("../etc/passwd", "/tmp/orthopoly", "orthopoly.py", "os"):
        with pytest.raises(MacsymaUserError):
            _load(vm, hostile)


# ---------------------------------------------------------------------------
# 5. Idempotence — re-loading is a no-op.
# ---------------------------------------------------------------------------


def test_load_orthopoly_is_idempotent() -> None:
    vm, backend = _fresh_session()
    _load(vm, "orthopoly")
    # Second call must not raise and the backend state must stay consistent.
    second_result = _load(vm, "orthopoly")
    assert second_result == IRString("orthopoly")
    assert backend._loaded_packages == {"orthopoly"}

    # The evaluator still works after the second call.
    p2_at_3 = vm.eval(
        IRApply(
            IRSymbol("Subst"),
            (IRInteger(3), IRSymbol("x"), _legendre_p(2, "x")),
        )
    )
    # P_2(3) = (3·9 − 1)/2 = 26/2 = 13
    assert p2_at_3 == IRInteger(13)


# ---------------------------------------------------------------------------
# 6. Per-session state — two backends are independent.
# ---------------------------------------------------------------------------


def test_two_backends_have_independent_loaded_state() -> None:
    vm_a, backend_a = _fresh_session()
    vm_b, backend_b = _fresh_session()

    _load(vm_a, "orthopoly")

    assert backend_a._loaded_packages == {"orthopoly"}
    assert backend_b._loaded_packages == set()

    # vm_a reduces, vm_b doesn't.
    a_result = vm_a.eval(_legendre_p(0, "x"))
    assert a_result == IRInteger(1)

    b_result = vm_b.eval(_legendre_p(0, "x"))
    # Unevaluated — head intact, no handler installed for backend_b.
    assert isinstance(b_result, IRApply)
    assert isinstance(b_result.head, IRSymbol)
    assert b_result.head.name == "LegendreP"


# ---------------------------------------------------------------------------
# 7. Regression — non-orthopoly ops still work without load.
# ---------------------------------------------------------------------------


def test_expand_still_works_without_load() -> None:
    """``expand((x+1)^2)`` does not need any package loaded."""
    vm, _ = _fresh_session()
    # expand((x+1)^2) — the canonical multivariate-expand fallback.
    inner = IRApply(
        IRSymbol("Pow"),
        (
            IRApply(IRSymbol("Add"), (IRSymbol("x"), IRInteger(1))),
            IRInteger(2),
        ),
    )
    expr = IRApply(IRSymbol("Expand"), (inner,))
    result = vm.eval(expr)
    # Result should not be the raw Expand(...) IRApply — handler fired.
    assert not (
        isinstance(result, IRApply)
        and isinstance(result.head, IRSymbol)
        and result.head.name == "Expand"
    )


def test_factor_still_works_without_load() -> None:
    vm, _ = _fresh_session()
    # factor(x^2 - 1) = (x-1)(x+1)
    expr = IRApply(
        IRSymbol("Factor"),
        (
            IRApply(
                IRSymbol("Sub"),
                (
                    IRApply(IRSymbol("Pow"), (IRSymbol("x"), IRInteger(2))),
                    IRInteger(1),
                ),
            ),
        ),
    )
    result = vm.eval(expr)
    # We don't care about exact ordering of factors; just that the result
    # is not the unfactored polynomial untouched.
    assert result != expr


# ---------------------------------------------------------------------------
# 8. Surface-name routing — ``load`` is wired through the name table.
# ---------------------------------------------------------------------------


def test_name_table_maps_load_and_orthopoly_surface_names() -> None:
    """Sanity check: the surface names route to the right IR heads."""
    from macsyma_runtime import MACSYMA_NAME_TABLE

    assert MACSYMA_NAME_TABLE["load"].name == "Load"
    assert MACSYMA_NAME_TABLE["legendre_p"].name == "LegendreP"
    assert MACSYMA_NAME_TABLE["legendre_q"].name == "LegendreQ"
    assert MACSYMA_NAME_TABLE["chebyshev_t"].name == "ChebyshevT"
    assert MACSYMA_NAME_TABLE["chebyshev_u"].name == "ChebyshevU"
    assert MACSYMA_NAME_TABLE["hermite"].name == "HermiteH"
    assert MACSYMA_NAME_TABLE["bessel_j"].name == "BesselJ"
    assert MACSYMA_NAME_TABLE["bessel_y"].name == "BesselY"


# ---------------------------------------------------------------------------
# 9. Symbol-form loading — Maxima accepts both ``load("foo")`` and
# ``load(foo)``.  Verify the bare symbol form also works.
# ---------------------------------------------------------------------------


def test_load_accepts_bare_symbol_argument() -> None:
    vm, backend = _fresh_session()
    result = vm.eval(IRApply(IRSymbol("Load"), (IRSymbol("orthopoly"),)))
    assert result == IRString("orthopoly")
    assert backend._loaded_packages == {"orthopoly"}


# ---------------------------------------------------------------------------
# 10. Non-integer first argument keeps the polynomial heads unevaluated.
# ---------------------------------------------------------------------------


def test_loaded_legendre_p_symbolic_n_is_unevaluated() -> None:
    """``legendre_p(n, x)`` with a free ``n`` symbol must round-trip."""
    vm, _ = _fresh_session()
    _load(vm, "orthopoly")

    expr = IRApply(
        IRSymbol("LegendreP"),
        (IRSymbol("n"), IRSymbol("x")),
    )
    result = vm.eval(expr)
    assert isinstance(result, IRApply)
    assert isinstance(result.head, IRSymbol)
    assert result.head.name == "LegendreP"


def test_loaded_legendre_p_negative_n_is_unevaluated() -> None:
    """Negative degree is undefined under our recurrence; stay symbolic."""
    vm, _ = _fresh_session()
    _load(vm, "orthopoly")

    expr = IRApply(IRSymbol("LegendreP"), (IRInteger(-1), IRSymbol("x")))
    result = vm.eval(expr)
    assert isinstance(result, IRApply)
    assert isinstance(result.head, IRSymbol)
    assert result.head.name == "LegendreP"
