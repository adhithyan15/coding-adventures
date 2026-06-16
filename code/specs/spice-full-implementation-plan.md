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
   - Status: current.
   - Convert parsed netlists into runnable analysis plans beyond the initial
     `.op`, `.dc`, `.ac`, and `.tran` subset.
   - Support richer expression/function surfaces and execution wiring for
     parsed `.func`, `.ic`, and `.nodeset` hints; `.end` boundary detection,
     map-backed `.include` / `.lib` source resolution, scalar `.param`
     evaluation, active-line expression rewriting, function-definition
     extraction, scalar function-call evaluation, and initial-condition /
     nodeset extraction plus DC warm-start execution aids now have shared
     diagnostic and solver footholds; transient scalar `.measure`-style output
     helpers now cover shared peak-to-peak and final-value measurement output,
     and parsed transient `.measure` / `.meas` cards can now feed those
     measurement helpers from deck text; parsed `.save` and `.probe` cards now
     drive stable Rust table output for operating-point, DC sweep, AC sweep, and
     transient results; parsed `.measure dc` / `.meas dc` cards now route DC
     sweep probe samples into the shared scalar measurement table surface;
     parsed `.measure ac` / `.meas ac` cards now route AC probe magnitudes over
     optional frequency windows into the same measurement table surface; parsed
     transient `.measure ... FIND ... AT=` cards now route single-time probe
     samples through the shared measurement table with interpolation between
     neighboring transient samples; parsed transient
     `.measure ... WHEN probe=target` cards now route first-crossing times over
     optional transient windows into the shared measurement table; parsed
     transient `.measure ... WHEN probe=target RISE|FALL|CROSS=n` cards now
     route counted threshold occurrences into the same stable measurement
     table; parsed transient `.measure ... TRIG ... TARG ...` cards now route
     trigger-to-target delay measurements with counted crossing controls into
     stable scalar rows; parsed transient `.four` deck cards now route harmonic
     analyses over transient outputs with optional `HARMONICS=` and `FROM=`
     controls; parsed `.op`, `.dc`, `.ac`, and `.tran` cards now resolve into
     shared cross-language analysis-plan metadata before execution, and callers
     can select one explicit or implicit plan with stable ambiguity errors and
     route `.op`, `.dc`, `.ac LIN`, `.ac DEC`, `.ac OCT`, or `.tran` into the
     matching solver plus deck-selected table output, including `.tran`
     `START` output filtering, `MAXSTEP` fixed-step caps, and `UIC`
     initial-condition intent; selected `.tran` execution now keeps `.tran
     TSTEP` as the deck output print grid while `MAXSTEP` caps internal solver
     stepping; deck executions now expose normalized selected output probes as
     an inspectable artifact alongside the stable table; selected `.measure`
     outputs now travel with deck execution results as structured measurements
     plus stable measurement tables.
   - Expand remaining deck-controlled analyses toward full SPICE compatibility
     while keeping unsupported control-flow diagnostics explicit.

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

16. Parameter/expression resolution foothold.
    - Status: completed in this parameter/expression resolution slice.
    - Python, Rust, and TypeScript now expose matching deck parameter resolvers
      that evaluate scalar whitespace-tokenized `.param` assignments before
      `.end`.
    - Active deck lines rewrite braced `{expr}` and quoted `'expr'`
      expressions using resolved scalar parameters, arithmetic, parentheses,
      unary signs, `pi`, and common SPICE numeric suffixes.
    - Stable diagnostics cover malformed `.param` cards, invalid parameter
      names, failed expressions, unresolved active-line expressions,
      and unterminated expression delimiters.

17. Initial condition/nodeset directive foothold.
    - Status: completed in this initial-condition/nodeset directive slice.
    - Python, Rust, and TypeScript now expose matching deck initial-condition
      resolvers that extract scalar `.ic` and `.nodeset` `V(node)=value` hints
      before `.end` while preserving non-condition active lines.
    - Numeric condition values reuse the shared scalar SPICE suffix/arithmetic
      expression behavior without parameter-aware execution wiring yet.
    - Stable diagnostics cover missing assignments, malformed targets,
      non-voltage hints, invalid expressions, and unresolved values.

