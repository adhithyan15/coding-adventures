## Unreleased — Rust Biquad/SOS fixture selection (AF-03)

- Registered `rust/dsp-filters` as a consumer of the shared `biquad-sos-v1`
  conformance fixture alongside Python and TypeScript.
- Expanded selector coverage to prove all three language consumers are
  selected, language-filtered runs stay scoped, version lookalikes stay out,
  and the selector fails closed if any required consumer is missing.
- Pinned the Rust Cargo dev-dependency trigger from `bounded-json` to
  `dsp-filters`, so changes to the JSON test helper select its consumer tests.
