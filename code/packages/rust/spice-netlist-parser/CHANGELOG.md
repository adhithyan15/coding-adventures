# Changelog

## Unreleased

- Parse SPICE `.tf V(output_node) input_source` transfer-function analysis
  cards.
- Parse SPICE `.sens V(output_node)` DC sensitivity analysis cards.
- Parse SPICE `.mc V(output_node) n_trials [tolerance] [distribution] [seed]`
  Monte Carlo DC analysis cards.

## 0.1.7

- Add independent-source `AC <magnitude> [phase]` parsing, including combined
  `DC <bias> AC <magnitude> [phase]` forms for AC analysis with separate DC
  bias and small-signal excitation.

## 0.1.6

- Add SPICE `M` MOSFET element parsing via `.model <name> NMOS|PMOS(...)`
  Level-1 cards, per-instance parameter overrides such as `W=...` and `L=...`,
  and subcircuit drain/gate/source/body terminal remapping.

## 0.1.5

- Add SPICE `Q` BJT element parsing via `.model <name> NPN|PNP(...)` cards
  with `IS`, `BF` / `BETA_F`, and `VT` parameters, including subcircuit
  terminal remapping.

## 0.1.4

- Add SPICE `D` diode element parsing via `.model <name> D(...)` cards with
  `IS` and `VT` parameters, including subcircuit terminal remapping.

## 0.1.3

- Add SPICE `H` / CCVS controlled-source parsing, including subcircuit
  controlling-source name remapping for expanded CCVS elements.

## 0.1.2

- Add SPICE `F` / CCCS controlled-source parsing, including subcircuit
  controlling-source name remapping for expanded CCCS elements.

## 0.1.1

- Add SPICE `E` / VCVS controlled-source parsing, including subcircuit node
  remapping for expanded VCVS elements.

## 0.1.0

- Add a first SPICE3 netlist parser slice for linear R/C/L circuits,
  independent V/I sources, VCCS elements, PWL/PULSE/SIN/EXP source waveforms,
  and `.op`, `.tran`, `.dc`, and `.ac` analysis cards.
- Add first `.subckt` / `X` instance expansion for hierarchical netlists made
  from supported primitive elements.
