# @coding-adventures/spice-engine

`@coding-adventures/spice-engine` provides SPICE-style circuit analysis
primitives for TypeScript.

The current slices implement DC operating-point analysis and fixed-step RC
transient analysis for linear circuits using modified nodal analysis (MNA). The
package supports resistors, capacitors, independent current sources,
independent voltage sources, ground aliases, node voltages, voltage source
branch currents, and backward-Euler capacitor companion models.