18. Function definition directive foothold.
    - Status: completed in this function-definition directive slice.
    - Python, Rust, and TypeScript now expose matching deck function resolvers
      that extract scalar `.func name(args) expression` definitions before
      `.end` while preserving non-function active lines.
    - Function bodies are stored as normalized expression strings with braced
      or quoted expression delimiters stripped; scalar function-call evaluation
      and active-line expansion are covered by the follow-up expression slice.
    - Stable diagnostics cover missing definitions, malformed signatures,
      invalid function names, invalid or duplicate arguments, and empty
      expressions.

19. Function-call expression resolution.
    - Status: completed in this function-call expression slice.
    - Python, Rust, and TypeScript now let the deck parameter resolver collect
      scalar `.func` definitions before `.end` and evaluate function calls in
      `.param` assignments plus braced or quoted active-line expressions.
    - Function names are case-insensitive, arguments are scalar expressions,
      local arguments shadow deck parameters inside function bodies, and stable
      diagnostics cover unknown functions, bad arity, and recursive calls.
    - The slice deliberately kept `.ic` / `.nodeset` hints as parsed metadata;
      execution-layer wiring for initial guesses is covered by the follow-up
      initial-condition execution-aid slice.

20. Initial-condition execution aids.
    - Status: completed in this initial-condition execution-aid slice.
    - Python, Rust, and TypeScript now map parsed `.nodeset` and `.ic`
      node-voltage hints into DC solver MNA warm-start vectors.
    - The DC operating-point wrappers apply those vectors to Newton solves,
      keeping branch-current guesses at zero and letting `.ic` values override
      `.nodeset` values for the same node.
    - Stable errors reject non-finite hint values, non-zero ground hints,
      unknown nodes, and malformed low-level initial-vector lengths.

21. Transient measurement output expansion.
    - Status: completed in this transient measurement output slice.
    - Python, Rust, and TypeScript now expose matching scalar transient probe
      measurement helpers and stable measurement table formatters.
    - The helpers normalize MAX, MIN, AVG, RMS, peak-to-peak, and final-value
      measurement modes over optional transient time windows.
    - This closes a small `.MEASURE` output-format gap while leaving full
      parsed deck-card execution and richer control-flow semantics in backlog.

22. Parsed transient measurement card routing.
    - Status: completed in this parsed measurement routing slice.
    - Python, Rust, and TypeScript now expose matching `resolve_deck_measurements`
      / `resolveDeckMeasurements` helpers that extract transient `.measure` and
      `.meas` cards before `.end` while preserving non-measure active lines.
    - The parsed card subset supports MAX, MIN, AVG, RMS, peak-to-peak, and
      final-value transient probe measurements with optional `FROM=` / `TO=`
      scalar time windows, reusing the shared SPICE suffix/arithmetic parser.
    - Matching deck execution helpers route parsed cards into stable transient
      measurement rows, while diagnostics keep unsupported analyses, modes,
      options, expressions, and invalid windows explicit.

23. Parsed save/probe output execution routing.
    - Status: completed across Python, Rust, and TypeScript in this parsed
      save/probe output-routing parity slice.
    - The packages now expose `resolve_deck_outputs` / `resolveDeckOutputs`,
      `select_deck_output_probes` / `selectDeckOutputProbes`, and deck-aware
      table formatters that extract `.save` plus scoped or global `.probe`
      cards before `.end`.
    - Selected probes are normalized, deduplicated in deck order, scoped by
      analysis for `.probe`, and routed into stable operating-point, DC sweep,
      AC sweep, and transient text tables.
    - Stable diagnostics cover missing probe lists and malformed output probes;
      richer `.measure` event/trigger modes and remaining non-DC measurement
      analyses remain in the deck execution backlog.

24. Parsed DC sweep measurement routing.
    - Status: completed in this DC sweep measurement routing slice.
    - Python, Rust, and TypeScript now expose matching DC sweep measurement
      helpers that compute MAX, MIN, AVG, RMS, peak-to-peak, and final-value
      probe measurements over optional sweep source-value windows.
    - Parsed `.measure dc` and `.meas dc` cards route into the shared scalar
      measurement table output, while mixed-analysis card lists still fail with
      explicit helper-specific diagnostics.
    - This closes the first non-transient `.measure` execution foothold; richer
      event/trigger modes and remaining non-DC analysis-specific measurement
      semantics remain in backlog.

