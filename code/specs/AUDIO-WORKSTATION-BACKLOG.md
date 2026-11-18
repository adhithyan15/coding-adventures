# Audio workstation foundations backlog

**Last reviewed:** 2026-11-17  
**Source audit:** `audio-workstation-roadmap.md` and `audio-workstation-inventory.json`, audited at `befa557a8e4c17f0091eeace8030859323cc0a24`  
**Repository delivery baseline:** `origin/main` `2522fa18a0e274ab6bd826a5adc9861c6953de9a`

This is the durable, prioritized delivery queue for connecting the repository's
existing math, circuit, DSP, music, and audio I/O foundations. A source file or
package is evidence of an implementation location only. It does not establish
matching APIs, physical accuracy, real-time suitability, or verification.

## Delivery rules

- Work one item at a time, in dependency order. Reprioritize with the coordinator
  between PRs.
- Keep specifications and fixtures ahead of implementation. Each item records
  a status and an evidence link or an explicit reason that evidence is absent.
- Separate **tracked source**, **native implementation**, **binding/adapter**,
  **executable test evidence**, and **verification evidence**. A binding is not
  a native port; a passing source-package test is not cross-language conformance.
- The 22 language/platform roots and 45-family inventory are an audit index, not
  a coverage score. A future capability manifest must retain absent cells and
  distinguish native code from bindings and runnable evidence. Record
  unexamined cells as **unassessed**.
- Do not label a package fully verified on the strength of sampled fuzz,
  bounded exhaustive domains, unit tests, a source-level SAT/SMT result, or an
  unproved floating-point claim. State each assurance rung and its domain.

## Prioritized items

