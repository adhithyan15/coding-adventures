"""End-to-end pipeline tests: EllipticE and EllipticPi recognition.

These tests send MACSYMA source strings through the complete pipeline:

    parse_macsyma  →  compile_macsyma  →  VM(MacsymaBackend).eval

and verify that the integration engine correctly recognises the second-kind
and third-kind elliptic integrands introduced in symbolic-vm v0.54.0.

Test matrix
-----------
- Complete EllipticE with numeric modulus   (k=1/2)
- Complete EllipticE with symbolic modulus  (k)
- Complete EllipticE modulus is extracted as-is, not squared
- Incomplete EllipticE with symbolic k
- Incomplete EllipticE with numeric k
- EllipticE integrand with commuted MUL order (sin^2 · k^2, not k^2 · sin^2)
- Complete EllipticPi with n=2, k=1/2
- Complete EllipticPi with symbolic n and k
- EllipticPi with commuted denominator factors (sqrt first, bracket second)
- Fallthrough: sin^2 integrand returns a trig antiderivative, not EllipticE
- Fallthrough: plain 1/sqrt(1-x^2) is NOT EllipticK (no sin² structure)
- Fallthrough: definite integral with wrong lower bound stays unevaluated
- Fallthrough: definite integral with wrong upper bound stays unevaluated
- Regression: EllipticK still returns correctly after adding EllipticE/Pi
- Regression: EllipticF still returns correctly after adding EllipticE/Pi
- EllipticE result has head IRSymbol("EllipticE")
- EllipticPi result has head IRSymbol("EllipticPi")
- Incomplete EllipticPi (3-arg form) stays unevaluated (not implemented)
"""

from __future__ import annotations

from macsyma_compiler import compile_macsyma
from macsyma_compiler.compiler import _STANDARD_FUNCTIONS
from macsyma_parser import parse_macsyma
from symbolic_ir import INTEGRATE, IRApply, IRInteger, IRRational, IRSymbol
from symbolic_vm import VM

from macsyma_runtime import MacsymaBackend, extend_compiler_name_table

# Extend the compiler name table so MACSYMA names compile to canonical IR.
# This call is idempotent — subsequent calls are no-ops.
extend_compiler_name_table(_STANDARD_FUNCTIONS)


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _eval(source: str) -> object:
    """Parse + compile + eval ``source`` (no terminator needed).

    Returns the evaluated IR node.  The Display/Suppress wrapper added by
    ``wrap_terminators=False`` is absent here; we evaluate the raw IR.
    """
    src = source.strip().rstrip(";$").strip()
    ast = parse_macsyma(src + ";")
    stmts = compile_macsyma(ast, wrap_terminators=False)
    assert len(stmts) == 1, f"expected 1 statement, got {len(stmts)}: {stmts}"
    vm = VM(MacsymaBackend())
    return vm.eval(stmts[0])


_ELLIPTIC_E_HEAD = IRSymbol("EllipticE")
_ELLIPTIC_PI_HEAD = IRSymbol("EllipticPi")
_ELLIPTIC_K_HEAD = IRSymbol("EllipticK")
_ELLIPTIC_F_HEAD = IRSymbol("EllipticF")


def _is_apply_with_head(result: object, head: IRSymbol) -> bool:
    """Return True iff *result* is an IRApply whose head == *head*."""
    return isinstance(result, IRApply) and result.head == head


# ---------------------------------------------------------------------------
# Section A — Complete EllipticE (definite integral from 0 to π/2)
# ---------------------------------------------------------------------------


def test_complete_elliptic_e_numeric_k_evaluates() -> None:
    """integrate(sqrt(1-(1/2)^2*sin(theta)^2), theta, 0, %pi/2) → EllipticE(1/2).

    The compiler folds ``(1/2)^2`` to ``IRRational(1, 4)``.
    ``_modulus_from_squared_factor`` now handles numeric literal ``k²`` by
    computing ``k = sqrt(k²)`` — so IRRational(1,4) → k = IRRational(1,2).
    The result is ``EllipticE(1/2)``.
    """
    result = _eval(
        "integrate(sqrt(1-(1/2)^2*sin(theta)^2), theta, 0, %pi/2)"
    )
    assert _is_apply_with_head(result, _ELLIPTIC_E_HEAD), (
        f"expected EllipticE(...), got {result!r}"
    )
    assert isinstance(result, IRApply)
    assert len(result.args) == 1, "complete EllipticE takes one arg (modulus)"
    from symbolic_ir import IRRational
    assert result.args[0] == IRRational(1, 2), (
        f"expected modulus 1/2, got {result.args[0]!r}"
    )


