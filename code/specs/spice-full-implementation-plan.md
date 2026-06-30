# SPICE Full Implementation Plan

This plan tracks the remaining work to turn the SPICE packages into a practical
SPICE2/SPICE3-style simulator across Python, Rust, and TypeScript. The target is
not unlimited vendor compatibility with every HSPICE, Spectre, or ngspice
extension. The target is a documented, cross-language, production-usable SPICE
core with predictable gaps, stable diagnostics, and native web support.

The application roadmap is staged by dialect confidence:

1. Berkeley SPICE / SPICE2-SPICE3 core conformance first.
2. ngspice compatibility as an explicit open-source dialect layer.
3. LTspice compatibility as the later vendor-dialect and product-experience
   target.

Rust is the primary app/runtime spine for Mosaic-backed user interfaces, native
execution, and WebAssembly packaging. Python and TypeScript remain first-class
conformance ports: parser, solver, diagnostic, and output semantics should move
in sync unless a workstream explicitly documents a Rust-only acceleration path
with stable cross-language results.

## Completion Bar

A workstream is complete only when the Python, Rust, and TypeScript surfaces are
aligned, package tests cover the new behavior, examples or docs explain the
user-facing entrypoint, and text or structured outputs are stable enough for
downstream tools to compare.

No backward-compatibility promise exists for pre-release parser APIs. If the
current parser surface conflicts with Berkeley SPICE correctness, shared grammar
contracts, source-span diagnostics, or cross-language parity, break it and fix
the Rust, Python, and TypeScript surfaces together.

## Current PR Slice

1. Rust Berkeley Mosaic app shell event summary.
   - Status: current PR completion candidate.
   - Add schema-versioned Rust app shell event summaries and JSON helpers that
     derive compact event-kind, severity, diagnostic, repaired-state, and
     capability counts from Berkeley app shell event logs.
   - Preserve package name, source fingerprint, ready/blocked route, status
     severity, status event ID, primary action, event counts, counted totals,
     diagnostic count, repaired-state count, and advertised capability count so
     Mosaic, WebAssembly, and product shells can gate startup dashboards without
     walking the full event stream.
   - Keep this as a Rust-only app-substrate packaging slice over the public
     parser contract; Python and TypeScript parser parity remains aligned when
     parser behavior changes.

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
   - Status: completed through the sparse-solver diagnostics and production
     profile slices.
   - Python, Rust, and TypeScript now expose stable DC solver diagnostics with
     matrix size, selected real solver path, tolerance, convergence aid, and
     final Newton delta metadata.
   - Large real DC and complex AC matrix solves now route through sparse-row
     solver implementations in all three packages when the shared threshold is
     reached.
   - Python now uses an optional SciPy sparse-LU backend for large real DC
     solves with an explicit native sparse fallback, while Rust and TypeScript
     expose their native sparse-row backend choices.
   - DC diagnostics now include stable solver profiles with structural nonzero
     counts, density, fill-in, backend, and fallback metadata.

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
      ignoring lines after the deck boundary. Unsupported `.control` blocks
      are also excluded from active lines, with per-command diagnostics for
      non-comment lines inside the block.
    - Package README, changelog, and tests document and lock this shared
      parser/planner foothold before control block execution is implemented.

15. Include/library source resolution.
    - Status: completed in this include/library resolution slice.
    - Python, Rust, and TypeScript now expose matching map-backed deck source
      resolvers that expand `.include` files into active deck lines before
      `.end`.
    - The resolvers also support selected `.lib path section` expansion from
      named `.lib` / `.endl` library sections.
    - Stable diagnostics cover missing include files, missing library files,
      absent or unterminated sections, include/library cycles, and
      still-unsupported `.control` directives whose body commands are excluded
      from active source-resolved solver input.

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

39. Deck selected Fourier artifact routing.
    - Status: completed in this deck Fourier artifact slice.
    - Python, Rust, and TypeScript `run_deck_analysis` / `runDeckAnalysis`
      results now include selected transient `.four` outputs and a stable
      Fourier table for `.tran` executions.
    - Fourier cards are exposed only for the selected transient analysis, so
      mixed-analysis decks can inspect harmonic artifacts without reparsing
      output tables.

40. Deck selected-run artifact summary.
    - Status: completed in this deck run-artifact summary slice.
    - Python, Rust, and TypeScript `run_deck_analysis` / `runDeckAnalysis`
      results now include selected-run artifact summaries and stable count
      tables for result rows, normalized output probes, measurement artifacts,
      and Fourier artifacts.
    - The summary is list-shaped so future nested sweeps can append more
      deck-owned run artifacts without changing the public field shape.

41. Parsed `.print` output routing.
    - Status: completed in this parsed print output-routing slice.
    - Python, Rust, and TypeScript now treat scoped
      `.print <analysis> V(node) I(source)` cards as deck output selections
      alongside `.save` and `.probe`.
    - Selected probes are normalized and deduplicated in deck order, while
      diagnostics distinguish missing `.print` probes, unsupported `.print`
      analyses, and malformed output probes.

42. Parsed `.plot` output routing.
    - Status: completed in this parsed plot output-routing slice.
    - Python, Rust, and TypeScript now treat scoped
      `.plot <analysis> V(node) I(source)` cards as deck output selections
      alongside `.save`, `.probe`, and `.print`.
    - Selected probes are normalized and deduplicated in deck order, while
      diagnostics distinguish missing `.plot` probes, unsupported `.plot`
      analyses, and malformed output probes.

43. Deck `.control` block exclusion.
    - Status: completed in this control-block exclusion slice.
    - Python, Rust, and TypeScript now exclude unsupported `.control` / `.endc`
      blocks from active deck-control lines and source-resolved solver input.
    - The existing unsupported `.control` directive diagnostic is preserved,
      while unrecognized non-comment commands inside the block emit stable
      `SPICE_DECK_CONTROL_COMMAND` diagnostics.

44. Selected `.control` command routing.
    - Status: completed in this selected control-command routing slice.
    - Python, Rust, and TypeScript now normalize selected `.control` block
      analysis/output commands (`op`, `dc`, `ac`, `tran`, `print`, and `plot`)
      into the same dotted deck cards consumed by the existing analysis and
      output resolvers.
    - Unrecognized non-comment commands inside `.control` blocks remain
      diagnostic-only so unsupported control flow is still explicit.

45. Selected `.control` save/probe routing.
    - Status: completed in this selected control save/probe routing slice.
    - Python, Rust, and TypeScript now normalize selected `.control` block
      output-selection commands (`save` and `probe`) into the same `.save` and
      `.probe` deck cards consumed by existing output resolvers.
    - Unrecognized non-comment commands inside `.control` blocks remain
      diagnostic-only so unsupported control flow is still explicit.

46. Selected `.control` measurement routing.
    - Status: completed in this selected control measurement-routing slice.
    - Python, Rust, and TypeScript now normalize selected `.control` block
      measurement commands (`measure` and `meas`) into the same `.measure` and
      `.meas` deck cards consumed by existing measurement resolvers.
    - Unrecognized non-comment commands inside `.control` blocks remain
      diagnostic-only so unsupported control flow is still explicit.

47. Selected `.control` Fourier routing.
    - Status: completed in this selected control Fourier-routing slice.
    - Python, Rust, and TypeScript now normalize selected `.control` block
      harmonic output commands (`four` and `fourier`) into the same `.four`
      deck cards consumed by existing Fourier resolvers.
    - Unrecognized non-comment commands inside `.control` blocks remain
      diagnostic-only so unsupported control flow is still explicit.

48. Selected `.control` run marker routing.
    - Status: completed in this selected control run-marker routing slice.
    - Python, Rust, and TypeScript now accept selected `.control` block `run`
      execution markers as no-op control commands after selected analysis and
      output commands have normalized into deck cards.
    - Other unrecognized non-comment commands inside `.control` blocks remain
      diagnostic-only so unsupported control flow is still explicit.