| ID / priority | Dependencies | Scope and acceptance evidence | Status and evidence |
| --- | --- | --- | --- |
| AF-00 / P0: Correct demonstrated-false filter attestations | None | Remove only claims contradicted by real operations or source loops; exercise direct convolution and invalid-input behavior through real public exports; parse and type-check Warrant documents. Retain non-refuted output shape, depth, and design declarations. | In progress in the first delivery PR. Direct FIR is full linear convolution, so `[2] * [2]` yields `[4]`; empty input is rejected. The former `fir_round_trip` contradicted this. Specific single-input FIR/FFT-FIR and Rust/Python IIR work and heap rows have counterexamples and are withdrawn from canonical/Rust/Python copies. Unmapped common byte caps were removed; lane-specific sample caps remain in source and need bindings before Warrant can express them. No Rust DSP evidence bindings or generated harness exist at the delivery baseline. |
| AF-01 / P0: Audit filter-design preconditions and numeric inputs | AF-00 | State input ranges and error behavior that match each exported design function. Cover odd/even/zero tap counts, cutoff ordering and range, window selection, finite outputs for extreme finite inputs, and language differences with real public API tests. Add Warrant preconditions and executable error claims only when the model types represent fractional cutoffs and the export behavior exactly. | Newly discovered, not started. Rust `design_low_pass` asserts odd/nonzero/in-range tap counts and finite cutoff in `[0, 0.5]`; `design_high_pass` delegates to it; `design_band_pass` asserts valid tap counts and `0 <= low < high <= 0.5`. Python/TypeScript reject invalid design inputs through exceptions, but the current Warrant `-> Bytes` model has no declared error cases or lane binding to normalize them. The Warrant models normalized cutoffs as `Nat`, so they cannot express these fractions. Unconditional Warrant `no_panic` rows were removed for design inputs; precise accepted/rejected domains remain release-blocking. Separate numeric defect to resolve: Python `design_low_pass(5e-324, 1, "rectangular")` and `design_band_pass(0, 5e-324, 1, "hamming")` produce infinity from reciprocal underflow/overflow; add finite-output regressions and a fail-closed input rule. |
| AF-02 / P1: Minimal cross-language conformance contract | AF-00, AF-01 | Specify only the chosen operations and their reset/state semantics, sample precision, comparison rule, and fixture schema. Publish one exact binary impulse-response fixture and run it from Python and TypeScript through public exports. Exercise whole-input, chunked, samplewise, reset, and one-shot SOS behavior independently. Ensure each shared-fixture edit selects both consuming packages in CI; missing/duplicate IDs and changed expected samples fail. | Not started. V0 scope is stateful Biquad processing and one-shot `sosfilt`; do not treat one-shot SOS as streaming. The larger shared audio/math/units schema is deferred. |
| AF-03 / P2: Rust Biquad and SOS parity | AF-02 | Implement a public Rust API only after the conformance contract lands. Match Python/TypeScript semantics on shared impulse and response fixtures, reset behavior, coefficient normalization, and empty-input behavior. Add unit, deterministic property, and fuzz evidence with explicit floating-point limitations. | Not started. Rust `dsp-filters` has FIR/FFT-FIR/IIR/design APIs but no equivalent public Biquad/SOS API. |
| AF-04 / P3: Make filter assurance claims executable and sound | AF-00, AF-01 | Define the bytes-to-sample representation and precision per lane; bind each Warrant operation to public exports; regenerate and run real-operation evidence; record seeded property/fuzz counts and negative controls. Restore withdrawn resource rows only with measured or proved bounds. | Open. Rust `dsp-filters` has no `evidence.bindings`, generated Warrant harness, or assurance report. Source limits are element counts: Python/Rust signal `2^24`, taps `2^20`, IIR order `1024`; TypeScript signal `2^24`, taps `2^16`, IIR order `2^10`. Python accepts general numeric sequences, Rust uses `f32`, and TypeScript accepts `Float64Array` or generic numeric arrays. Warrant byte lengths cannot stand for those sample counts without an explicit lane binding. Existing Warrant rows are obligations, not executed evidence. The former shared byte caps were withdrawn; the native per-lane caps remain in source and need matching executable evidence. |
| AF-05 / P4: Verification substrate for numerical DSP | AF-04, existing verification toolchain | Use the existing Warrant/VER toolchain rather than adding a second soft-float stack. Bring the relevant source subset, float semantics, and certificate checks live; attach source hashes, widths, input domains, SAT/CNF and DRAT replay evidence, and a faulty-mutation control. Define NaN, infinities, subnormals, FMA, and rounding semantics before claiming a floating-point proof. Bound sequential behavior by a stated finite horizon or a valid induction/refinement argument. | Open and release-blocking for any full-verification label. VER04p records float-proof work as not live; VER09/VER11 propositional SAT/DRAT do not prove native floating-point behavior. An abstract control proof does not establish native audio accuracy, stability, or physical fidelity. |
| AF-06 / P5: Stateful block DSP and solver boundaries | AF-02, AF-03, AF-05 | Establish persistent state, reset/state serialization, arbitrary block partition invariance, coefficient updates, and bounded allocation/work for streaming filters. Separately define resumable circuit transient state and ensure paused runs match uninterrupted runs. | Not started. Existing FIR/IIR transforms are primarily whole-buffer APIs. Rust `fir_fft` is whole-buffer FFT convolution, not overlap-add. |
| AF-07 / P6: Resampling and calibrated circuit-probe conversion | AF-05, AF-06 | Specify timestamps, interpolation, anti-alias filtering, decimation, volts/current calibration, DC/clipping policy, and output sample format. Compare against analytic or independently refined references. | Not started. |
| AF-08 / P7: Reusable simulation numerics and physical circuit corpus | AF-05, AF-07 | Reuse existing SPICE and RK4/matrix kernels. Add analytic RC/RLC gates, then diode/transistor/op-amp stages and a genuinely self-starting oscillator with documented perturbation assumptions. Record timestep convergence, KCL/residual/energy checks where applicable, spectra, and physical-model assumptions. | Not started. A prescribed source waveform does not demonstrate circuit startup or sustained oscillation. |
| AF-09 / P8: Multichannel prepared render graph and devices | AF-06, AF-07 | Add multichannel floating-point blocks, routing/panning, prepared graphs, sample-accurate events, cross-thread transport, capture/duplex, and platform backends. Measure callback deadlines, allocation, blocking, underruns, and device changes on named hardware. | Not started. Existing fixed-point mono mixer, bounded PCM storage, and Core Audio output are footholds, not a portable real-time device engine. |
| AF-10 / P9: Instruments, MIDI, effects, and workstation | AF-09 | Add MIDI import/export and live control, polyphonic voices, sample playback, production effects, automation, editing, project persistence, and export only on top of the verified engine contracts. | Not started. Existing synthesis is hardware-IR synthesis, not an audio synthesizer. |
| AF-11 / P10: Native implementation and capability manifest | AF-02 through AF-10, per-root evidence | Track native source, bindings, executable fixture evidence, and verification evidence separately for all 22 roots. Port by numeric → DSP → circuit → music gates, preserve absent/unassessed cells, and identify platform/toolchain availability per root. | Not started. Inventory covers 45 families across 22 tracked roots; no coverage percentage is implied. |

## Assurance boundary for this queue

For each implementation PR, report separately: ordinary unit tests; bounded
exhaustive domains and their counts; seeded property tests; fuzz cases actually
run; resource measurements or proofs and their input bounds; structural/control
invariants; SAT/SMT encodings and independent certificate replay; and numerical
or physical reference agreement. A successful result at one rung does not
substitute for another. Any unavailable rung remains an explicit open item and
blocks a claim that the package is fully verified.
