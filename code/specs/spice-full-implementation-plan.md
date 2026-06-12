# SPICE Full Implementation Plan

This plan tracks the remaining work to turn the SPICE packages into a practical
SPICE2/SPICE3-style simulator across Python, Rust, and TypeScript. The target is
not unlimited vendor compatibility with every HSPICE, Spectre, or ngspice
extension. The target is a documented, cross-language, production-usable SPICE
core with predictable gaps, stable diagnostics, and native web support.

## Completion Bar

A workstream is complete only when the Python, Rust, and TypeScript surfaces are
aligned, package tests cover the new behavior, examples or docs explain the
user-facing entrypoint, and text or structured outputs are stable enough for
downstream tools to compare.

## Current PR Slice

1. Deck execution layer.
   - Status: next.
   - Convert parsed netlists into runnable analysis plans beyond the initial
     `.op`, `.dc`, `.ac`, and `.tran` subset.
   - Support `.param`, `.func`, expressions, `.ic`, and `.nodeset`; `.end`
     boundary detection and map-backed `.include` / `.lib` source resolution
     now have shared diagnostic footholds.
   - Expand `.measure`, `.save`, and `.probe` execution toward full SPICE
     compatibility while keeping unsupported control-flow diagnostics explicit.

## Completed Slices

1. Transient/adaptive transient corner parity in Python and TypeScript.
   - Status: completed in PR 5401.
   - Rust already exposes `transient_corners`, `transient_adaptive_corners`,
     `CornerTransientResult`, `CornerAdaptiveTransientResult`,
     `format_corner_transient_table`, and
     `format_corner_adaptive_transient_table`.
   - Python and TypeScript now expose matching named-corner wrappers, result
     shapes, stable tab-separated tables, changelog entries, and parity tests.

2. PSS named-corner parity in Python and TypeScript.
   - Status: completed in PR 5405.
   - Rust already exposes `pss_corners`, `CornerPssResult`, and
     `format_corner_pss_table`, plus `format_pss_table` for base output.
   - Python and TypeScript now expose matching base PSS table helpers,
     named-corner PSS wrappers, named-corner table helpers, changelog entries,
     and parity tests.

3. Fourier named-corner parity in Python and TypeScript.
   - Status: completed in PR 5413.
   - Rust already exposes `fourier_corners`, `CornerFourierResult`, and
     `format_corner_fourier_table`.
   - Python and TypeScript now expose matching result shapes, named-corner
     Fourier wrappers, named-corner table helpers, changelog entries, and parity
     tests.

4. Distortion and pole-zero named-corner parity in Python and TypeScript.
   - Status: completed in PR 5417.
   - Rust already exposes `distortion_from_transient_corners`,
     `CornerDistortionResult`, `format_corner_distortion_table`,
     `pole_zero_corners`, `CornerPoleZeroResult`, `PoleZeroTopology`, and
     `format_corner_pole_zero_table`.
   - Python and TypeScript now expose matching result shapes, named-corner
     wrappers, named-corner table helpers, changelog entries, and parity tests.

5. Netlist-to-analysis-plan execution for `.op`, `.dc`, `.ac`, and `.tran`.
   - Status: completed in PR 5421.
   - Python, Rust, and TypeScript expose matching plan builders and execution
     helpers that run parsed `.op`, `.dc`, `.ac dec` / `.ac log`, and `.tran`
     cards against the parsed circuit in deck order.

6. Initial `.measure`, `.save`, and `.probe` output selection.
   - Status: completed in this output-selection slice.
   - Python, Rust, and TypeScript parse `.save`, scoped or global `.probe`,
     and `.measure` / `.meas` cards.
   - The first `.measure` execution subset supports `FIND ... AT=<value>` and
     `MAX`, `MIN`, `AVG`, and `RMS` over optional `FROM=<value>` /
     `TO=<value>` ranges for `.op`, `.dc`, `.ac`, and `.tran` analysis-plan
     results.

7. Sparse solver productionization and convergence diagnostics.
   - Status: completed in this sparse-solver diagnostics slice.
   - Python, Rust, and TypeScript now expose stable DC solver diagnostics with
     matrix size, selected real solver path, tolerance, convergence aid, and
     final Newton delta metadata.
   - Large real DC and complex AC matrix solves now route through sparse-row
     solver implementations in all three packages when the shared threshold is
     reached.

8. Device model audit fixtures and model-card alias compatibility.
   - Status: completed in this device-model alias fixture slice.
   - Python, Rust, and TypeScript now expose matching model-card type
     normalization, supported-parameter alias normalization, typed device
     builders, and canonical audit fixtures for diode, BJT, JFET, and Level-1
     MOS `.model` cards.
   - Unsupported model-card parameters are surfaced as explicit unsupported
     keys, and MOS cards deliberately reject non-Level-1 models until the
     Level 2/3 or BSIM scope is chosen.

9. Mixed-signal hardware VM bridge.
   - Status: completed in this mixed-signal VM bridge slice.
   - Python, Rust, and TypeScript now expose matching digital event stream
     fixtures, finite-edge PWL voltage-source conversion, bridge breakpoint
     schedules, fixed/adaptive digital transient bridge runners, named-corner
     bridge wrappers, thresholded probe sampling, stable event/schedule tables,
     and deterministic VCD text output for SPICE probe / hardware-VM trace
     correlation.

