### Added — Section sub-tags as direct nodes error

`HostTableHead` / `HostTableBody` / `HostTableFoot` / `HostTableColGroup`
appearing outside a HostTable (i.e. as direct children of a non-table
container) surface as `UnsupportedPrimitive("HostTable<X> outside HostTable")`.

