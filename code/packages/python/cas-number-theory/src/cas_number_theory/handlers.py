"""VM handlers for number-theory IR heads."""
from __future__ import annotations
from collections.abc import Callable
from typing import TYPE_CHECKING

from symbolic_ir import IRApply, IRInteger, IRNode, IRSymbol

from cas_number_theory.primality import is_prime, next_prime, prev_prime
from cas_number_theory.factorize import factor_integer
from cas_number_theory.arithmetic import divisors, totient, moebius_mu, jacobi_symbol, integer_length
from cas_number_theory.crt import chinese_remainder

if TYPE_CHECKING:
    from symbolic_vm.vm import VM

# Handler type mirrors symbolic_vm.backend.Handler — a callable that takes
# a VM and an IRApply and returns an IRNode. Defined locally to avoid a
# runtime dependency on symbolic_vm (which would create a circular dep).
Handler = Callable[["VM", IRApply], IRNode]

_LIST = IRSymbol("List")
_TRUE = IRSymbol("True")
_FALSE = IRSymbol("False")


def _as_int(node: IRNode) -> int | None:
    """Extract integer value from IRInteger, or None."""
    if isinstance(node, IRInteger):
        return node.value
    return None


def is_prime_handler(_vm: "VM", expr: IRApply) -> IRNode:
    if len(expr.args) != 1:
        return expr
    n = _as_int(expr.args[0])
    if n is None:
        return expr
    return _TRUE if is_prime(n) else _FALSE


def next_prime_handler(_vm: "VM", expr: IRApply) -> IRNode:
    if len(expr.args) != 1:
        return expr
    n = _as_int(expr.args[0])
    if n is None:
        return expr
    return IRInteger(next_prime(n))


def prev_prime_handler(_vm: "VM", expr: IRApply) -> IRNode:
    if len(expr.args) != 1:
        return expr
    n = _as_int(expr.args[0])
    if n is None:
        return expr
    result = prev_prime(n)
    if result is None:
        return expr  # unevaluated for n <= 2
    return IRInteger(result)


def factor_integer_handler(_vm: "VM", expr: IRApply) -> IRNode:
    if len(expr.args) != 1:
        return expr
    n = _as_int(expr.args[0])
    if n is None or n <= 0:
        return expr
    factors = factor_integer(n)
    pairs = tuple(
        IRApply(_LIST, (IRInteger(p), IRInteger(e)))
        for p, e in factors
    )
    return IRApply(_LIST, pairs)


def divisors_handler(_vm: "VM", expr: IRApply) -> IRNode:
    if len(expr.args) != 1:
        return expr
    n = _as_int(expr.args[0])
    if n is None or n <= 0:
        return expr
    return IRApply(_LIST, tuple(IRInteger(d) for d in divisors(n)))


def totient_handler(_vm: "VM", expr: IRApply) -> IRNode:
    if len(expr.args) != 1:
        return expr
    n = _as_int(expr.args[0])
    if n is None or n <= 0:
        return expr
    return IRInteger(totient(n))


def moebius_mu_handler(_vm: "VM", expr: IRApply) -> IRNode:
    if len(expr.args) != 1:
        return expr
    n = _as_int(expr.args[0])
    if n is None or n <= 0:
        return expr
    return IRInteger(moebius_mu(n))


def jacobi_symbol_handler(_vm: "VM", expr: IRApply) -> IRNode:
    if len(expr.args) != 2:
        return expr
    a = _as_int(expr.args[0])
    n = _as_int(expr.args[1])
    if a is None or n is None or n <= 0 or n % 2 == 0:
        return expr
    return IRInteger(jacobi_symbol(a, n))


def chinese_remainder_handler(_vm: "VM", expr: IRApply) -> IRNode:
    if len(expr.args) != 2:
        return expr
    r_node, m_node = expr.args
    if not (isinstance(r_node, IRApply) and r_node.head.name == "List"):
        return expr
    if not (isinstance(m_node, IRApply) and m_node.head.name == "List"):
        return expr
    remainders = [_as_int(a) for a in r_node.args]
    moduli = [_as_int(a) for a in m_node.args]
    if any(x is None for x in remainders) or any(x is None for x in moduli):
        return expr
    result = chinese_remainder(remainders, moduli)  # type: ignore[arg-type]
    if result is None:
        return expr  # non-coprime moduli
    return IRInteger(result)


def integer_length_handler(_vm: "VM", expr: IRApply) -> IRNode:
    nargs = len(expr.args)
    if nargs not in (1, 2):
        return expr
    n = _as_int(expr.args[0])
    if n is None:
        return expr
    if nargs == 2:
        b = _as_int(expr.args[1])
        if b is None or b < 2:
            return expr
        return IRInteger(integer_length(n, b))
    return IRInteger(integer_length(n))


def build_number_theory_handler_table() -> dict[str, Handler]:
    """Return handler table for number-theory IR heads."""
    return {
        "IsPrime": is_prime_handler,
        "NextPrime": next_prime_handler,
        "PrevPrime": prev_prime_handler,
        "FactorInteger": factor_integer_handler,
        "Divisors": divisors_handler,
        "Totient": totient_handler,
        "MoebiusMu": moebius_mu_handler,
        "JacobiSymbol": jacobi_symbol_handler,
        "ChineseRemainder": chinese_remainder_handler,
        "IntegerLength": integer_length_handler,
    }