10. Verilog-A/custom-model foothold.
    - Status: completed in this custom-model foothold slice.
    - Python, Rust, and TypeScript now expose matching two-terminal
      custom-model result shapes and source-subset diagnostics.
    - Python and TypeScript expose evaluator hooks plus linear-conductance
      helpers; Rust exposes a cloneable, comparable
      `CustomModelKind::LinearConductance` fast path for native execution.
    - The accepted source subset is intentionally diagnostic-only:
      module headers plus `I(p,n) <+ ...` contributions are accepted while
      dynamic, event, system-task, analog-function, and branch declarations are
      rejected until full Verilog-A compiler scope is chosen.

11. Compatibility corpus and release readiness.
    - Status: completed in this compatibility corpus release-readiness slice.
    - Python, Rust, and TypeScript now expose matching compatibility corpus
      deck fixtures for `.op`, `.dc`, `.ac`, `.tran`, and `.tf` coverage.
    - Each deck carries documented oracle metadata, golden values with
      tolerances, and explicit known incompatibility notes.
    - Matching release-readiness gate reports and stable tab-separated corpus
      and gate summaries let package checks detect incomplete or malformed
      compatibility fixtures before release.

12. DC corner and temperature parity closure.
    - Status: completed in this DC corner/temperature parity slice.
    - Python and TypeScript now expose Rust-matching named-corner DC table
      helpers with stable `Corner` / `Index` columns.
    - Python and TypeScript now expose nominal and named-corner DC temperature
      sweep result shapes, `.temp`-style helpers, and stable tab-separated
      table helpers.
    - Package README, changelog, and tests document and lock the new
      cross-language API surface.

13. Remaining cross-language table parity closure.
    - Status: completed in this stable table parity slice.
    - Python and TypeScript now expose Rust-matching stable table helpers for
      `.DC` source sweeps, named-corner `.DC` source sweeps, named-corner `.AC`
      phasors, and named-corner `.TF` gain / impedance rows.
    - Rust-only order-preserving parallel named-corner wrappers remain native
      acceleration surfaces; Python and TypeScript keep sequential API aliases
      until worker policy and browser constraints are specified.
    - Package README, changelog, and tests document and lock the final
      cross-language table columns for this backlog item.

14. Deck-control boundary diagnostics.
    - Status: completed in this deck-boundary diagnostics slice.
    - Python, Rust, and TypeScript now expose matching helpers that return the
      active deck lines before `.end`.
    - The helpers emit stable unsupported-feature diagnostics for `.include`,
      `.lib`, and `.control` directives that appear before `.end`, while
      ignoring lines after the deck boundary.
    - Package README, changelog, and tests document and lock this shared
      parser/planner foothold before include/library resolution and control
      block execution are implemented.

15. Include/library source resolution.
    - Status: completed in this include/library resolution slice.
    - Python, Rust, and TypeScript now expose matching map-backed deck source
      resolvers that expand `.include` files into active deck lines before
      `.end`.
    - The resolvers also support selected `.lib path section` expansion from
      named `.lib` / `.endl` library sections.
    - Stable diagnostics cover missing include files, missing library files,
      absent or unterminated sections, include/library cycles, and
      still-unsupported `.control` directives.

## Backlog

1. Deck execution layer.
   - Convert parsed netlists into runnable analysis plans.
   - Support `.param`, `.func`, expressions, `.ic`, and `.nodeset`.
   - Expand the initial `.measure`, `.save`, and `.probe` support toward full
     SPICE compatibility, including additional measure modes and richer output
     formats.
   - Define a deliberate `.control` subset; explicit unsupported-feature
     diagnostics are now present for the current non-executed state.

2. Production solver core.
   - Finish sparse real and complex matrix paths.
   - Use a Rust production sparse path suitable for large decks, a Python
     SciPy-backed path with a structured fallback, and a TypeScript native sparse
     or WASM strategy for browser workloads.
   - Harden Newton damping, device limiting, convergence aids, tolerances, and
     diagnostics.

3. Device model depth.
   - Audit diode, BJT, JFET, and MOS Level 1 behavior against reference decks.
   - Decide whether Level 2/3 MOS is in scope before BSIM; if BSIM lands, make
     Rust the first fast path and port stable semantics outward.
   - Expand temperature behavior, capacitance, noise, charge conservation, model
     card aliases, and error messages.

4. Analysis completion.
   - Generalize pole-zero beyond constrained fixture helpers.
   - Expand nonlinear distortion coverage.
   - Integrate `.FOUR` and `.MEASURE` with transient outputs.
   - Support nested sweeps across temperature, parameters, corners, and Monte
     Carlo trials.
   - Stabilize raw, CSV, JSON, and browser-friendly result formats.

5. Mixed-signal integration.
   - Connect SPICE transient stepping to the hardware VM scheduler.
   - Support bidirectional analog/digital thresholds, event scheduling,
     breakpoint coordination, and VCD correlation.
   - Keep mixed-signal coupling deterministic across Python, Rust, and
     TypeScript.

6. Verilog-A and custom models.
   - Specify the accepted model subset and residual/Jacobian hooks.
   - Add parser or compiler support with sandboxing for TypeScript/web usage.
   - Provide a Rust-native fast path for compiled models.

## Suggested PR Queue

1. Deck execution layer.
