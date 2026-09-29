### Added — `HostTable` lowering (spec §5)

- `HostTable [name] { section sub-tags... }` lowers to a hand-rolled
  `<Grid>` with `Grid.RowDefinitions` driven by the present section
  sub-tags. Each section appears at most once per HostTable; duplicates
  produce a `DuplicateTableSection` error.

