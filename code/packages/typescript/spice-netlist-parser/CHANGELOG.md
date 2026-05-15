# Changelog

## Unreleased

- Parse independent-source `AC <magnitude> [phase]` specs, including combined
  `DC <bias> AC <magnitude> [phase]` forms, and pass the AC phasor through to
  TypeScript SPICE engine AC analysis while preserving DC bias.

## 0.1.6

- Add SPICE `M` MOSFET element parsing via `.model <name> NMOS(...)` and
  `.model <name> PMOS(...)` cards with Level-1 parameter aliases and
  per-instance overrides such as `W=...` and `L=...`.
- Remap MOSFET drain/gate/source/body terminals during subcircuit expansion.

## 0.1.5

- Add SPICE `Q` BJT element parsing via `.model <name> NPN(...)` and
  `.model <name> PNP(...)` cards with `IS`, `BF` / `BETA_F`, and `VT`
  parameters, including subcircuit collector/base/emitter remapping.

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
