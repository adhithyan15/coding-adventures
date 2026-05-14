# @coding-adventures/spice-engine

`@coding-adventures/spice-engine` provides SPICE-style circuit analysis
primitives for TypeScript.

The initial slice implements DC operating-point analysis for linear circuits
using modified nodal analysis (MNA). It supports resistors, independent current
sources, independent voltage sources, ground aliases, node voltages, and voltage
source branch currents.
