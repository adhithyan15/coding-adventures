### Added — `HostScroll` lowering (spec §4.3)

- `HostScroll` lowers to `<ScrollViewer>` wrapping its children.
  Direction keyword maps to scroll-bar visibility:
  - default (vertical): `VerticalScrollBarVisibility="Auto"` + `HorizontalScrollBarVisibility="Disabled"`
  - `direction: horizontal`: H=Auto, V=Disabled
  - `direction: both`: both Auto

