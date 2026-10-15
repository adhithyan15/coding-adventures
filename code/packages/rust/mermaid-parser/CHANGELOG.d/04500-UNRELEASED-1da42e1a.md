## [Unreleased]

### Fixed

- Integrate per-field curve configuration updates with the existing bounded edge-ID index; authored-source controls compare complete IR for repeated curve-only directives, duplicate IDs, animation/order/style preservation and unknown-edge refusal.

- Resolve Swimlane animation configurations through the same deterministic authored edge-ID index as class/style targets. Preserve duplicate IDs, actual update order, unknown-edge refusal and all prior work controls; repeated configurations no longer discover unrelated edges each time.
- Preserve node-shape metadata on edge endpoints and literal @{...} node-label text when parsing standalone animation configurations, with complete diagram equality controls.
