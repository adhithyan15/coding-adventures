---
category: Testing & coverage
---

# A Warrant source manifest must hash only the crate src tree

The fixed-point generated test harness accepts a source directory and uses it to write the
manifest's `source_sha256`. Its initial wrapper passed `CARGO_MANIFEST_DIR` itself instead of
`CARGO_MANIFEST_DIR/src`, so the test manifest hashed integration tests and generated harness
files while the proof manifest framed only `src/**/*.rs`. Both hashes were valid digests, but they
referred to different trees and the assurance loader correctly rejected the package.

Pass the exact crate `src` directory to the generated `run_main`, as the harness contract says.
Then regenerate the test manifest and verify its source digest equals `source_tree_framing(src)`
used by the proof/bench producer and checker. A zero-exit test run does not guarantee manifest
identity; compare the derived source-frame digest across all manifests.
