### Fixed — a quoted value no longer decides whether a button keeps its children (#15921)

`host_content_control_children` checked for a `Content=` attribute with
`attrs.contains(" Content=")`. `escape_xaml_attr` leaves spaces and `=` alone,
so an accessible name like `Open Content=details` contains that substring. A
label-less `HostButton` or `HostLink` with such a name dropped its children,
while the degradation analyzer (#15921) correctly reported that XAML keeps
them. `xaml_attrs_set` now matches attribute names outside quoted values.

