# Changelog

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
