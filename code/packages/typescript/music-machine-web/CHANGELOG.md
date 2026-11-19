# Changelog

## Unreleased

- Added `tests/error_paths.test.ts` and `tests/types_standalone.test.ts` covering previously unexercised error and edge paths (resource-budget guards in the text, portable-text and JSON parsers; builder section/motif/meter helpers; `TextScore`/`PortableScore` constructor budgets; render duration and sample-budget guards; `mixPcmSamples` overflow checks; validation scanners; fail-fast serializer stubs). Line coverage rises from 94.98% (Windows) to about 99.6%, giving the 95% gate real headroom. Threshold unchanged; no source behaviour changed.

## 0.1.0

- Initial release of `@coding-adventures/music-machine-web`.
- Sheet music text score parser with linear-time directive and token scanners.
- Canonical `music-machine-score/v2` multi-track parser, builder, motif capturer, and arranger.
- Deterministic JSON score interchange with depth bounding and prototype pollution prevention.
- Additive synthesizer and percussion kit sequence rendering with hi-hat choke dynamics.
- Browser Web Audio sink playback integration.
- Full VER15 proofs against CWE-20, CWE-190, CWE-400, CWE-674, CWE-1321, and CWE-1333.
