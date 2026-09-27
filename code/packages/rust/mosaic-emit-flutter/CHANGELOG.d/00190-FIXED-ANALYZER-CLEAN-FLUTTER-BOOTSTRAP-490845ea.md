### Fixed - analyzer-clean Flutter bootstrap

Project emission now owns `analysis_options.yaml`, the matching
`flutter_lints` dependency, and a package-name-correct Mosaic widget smoke test,
so the documented `flutter create` bootstrap no longer installs a broken
counter-app test or an unresolved lint include. Indexed and plain `For`
lowering also omits authored item bindings when the generated body never reads
them. CI analyzes the whole generated project and runs its permissive widget
test instead of hiding bootstrap failures behind `dart analyze lib`.

