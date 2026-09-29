### Fixed - project-shell widget-slot hydration

Generated Flutter shells now accept a host-provided `Widget` in the props map
for `node` slots and pass it to the component through `mosaicWidget`. Missing or
mistyped values retain the deterministic `SizedBox.shrink` fallback.