49. Selected `.control` quit marker routing.
    - Status: completed in this selected control quit-marker routing slice.
    - Python, Rust, and TypeScript now accept selected `.control` block `quit`
      interpreter-exit markers as no-op control commands after selected
      analysis and output commands have normalized into deck cards.
    - Other unrecognized non-comment commands inside `.control` blocks remain
      diagnostic-only so unsupported control flow is still explicit.

50. Selected `.control` noaskquit option routing.
    - Status: completed in this selected control noaskquit-option routing slice.
    - Python, Rust, and TypeScript now accept exact selected `.control` block
      `set noaskquit` UI options as no-op control commands.
    - Other `set` variables and unrecognized non-comment commands inside
      `.control` blocks remain diagnostic-only so unsupported script state and
      control flow are still explicit.

51. Selected `.control` reset marker routing.
    - Status: completed in this selected control reset-marker routing slice.
    - Python, Rust, and TypeScript now accept selected `.control` block `reset`
      session-reset markers as no-op control commands after selected analysis
      and output commands have normalized into deck cards.
    - Other unrecognized non-comment commands inside `.control` blocks remain
      diagnostic-only so unsupported stateful script execution is still
      explicit.

52. Selected `.control` ASCII filetype option routing.
    - Status: completed in this selected control filetype-ascii option routing
      slice.
    - Python, Rust, and TypeScript now accept exact selected `.control` block
      `set filetype=ascii` output-format options as no-op control commands.
    - Other `set` variables, binary rawfile options, file-writing commands, and
      unrecognized non-comment commands inside `.control` blocks remain
      diagnostic-only so unsupported file I/O and script state are explicit.

53. Selected `.control` rawfile output option routing.
    - Status: completed in this selected control rawfile-output option routing
      slice.
    - Python, Rust, and TypeScript now accept exact selected `.control` block
      `set wr_vecnames` and `set wr_singlescale` rawfile output toggles as
      no-op control commands.
    - Other `set` variables, rawfile file-writing commands, and unrecognized
      non-comment commands inside `.control` blocks remain diagnostic-only so
      unsupported file I/O and script state are explicit.

54. Selected `.control` appendwrite option routing.
    - Status: completed in this selected control appendwrite-option routing
      slice.
    - Python, Rust, and TypeScript now accept exact selected `.control` block
      `set appendwrite` rawfile append-write options as no-op control commands.
    - Binary rawfile formats, actual rawfile serialization, other `set`
      variables, and unrecognized non-comment commands inside `.control` blocks
      remain diagnostic-only so unsupported file I/O and script state are
      explicit.

55. Selected `.control` rawfile write marker routing.
    - Status: completed in this selected control rawfile-write marker routing
      slice.
    - Python, Rust, and TypeScript now accept selected `.control` block
      `write <rawfile> [probes...]` rawfile-write markers as no-op control
      commands when a target path token is present.
    - Actual rawfile serialization, binary rawfile formats, other file-writing
      commands, other `set` variables, and unrecognized non-comment commands
      inside `.control` blocks remain diagnostic-only so unsupported file I/O
      and script state are explicit.

56. Selected `.control` WRDATA marker routing.
    - Status: completed in this selected control WRDATA marker routing slice.
    - Python, Rust, and TypeScript now accept selected `.control` block
      `wrdata <file> <probes...>` ASCII data-write markers as no-op control
      commands when both a target path token and at least one vector/probe token
      are present.
    - Actual ASCII data-file serialization, rawfile serialization, binary
      rawfile formats, other file-writing commands, other `set` variables, and
      unrecognized non-comment commands inside `.control` blocks remain
      diagnostic-only so unsupported file I/O and script state are explicit.

57. Selected `.control` inspection marker routing.
    - Status: completed in this selected control inspection-marker routing
      slice.
    - Python, Rust, and TypeScript now accept selected `.control` block
      read-only `display` and `listing` inspection commands as no-op control
      commands.
    - Actual console/listing output, mutating control-flow commands, other
      `set` variables, and unrecognized non-comment commands inside `.control`
      blocks remain diagnostic-only so unsupported UI output and script state
      are explicit.

58. Selected `.control` SHOW marker routing.
    - Status: completed in this selected control show-marker routing slice.
    - Python, Rust, and TypeScript now accept selected `.control` block
      read-only `show` and `showmod` device/model inspection commands as no-op
      control commands.
    - Actual console/model inspection output, mutating control-flow commands,
      other `set` variables, and unrecognized non-comment commands inside
      `.control` blocks remain diagnostic-only so unsupported UI output and
      script state are explicit.

59. Selected `.control` introspection marker routing.
    - Status: completed in this selected control introspection-marker routing
      slice.
    - Python, Rust, and TypeScript now accept selected `.control` block
      read-only `status`, `version`, and `help` UI introspection commands as
      no-op control commands.
    - Actual console/help output, mutating control-flow commands, other `set`
      variables, and unrecognized non-comment commands inside `.control`
      blocks remain diagnostic-only so unsupported UI output and script state
      are explicit.

60. Selected `.control` console/debug marker routing.
    - Status: completed in this selected control console/debug-marker routing
      slice.
    - Python, Rust, and TypeScript now accept selected `.control` block
      read-only `echo`, `rusage`, and `where` console/debug commands as no-op
      control commands.
    - Actual console/debug output, mutating control-flow commands, external
      script execution, other `set` variables, and unrecognized non-comment
      commands inside `.control` blocks remain diagnostic-only so unsupported
      UI output and script state are explicit.

61. Selected `.control` script execution policy diagnostics.
    - Status: completed in this selected control script-policy diagnostics
      slice.
    - Python, Rust, and TypeScript now emit explicit diagnostics for selected
      `.control` block `source` and `shell` external script/shell commands
      instead of generic unsupported-command diagnostics.
    - External script execution, shelling out, working-directory mutation,
      control flow, variables, and unrecognized non-comment commands inside
      `.control` blocks remain diagnostic-only so unsupported script execution
      policy is explicit.

62. Selected `.control` working-directory policy diagnostics.
    - Status: completed in this selected control working-directory policy
      diagnostics slice.
    - Python, Rust, and TypeScript now emit explicit diagnostics for selected
      `.control` block `cd` working-directory mutation commands instead of
      generic unsupported-command diagnostics.
    - Working-directory mutation, external script execution, shelling out,
      control flow, variables, and unrecognized non-comment commands inside
      `.control` blocks remain diagnostic-only so unsupported execution policy
      is explicit.

63. Selected `.control` control-flow policy diagnostics.
    - Status: completed in this selected control-flow policy diagnostics
      slice.
    - Python, Rust, and TypeScript now emit explicit diagnostics for selected
      `.control` block control-flow commands, including `if`, `while`,
      `foreach`, and `repeat`, instead of generic unsupported-command
      diagnostics.
    - Control-flow execution, variables, working-directory mutation, external
      script execution, shelling out, and unrecognized non-comment commands
      inside `.control` blocks remain diagnostic-only so unsupported execution
      policy is explicit.

64. Selected `.control` variable policy diagnostics.
    - Status: completed in this selected variable policy diagnostics slice.
    - Python, Rust, and TypeScript now emit explicit diagnostics for selected
      `.control` block variable/state mutation commands, including `let`,
      `alter`, `alterparam`, `set`, and `unset`, instead of generic
      unsupported-command diagnostics.
    - Accepted no-op `set` options still route as no-op markers; variable
      mutation, circuit mutation, control-flow execution, working-directory
      mutation, external script execution, shelling out, and unrecognized
      non-comment commands inside `.control` blocks remain diagnostic-only so
      unsupported execution policy is explicit.

65. Deck run output-probe artifact names.
    - Status: completed in this deck run output-probe artifact slice.
    - Python, Rust, and TypeScript selected deck executions now include the
      normalized output-probe names inside selected-run artifacts alongside
      the existing output-probe counts.
    - The stable run-artifact table now includes `OutputProbeList`, so callers
      can inspect deck-owned probe provenance without reparsing solver output
      tables.

