### Added — `ComponentRegistry` public type

- New `ComponentRegistry` + `ComponentRef` types re-exported from
  the crate root. The registry maps PascalCase tag names →
  `(xmlns_prefix, xmlns_value, package_name)` and is the input the
  emitter consumes when resolving a non-kernel tag.
- The CLI (mosaic-compile) is responsible for populating the
  registry from parsed dependency manifests; the emitter takes the
  already-resolved data and emits the XAML reference.
- Tests use the registry directly — `ComponentRegistry::new()` +
  `.register("Grid", "grid", "using:Mosaic.Package.Grid", "mosaic-pkg-grid")`.

