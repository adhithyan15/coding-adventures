# Changelog

## Unreleased

- Validate and lower MOS model-card `CJS` and `CJD` aliases as canonical `CBS`
  and `CBD` junction capacitances.
- Reject negative and non-finite MOS model-card `AF` values before lowering
  valid flicker-noise exponents.
- Reject negative and non-finite MOS model-card `KF` values before lowering
  valid flicker-noise coefficients.
- Reject MOS model-card `FC` values outside `[0, 1)` or non-finite values
  before lowering valid forward-bias depletion coefficients.
- Reject negative and non-finite MOS model-card `MJSW` values before lowering
  valid sidewall-junction grading coefficients.
- Reject negative and non-finite MOS model-card `MJ` values before lowering
  valid bottom-junction grading coefficients.
- Reject zero, negative, and non-finite MOS model-card `PB` values before
  lowering valid bulk-junction potentials.
- Reject negative and non-finite MOS model-card `JS` values before lowering
  valid junction saturation-current densities.
- Reject negative and non-finite MOS model-card `CJSW` values before lowering
  valid sidewall-junction capacitance densities.
- Reject negative and non-finite MOS model-card `CJ` values before lowering
  valid bottom-junction capacitance densities.
- Reject negative and non-finite MOS instance `PS` values before lowering
  valid source diffusion perimeters.
- Reject negative and non-finite MOS instance `PD` values before lowering
  valid drain diffusion perimeters.
- Reject negative and non-finite MOS instance `AS` values before lowering
  valid source diffusion areas.
- Reject negative and non-finite MOS instance `AD` values before lowering
  valid drain diffusion areas.
- Reject negative and non-finite MOS instance `NRS` values before lowering
  valid source diffusion square counts.
- Reject negative and non-finite MOS instance `NRD` values before lowering
  valid drain diffusion square counts.
- Reject MOS model-card `LD` values that are non-finite, negative, or leave a
  non-positive effective Level-1 channel length.
- Reject negative and non-finite MOS model-card `RSH` values before lowering
  Level-1 sheet resistance.
- Reject negative and non-finite MOS model-card `RS` values before lowering
  Level-1 source resistance.
- Reject negative and non-finite MOS model-card `RD` values before lowering
  Level-1 drain resistance.
- Reject zero, negative, and non-finite MOS model-card `TNOM` / `T_NOM` values
  before lowering Level-1 nominal temperature.
- Reject zero, negative, and non-finite MOS model-card `IS` values before
  lowering Level-1 saturation current.
- Reject zero, negative, and non-finite MOS model-card `L` values before
  lowering Level-1 default length.
- Reject zero, negative, and non-finite MOS model-card `W` values before
  lowering Level-1 default width.
- Reject zero, negative, and non-finite MOS model-card `PHI` values before
  lowering Level-1 surface potential.
- Reject negative and non-finite MOS model-card `GAMMA` values before lowering
  Level-1 body effect.
- Reject non-finite MOS model-card `LAMBDA` / `LAM` values and lower the `LAM`
  alias into Level-1 channel-length modulation.
- Reject non-finite MOS model-card `VT0` / `VTO` / `VTH` values and lower the
  `VTH` alias into Level-1 threshold voltage.
- Reject zero, negative, and non-finite explicit MOS model-card `KP` values.
- Lower and validate MOS model-card `U0` / `UO`, deriving `KP` from surface
  mobility and `TOX` when no explicit transconductance is supplied.
- Validate and lower MOS model-card `TOX` into Level-1 oxide thickness instead
  of silently discarding it.
- Reject non-finite and non-Level-1 MOS model-card `LEVEL` values while
  preserving explicit and implicit Level 1 cards.
- Parse `.save`, scoped or global `.probe`, and `.measure` / `.meas` cards,
  and expose `select_outputs()` / `measure_results()` helpers plus matching
  `ParsedNetlist` methods for analysis-plan results.
- Add a deck execution layer with `build_analysis_plan()`, `run_analysis_plan()`,
  `run_netlist()`, plus matching `ParsedNetlist` methods for runnable `.op`,
  `.dc`, `.ac dec` / `.ac log`, and `.tran` cards.

