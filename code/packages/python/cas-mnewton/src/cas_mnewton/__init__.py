"""cas-mnewton — Newton's method numeric root finder.

Provides ``mnewton(f, x, x0)`` which iterates Newton's method
symbolically/numerically: x_{n+1} = x_n - f(x_n)/f'(x_n).

Public API
----------
``mnewton_solve(f_ir, x_sym, x0_ir, tol, max_iter)``
    Pure function — no VM dependency; caller provides eval_fn.
``build_mnewton_handler_table()``
    Returns the handler dict for wiring into SymbolicBackend.
"""

from cas_mnewton.handlers import build_mnewton_handler_table
from cas_mnewton.newton import MNewtonError, mnewton_solve

__all__ = ["build_mnewton_handler_table", "mnewton_solve", "MNewtonError"]
