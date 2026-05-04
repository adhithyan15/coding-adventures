"""IR head symbols for the Laplace transform package.

These symbols are also imported by cas-fourier so they are canonical
here — do not redefine DiracDelta or UnitStep in any other package.

Mathematical context
--------------------
The Laplace transform is a powerful integral transform that converts a
function of time f(t) into a function of complex frequency F(s):

    L{f(t)} = F(s) = ∫₀^∞ f(t) e^{-st} dt

It is the workhorse of control theory, signal processing, and differential
equation solving. The inverse Laplace transform (ILT) recovers f(t) from F(s).

Special functions
-----------------
DiracDelta(t) — the Dirac delta distribution, δ(t). Informally it is "zero
everywhere except at t=0 where it is infinity, and ∫δ(t)dt = 1". The key
property for Laplace transforms is L{δ(t)} = 1.

UnitStep(t) — the Heaviside step function, u(t) or H(t). It equals 0 for
t < 0, 1 for t > 0, and by convention ½ at t = 0. Used to model systems
that are "switched on" at t=0. L{u(t)} = 1/s.
"""

from __future__ import annotations

from symbolic_ir import IRSymbol

LAPLACE = IRSymbol("Laplace")
ILT = IRSymbol("ILT")  # Inverse Laplace Transform
DIRAC_DELTA = IRSymbol("DiracDelta")  # δ(t) — Dirac delta distribution
UNIT_STEP = IRSymbol("UnitStep")  # u(t) — Heaviside unit step
