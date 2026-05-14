# @coding-adventures/spice-engine

`@coding-adventures/spice-engine` provides SPICE-style circuit analysis
primitives for TypeScript.

The current slices implement DC operating-point analysis, DC source sweeps, DC
small-signal transfer-function analysis, AC small-signal frequency sweeps, and
fixed-step RC/RL transient analysis for linear circuits using modified nodal
analysis (MNA). The package supports resistors, capacitors, inductors,
independent current sources, independent voltage sources, ground aliases, node
voltages, voltage source branch currents, and backward-Euler reactive-element
companion models.
