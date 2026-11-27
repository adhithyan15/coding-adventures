# Changelog

## Unreleased

- Expose one-transition stepping and support bounded recursive finite JSON data
  in stimuli, state, and effects. Shared fixture conformance runs from the build
  driver outside the pure package boundary.
- Align the declared Node.js minimum with the repository CI runner.

## 0.1.0

- Add a pure reusable deterministic semantic engine with budgeted stepping,
  portable snapshots, replay, and separate batch and protected scheduler
  profiles; no command-line or host-output adapter is included.
- Add a synthetic CPU stepper, checked 16-bit bus, RAM, configuration ROM,
  simulated console, two board maps, and portable machine replay fixtures.
- Add a bounded adapter to the repository RV32I simulator and a portable
  instruction fixture that runs the same guest on both board maps.