## 0.3.0 — 2026-06-05

- Resolve `.temp` cards into Kelvin engine-call temperatures and let explicit
  `.noise temp=<kelvin>` overrides win over deck-level operating temperatures.
- Route selected `.options` keys into engine-call helpers:
  `dc_op_kwargs()` for DC Newton options and `transient_kwargs()` for
  transient method/adaptive-step options.
- Parse SPICE `.four <frequency> <V(node)|I(source)>...` Fourier-analysis
  cards.
- Parse SPICE `.print <analysis> <V(node)|I(source)>...` and
  `.plot <analysis> <V(node)|I(source)>...` output cards.
- Parse SPICE `.temp <celsius> [celsius ...]` operating-temperature cards.
- Parse MOS Level-1 capacitance parameters with `.model ... NMOS|PMOS(... CGSO=<c>
  CGDO=<c> CGBO=<c> CBS=<c> CBD=<c>)`.
- Parse diode model-card emission coefficients with `.model ... D(... N=<n>)`
  and pass them into Python `Diode` elements.
- Parse diode model-card reverse-breakdown parameters with
  `.model ... D(... BV=<v> IBV=<i>)`.
- Parse diode model-card junction capacitance with
  `.model ... D(... CJO=<c>)` / `.model ... D(... CJ0=<c>)`.
- Parse diode model-card transit time with `.model ... D(... TT=<time>)`.
- Parse BJT model-card capacitances with `.model ... NPN|PNP(... CJE=<c>
  CJC=<c>)` and pass them into Python `BJT` elements.
- Parse BJT model-card forward transit time with
  `.model ... NPN|PNP(... TF=<time>)`.
- Parse BJT model-card reverse transit time with
  `.model ... NPN|PNP(... TR=<time>)`.
- Parse and validate transient integration methods from
  `.tran ... method=<euler|trap|gear2>`, and expose fallback routing from
  `.options method=<...>`.
- Parse conservative SPICE `T` transmission-line cards of the form
  `Tname n1 n2 n3 n4 Z0=<ohms> TD=<seconds>`, including subcircuit node
  remapping and validation for unsupported, missing, non-finite, and
  non-positive parameters.
- Reject SPICE `K` mutual-inductor cards that reference missing inductors or
  use non-finite coupling coefficients.
- Parse SPICE `K` mutual-inductor cards into `MutualInductor` elements,
  including subcircuit-local inductor reference remapping.

## 0.2.0 — 2026-05-16

- Parse SPICE `J` JFET elements via `.model <name> NJF(...)` and
  `.model <name> PJF(...)` cards with `BETA` / `B`, `VTO`, and `LAMBDA`
  parameters, including subcircuit drain/gate/source remapping.
- Parse capacitor `IC=<voltage>` initial-voltage parameters.
- Parse inductor `IC=<current>` initial-current parameters.
- Parse SPICE independent source `AC <magnitude> [phase]` specifications for
  voltage and current sources, including `DC <value> AC ...` mixed bias/input
  forms.
- Parse SPICE `.tf V(output_node) input_source` transfer-function analysis
  cards.
- Parse SPICE `.sens V(output_node)` DC sensitivity analysis cards.
- Parse SPICE `.mc V(output_node) n_trials [tolerance] [distribution] [seed]`
  Monte Carlo DC analysis cards.
- Parse SPICE `.noise V(output_node) input_source [freq ...] [temp=<kelvin>]`
  AC noise analysis cards.
- Parse SPICE `.options key=value ...` simulator-options cards.

## 0.1.6

- Parse `.model <name> NMOS(...)` and `.model <name> PMOS(...)` cards into
  `mosfet_models.MOSFET` Level-1 wrappers.
- Parse `M` MOSFET elements into `spice_engine.Mosfet` instances, including
  per-instance Level-1 parameter overrides such as `W=...` and `L=...`.
- Remap MOSFET drain/gate/source/body terminals during `.subckt` expansion.

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