25. Parsed AC sweep measurement routing.
    - Status: completed in this AC sweep measurement routing slice.
    - Python, Rust, and TypeScript now expose matching AC sweep measurement
      helpers that compute MAX, MIN, AVG, RMS, peak-to-peak, and final-value
      probe measurements over complex probe magnitudes and optional frequency
      windows.
    - Parsed `.measure ac` and `.meas ac` cards route into the shared scalar
      measurement table output, while mixed-analysis card lists still fail with
      explicit helper-specific diagnostics.
    - Richer event/trigger modes and remaining non-DC/non-AC analysis-specific
      measurement semantics remain in backlog.

26. Transient FIND/AT measurement routing.
    - Status: completed in this transient FIND/AT measurement routing slice.
    - Python, Rust, and TypeScript now expose matching transient FIND/AT
      helpers that sample exact transient points or linearly interpolate between
      neighboring samples for scalar probe values.
    - Parsed `.measure tran ... FIND ... AT=` and `.meas transient ... FIND
      ... AT=` cards route into the shared scalar measurement table output with
      the AT time recorded as the point window.
    - Follow-up slices route WHEN crossings and RISE, FALL, CROSS counters;
      target-delay measurements remain in backlog.

27. Transient WHEN crossing measurement routing.
    - Status: completed in this transient WHEN crossing measurement slice.
    - Python, Rust, and TypeScript now expose matching transient WHEN helpers
      that return the first linearly interpolated crossing time where a probe
      equals a target value.
    - Parsed `.measure tran ... WHEN probe=target` and `.meas transient ...
      WHEN probe=target` cards route into the shared scalar measurement table
      output with optional `FROM=` / `TO=` windows.
    - This first-crossing slice left RISE, FALL, CROSS occurrence counters and
      target-delay measurements for follow-up work.

28. Transient WHEN crossing counter routing.
    - Status: completed in this transient WHEN crossing counter slice.
    - Python, Rust, and TypeScript now expose matching counted transient WHEN
      helpers that return the selected `RISE`, `FALL`, or `CROSS` occurrence
      for a probe/target threshold crossing.
    - Parsed `.measure tran ... WHEN probe=target RISE=n`, `FALL=n`, or
      `CROSS=n` cards route through the shared scalar measurement table over
      optional `FROM=` / `TO=` windows.
    - Target-delay measurement forms were routed in follow-up slice 29.

29. Transient TRIG/TARG delay measurement routing.
    - Status: completed in this transient trigger-delay measurement slice.
    - Python, Rust, and TypeScript now expose matching transient delay helpers
      that return target crossing time minus trigger crossing time for two
      transient probes.
    - Parsed `.measure tran ... TRIG probe VAL=value RISE|FALL|CROSS=n TARG
      probe VAL=value RISE|FALL|CROSS=n` cards route through the shared scalar
      measurement table over optional `FROM=` / `TO=` windows, with target
      search beginning at the resolved trigger time.
    - The parser also accepts compact `probe=value` trigger and target forms
      for parity with the existing WHEN syntax.

30. Transient `.FOUR` deck-card routing.
    - Status: completed in this transient Fourier deck-routing slice.
    - Python, Rust, and TypeScript now expose matching `.four` / `.FOUR`
      deck-card resolvers that extract fundamental frequency, probe list,
      optional `HARMONICS=`, and optional `FROM=` before `.end`.
    - Matching transient deck helpers route parsed cards into the existing
      SPICE-style Fourier harmonic result shapes, reusing the shared scalar
      suffix/arithmetic parser and stable diagnostics for malformed cards.
    - This closes the first parsed `.FOUR` execution foothold while leaving
      broader output-plan integration and nested sweep execution in backlog.