66. Deck run measurement artifact names.
    - Status: completed in this deck run measurement artifact slice.
    - Python, Rust, and TypeScript selected deck executions now include the
      selected measurement names inside selected-run artifacts alongside the
      existing measurement counts.
    - The stable run-artifact table now includes `MeasurementList`, so callers
      can inspect deck-owned measurement provenance without reparsing
      measurement tables.

67. Deck run Fourier artifact probes.
    - Status: completed in this deck run Fourier artifact slice.
    - Python, Rust, and TypeScript selected deck executions now include the
      selected Fourier probe names inside selected-run artifacts alongside the
      existing Fourier result counts.
    - The stable run-artifact table now includes `FourierList`, so callers can
      inspect deck-owned Fourier provenance without reparsing Fourier tables.

68. Deck run transfer-function routing.
    - Status: completed in this deck run transfer-function routing slice.
    - Python, Rust, and TypeScript selected deck executions now parse top-level
      `.tf V(node) SOURCE` analysis cards into the shared deck-analysis plan
      surface.
    - Selected `.tf` deck executions route through the existing transfer-function
      solver and stable transfer-function table while exposing a one-row deck-run
      artifact for the transfer probe.

69. Deck run sensitivity routing.
    - Status: completed in this deck run sensitivity routing slice.
    - Python, Rust, and TypeScript selected deck executions now parse top-level
      `.sens V(node)` analysis cards into the shared deck-analysis plan surface.
    - Selected `.sens` deck executions route through the existing DC sensitivity
      solver and stable sensitivity table while exposing a one-row deck-run
      artifact for the sensitivity probe.

70. Deck run noise routing.
    - Status: completed in this deck run noise routing slice.
    - Python, Rust, and TypeScript selected deck executions now parse top-level
      `.noise V(node) SOURCE` analysis cards into the shared deck-analysis plan
      surface, including optional LIN/DEC/OCT frequency sweep controls.
    - Selected `.noise` deck executions route through the existing AC noise
      solver and stable noise table while exposing deck-run artifacts for the
      noise output probe and selected frequency rows.

71. Deck run output-directive artifacts.
    - Status: completed in this deck run output-directive artifact slice.
    - Python, Rust, and TypeScript selected deck executions now expose the
      selected output directive kinds that contributed to each deck-run
      artifact, alongside the existing selected output probe list.
    - The stable run-artifact table now includes `OutputDirectiveList`, so
      callers can distinguish `.save`, `.probe`, `.print`, and `.plot`
      provenance without reparsing deck output cards.

72. Deck run analysis-source artifacts.
    - Status: completed in this deck run analysis-source artifact slice.
    - Python, Rust, and TypeScript selected deck executions now copy each
      selected analysis plan's source name into deck-run artifacts, preserving
      `.dc`, `.tf`, and `.noise` input-source provenance beside the existing
      output, measurement, Fourier, and directive lists.
    - The stable run-artifact table now includes `SourceName`, with an empty
      cell for analysis cards that do not name a source.

73. Deck run sweep artifacts.
    - Status: completed in this deck run sweep artifact slice.
    - Python, Rust, and TypeScript selected deck executions now copy sweep-shape
      metadata from selected analysis plans into deck-run artifacts, preserving
      `.dc` source sweep bounds plus `.ac` and `.noise` sweep kind, point count,
      and frequency bounds.
    - The stable run-artifact table now includes `SweepKind`, `StartValue`,
      `StopValue`, `StepValue`, `PointCount`, `StartFrequencyHz`, and
      `StopFrequencyHz`, with empty cells for analysis cards without that
      sweep metadata.

74. Deck run output-node artifacts.
    - Status: completed in this deck run output-node artifact slice.
    - Python, Rust, and TypeScript selected deck executions now copy each
      selected analysis plan's output node into deck-run artifacts, preserving
      `.tf`, `.sens`, and `.noise` output-target provenance beside source and
      sweep metadata.
    - The stable run-artifact table now includes `OutputNode`, with an empty
      cell for analysis cards that do not select a single output node.

75. Deck run transient timing artifacts.
    - Status: completed in this deck run transient timing artifact slice.
    - Python, Rust, and TypeScript selected deck executions now copy `.tran`
      timing controls into deck-run artifacts, preserving print step, stop
      time, optional start and max step, and UIC intent.
    - The stable run-artifact table now includes `StepTime`, `StopTime`,
      `StartTime`, `MaxStep`, and `UseInitialConditions`, with empty cells for
      non-transient analyses.

76. Deck run result-column artifacts.
    - Status: completed in this deck run result-column artifact slice.
    - Python, Rust, and TypeScript selected deck executions now copy stable
      result table column names into deck-run artifacts alongside row counts.
    - The stable run-artifact table now includes `ResultColumns` and
      `ResultColumnList`, so downstream callers can inspect result shape
      without reparsing solver output tables.

77. Deck run diagnostic artifacts.
    - Status: completed in this deck run diagnostic artifact slice.
    - Python, Rust, and TypeScript selected deck executions now carry selected
      analysis diagnostic counts and code lists in deck-run artifacts.
    - The stable run-artifact table now includes `Diagnostics` and
      `DiagnosticCodeList`, preserving parser diagnostic provenance beside the
      selected analysis/output/measurement/Fourier metadata surface.

78. Deck run artifact CSV format.
    - Status: completed in this deck run artifact CSV slice.
    - Python, Rust, and TypeScript now expose matching
      `format_deck_run_artifact_csv` / `formatDeckRunArtifactCsv` helpers for
      selected deck-run artifacts.
    - The CSV helpers preserve the same stable columns as the tab-separated
      run-artifact table while applying deterministic CSV escaping for browser,
      spreadsheet, and downstream data-pipeline consumers.

79. Deck run artifact JSON format.
    - Status: completed in this deck run artifact JSON slice.
    - Python, Rust, and TypeScript now expose matching
      `format_deck_run_artifact_json` / `formatDeckRunArtifactJson` helpers for
      selected deck-run artifacts.
    - The JSON helpers preserve the same stable key order and normalized cell
      values as the tab-separated run-artifact table for browser and downstream
      data-pipeline consumers.

80. Deck selected table CSV format.
    - Status: completed in this deck table CSV slice.
    - Python, Rust, and TypeScript now expose matching `format_deck_table_csv`
      / `formatDeckTableCsv` helpers for stable tab-separated deck output
      tables.
    - The helper reuses deterministic CSV escaping so selected result,
      measurement, Fourier, and run-artifact tables can be exported to browser,
      spreadsheet, and downstream data-pipeline consumers without per-analysis
      CSV formatters.

81. Deck selected table JSON format.
    - Status: completed in this deck table JSON slice.
    - Python, Rust, and TypeScript now expose matching `format_deck_table_json`
      / `formatDeckTableJson` helpers for stable tab-separated deck output
      tables.
    - The helper emits compact JSON records keyed by each table's header row,
      so selected result, measurement, Fourier, and run-artifact tables can be
      consumed by browser clients without per-analysis JSON formatters.

82. Deck selected table records API.
    - Status: completed in this deck table records slice.
    - Python, Rust, and TypeScript now expose matching `deck_table_records` /
      `deckTableRecords` helpers for stable tab-separated deck output tables.
    - The helper returns header-keyed native records so browser and host
      integrations can inspect selected result, measurement, Fourier, and
      run-artifact tables without reparsing raw text or JSON strings.

83. Deck execution output directives artifact.
    - Status: completed in this deck output directives slice.
    - Python, Rust, and TypeScript selected deck executions now expose
      normalized output directives beside selected output probes on the
      execution result.
    - Host and browser integrations can inspect which `.save`, `.probe`,
      `.print`, or `.plot` cards selected the output table without reparsing
      run-artifact tables.

84. Deck execution analysis directive artifacts.
    - Status: completed in this deck analysis directives slice.
    - Python, Rust, and TypeScript selected deck executions now expose the
      normalized selected analysis directive beside selected output probes and
      output directives on the execution result.
    - Selected-run artifacts and stable run-artifact tables now include
      `AnalysisDirectiveList`, so host and browser integrations can inspect
      which `.op`, `.dc`, `.ac`, `.tran`, `.tf`, `.sens`, or `.noise` card
      drove the run without reparsing the selected plan or deck text.

