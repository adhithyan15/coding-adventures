### Fixed - Collision-safe repeated-loop projections

Each `For` loop now receives a distinct generated row-view-model and projection
name when a package-expanded component reuses an `as:` alias. The first alias
keeps its stable historical name and later loops receive numeric suffixes, so
TaskApp's sheet and task-list `row` loops no longer share the wrong collection.

