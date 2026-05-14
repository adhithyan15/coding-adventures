# spice-engine

`spice-engine` provides SPICE-style circuit analysis primitives for Rust.

The initial slice implements DC operating-point analysis for linear circuits
using modified nodal analysis (MNA). It supports resistors, independent current
sources, independent voltage sources, ground aliases, node voltages, and voltage
source branch currents.
