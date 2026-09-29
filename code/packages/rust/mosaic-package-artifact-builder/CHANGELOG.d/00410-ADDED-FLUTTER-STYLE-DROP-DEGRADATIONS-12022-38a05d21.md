### Added — Flutter style-drop degradations (#12022)

`analyze_package_degradations` now includes Flutter
`style.property-dropped` entries reported by `mosaic-emit-flutter`. The entries
remain non-gating until the known product/package losses are retired and the
UI84 hard gate is enabled.

