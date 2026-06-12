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

1. Netlist-to-analysis-plan execution for `.op`, `.dc`, `.ac`, and `.tran`.
   - Status: in progress.
   - Python, Rust, and TypeScript should expose matching plan builders and
     execution helpers that run parsed `.op`, `.dc`, `.ac dec` / `.ac log`, and
     `.tran` cards against the parsed circuit in deck order.
   - The slice deliberately leaves `.measure`, `.save`, `.probe`, `.control`,
     and non-log AC sweep execution to later backlog items.

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

## Backlog

1. Cross-language parity closure.
   - Close remaining Rust-first SPICE surface gaps in Python and TypeScript.
   - Audit named-corner coverage for transient, adaptive transient, PSS,
     Fourier, distortion, pole-zero, temperature DC, and digital-bridge outputs.
   - Keep API names idiomatic per language while preserving matching result
     semantics and table columns.

2. Deck execution layer.
   - Convert parsed netlists into runnable analysis plans.
   - Support `.include`, `.lib`, `.end`, `.param`, `.func`, expressions, `.ic`,
     `.nodeset`, `.measure`, `.save`, and `.probe`.
   - Define a deliberate `.control` subset or emit explicit unsupported-feature
     diagnostics.

3. Production solver core.
   - Finish sparse real and complex matrix paths.
   - Use a Rust production sparse path suitable for large decks, a Python
     SciPy-backed path with a structured fallback, and a TypeScript native sparse
     or WASM strategy for browser workloads.
   - Harden Newton damping, device limiting, convergence aids, tolerances, and
     diagnostics.

4. Device model depth.
   - Audit diode, BJT, JFET, and MOS Level 1 behavior against reference decks.
   - Decide whether Level 2/3 MOS is in scope before BSIM; if BSIM lands, make
     Rust the first fast path and port stable semantics outward.
   - Expand temperature behavior, capacitance, noise, charge conservation, model
     card aliases, and error messages.

5. Analysis completion.
   - Generalize pole-zero beyond constrained fixture helpers.
   - Expand nonlinear distortion coverage.
   - Integrate `.FOUR` and `.MEASURE` with transient outputs.
   - Support nested sweeps across temperature, parameters, corners, and Monte
     Carlo trials.
   - Stabilize raw, CSV, JSON, and browser-friendly result formats.

6. Mixed-signal integration.
   - Connect SPICE transient stepping to the hardware VM scheduler.
   - Support bidirectional analog/digital thresholds, event scheduling,
     breakpoint coordination, and VCD correlation.
   - Keep mixed-signal coupling deterministic across Python, Rust, and
     TypeScript.

7. Verilog-A and custom models.
   - Specify the accepted model subset and residual/Jacobian hooks.
   - Add parser or compiler support with sandboxing for TypeScript/web usage.
   - Provide a Rust-native fast path for compiled models.

8. Compatibility corpus and release hardening.
   - Build a deck corpus compared against ngspice or another documented oracle.
   - Record golden tolerances and known incompatibilities.
   - Add examples, API docs, changelogs, package version gates, and CI jobs that
     prove Python, Rust, and TypeScript stay in parity.

## Suggested PR Queue

1. Netlist-to-analysis-plan execution for `.op`, `.dc`, `.ac`, and `.tran`.
2. `.measure`, `.save`, and `.probe` output selection.
3. Sparse solver productionization and convergence diagnostics.
4. Device model audit fixtures and model-card alias compatibility.
5. Mixed-signal hardware VM bridge.
6. Verilog-A/custom-model foothold.
7. Compatibility corpus and release readiness.
