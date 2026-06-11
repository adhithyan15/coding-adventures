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

1. Transient/adaptive transient corner parity in Python and TypeScript.
   - Status: in progress.
   - Rust already exposes `transient_corners`, `transient_adaptive_corners`,
     `CornerTransientResult`, `CornerAdaptiveTransientResult`,
     `format_corner_transient_table`, and
     `format_corner_adaptive_transient_table`.
   - Python and TypeScript need matching named-corner wrappers, result shapes,
     stable tab-separated tables, changelog entries, and parity tests.

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

1. Transient/adaptive transient corner parity in Python and TypeScript.
2. PSS/Fourier/distortion/pole-zero corner audit and parity closure.
3. Netlist-to-analysis-plan execution for `.op`, `.dc`, `.ac`, and `.tran`.
4. `.measure`, `.save`, and `.probe` output selection.
5. Sparse solver productionization and convergence diagnostics.
6. Device model audit fixtures and model-card alias compatibility.
7. Mixed-signal hardware VM bridge.
8. Verilog-A/custom-model foothold.
9. Compatibility corpus and release readiness.