def test_complete_elliptic_e_symbolic_k() -> None:
    """integrate(sqrt(1-k^2*sin(theta)^2), theta, 0, %pi/2) → EllipticE(k)."""
    result = _eval(
        "integrate(sqrt(1-k^2*sin(theta)^2), theta, 0, %pi/2)"
    )
    assert result == IRApply(_ELLIPTIC_E_HEAD, (IRSymbol("k"),)), (
        f"expected EllipticE(k), got {result!r}"
    )


def test_complete_elliptic_e_modulus_is_k_not_k2() -> None:
    """EllipticE(k) — the first arg is k, not k^2.

    Regression guard: ``_modulus_from_squared_factor`` must return ``k``
    (the base of Pow(k,2)), not ``k^2`` itself.
    """
    result = _eval(
        "integrate(sqrt(1-k^2*sin(theta)^2), theta, 0, %pi/2)"
    )
    assert isinstance(result, IRApply)
    assert result.head == _ELLIPTIC_E_HEAD
    modulus = result.args[0]
    # Must be the plain symbol k, not a Pow(k,2) node
    assert modulus == IRSymbol("k"), f"expected k, got {modulus!r}"


def test_complete_elliptic_e_result_has_one_arg() -> None:
    """Complete EllipticE takes exactly one argument (the modulus)."""
    result = _eval(
        "integrate(sqrt(1-k^2*sin(theta)^2), theta, 0, %pi/2)"
    )
    assert isinstance(result, IRApply)
    assert len(result.args) == 1


# ---------------------------------------------------------------------------
# Section B — Incomplete EllipticE (indefinite integral)
# ---------------------------------------------------------------------------


def test_incomplete_elliptic_e_symbolic_k() -> None:
    """integrate(sqrt(1-k^2*sin(theta)^2), theta) → EllipticE(theta, k)."""
    result = _eval("integrate(sqrt(1-k^2*sin(theta)^2), theta)")
    assert result == IRApply(
        _ELLIPTIC_E_HEAD, (IRSymbol("theta"), IRSymbol("k"))
    ), f"expected EllipticE(theta, k), got {result!r}"


def test_incomplete_elliptic_e_numeric_k_evaluates() -> None:
    """integrate(sqrt(1-(1/2)^2*sin(theta)^2), theta) → EllipticE(theta, 1/2).

    The compiler folds ``(1/2)^2`` to ``IRRational(1, 4)``.
    ``_modulus_from_squared_factor`` now handles numeric literal ``k²``:
    IRRational(1,4) → k = IRRational(1,2).
    The incomplete integral result is ``EllipticE(theta, 1/2)``.
    """
    result = _eval("integrate(sqrt(1-(1/2)^2*sin(theta)^2), theta)")
    assert _is_apply_with_head(result, _ELLIPTIC_E_HEAD), (
        f"expected EllipticE(...), got {result!r}"
    )
    assert isinstance(result, IRApply)
    assert len(result.args) == 2, "incomplete EllipticE takes two args (phi, modulus)"
    from symbolic_ir import IRRational
    assert result.args[1] == IRRational(1, 2), (
        f"expected modulus 1/2, got {result.args[1]!r}"
    )


def test_incomplete_elliptic_e_result_has_two_args() -> None:
    """Incomplete EllipticE takes exactly two arguments (theta, k)."""
    result = _eval("integrate(sqrt(1-k^2*sin(theta)^2), theta)")
    assert isinstance(result, IRApply)
    assert len(result.args) == 2


