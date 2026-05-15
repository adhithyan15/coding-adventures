"""MACSYMA stress test — 53 graduate-level math problems.

Tests symbolic computation capability across:
- Calculus (derivatives, integrals, limits)
- Algebra (factor, solve, expand)
- ODEs (ode2)
- Linear algebra (matrices, determinants)
- Series (Taylor, Sum)
- Complex numbers (Re, Im)
- Special functions
"""
from __future__ import annotations

import sys
import traceback

from macsyma_compiler import compile_macsyma
from macsyma_compiler.compiler import _STANDARD_FUNCTIONS
from macsyma_parser import parse_macsyma
from symbolic_vm import VM
from macsyma_runtime import MacsymaBackend, extend_compiler_name_table

extend_compiler_name_table(_STANDARD_FUNCTIONS)


def _eval(src: str) -> object:
    tree = parse_macsyma(src + ";")
    ir = compile_macsyma(tree)
    vm = VM(MacsymaBackend())
    if isinstance(ir, list):
        return vm.eval_program(ir)
    return vm.eval(ir)


def _result_str(r: object) -> str:
    try:
        return str(r)
    except Exception:
        return repr(r)


TESTS: list[tuple[str, str, str]] = [
    # --- Differentiation -------------------------------------------------------
    ("diff(x^3, x)",            "3*x^2",          "d/dx x^3"),
    ("diff(x^3, x, 2)",         "6*x",            "d^2/dx^2 x^3"),
    ("diff(sin(x)*cos(x), x)",  "cos^2-sin^2",    "d/dx sin*cos"),
    ("diff(exp(x^2), x)",       "2*x*e^(x^2)",    "chain rule e^(x^2)"),
    ("diff(log(x), x)",         "1/x",            "d/dx log(x)"),
    # --- Integration -----------------------------------------------------------
    ("integrate(x^2, x)",                "x^3/3",           "integral x^2"),
    ("integrate(sin(x), x)",             "-cos(x)",         "integral sin(x)"),
    ("integrate(exp(x), x)",             "e^x",             "integral e^x"),
    ("integrate(1/(1+x^2), x)",          "atan(x)",         "integral 1/(1+x^2)"),
    ("integrate(x*exp(x), x)",           "xe^x - e^x",      "IBP: x*e^x"),
    ("integrate(1/x, x)",                "log(x)",          "integral 1/x"),
    # --- Limits ----------------------------------------------------------------
    ("limit(sin(x)/x, x, 0)",           "1",               "L'Hopital sin(x)/x"),
    ("limit((1-cos(x))/x^2, x, 0)",     "1/2",             "L'Hopital (1-cos)/x^2"),
    ("limit(x*log(x), x, 0, plus)",     "0",               "0*inf form x*log(x)"),
    ("limit(exp(-x), x, inf)",          "0",               "exp(-inf)"),
    ("limit(x^2, x, 3)",                "9",               "direct sub x^2 at 3"),
    # --- Algebra ---------------------------------------------------------------
    ("factor(x^2 - 1)",                 "(x-1)*(x+1)",     "factor x^2-1"),
    ("factor(x^2 - y^2)",               "(x-y)*(x+y)",     "factor x^2-y^2"),
    ("expand((x+1)^2)",                 "x^2+2x+1",        "expand (x+1)^2"),
    ("expand((a+b)^2)",                 "a^2+2ab+b^2",     "expand (a+b)^2"),
    ("expand((a+b)^3)",                 "a^3+3a^2b+...",   "expand (a+b)^3"),
    # --- Solve -----------------------------------------------------------------
    ("solve(x^2 - 4, x)",               "[x=2, x=-2]",     "solve x^2=4"),
    ("solve(x^2 + 1, x)",               "complex roots",   "solve x^2=-1"),
    ("solve(x^3 - x, x)",               "[0,1,-1]",        "solve x^3=x"),
    # --- Series ----------------------------------------------------------------
    ("taylor(sin(x), x, 0, 5)",         "x-x^3/6+x^5/120","Taylor sin"),
    ("taylor(exp(x), x, 0, 4)",         "1+x+x^2/2+...",  "Taylor exp"),
    ("sum(k, k, 1, n)",                 "n*(n+1)/2",       "sum k from 1 to n"),
    ("sum(k^2, k, 1, 5)",               "55",              "sum k^2 1..5"),
    ("sum(1/2^k, k, 0, inf)",           "2",               "geometric 1/2^k"),
    ("sum(1/3^k, k, 0, inf)",           "3/2",             "geometric 1/3^k"),
    # --- ODEs ------------------------------------------------------------------
    ("ode2(diff(y, x) + y, y, x)",              "y=C1*e^(-x)",     "ODE y'+y=0"),
    ("ode2(diff(y, x) - 2*y, y, x)",            "y=C1*e^(2x)",     "ODE y'=2y"),
    ("ode2(diff(y, x, 2) + y, y, x)",           "y=C1*sin+C2*cos", "ODE y''+y=0"),
    # --- Matrices --------------------------------------------------------------
    ("determinant(matrix([1,2],[3,4]))",         "-2",              "det 2x2"),
    ("determinant(matrix([1,0,0],[0,2,0],[0,0,3]))",  "6",         "det 3x3 diag"),
    ("determinant(matrix([1,2,3],[4,5,6],[7,8,9]))",  "0",         "det 3x3 singular"),
    # --- Complex ---------------------------------------------------------------
    ("realpart(3 + 2*%i)",              "3",               "Re(3+2i)"),
    ("imagpart(3 + 2*%i)",             "2",               "Im(3+2i)"),
    ("re(3 + 2*%i)",                   "3",               "re alias"),
    ("im(3 + 2*%i)",                   "2",               "im alias"),
    # --- Elliptic integrals ----------------------------------------------------
    ("integrate(1/sqrt(1-0.5^2*sin(theta)^2), theta, 0, %pi/2)",
     "EllipticK(0.5)", "EllipticK"),
    ("integrate(sqrt(1-0.5^2*sin(theta)^2), theta, 0, %pi/2)",
     "EllipticE(0.5)", "EllipticE complete"),
    # --- Simplification --------------------------------------------------------
    ("radcan(exp(2*log(x)))",           "x^2",             "radcan e^(2ln x)"),
    ("ratsimp(x^2/x)",                  "x",               "ratsimp x^2/x"),
    ("trigsimp(sin(x)^2 + cos(x)^2)",   "1",               "trig identity"),
    # --- Pattern matching / rules ----------------------------------------------
    ("defrule(r1, cos(a)^2, 1 - sin(a)^2); apply1(cos(x)^2, r1)",
     "1-sin(x)^2", "defrule + apply1"),
    # --- Laplace ---------------------------------------------------------------
    ("laplace(exp(-t), t, s)",          "1/(s+1)",         "Laplace e^-t"),
    ("laplace(sin(t), t, s)",           "1/(s^2+1)",       "Laplace sin(t)"),
    # --- Assume / is -----------------------------------------------------------
    ("assume(x > 0); is(x > 0)",        "true",            "assume/is"),
    # --- Show time / display ---------------------------------------------------
    ("showtime: true",                  "done-ish",        "showtime switch"),
]

ok = warn = err = 0
for src, expected, label in TESTS:
    try:
        result = _eval(src)
        rs = _result_str(result)
        # Check if it returned unevaluated (head matches original call)
        unevaluated = False
        head = src.split("(")[0].strip()
        # Rough heuristic: if result string starts with the head name capitalized,
        # it's probably unevaluated
        head_cap = head[0].upper() + head[1:] if head else ""
        if rs.startswith(head_cap + "(") or rs.startswith(head + "("):
            unevaluated = True

        if unevaluated:
            print(f"[???] {label!r:35}  src={src!r}")
            print(f"       result={rs[:80]}")
            warn += 1
        else:
            print(f"[OK ] {label!r:35}  => {rs[:60]}")
            ok += 1
    except Exception as e:
        print(f"[ERR] {label!r:35}  src={src!r}")
        print(f"       {type(e).__name__}: {str(e)[:100]}")
        err += 1

total = ok + warn + err
print()
print(f"Results: {ok}/{total} fully evaluated, {warn} unevaluated, {err} errors")
print(f"Score: {100*ok//total}% ({ok} passing of {total} total)")
