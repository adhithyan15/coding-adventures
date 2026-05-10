# cas-ode-numeric

Fixed-step Runge-Kutta 4 numeric ODE integrator for the MACSYMA/SPICE symbolic VM pipeline.

## What it does

`cas-ode-numeric` provides `rk4_solve` — a function that numerically integrates a system of first-order ODEs:

```
dy₁/dt = f₁(t, y₁, …, yₙ)
⋮
dyₙ/dt = fₙ(t, y₁, …, yₙ)
```

where each `fᵢ` is expressed as a **symbolic IR tree** and evaluated through the `symbolic_vm` at each integration step. This enables nonlinear circuit transient analysis (SPICE-style) where the device equations cannot be integrated in closed form.

## Why RK4?

RK4 (classical 4th-order Runge-Kutta) balances accuracy and simplicity:

- 4th-order local truncation error: O(h⁵), global error O(h⁴)
- 4 function evaluations per step
- No history needed (single-step method)
- Educational clarity — each stage has a direct physical interpretation

Production SPICE uses Gear-2 or trapezoidal methods for better stiff-ODE stability, but RK4 is correct for educational simulators and weakly stiff circuits.

## Quick start

```python
from symbolic_ir import IRApply, IRInteger, IRSymbol, MUL
from symbolic_vm import VM
from macsyma_runtime import MacsymaBackend
from cas_ode_numeric import rk4_solve
import math

vm = VM(MacsymaBackend())
y = IRSymbol("y")

# dy/dt = -2y, y(0) = 1
f = IRApply(MUL, (IRInteger(-2), y))
traj = rk4_solve([f], [1.0], (0.0, 1.0), 0.001, vm, state_names=["y"])

t_end, (y_end,) = traj[-1]
print(f"y(1) ≈ {y_end:.6f}  (exact: {math.exp(-2.0):.6f})")
# y(1) ≈ 0.135335  (exact: 0.135335)
```

## SPICE application

For a series RLC circuit with unit step input (L=1, R=0.5, C=1):

```python
q = IRSymbol("q")   # charge
i = IRSymbol("i")   # current

# dq/dt = i
# di/dt = 1 - 0.5·i - q
f_q = i
f_i = IRApply(SUB, (IRApply(SUB, (IRInteger(1), IRApply(MUL, (IRFloat(0.5), i)))), q))

traj = rk4_solve([f_q, f_i], [0.0, 0.0], (0.0, 20.0), 0.01, vm,
                  state_names=["q", "i"])
```

## Stack position

```
macsyma-repl
└── macsyma-runtime   ← wires cas-ode (symbolic), cas-laplace, ...
    └── symbolic-vm
        └── symbolic-ir

cas-ode-numeric       ← this package (numeric RK4, standalone)
└── symbolic-vm
└── symbolic-ir
```

`cas-ode-numeric` is a *sibling* to `macsyma-runtime` — it does not depend on it. The SPICE program links both.