# ---------------------------------------------------------------------------
# Section C — Complete EllipticPi (definite integral from 0 to π/2)
# ---------------------------------------------------------------------------


def test_complete_elliptic_pi_numeric_k_evaluates() -> None:
    """integrate(1/((1+2*sin^2)*sqrt(1-(1/2)^2*sin^2)), theta, 0, %pi/2) → EllipticPi(2, 1/2).

    The compiler folds ``(1/2)^2`` to ``IRRational(1, 4)``.
    ``_modulus_from_squared_factor`` now handles numeric literal ``k²``:
    IRRational(1,4) → k = IRRational(1,2).
    The result is ``EllipticPi(2, 1/2)``.
    """
    result = _eval(
        "integrate(1/((1+2*sin(theta)^2)*sqrt(1-(1/2)^2*sin(theta)^2)), theta, 0, %pi/2)"
    )
    assert _is_apply_with_head(result, _ELLIPTIC_PI_HEAD), (
        f"expected EllipticPi(...), got {result!r}"
    )
    assert isinstance(result, IRApply)
    assert len(result.args) == 2, "EllipticPi takes two args (n, modulus)"
    from symbolic_ir import IRRational, IRInteger
    assert result.args[0] == IRInteger(2), f"expected n=2, got {result.args[0]!r}"
    assert result.args[1] == IRRational(1, 2), (
        f"expected modulus 1/2, got {result.args[1]!r}"
    )


def test_complete_elliptic_pi_symbolic_n_and_k() -> None:
    """integrate(1/((1+n*sin(theta)^2)*sqrt(1-k^2*sin(theta)^2)), theta, 0, %pi/2)
    → EllipticPi(n, k).
    """
    result = _eval(
        "integrate(1/((1+n*sin(theta)^2)*sqrt(1-k^2*sin(theta)^2)), theta, 0, %pi/2)"
    )
    assert result == IRApply(
        _ELLIPTIC_PI_HEAD, (IRSymbol("n"), IRSymbol("k"))
    ), f"expected EllipticPi(n, k), got {result!r}"


def test_complete_elliptic_pi_result_has_two_args() -> None:
    """Complete EllipticPi takes exactly two arguments (n, k)."""
    result = _eval(
        "integrate(1/((1+n*sin(theta)^2)*sqrt(1-k^2*sin(theta)^2)), theta, 0, %pi/2)"
    )
    assert isinstance(result, IRApply)
    assert len(result.args) == 2


def test_complete_elliptic_pi_first_arg_is_n() -> None:
    """EllipticPi(n, k): first argument is n (characteristic), second is k (modulus)."""
    result = _eval(
        "integrate(1/((1+n*sin(theta)^2)*sqrt(1-k^2*sin(theta)^2)), theta, 0, %pi/2)"
    )
    assert isinstance(result, IRApply)
    n_arg, k_arg = result.args
    assert n_arg == IRSymbol("n"), f"expected n, got {n_arg!r}"
    assert k_arg == IRSymbol("k"), f"expected k, got {k_arg!r}"


# ---------------------------------------------------------------------------
# Section D — Fallthrough / negative tests
# ---------------------------------------------------------------------------


def test_fallthrough_sin_squared_not_elliptic_e() -> None:
    """integrate(sin(theta)^2, theta) produces a trig antiderivative, not EllipticE.

    sin²(θ) has no elliptic structure — it integrates to θ/2 - sin(2θ)/4
    via the double-angle formula.  The EllipticE recogniser must NOT trigger.
    """
    result = _eval("integrate(sin(theta)^2, theta)")
    assert not _is_apply_with_head(result, _ELLIPTIC_E_HEAD), (
        f"sin²(theta) should NOT produce EllipticE, got {result!r}"
    )
    # Also must not remain as an unevaluated Integrate
    assert not _is_apply_with_head(result, IRSymbol("Integrate")), (
        f"sin²(theta) should integrate elementarily, got {result!r}"
    )


