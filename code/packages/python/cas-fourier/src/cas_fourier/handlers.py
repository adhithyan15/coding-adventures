"""VM handlers for the Fourier transform IR heads.

These handlers are installed in the symbolic VM's ``SymbolicBackend``
via ``cas_handlers.py``. They follow the standard handler signature::

    def handler(vm: VM, expr: IRApply) -> IRNode

Each handler:

1. Validates argument count and types.
2. Delegates to the transform table (``fourier_transform`` or
   ``ifourier_transform``).
3. Calls ``vm.eval`` on the result to collapse any numeric
   sub-expressions that the table left as raw IR.
4. Falls through (returns ``expr`` unchanged) if the input is malformed.

Handler table key:
    "Fourier"  → fourier_handler
    "IFourier" → ifourier_handler
"""

from __future__ import annotations

from typing import TYPE_CHECKING

from symbolic_ir import IRApply, IRNode, IRSymbol

from cas_fourier.heads import FOURIER, IFOURIER
from cas_fourier.inverse import ifourier_transform
from cas_fourier.table import fourier_transform

if TYPE_CHECKING:
    from symbolic_vm.vm import VM


def fourier_handler(vm: VM, expr: IRApply) -> IRNode:
    """Evaluate ``Fourier(f, t, ω)`` — symbolic Fourier transform.

    Calling convention: ``Fourier(f, t, ω)``
        f  — the time-domain expression (any IRNode)
        t  — the integration variable   (must be IRSymbol)
        ω  — the frequency variable     (must be IRSymbol)

    Behaviour
    ---------
    - Wrong arity (not 3 args): return ``expr`` unevaluated.
    - Non-symbol ``t`` or ``ω``: return ``expr`` unevaluated.
    - Otherwise: compute ``fourier_transform(f, t, ω)`` and run the
      result through ``vm.eval()`` so that numeric sub-expressions
      collapse (e.g. ``Add(1, 1)`` → ``2``).

    Examples::

        Fourier(DiracDelta(t), t, ω) → 1
        Fourier(1, t, ω)             → Mul(Mul(2, %pi), DiracDelta(ω))
        Fourier(unknown(t), t, ω)    → Fourier(unknown(t), t, ω)  [unevaluated]
    """
    if len(expr.args) != 3:
        return expr

    f, t, omega = expr.args

    if not isinstance(t, IRSymbol) or not isinstance(omega, IRSymbol):
        return expr

    result = fourier_transform(f, t, omega)
    return vm.eval(result)


def ifourier_handler(vm: VM, expr: IRApply) -> IRNode:
    """Evaluate ``IFourier(F, ω, t)`` — symbolic inverse Fourier transform.

    Calling convention: ``IFourier(F, ω, t)``
        F  — the frequency-domain expression (any IRNode)
        ω  — the frequency variable          (must be IRSymbol)
        t  — the time variable               (must be IRSymbol)

    Behaviour
    ---------
    - Wrong arity (not 3 args): return ``expr`` unevaluated.
    - Non-symbol ``ω`` or ``t``: return ``expr`` unevaluated.
    - Otherwise: compute ``ifourier_transform(F, ω, t)`` and run the
      result through ``vm.eval()``.

    Examples::

        IFourier(1, ω, t)             → DiracDelta(t)
        IFourier(DiracDelta(ω), ω, t) → Div(1, Mul(2, %pi))
        IFourier(unknown, ω, t)       → IFourier(unknown, ω, t)  [unevaluated]
    """
    if len(expr.args) != 3:
        return expr

    F, omega, t = expr.args

    if not isinstance(omega, IRSymbol) or not isinstance(t, IRSymbol):
        return expr

    result = ifourier_transform(F, omega, t)
    return vm.eval(result)


def build_fourier_handler_table() -> dict[str, object]:
    """Return the handler table fragment for the symbolic VM.

    To be merged into the VM's CAS handler table via ``**``-spread
    in ``cas_handlers.build_cas_handler_table()``:

    .. code-block:: python

        from cas_fourier.handlers import build_fourier_handler_table
        table = {
            ...,
            **build_fourier_handler_table(),
        }

    Returns
    -------
    dict[str, Handler]
        Mapping from IR head name to handler callable.
    """
    return {
        FOURIER.name: fourier_handler,
        IFOURIER.name: ifourier_handler,
    }
