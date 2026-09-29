### Added - host-driven prop refresh callback

Generated Flutter shells register a prop-change handler on `MosaicHost` and
reapply the host's current props when native content-surface input changes
browser state. The default generated host keeps a no-op implementation, so
existing injected hosts remain source-compatible.

