# spice-engine

SPICE-compatible analog circuit simulator. Modified Nodal Analysis (MNA) with Newton-Raphson DC operating-point solver and forward-Euler transient analysis.

See [`code/specs/spice-engine.md`](../../../specs/spice-engine.md).

## Quick start

```python
from spice_engine import Circuit, Resistor, VoltageSource, dc_op

# Voltage divider: V1 = 10V, R1 = 1k, R2 = 1k -> V_mid should be 5V
circuit = Circuit()
circuit.add(VoltageSource("V1", "vin", "0", voltage=10.0))
circuit.add(Resistor("R1", "vin", "vmid", 1000.0))
circuit.add(Resistor("R2", "vmid", "0", 1000.0))

result = dc_op(circuit)
print(result.node_voltages)        # {"vin": 10.0, "vmid": 5.0}
print(result.branch_currents)      # {"I(V1)": -0.005}  (5 mA flowing in)
print(result.converged)
```

## v0.1.0 scope

- Element classes: `Resistor`, `Capacitor`, `Inductor`, `VoltageSource`, `CurrentSource`, `Diode` (Shockley), `Mosfet` (uses mosfet_models.MOSFET).
- `Circuit` container with `add()`.
- MNA matrix construction (one stamp function per element type).
- Gaussian elimination linear solver with partial pivoting.
- `dc_op(circuit, max_iterations, tol)`: Newton-Raphson DC solver. Iterates linearization until x converges to within tol.
- `transient(circuit, t_stop, t_step)`: forward-Euler with capacitor companion models (C/h conductance + history current source).
- Diode + MOSFET nonlinearities handled via Newton iteration.
- Ground node identifiers: `'0'`, `'gnd'`, `'GND'`.

## Out of scope (v0.2.0)

- AC analysis (.ac) with frequency sweep.
- Backward-Euler / trapezoidal / Gear-2 transient methods.
- Adaptive timestep with LTE control.
- Convergence aids: Gmin stepping, source stepping, pseudo-transient continuation.
- SPICE3 netlist parser (`.tran`, `.dc`, `.ac` directives).
- BJTs, JFETs.
- Verilog-A behavioral models.
- Sparse matrix solver (currently O(n³) Gaussian elimination).

MIT.