85. Deck run table artifacts.
    - Status: completed in this deck run table artifact slice.
    - Python, Rust, and TypeScript selected deck executions now expose stable
      table count/name lists inside selected-run artifacts.
    - The stable run-artifact table now includes `Tables` and `TableList`, so
      host and browser integrations can inspect which selected result,
      measurement, Fourier, and run-artifact tables belong to a run without
      deriving the inventory from optional side tables.

86. Deck execution table inventory artifacts.
    - Status: completed in this deck execution table inventory slice.
    - Python, Rust, and TypeScript selected deck executions now expose stable
      table count/name lists directly on the execution result beside selected
      analysis directives, output probes, output directives, and selected-run
      artifacts.
    - Host and browser integrations can inspect the selected result,
      measurement, Fourier, and run-artifact table inventory without drilling
      into the run-artifact table or reconstructing optional side-table
      presence.

87. Deck execution table export artifacts.
    - Status: completed in this deck execution table export artifact slice.
    - Python, Rust, and TypeScript selected deck executions now expose ordered
      table export artifacts beside the stable table count/name inventory.
    - Each table artifact carries the stable table text, deterministic CSV,
      compact JSON records, and host-native header-keyed records for the
      selected result, measurement, Fourier, and run-artifact tables.

88. Deck run control diagnostic artifacts.
    - Status: completed in this deck run control diagnostic artifact slice.
    - Python, Rust, and TypeScript selected deck executions now add existing
      `.control` body policy diagnostics to selected-run artifact
      `Diagnostics` / `DiagnosticCodeList` metadata.
    - The diagnostic counts and code lists propagate through stable
      run-artifact tables, CSV/JSON export helpers, and ordered table export
      artifacts without executing control-flow, variables, external scripts,
      or working-directory mutations.

89. Deck run control command inventory artifacts.
    - Status: completed in this deck run control command inventory artifact
      slice.
    - Python, Rust, and TypeScript control analyzers now expose normalized
      `.control` command lines separately from full active deck input.
    - Selected deck executions now carry those normalized command inventories
      through `ControlLines` / `ControlLineList` run-artifact metadata,
      stable run-artifact tables, CSV/JSON export helpers, and ordered table
      export artifacts.

90. Deck execution control command inventory artifacts.
    - Status: completed in this deck execution control command inventory
      artifact slice.
    - Python, Rust, and TypeScript selected deck executions now expose
      normalized `.control` command inventories directly on the execution
      result beside selected table, output, measurement, Fourier, and
      analysis-directive artifacts.
    - Host and browser integrations can inspect accepted `.control` block
      command provenance without drilling into selected-run artifact tables.

91. Deck execution diagnostic artifacts.
    - Status: completed in this deck execution diagnostic artifact slice.
    - Python, Rust, and TypeScript selected deck executions now expose selected
      diagnostic inventories directly on the execution result beside selected
      table, output, measurement, Fourier, analysis-directive, and control
      command artifacts.
    - Host and browser integrations can inspect selected execution diagnostic
      provenance without drilling into selected-run artifact tables.

92. Deck rawfile write marker artifacts.
    - Status: completed in this deck rawfile write marker artifact slice.
    - Python, Rust, and TypeScript control analyzers now expose normalized
      accepted `.control` `write` and `wrdata` marker inventories.
    - Selected deck executions carry those write-marker inventories directly and
      through selected-run artifact tables, CSV/JSON helpers, and ordered table
      export artifacts without serializing files.

93. Deck rawfile option artifacts.
    - Status: completed in this deck rawfile option artifact slice.
    - Python, Rust, and TypeScript control analyzers now expose normalized
      accepted `.control` rawfile option inventories for `set filetype=ascii`,
      `set wr_vecnames`, `set wr_singlescale`, and `set appendwrite`.
    - Selected deck executions carry those rawfile option inventories directly
      and through selected-run artifact tables, CSV/JSON helpers, and ordered
      table export artifacts without serializing rawfiles.

94. Deck rawfile ASCII artifacts.
    - Status: completed in this deck rawfile ASCII artifact slice.
    - Python, Rust, and TypeScript selected deck executions now produce
      deterministic in-memory ASCII rawfile artifacts for accepted `.control`
      `write <rawfile> ...` markers.
    - Each selected execution exposes rawfile artifact count/list metadata plus
      stable rawfile artifact table, CSV, compact JSON, and host-native record
      summaries while `wrdata` markers and filesystem writes remain
      metadata-only.

95. Deck WRDATA ASCII artifacts.
    - Status: completed in this deck WRDATA ASCII artifact slice.
    - Python, Rust, and TypeScript selected deck executions now produce
      deterministic in-memory ASCII data-file artifacts for accepted
      `.control` `wrdata <file> ...` markers.
    - Each selected execution exposes WRDATA artifact count/list metadata plus
      stable WRDATA artifact table, CSV, compact JSON, and host-native record
      summaries while filesystem writes remain metadata-only.

96. Deck WRDATA rawfile option rendering artifacts.
    - Status: completed in this deck WRDATA rawfile option rendering artifact
      slice.
    - Python, Rust, and TypeScript WRDATA artifacts now carry accepted
      rawfile/data-write option inventories through stable `Options` /
      `RawfileOptionList` summary fields.
    - In-memory WRDATA data files now render deterministic `VectorNames` and
      `Scale` metadata when accepted `set wr_vecnames` and `set wr_singlescale`
      controls are present, while filesystem writes remain metadata-only.

97. Deck WRDATA probe column artifacts.
    - Status: completed in this deck WRDATA probe column artifact slice.
    - Python, Rust, and TypeScript `format_deck_wrdata_ascii` /
      `formatDeckWrdataAscii` helpers now treat explicit
      `wrdata <file> <probes...>` marker probes as data-file column selectors.
    - In-memory WRDATA data files preserve the scale column plus requested
      matching probe columns in marker order, while filesystem writes remain
      metadata-only.

98. Deck WRDATA unmatched probe artifact inventories.
    - Status: completed in this deck WRDATA unmatched probe artifact slice.
    - Python, Rust, and TypeScript WRDATA artifact summaries now expose stable
      matched and unmatched probe counts/lists beside the requested probe list.
    - Stable WRDATA artifact table, CSV, compact JSON, and host-native record
      exports make ignored `wrdata` probe names auditable while keeping
      filesystem writes metadata-only.

99. Deck rawfile write probe artifact inventories.
    - Status: completed in this deck rawfile write probe artifact slice.
    - Python, Rust, and TypeScript rawfile artifact summaries now expose stable
      matched and unmatched probe counts/lists beside the requested `write`
      probe list.
    - In-memory ASCII rawfile artifacts now keep the scale column plus matching
      requested vector columns, while stable table, CSV, compact JSON, and
      host-native record exports make ignored `write` probe names auditable.

100. Deck control policy diagnostic artifacts.
    - Status: completed in this deck control policy artifact slice.
    - Python, Rust, and TypeScript selected executions now expose
      policy-blocked `.control` commands as stable artifacts with line,
      category, command, code, severity, and message fields.
    - Stable table, CSV, compact JSON, and host-native record exports make
      `source` / `shell`, `cd`, control-flow, and variable/state policy
      diagnostics auditable while preserving explicit non-execution behavior.

101. Deck control policy summary artifacts.
    - Status: completed in this deck control policy summary artifact slice.
    - Python, Rust, and TypeScript selected executions now group
      policy-blocked `.control` command artifacts by category with stable
      counts, line lists, command lists, code lists, and severity lists.
    - Matching table, CSV, compact JSON, and host-native record exports make the
      policy surface easier to inventory without parsing row-level diagnostic
      artifacts.

102. Deck control policy run-artifact inventories.
    - Status: completed in this deck control policy run-artifact inventory slice.
    - Python, Rust, and TypeScript selected run artifacts now expose
      policy-blocked `.control` command counts, category lists, code lists, and
      severity lists beside existing command, write-marker, rawfile-option, and
      diagnostic inventories.
    - Stable run-artifact table, CSV, compact JSON, `table_artifacts`, and
      host-native record exports now surface the policy inventory without
      requiring callers to parse separate policy artifact tables.

