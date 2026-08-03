# Changelog

## Unreleased

- Validate positive finite BJT model-card `IS` saturation current.
- Validate finite, non-negative BJT model-card `TF` and `TR` transit times.
- Validate BJT model-card `CJC` / `CJC0` / `CBC` base-collector capacitance and
  lower the `CJC0` alias instead of silently dropping it.
- Validate BJT model-card `CJE` / `CJE0` / `CBE` base-emitter capacitance and
  lower the `CJE0` alias instead of silently dropping it.
- Validate BJT model-card `VT` / `V_T` thermal voltage and lower the `V_T`
  alias instead of silently dropping it.
- Validate and lower the BJT model-card `HFE` forward-beta alias.
- Validate BJT model-card `BF` / `BETA` / `BETA_F` forward beta and lower the
  `BETA` alias instead of silently dropping it.
- Validate diode model-card `CJO` / `CJ` / `CJ0` junction capacitance and
  lower the `CJ` alias instead of silently dropping it.
- Validate diode model-card `VT` / `V_T` thermal voltage and lower the `V_T`
  alias instead of silently dropping it.
- Validate diode model-card `IS` / `JS` saturation current and lower the `JS`
  alias instead of silently dropping it.
- Validate and lower JFET model-card `CGD` and `CGD0` gate-drain capacitance
  parameters instead of silently dropping them.
- Validate and lower JFET model-card `CGS` and `CGS0` gate-source capacitance
  parameters instead of silently dropping them.
- Validate MOS model-card `NSS` and `TPG` process parameters and derive missing
  `VT0`, `GAMMA`, and `PHI` values from `N_SUB` / `TOX` through shared engine semantics.
- Validate and lower MOS model-card `CJS` and `CJD` aliases as canonical `CBS`
  and `CBD` junction capacitances.
- Reject negative and non-finite MOS model-card `AF` values and lower valid
  flicker-noise exponents instead of silently dropping them.
- Reject negative and non-finite MOS model-card `KF` values and lower valid
  flicker-noise coefficients instead of silently dropping them.
- Reject MOS model-card `FC` values outside `[0, 1)` or non-finite values and
  lower valid forward-bias depletion coefficients instead of silently dropping them.
- Reject negative and non-finite MOS model-card `MJSW` values and lower valid
  sidewall-junction grading coefficients instead of silently dropping them.
- Reject negative and non-finite MOS model-card `MJ` values and lower valid
  bottom-junction grading coefficients instead of silently dropping them.
- Reject zero, negative, and non-finite MOS model-card `PB` values and lower
  valid bulk-junction potentials instead of silently dropping them.
- Reject negative and non-finite MOS model-card `JS` values and lower valid
  junction saturation-current densities instead of silently dropping them.
- Reject negative and non-finite MOS model-card `CJSW` values and lower valid
  sidewall-junction capacitance densities instead of silently dropping them.
- Reject negative and non-finite MOS model-card `CJ` values and lower valid
  bottom-junction capacitance densities instead of silently dropping them.
- Reject negative and non-finite MOS instance `PS` values and lower valid
  source diffusion perimeters instead of silently dropping them.
- Reject negative and non-finite MOS instance `PD` values and lower valid
  drain diffusion perimeters instead of silently dropping them.
- Reject negative and non-finite MOS instance `AS` values and lower valid
  source diffusion areas instead of silently dropping them.
- Reject negative and non-finite MOS instance `AD` values and lower valid
  drain diffusion areas instead of silently dropping them.
- Reject negative and non-finite MOS instance `NRS` values and lower valid
  source diffusion square counts instead of silently dropping them.
- Reject negative and non-finite MOS instance `NRD` values and lower valid
  drain diffusion square counts instead of silently dropping them.
- Reject MOS model-card `LD` values that are non-finite, negative, or leave a
  non-positive effective Level-1 channel length, and lower valid values.
- Reject negative and non-finite MOS model-card `RSH` values and lower valid
  Level-1 sheet resistance instead of silently dropping it.
- Reject negative and non-finite MOS model-card `RS` values and lower valid
  Level-1 source resistance instead of silently dropping it.
- Reject negative and non-finite MOS model-card `RD` values and lower valid
  Level-1 drain resistance instead of silently dropping it.
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
  and expose `selectOutputs()` / `measureResults()` helpers plus matching
  `ParsedNetlist` methods for analysis-plan results.
- Add a deck execution layer with `buildAnalysisPlan()`, `runAnalysisPlan()`,
  `runNetlist()`, plus matching `ParsedNetlist` methods for runnable `.op`,
  `.dc`, `.ac dec` / `.ac log`, and `.tran` cards.

## 0.3.0 — 2026-06-05

- Resolve `.temp` cards into Kelvin engine-call temperatures and let explicit
  `.noise temp=<kelvin>` overrides win over deck-level operating temperatures.
- Route selected `.options` keys into engine-call helpers:
  `dcOpOptions()` for DC Newton options and `adaptiveTransientOptions()` for
  adaptive transient options.
- Parse SPICE `.four <frequency> <V(node)|I(source)>...` Fourier-analysis
  cards.
- Parse SPICE `.print <analysis> <V(node)|I(source)>...` and
  `.plot <analysis> <V(node)|I(source)>...` output cards.
- Parse SPICE `.temp <celsius> [celsius ...]` operating-temperature cards.
- Parse MOS Level-1 capacitance parameters with `.model ... NMOS|PMOS(... CGSO=<c>
  CGDO=<c> CGBO=<c> CBS=<c> CBD=<c>)`.
- Parse diode model-card emission coefficients with `.model ... D(... N=<n>)`
  and pass them into TypeScript `diode` elements.
- Parse diode model-card reverse-breakdown parameters with
  `.model ... D(... BV=<v> IBV=<i>)`.
- Parse diode model-card junction capacitance with
  `.model ... D(... CJO=<c>)` / `.model ... D(... CJ0=<c>)`.
- Parse diode model-card transit time with `.model ... D(... TT=<time>)`.
- Parse BJT model-card capacitances with `.model ... NPN|PNP(... CJE=<c>
  CJC=<c>)` and pass them into TypeScript `bjt` elements.
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
- Parse SPICE `K` mutual-inductor cards into `mutualInductor` elements,
  including subcircuit-local inductor reference remapping.
- Parse SPICE `J` JFET elements via `.model <name> NJF(...)` and
  `.model <name> PJF(...)` cards with `BETA` / `B`, `VTO`, and `LAMBDA`
  parameters, including subcircuit drain/gate/source remapping.
- Parse capacitor `IC=<voltage>` initial-voltage parameters.
- Parse inductor `IC=<current>` initial-current parameters.
- Parse independent-source `AC <magnitude> [phase]` specs, including combined
  `DC <bias> AC <magnitude> [phase]` forms, and pass the AC phasor through to
  TypeScript SPICE engine AC analysis while preserving DC bias.
- Parse SPICE `.tf V(output_node) input_source` transfer-function analysis
  cards.
- Parse SPICE `.sens V(output_node)` DC sensitivity analysis cards.
- Parse SPICE `.mc V(output_node) n_trials [tolerance] [distribution] [seed]`
  Monte Carlo DC analysis cards.
- Parse SPICE `.noise V(output_node) input_source [freq ...] [temp=<kelvin>]`
  AC noise analysis cards.
- Parse SPICE `.options key=value ...` simulator-options cards.

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
