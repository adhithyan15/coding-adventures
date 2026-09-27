### X1 — `border-radius` lowered to invalid `BorderRadius`

`css_property_to_xaml_setter` had no entry for `border-radius`, so
the kebab-to-pascal fallback produced `BorderRadius` — which isn't
a real WinUI 3 property. The XAML markup compiler rejected it
silently (`XamlCompiler.exe` exits 1 with no diagnostic). Fixed
by adding the explicit `"border-radius" => "CornerRadius"` mapping
(`UIElement.CornerRadius` is the actual WinUI property).

Regression test: `border_radius_lowers_to_corner_radius`.