103. Deck control policy table export artifacts.
    - Status: completed in this deck control policy table export artifact slice.
    - Python, Rust, and TypeScript selected executions now include
      `control-policy` and `control-policy-summary` entries in stable table
      inventories whenever policy-blocked `.control` commands are present.
    - The ordered table export artifacts now carry those row-level and
      category-summary policy tables with stable table, CSV, compact JSON, and
      host-native record payloads, and selected-run `TableList` metadata names
      the exported policy tables beside result and run-artifact tables.

104. Deck output-plan inventory artifacts.
    - Status: completed in this deck output-plan inventory artifact slice.
    - Python, Rust, and TypeScript selected executions now expose stable
      output-plan artifacts with selected result-column, output-probe,
      output-directive, and table inventories.
    - Matching table, CSV, compact JSON, and host-native record exports let
      host and browser integrations audit the output plan without reparsing
      selected result tables or selected-run artifact rows.

105. Deck output-plan table export artifacts.
    - Status: completed in this deck output-plan table export artifact slice.
    - Python, Rust, and TypeScript selected executions now include
      `output-plan` in stable table inventories and selected-run `TableList`
      metadata.
    - Ordered table export artifacts now carry the output-plan table with
      stable table, CSV, compact JSON, and host-native record payloads beside
      result, optional side tables, and selected-run artifact exports.

106. Deck output-plan directive-kind artifacts.
    - Status: completed in this deck output-plan directive-kind artifact slice.
    - Python, Rust, and TypeScript selected output-plan artifacts now expose
      normalized selected output directive kind counts/lists beside selected
      directive tokens.
    - Table, CSV, compact JSON, and host-native record exports make `.save`,
      `.probe`, `.print`, and `.plot` selection provenance auditable without
      parsing directive strings.

107. Deck output-plan directive analysis-kind artifacts.
    - Status: completed in this deck output-plan directive analysis-kind
      artifact slice.
    - Python, Rust, and TypeScript selected output-plan artifacts now expose
      normalized selected output directive analysis scope counts/lists beside
      directive kind inventories.
    - Table, CSV, compact JSON, and host-native record exports distinguish
      global `.save` / `.probe` selections from scoped `.probe`, `.print`, and
      `.plot` selections without reparsing directive cards.

108. Deck output-plan directive line artifacts.
    - Status: completed in this deck output-plan directive line artifact
      slice.
    - Python, Rust, and TypeScript selected output-plan artifacts now expose
      selected output directive source line counts/lists beside directive
      scope inventories.
    - Table, CSV, compact JSON, and host-native record exports make selected
      `.save`, `.probe`, `.print`, and `.plot` provenance traceable back to
      deck source lines without reparsing directive cards.

109. Deck output-plan probe source line artifacts.
    - Status: completed in this deck output-plan probe source line artifact
      slice.
    - Python, Rust, and TypeScript selected output-plan artifacts now expose
      selected output probe source line counts/lists aligned with selected
      output-probe inventories.
    - Table, CSV, compact JSON, and host-native record exports make each
      deduplicated selected output probe traceable to the deck source line that
      first selected it for that analysis.

110. Deck output-plan result row artifacts.
    - Status: completed in this deck output-plan result row artifact slice.
    - Python, Rust, and TypeScript selected output-plan artifacts now expose
      selected result row counts beside result-column inventories.
    - Table, CSV, compact JSON, and host-native record exports make the shape
      of the selected result table auditable from the output-plan artifact
      without reparsing the result table itself.

111. Deck output-plan analysis source artifacts.
    - Status: completed in this deck output-plan analysis source artifact
      slice.
    - Python, Rust, and TypeScript selected output-plan artifacts now expose
      selected analysis line/source metadata beside directive inventories.
    - Table, CSV, compact JSON, and host-native record exports make the
      selected plan's analysis line and source name auditable from the
      output-plan artifact without reparsing the analysis plan.

112. Deck output-plan analysis output-node artifacts.
    - Status: completed in this deck output-plan analysis output-node artifact
      slice.
    - Python, Rust, and TypeScript selected output-plan artifacts now expose
      selected analysis output-node metadata beside analysis line/source
      provenance.
    - Table, CSV, compact JSON, and host-native record exports make `.tf`,
      `.sens`, and `.noise` selected output nodes auditable from the
      output-plan artifact without reparsing the analysis plan.

113. Deck output-plan analysis sweep artifacts.
    - Status: completed in this deck output-plan analysis sweep artifact slice.
    - Python, Rust, and TypeScript selected output-plan artifacts now expose
      selected sweep kind/count/value metadata, AC/noise frequency bounds, and
      transient timing plus `UIC` metadata beside analysis line/source/output
      provenance.
    - Table, CSV, compact JSON, and host-native record exports make `.dc`,
      `.ac`, `.tran`, and `.noise` sweep inputs auditable from the output-plan
      artifact without reparsing the analysis plan.

114. Deck whole-run analysis execution.
    - Status: completed in this deck whole-run execution slice.
    - Python, Rust, and TypeScript now expose `run_deck` / `runDeck` whole-deck
      executors that run every parsed `.op`, `.dc`, `.ac`, `.tran`, `.tf`,
      `.sens`, and `.noise` analysis card in source order while preserving
      duplicate directives and defaulting analysis-less decks to an implicit
      `.op`.
    - Whole-run executions aggregate ordered selected-run artifacts as stable
      table, CSV, compact JSON, and host-native record exports, and each
      selected-run artifact now carries whole-deck analysis kind/directive
      inventories beside the selected analysis directive metadata.

115. Nonlinear convergence hardening.
   - Status: completed in this nonlinear convergence hardening slice.
   - Python, Rust, and TypeScript DC operating-point solves now apply a
     configurable Newton update limit only when nonlinear devices are present,
     keeping linear one-pass solves unchanged.
   - `DcResult.diagnostics` now reports the active Newton step limit, clipped
     Newton-step count, and minimum damping factor so difficult deck
     convergence is auditable without reparsing iteration traces.
   - Cross-language tests cover both the inactive linear sparse-ladder path and
     a damped nonlinear first-step solve with convergence aids disabled.

116. Device model behavior audit fixtures.
   - Status: completed in this device model behavior audit fixture slice.
   - Python, Rust, and TypeScript now expose matching runnable
     `device_model_behavior_audit_fixtures` /
     `deviceModelBehaviorAuditFixtures` one-device DC bias fixtures for diode,
     BJT, JFET, and Level-1 MOS models.
   - Each fixture carries the normalized model card, a constructed executable
     circuit, reference deck lines, selected probe node, and stable expected
     probe-voltage window so device-depth audits can compare behavior rather
     than only model-card aliases.
   - Cross-language tests execute every fixture through DC operating-point
     solving and verify the probe window plus reference-deck metadata.

117. Device model temperature audit fixtures.
   - Status: completed in this device model temperature audit fixture slice.
   - Python, Rust, and TypeScript now expose matching runnable
     `device_model_temperature_audit_fixtures` /
     `deviceModelTemperatureAuditFixtures` one-device DC temperature-sweep
     fixtures for diode, BJT, JFET, and Level-1 MOS models.
   - Each fixture extends the runnable behavior circuits with `.temp`
     reference-deck metadata, nominal temperature, energy-gap metadata,
     temperature-behavior notes, selected probe node, and stable
     per-temperature probe-voltage windows.
   - Cross-language tests execute every fixture through DC temperature sweeps
     and verify the probe windows plus reference-deck metadata, including the
     explicit JFET temperature-invariant policy.

