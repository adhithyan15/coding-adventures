"""cas_summation — Symbolic summation and product evaluation.

Public API
----------
evaluate_sum(f, k, lo, hi, vm) -> IRNode
    Evaluate Σ_{k=lo}^{hi} f(k) symbolically.

evaluate_product(f, k, lo, hi, vm) -> IRNode
    Evaluate Π_{k=lo}^{hi} f(k) symbolically.

Both return an IR node with the closed form, or the unevaluated
``SUM``/``PRODUCT`` node when no closed form is known.

Supported sum families:
- Constant summand:        Σ c = c·(b−a+1)
- Geometric finite/inf:    Σ r^k = r^a·(r^(b−a+1)−1)/(r−1)  or  r^a/(1−r)
- Power of index (m≤5):    Σ c·k^m via Faulhaber polynomial
- Classic infinite series: Basel (π²/6, π⁴/90), Leibniz (π/4), e, exp(x)
- Numeric small range:     direct computation for concrete integer bounds

Supported product families:
- Constant:    Π c = c^(b−a+1)
- Factorial:   Π_{k=1}^{n} k = GammaFunc(n+1)
- Scaled:      Π_{k=1}^{n} c·k = c^n · GammaFunc(n+1)
- Numeric:     direct for concrete integer bounds

See the spec at ``code/specs/phase25-symbolic-summation.md`` and the
gap-analysis at ``code/specs/macsyma-gap-analysis-phases18-25.md``
for the full mathematical background.
"""

from cas_summation.summation import evaluate_product, evaluate_sum

__all__ = [
    "evaluate_sum",
    "evaluate_product",
]
