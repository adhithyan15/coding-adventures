## [Unreleased] — verify exact subgraph title glyph geometry

Retain incoming title offsets and metadata, with both exhaustive default config
literals adapted without changing their assertions. Add36 cross-layer cases
over four directions, node padding None/100/65536 and title margins
zero/100+7/65536+65536. Compare complete emitted glyph runs against independent
declared title boxes and require glyph extents inside those boxes; preserve
input layout exactly. Final3d full affected14-package suites pass855/0/0 over
32targets with strict alltargets EXIT0. Original common-origin work,8192 and
production-limit controls remain unchanged. Miri DEFERRED/PENDING, never passed.
