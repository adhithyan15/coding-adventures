### Group A — WinUI value translation (X5)

`build_style_fragment` gained a value-translation layer
(`translate_xaml_value`) below the X1 name-mapping and X4 color
PascalCasing. `css_property_to_xaml_setter` now returns
`Option<String>` so CSS-only properties can be dropped.

- **px-strip** — length setters (`FontSize`, `Height`, `Width`,
  `Padding`, `Margin`, `BorderThickness`, `CornerRadius`) emit bare
  numbers / `Thickness`: `12px`→`12`, `0,0,0,1px`→`0,0,0,1`.
- **drop CSS-only props** — `border-collapse`, `border-style`,
  `outline`, `text-decoration`, `box-shadow` return `None` (omitted,
  not emitted as invalid attrs / `<Setter>`s).
- **drop `Width="100%"`** — WinUI `Width` is a `Double`, not a
  percentage.
- **`text-align` → `TextAlignment`** with a PascalCase value
  (`center`→`Center`, `right`→`Right`, `left`→`Left`). The old output
  emitted `<Setter Property="TextAlign" Value="center"/>` — invalid
  on both the property name and the value.
- **`font-weight`** → WinUI `FontWeights` constant
  (`normal`→`Normal`, `bold`→`Bold`, `600`/`semibold`→`SemiBold`,
  `500`/`medium`→`Medium`).
- `{x:Bind …}` markup-extension values pass through unmangled (never
  px-stripped or case-mangled).

Tests: `x5_px_units_stripped_from_length_setters`,
`x5_css_only_properties_are_dropped`,
`x5_percentage_width_is_dropped`,
`x5_text_align_maps_to_textalignment_pascalcase`,
`x5_font_weight_maps_to_named_constant`,
`x5_binding_value_passes_through_unmangled`,
`x5_strip_px_units_preserves_thickness_shape`,
`group_a_cell_style_is_valid_winui`. Updated
`x4_non_color_setters_pass_through_unchanged` and
`box_partitions_style_between_border_and_textblock_resource` which
asserted the old (now-invalid) `FontWeight="normal"` / `"500"`.

