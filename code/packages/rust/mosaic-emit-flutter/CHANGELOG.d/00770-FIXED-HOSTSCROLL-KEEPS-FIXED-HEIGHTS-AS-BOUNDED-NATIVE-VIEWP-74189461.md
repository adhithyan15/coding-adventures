### Fixed — HostScroll keeps fixed heights as bounded native viewports

- Fixed Flutter `HostScroll` lowering to preserve authored fixed heights as
  bounded native viewports instead of expanding to the full child content.
- Honored child `flex-grow` in height-bounded Columns so native page surfaces
  yield viewport space to shared drawers and utility panels.