31. Deck analysis-plan directive resolver.
    - Status: completed in this analysis-plan resolver slice.
    - Python, Rust, and TypeScript now expose matching helpers for extracting
      `.op`, `.dc`, `.ac`, and `.tran` cards before `.end` into stable
      analysis-plan metadata while preserving non-analysis active lines.
    - The resolver evaluates scalar SPICE suffix/arithmetic values for DC
      sweep ranges, AC sweep points and frequency limits, and transient step,
      stop, start, max-step, and `UIC` controls.
    - This closes the first runnable-analysis metadata foothold while leaving
      actual deck dispatch into solver executions in backlog.

32. Deck analysis-plan selector.
    - Status: completed in this analysis-plan selection slice.
    - Python, Rust, and TypeScript now expose matching helpers for selecting one
      explicit `.op`, `.dc`, `.ac`, or `.tran` analysis plan by normalized
      analysis alias.
    - Decks without analysis cards default to an implicit `.op` plan, while
      decks with multiple candidate cards report stable ambiguity errors before
      solver dispatch.
    - This gives deck execution helpers a deterministic single-plan bridge
      while leaving full selected-plan-to-solver dispatch in backlog.

33. Selected deck analysis execution routing.
    - Status: completed in this selected-plan execution routing slice.
    - Python, Rust, and TypeScript now expose matching helpers that select one
      deck analysis plan, execute `.op`, `.dc`, `.ac DEC`, or `.tran` against an
      existing `Circuit`, and return the selected plan, solver result, and
      deck-selected output table.
    - The bridge preserves stable ambiguity and invalid-card diagnostics and
      explicitly reports unsupported `.ac LIN` / `.ac OCT` execution modes for
      future solver-grid expansion.

34. Deck AC LIN/OCT execution routing.
    - Status: completed in this deck AC grid routing slice.
    - Python, Rust, and TypeScript now route selected `.ac LIN`, `.ac DEC`, and
      `.ac OCT` plans through matching solver executions and deck-selected AC
      table output.
    - The execution bridge uses SPICE-style linear total-point grids,
      points-per-decade grids, and points-per-octave grids while preserving
      selected-plan ambiguity and invalid-card diagnostics.

35. Deck transient START/MAXSTEP/UIC routing.
    - Status: completed in this transient deck-control routing slice.
    - Python, Rust, and TypeScript now route selected `.tran` `START` output
      filtering, `MAXSTEP` fixed-step caps, and `UIC` initial-condition intent
      through matching solver executions and deck-selected transient tables.
    - This closes the first deck-owned transient execution-control foothold
      while leaving richer run artifacts and output-plan integration in
      backlog.

36. Deck transient print-step output routing.
    - Status: completed in this transient print-step routing slice.
    - Python, Rust, and TypeScript now keep `.tran TSTEP` as the stable deck
      output print grid while using `MAXSTEP` only as an internal fixed-step
      cap.
    - This separates deck-visible transient output rows from internal solver
      stepping and preserves stable selected-plan transient tables.

37. Deck selected-output artifact metadata.
    - Status: completed in this output-probe artifact slice.
    - Python, Rust, and TypeScript `run_deck_analysis` / `runDeckAnalysis`
      results now include the normalized deck-selected output probes alongside
      the selected plan, solver result, and stable table.
    - This gives callers a structured deck-owned output artifact without
      reparsing table text.

38. Deck selected measurement artifact routing.
    - Status: completed in this deck measurement artifact slice.
    - Python, Rust, and TypeScript `run_deck_analysis` / `runDeckAnalysis`
      results now include selected `.measure` / `.meas` outputs and a stable
      measurement table for `.dc`, `.ac`, and `.tran` executions.
    - Measurement cards are selected by the executed analysis, so mixed-analysis
      decks can expose the chosen analysis artifact without reparsing output
      tables.

## Backlog

1. Deck execution layer.
   - Expand selected-plan execution beyond fixed-step transient basics,
     including richer deck-owned run artifacts beyond selected output probes
     and selected measurement artifacts, plus output-plan integration beyond
     stable table routing.
   - Expand deck-controlled output-plan integration beyond stable table
     routing toward full SPICE compatibility.
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
   - Expand parsed `.FOUR` / `.MEASURE` integration across output plans and
     nested sweeps.
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