118. Device model capacitance audit fixtures.
   - Status: completed in this device model capacitance audit fixture slice.
   - Python, Rust, and TypeScript now expose matching runnable
     `device_model_capacitance_audit_fixtures` /
     `deviceModelCapacitanceAuditFixtures` one-device AC fixtures for diode,
     BJT, JFET, and Level-1 MOS model cards.
   - Each fixture carries the normalized model card, an executable AC circuit,
     `.ac` reference deck lines, selected probe node, frequency, stable
     expected probe-magnitude window, and a short capacitance-behavior note.
   - The JFET fixture deliberately records the current conductance-only AC
     response because JFET capacitance remains intentionally unmodeled until
     that policy is chosen.
   - Cross-language tests execute every fixture through AC sweep solving and
     verify the probe-magnitude window plus reference-deck metadata.

119. Device model noise audit fixtures.
   - Status: completed in this device model noise audit fixture slice.
   - Python, Rust, and TypeScript now expose matching runnable
     `device_model_noise_audit_fixtures` /
     `deviceModelNoiseAuditFixtures` one-device `.noise` fixtures for diode,
     BJT, JFET, and Level-1 MOS model cards.
   - Each fixture carries the normalized model card, an executable circuit,
     `.noise` reference deck lines, output node, input source, frequency,
     expected noise element/type, stable source/output PSD windows, and a
     short noise-behavior note.
   - Rust and TypeScript now include diode/BJT shot-noise sources and JFET
     channel thermal-noise sources in addition to the existing resistor and
     Level-1 MOS thermal noise coverage.
   - TypeScript BJT small-signal behavior and AC stamping now derive
     transconductance and diffusion capacitance from the converged operating
     point, matching Python and Rust.
   - Cross-language tests execute every fixture through `.noise` solving and
     verify the PSD windows plus reference-deck metadata.

120. Device model charge audit fixtures.
   - Status: completed in PR 6794.
   - Python, Rust, and TypeScript now expose matching runnable
     `device_model_charge_audit_fixtures` /
     `deviceModelChargeAuditFixtures` one-device `.tran` fixtures for diode,
     BJT, JFET, and Level-1 MOS model cards.
   - Each fixture carries the normalized model card, an executable circuit,
     `.tran` reference deck lines, selected probe node, timestep, stoptime,
     explicit terminal storage capacitance, stable first/final probe-voltage
     windows, and a short charge-behavior note.
   - The fixtures deliberately keep explicit terminal capacitors for comparable
     probe windows while recording which model-card charge terms are
     transient-stamped and which JFET/MOS charge policies remain
     explicit-storage-only.
   - Cross-language tests execute every fixture through transient solving and
     verify the probe windows plus reference-deck metadata.

121. Diode model-card transient charge stamping.
   - Status: completed in PR 6798.
   - Python, Rust, and TypeScript transient solvers now stamp diode
     model-card `CJO` / `junction_capacitance` / `junctionCapacitance` and
     `TT` / `transit_time` / `transitTime` as an anode-cathode storage
     companion.
   - The transient companion uses the previous diode bias to derive
     `C = Cjo + Tt * gd`, reuses the existing Euler/trapezoidal/Gear-2
     capacitor history policy, and seeds the synthetic diode charge state from
     the initial operating point.
   - Cross-language tests cover both junction-capacitance current-step delay
     and transit-time forward-charge retention after turnoff.

122. BJT model-card transient charge stamping.
   - Status: completed in PR 6803.
   - Python, Rust, and TypeScript transient solvers now stamp BJT model-card
     `CJE` / `base_emitter_capacitance` / `baseEmitterCapacitance`,
     `CJC` / `base_collector_capacitance` / `baseCollectorCapacitance`,
     `TF` / `forward_transit_time` / `forwardTransitTime`, and
     `TR` / `reverse_transit_time` / `reverseTransitTime` as base-emitter and
     base-collector storage companions.
   - The companions use the previous junction bias to derive
     `Cbe = Cje + Tf * gm_forward` and `Cbc = Cjc + Tr * gm_reverse`, reuse the
     existing Euler/trapezoidal/Gear-2 capacitor history policy, and seed the
     synthetic BJT charge states from the initial operating point.
   - Cross-language tests cover base-emitter capacitance current-step delay and
     forward transit-time charge retention after turnoff.

123. JFET model-card transient charge stamping.
   - Status: completed in PR 6812.
   - Python, Rust, and TypeScript transient solvers now stamp JFET model-card
     `CGS` / `gate_source_capacitance` / `gateSourceCapacitance` and `CGD` /
     `gate_drain_capacitance` / `gateDrainCapacitance` as fixed gate-source and
     gate-drain storage companions.
   - The companions reuse the existing Euler/trapezoidal/Gear-2 capacitor
     history policy, seed synthetic JFET charge states from the initial
     operating point, and contribute matching small-signal AC susceptance.
   - Cross-language tests cover gate-step delay and high-frequency gate-drive
     shunting.

124. MOS Level-1 transient overlap charge stamping.
   - Status: completed in PR 6816.
   - Python, Rust, and TypeScript transient solvers now stamp Level-1 MOS
     model-card `CGSO` / `gate_source_overlap_capacitance`, `CGDO` /
     `gate_drain_overlap_capacitance`, and `CGBO` /
     `gate_bulk_overlap_capacitance` as fixed gate-source, gate-drain, and
     gate-body overlap storage companions.
   - The companions reuse the existing Euler/trapezoidal/Gear-2 capacitor
     history policy and seed the synthetic MOS overlap charge states from the
     initial operating point.
   - Cross-language tests cover MOS gate-step delay through `CGSO` overlap
     storage, and charge-audit fixtures now record the MOS overlap terms as
     transient-stamped.

125. MOS Level-1 transient bulk-junction charge stamping.
   - Status: completed in PR 6822.
   - Python, Rust, and TypeScript transient solvers now stamp Level-1 MOS
     model-card `CBS` / `source_bulk_capacitance` / source-bulk
     capacitance and `CBD` / `drain_bulk_capacitance` / drain-bulk
     capacitance as zero-bias source-body and drain-body storage companions.
   - The companions reuse the existing Euler/trapezoidal/Gear-2 capacitor
     history policy and seed the synthetic MOS bulk-junction charge states
     from the initial operating point.
   - Cross-language tests cover drain-step delay through `CBD` bulk-junction
     storage, and charge-audit fixtures now record the MOS bulk terms as
     transient-stamped.

126. MOS Level-1 bulk-junction depletion charge shaping.
   - Status: completed in PR 6827.
   - Python, Rust, and TypeScript Level-1 MOS model-card parameters now include
     `PB` / `bulk_junction_potential` / `PB` and `MJ` /
     `bulk_junction_grading_coefficient` / `MJ` for bulk-junction depletion
     shaping.
   - AC operating-point capacitance reports and transient MOS source-body /
     drain-body charge companions shape `CBS` / `source_bulk_capacitance` /
     `CBS` and `CBD` / `drain_bulk_capacitance` / `CBD` under reverse
     source-bulk and drain-bulk bias.
   - Cross-language tests cover model-card alias propagation and a
     reverse-biased drain-step transient where `MJ` reduces the effective
     `CBD` companion relative to the zero-bias capacitance.
   - Charge-audit fixtures now record MOS bulk-junction storage as
     depletion-shaped while preserving the same runnable one-device `.tran`
     fixture shape.

127. Device model reference-deck audit matrix.
   - Status: completed in PR 6832.
   - Python, Rust, and TypeScript now expose a flattened
     `device_model_reference_deck_audit_fixtures` /
     `deviceModelReferenceDeckAuditFixtures` surface that summarizes the
     runnable DC operating-point, temperature, AC capacitance, noise, and
     transient charge fixture decks for every supported diode, BJT, JFET, and
     Level-1 MOS model family.
   - Each row carries the normalized model card, analysis kind, stable
     reference label, expected behavior note, and reference deck lines so model
     depth audits can compare coverage without re-discovering fixture families.
   - Cross-language tests lock the four-family by five-analysis coverage matrix
     and verify every row has model-card deck metadata, an `.end` boundary, and
     a non-empty behavior note.

