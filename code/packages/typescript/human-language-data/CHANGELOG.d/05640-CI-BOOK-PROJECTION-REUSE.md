### Fixed — repeated book projection in CI tests

- Generate the immutable repository book projection once for the book CLI test
  file. Nine independent assertions reuse a read-only map instead of rendering
  the whole corpus nine times. Mutable fixture tests continue regenerating.
- Keep the assertion timeout unchanged; give the shared corpus setup a bounded
  60-second budget. The local file run fell from 229 seconds to about 20 seconds.
