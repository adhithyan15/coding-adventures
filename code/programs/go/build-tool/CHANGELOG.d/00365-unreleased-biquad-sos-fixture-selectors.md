## Unreleased — shared Biquad/SOS fixture selects both DSP filter suites (AF-02)

- Added the `biquad-sos-v1` shared fixture as a repository-input relation for
  `python/dsp-filters` and `typescript/dsp-filters`.
- Added a selector regression proving both consumers run for fixture changes,
  language-filtered runs select only their own consumer, and versioned lookalike
  paths select neither.