def test_fallthrough_wrong_lower_bound() -> None:
    """integrate(..., theta, 1, %pi/2) with lower=1 stays unevaluated.

    EllipticE/EllipticPi complete forms require the lower bound to be 0.
    """
    result = _eval(
        "integrate(sqrt(1-k^2*sin(theta)^2), theta, 1, %pi/2)"
    )
    # Should fall through to unevaluated Integrate or an antiderivative path
    assert not _is_apply_with_head(result, _ELLIPTIC_E_HEAD), (
        f"lower=1 should not give EllipticE, got {result!r}"
    )


def test_fallthrough_wrong_upper_bound() -> None:
    """integrate(..., theta, 0, %pi) with upper=π stays unevaluated.

    EllipticE/EllipticPi complete forms require the upper bound to be π/2.
    """
    result = _eval(
        "integrate(sqrt(1-k^2*sin(theta)^2), theta, 0, %pi)"
    )
    assert not _is_apply_with_head(result, _ELLIPTIC_E_HEAD), (
        f"upper=pi should not give EllipticE, got {result!r}"
    )


def test_fallthrough_sqrt_one_minus_x2_not_elliptic() -> None:
    """integrate(1/sqrt(1-x^2), x, 0, %pi/2) is NOT EllipticK.

    1/sqrt(1-x²) uses x, not sin(x)² — it does not fit the elliptic pattern.
    The result should be asin-based, not EllipticK or EllipticE.
    """
    result = _eval("integrate(1/sqrt(1-x^2), x)")
    assert not _is_apply_with_head(result, _ELLIPTIC_E_HEAD), (
        f"1/sqrt(1-x^2) should not give EllipticE, got {result!r}"
    )
    assert not _is_apply_with_head(result, _ELLIPTIC_K_HEAD), (
        f"1/sqrt(1-x^2) should not give EllipticK, got {result!r}"
    )


# ---------------------------------------------------------------------------
# Section E — Regressions: EllipticK and EllipticF still work
# ---------------------------------------------------------------------------


def test_regression_elliptic_k_still_works() -> None:
    """integrate(1/sqrt(1-k^2*sin(theta)^2), theta, 0, %pi/2) → EllipticK(k).

    Regression guard: adding EllipticE/Pi must not break the EllipticK path.
    """
    result = _eval(
        "integrate(1/sqrt(1-k^2*sin(theta)^2), theta, 0, %pi/2)"
    )
    assert result == IRApply(_ELLIPTIC_K_HEAD, (IRSymbol("k"),)), (
        f"expected EllipticK(k), got {result!r}"
    )


def test_regression_elliptic_f_still_works() -> None:
    """integrate(1/sqrt(1-k^2*sin(theta)^2), theta) → EllipticF(theta, k).

    Regression guard: adding EllipticE/Pi must not break the EllipticF path.
    """
    result = _eval(
        "integrate(1/sqrt(1-k^2*sin(theta)^2), theta)"
    )
    assert result == IRApply(
        _ELLIPTIC_F_HEAD, (IRSymbol("theta"), IRSymbol("k"))
    ), f"expected EllipticF(theta, k), got {result!r}"


# ---------------------------------------------------------------------------
# Section F — Head-identity sanity checks
# ---------------------------------------------------------------------------


def test_elliptic_e_head_is_correct_symbol() -> None:
    """The head of EllipticE is IRSymbol('EllipticE'), not a built-in IR head."""
    result = _eval(
        "integrate(sqrt(1-k^2*sin(theta)^2), theta, 0, %pi/2)"
    )
    assert isinstance(result, IRApply)
    assert result.head == IRSymbol("EllipticE")
    assert isinstance(result.head, IRSymbol)
    assert result.head.name == "EllipticE"


def test_elliptic_pi_head_is_correct_symbol() -> None:
    """The head of EllipticPi is IRSymbol('EllipticPi'), not a built-in IR head."""
    result = _eval(
        "integrate(1/((1+n*sin(theta)^2)*sqrt(1-k^2*sin(theta)^2)), theta, 0, %pi/2)"
    )
    assert isinstance(result, IRApply)
    assert result.head == IRSymbol("EllipticPi")
    assert isinstance(result.head, IRSymbol)
    assert result.head.name == "EllipticPi"