128. Device model reference-deck audit table.
   - Status: completed in PR 6836.
   - Python, Rust, and TypeScript now expose stable
     `format_device_model_reference_deck_audit_table` /
     `formatDeviceModelReferenceDeckAuditTable` helpers for the flattened
     reference-deck audit matrix.
   - The tab-separated table locks `name`, `kind`, `analysis`, `model`,
     `reference`, `expected_behavior`, and `deck_lines` columns so release
     checks and reference-deck comparisons can diff model-depth coverage
     without inspecting every fixture object.
   - Cross-language tests lock the exact header, row count, and first/last rows
     for the diode operating-point and Level-1 MOS transient-storage audit
     entries.

129. Device model reference-deck audit release gate.
   - Status: completed in PR 6840.
   - Python, Rust, and TypeScript now expose
     `device_model_reference_deck_audit_gate` /
     `deviceModelReferenceDeckAuditGate` helpers plus stable
     `format_device_model_reference_deck_audit_gate_report` /
     `formatDeviceModelReferenceDeckAuditGateReport` output.
   - The release gate validates the required diode, BJT, JFET, and Level-1 MOS
     by operating-point, temperature, AC, noise, and transient coverage matrix,
     checks each row has documented model/reference/deck metadata, and reports
     missing coverage as stable tab-separated issue rows.
   - Cross-language tests lock the passing report header/body and a negative
     missing `NMOS:tran` coverage issue so the audit matrix is enforceable by
     release automation.

130. Device model reference-deck audit record exports.
   - Status: completed in PR 6844.
   - Python, Rust, and TypeScript now expose
     `device_model_reference_deck_audit_records` /
     `deviceModelReferenceDeckAuditRecords` helpers plus stable
     `format_device_model_reference_deck_audit_csv` /
     `formatDeviceModelReferenceDeckAuditCsv` and
     `format_device_model_reference_deck_audit_json` /
     `formatDeviceModelReferenceDeckAuditJson` output.
   - The exports reuse the reference-deck audit table contract to provide
     header-keyed records plus browser/release-friendly CSV and compact JSON
     for dashboards, release automation, and reference-deck comparison tools.
   - Cross-language tests lock the first diode operating-point row, final
     Level-1 MOS transient-storage row metadata, CSV comma escaping, and JSON
     parseability so downstream consumers can rely on the audit matrix shape.

131. Device model reference-deck audit summaries.
   - Status: completed in PR 6850.
   - Python, Rust, and TypeScript now expose
     `device_model_reference_deck_audit_summary` /
     `deviceModelReferenceDeckAuditSummary` helpers plus stable summary table,
     header-keyed records, CSV, and compact JSON output.
   - The summary condenses the reference-deck audit matrix by model family,
     preserving expected analysis order, missing-analysis gaps, total deck-line
     counts, and reference labels for release dashboards and coverage reviews.
   - Cross-language tests lock the four expected summary rows, CSV/JSON
     exports, and a negative missing `NMOS:tran` summary case so downstream
     consumers can audit coverage without scanning every fixture row.

132. Berkeley SPICE grammar foundation.
   - Status: completed in PR 6861.
   - `code/grammars/spice/berkeley.tokens` and
     `code/grammars/spice/berkeley.grammar` now define the first shared syntax
     contract for normalized Berkeley SPICE logical cards.
   - The grammar intentionally parses stable card shape and preserves generic
     card atoms. Semantic passes own device arity, model legality, expression
     resolution, include/library loading, control policies, and
     dialect-specific behavior.
   - Grammar-tool tests parse, validate, cross-validate, and compile the token
     and parser grammars so future parser rewrites can break old ad hoc parsing
     behavior against a checked source grammar instead of guessing.

133. Rust Berkeley SPICE parser/app facade.
   - Status: completed in PR 6902.
   - Rust `spice-netlist-parser` now exposes a Berkeley SPICE logical-card
     syntax facade with grammar metadata, normalized cards, leading `+`
     continuation handling, source spans, token names, stable diagnostics, and
     analysis inventory.
   - The Rust app-deck facade parses syntax once, reports syntax/lowering
     diagnostics, exposes analysis inventory, and can run source-order or
     selected runnable analyses through the simulator parser for Mosaic-backed
     UI runtimes.

134. Rust Berkeley syntax lowerer routing.
   - Status: completed in PR 6907.
   - Rust `spice-netlist-parser` semantic lowering now routes through the
     Berkeley syntax facade by default, so normalized logical cards, leading
     `+` continuations, source spans, and stable syntax diagnostics drive the
     existing simulator parser.
   - The parser no longer performs a duplicate physical-line pass before
     semantic lowering, preserving the Mosaic app facade as the Rust runtime
     entrypoint over one shared syntax substrate.

135. Python and TypeScript Berkeley syntax facade parity.
   - Status: completed in PR 6924.
   - Python and TypeScript `spice-engine` surfaces now mirror the Rust Berkeley
     logical-card syntax facade with embedded grammar metadata, normalized
     cards, leading `+` continuation handling, source spans, token names,
     stable diagnostics, and analysis inventory.
   - Cross-language tests lock the shared grammar metadata and representative
     logical-card / diagnostic behavior so future parser work can keep editor
     and app-substrate surfaces in sync.

136. Rust Berkeley Mosaic app artifacts.
   - Status: completed in PR 6934.
   - Rust `spice-netlist-parser` now exposes Berkeley app-deck artifacts for
     Mosaic-facing UI surfaces, including canonical normalized source,
     syntax-card-indexed result tables, output-plan artifacts, run-artifact
     summaries, and rawfile / wrdata artifact metadata.
   - The slice builds on the public Berkeley parser contract and existing engine
     deck artifacts as a Rust-only app-substrate acceleration layer while
     Python and TypeScript parser parity remains aligned for shared syntax
     behavior.

137. Rust Berkeley Mosaic waveform inspection.
   - Status: completed in PR 6936.
   - Rust `spice-netlist-parser` Berkeley app-deck artifacts now expose numeric
     plot-ready waveform series derived from stable result tables for Mosaic UI
     surfaces.
   - Selected-card waveform access covers transient series, and AC result
     tables derive probe-grouped magnitude and phase series while preserving the
     same public parser contract.

138. Rust Berkeley Mosaic app session state.
   - Status: completed in PR 6944.
   - Rust `spice-netlist-parser` Berkeley app-deck snapshots now expose source
     fingerprints, selected-analysis state, runnable/blocked status,
     diagnostics, table/probe metadata, and selected waveform availability for
     Mosaic UI surfaces.
   - The slice preserves the Rust app facade as a UI substrate over the public
     Berkeley parser contract while Python and TypeScript parser parity remains
     aligned for shared syntax behavior.

139. Rust Berkeley Mosaic editor controls.
   - Status: completed in PR 6948.
   - Rust `spice-netlist-parser` Berkeley app-deck controls now expose
     per-analysis select/run/table/waveform actions with stable enabled states
     and disabled reasons for Mosaic UI surfaces.
   - The slice keeps editor controls derived from app session state so UI hosts
     can render parser and execution status without duplicating simulator
     internals.

140. Rust Berkeley Mosaic editor command plans.
   - Status: completed in PR 6954.
   - Rust `spice-netlist-parser` Berkeley app-deck command plans now expose
     per-analysis command IDs, action kinds, target names, enabled states, and
     disabled reasons for Mosaic host menu, toolbar, and panel wiring.
   - The slice keeps command descriptors derived from editor controls so hosts
     can dispatch select/run/table/waveform actions without reinterpreting
     labels or parser internals.

141. Rust Berkeley Mosaic persisted editor state.
   - Status: completed in PR 6958.
   - Rust `spice-netlist-parser` Berkeley app-deck snapshots now resolve saved
     selected-card and active-command IDs against the current deck, including
     stale-state repair flags for source edits.
   - The slice keeps persisted UI state derived from command descriptors and
     app session state so hosts can restore selection without duplicating parser
     or simulator internals.

142. Rust Berkeley Mosaic host surface.
   - Status: completed in PR 6961.
   - Rust `spice-netlist-parser` Berkeley app-deck host surfaces now expose
     stable source, diagnostics, analysis, table, and waveform panel
     descriptors with panel IDs, kind tags, target names, enabled states, active
     state, and disabled reasons.
   - The slice keeps panel routing derived from persisted editor-state
     snapshots so Mosaic shells can wire UI regions without duplicating parser
     or simulator internals.

