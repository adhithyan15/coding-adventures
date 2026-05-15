# Changelog

## 0.1.5

- Parse `.model <name> NPN(...)` and `.model <name> PNP(...)` cards into
  `ModelCard` records.
- Parse `Q` BJT elements into `spice_engine.BJT` instances using `IS`,
  `BF`/`BETA_F`, and `VT` model parameters.
- Remap BJT collector/base/emitter terminals during `.subckt` expansion.

## 0.1.4

- Parse `.model <name> D(...)` cards into `ModelCard` records.
- Parse `D` diode elements into `spice_engine.Diode` instances using `IS` and
  `VT` model parameters.
- Remap diode terminals during `.subckt` expansion.

## 0.1.3

- Add SPICE `H` / CCVS controlled-source parsing, including subcircuit output
  node remapping and local controlling source-name remapping.

## 0.1.2

- Add SPICE `F` / CCCS controlled-source parsing, including subcircuit output
  node remapping and local controlling source-name remapping.

## 0.1.1

- Add SPICE `E` / VCVS controlled-source parsing, including subcircuit node
  remapping for expanded VCVS elements.

## 0.1.0

- Add a first SPICE3 netlist parser slice for linear R/C/L circuits,
  independent V/I sources, VCCS elements, PWL/PULSE/SIN/EXP source waveforms,
  and `.op`, `.tran`, `.dc`, and `.ac` analysis cards.
- Add first `.subckt` / `X` instance expansion for hierarchical netlists made
  from supported primitive elements.