143. Rust Berkeley Mosaic host wire export.
   - Status: completed in PR 6965.
   - Rust `spice-netlist-parser` Berkeley app host-surface wire snapshots now
     expose schema-versioned native and JSON surfaces for Mosaic packaging and
     WebAssembly embedding.
   - The wire export flattens panel descriptors, diagnostics, active-panel IDs,
     repaired persisted editor-state metadata, and lower-case panel /
     diagnostic kinds so product shells can consume the app surface without
     Rust struct coupling.

144. Rust Berkeley Mosaic app package manifest.
   - Status: completed in PR 6971.
   - Rust `spice-netlist-parser` now exposes a schema-versioned Berkeley Mosaic
     app package manifest plus JSON helper for WebAssembly and product-shell
     capability discovery.
   - The manifest advertises the Berkeley grammar version, host-surface wire
     schema, source-fingerprint algorithm, panel kinds, editor action kinds,
     command targets, runnable analysis directives, and artifact capabilities
     before a host opens a deck.

145. Rust Berkeley Mosaic app bootstrap snapshot.
   - Status: completed in PR 6980.
   - Rust `spice-netlist-parser` now exposes schema-versioned Berkeley Mosaic
     app bootstrap snapshots plus JSON helpers that combine the static package
     manifest with deck-specific host-surface wire exports.
   - The bootstrap payload preserves blocked-deck diagnostics, repaired
     persisted editor-state metadata, active panels, package capabilities, and
     run availability in one startup envelope for Mosaic, WebAssembly, and
     product-shell startup.

146. Rust Berkeley Mosaic app startup summary.
   - Status: completed in PR 6988.
   - Rust `spice-netlist-parser` now exposes schema-versioned Berkeley Mosaic
     app startup summaries plus JSON helpers that derive compact ready/blocked
     startup routes from bootstrap payloads.
   - The summary preserves package name, source fingerprint, repaired
     persisted editor-state IDs, stale-state flags, active panel, diagnostic
     count, and blocking reason so Mosaic, WebAssembly, and product shells can
     make startup routing decisions without walking every host panel.

147. Rust Berkeley Mosaic app launch plan.
   - Status: completed in PR 6995.
   - Rust `spice-netlist-parser` now exposes schema-versioned Berkeley Mosaic
     app launch plans plus JSON helpers that derive ready/blocked product-shell
     entry actions from bootstrap payloads.
   - The launch plan preserves package name, source fingerprint, startup route,
     primary entry panel, entry target, repaired persisted editor-state IDs,
     stale-state flags, panel action descriptors, diagnostic count, and blocking
     reason so Mosaic, WebAssembly, and product shells can launch the correct
     surface without walking every host panel.

148. Rust Berkeley Mosaic app readiness report.
   - Status: completed in PR 7005.
   - Rust `spice-netlist-parser` now exposes schema-versioned Berkeley Mosaic
     app readiness reports plus JSON helpers that summarize startup route health
     from Berkeley app bootstrap payloads.
   - The report preserves package name, source fingerprint, startup route,
     parsed/execution availability, entry panel/action, panel/action
     availability counts, diagnostic severity counts, repaired persisted
     editor-state flags, and blocking reason so Mosaic, WebAssembly, and product
     shells can gate startup and telemetry without walking every host panel.

149. Rust Berkeley Mosaic app shell handoff.
   - Status: completed in PR 7012.
   - Rust `spice-netlist-parser` now exposes schema-versioned Berkeley Mosaic
     app shell handoffs plus JSON helpers that package the manifest, startup
     summary, launch plan, and readiness report into one compact bootstrap
     envelope.
   - The handoff preserves package capabilities, route readiness, launch entry
     actions, panel/action availability counts, diagnostic severity counts,
     repaired persisted editor-state flags, and blocking reason so Mosaic,
     WebAssembly, and product shells can start without walking the full
     host-surface export.

150. Rust Berkeley Mosaic app shell status.
   - Status: completed in PR 7022.
   - Rust `spice-netlist-parser` now exposes schema-versioned Berkeley Mosaic
     app shell statuses plus JSON helpers that derive a compact route, severity,
     status message, entry action, and diagnostic counts from shell handoffs.
   - The status payload preserves package name, source fingerprint, ready/blocked
     route, entry panel and primary action, diagnostic severity counts, and
     blocking reason so Mosaic, WebAssembly, and product shells can render
     startup chrome and telemetry without inspecting the full launch/readiness
     payload.

151. Rust Berkeley Mosaic app shell telemetry.
   - Status: completed in PR 7032.
   - Rust `spice-netlist-parser` now exposes schema-versioned Berkeley Mosaic
     app shell telemetry plus JSON helpers that derive compact route,
     entry-action, availability, diagnostic, repaired-state, and capability-count
     metrics from shell handoffs.
   - The telemetry payload preserves package name, source fingerprint,
     ready/blocked route, severity, status message, primary action, panel/action
     availability counts, diagnostic severity counts, stale/repaired-state
     flags, and advertised capability count so Mosaic, WebAssembly, and product
     shells can emit startup telemetry without inspecting the full
     launch/readiness payload.

152. Rust Berkeley Mosaic app shell event log.
   - Status: completed in PR 7042.
   - Rust `spice-netlist-parser` now exposes schema-versioned Berkeley Mosaic
     app shell event logs plus JSON helpers that derive stable status, route,
     primary-action, diagnostic, repaired-state, and capability events from
     shell handoffs.
   - The event log preserves package name, source fingerprint, ready/blocked
     route, event severity, status messages, primary action, diagnostic counts,
     repaired-state count, and advertised capability count so Mosaic,
     WebAssembly, and product shells can append startup event streams without
     inspecting the full launch/readiness payload.

## Backlog

1. Grammar-backed parser and app facade.
   - Keep Python and TypeScript parser contract parity aligned with the Rust
     syntax facade as the grammar evolves, even if that breaks current
     pre-release parser APIs.
   - Continue expanding the Rust Mosaic app facade beyond host panel surfaces
     toward packaging, WebAssembly embedding, and product integration backed by
     the same public parser contract.

2. Deck compatibility follow-up.
   - Expand deck-owned output compatibility beyond source-order analysis
     execution and stable artifact exports toward nested sweeps, raw-format
     interoperability, and remaining vendor-style output controls.
   - Expand deck-controlled output-plan integration beyond stable table
     routing and scoped `.save`, `.probe`, `.print`, `.plot`, and `.four`
     selection toward full SPICE compatibility.
   - Expand the deliberate `.control` subset beyond simple analysis/output
     command routing, including control flow, variables, and script execution
     policy.

3. Production solver core follow-up.
   - Sparse real/complex matrix paths now have cross-language native coverage,
     and Python real DC solves now use an optional SciPy sparse-LU backend with
     structured native fallback metadata.
   - Rust, Python, and TypeScript DC diagnostics now expose stable solver
     profiles for sparse activation, structural nonzeros, density, fill-in,
     backend choice, and fallback reasons.
   - Remaining solver-core work should focus on nonlinear hardening: Newton
     damping, device limiting, tolerance policy, and additional convergence
     diagnostics for difficult transistor decks.

4. Device model depth.
   - Audit diode, BJT, JFET, and MOS Level 1 behavior against reference decks.
   - Decide whether Level 2/3 MOS is in scope before BSIM; if BSIM lands, make
     Rust the first fast path and port stable semantics outward.
   - Expand temperature behavior, capacitance, noise, charge conservation, model
     card aliases, and error messages.

5. Analysis completion.
   - Generalize pole-zero beyond constrained fixture helpers.
   - Expand nonlinear distortion coverage.
   - Expand parsed `.FOUR` / `.MEASURE` integration across output plans and
     nested sweeps.
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

## Suggested PR Queue

1. Device model depth.
2. Analysis completion.
3. Mixed-signal integration.
