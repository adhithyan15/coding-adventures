### Fixed - Valid styled text composition

Text primitives now split MSL box paint from typography. Backgrounds, borders,
corner radii, padding, and sizing render on a native `Border`, while foreground
and font properties remain on the nested `TextBlock`. This preserves pill/chip
styling without sending unsupported `CornerRadius` or `Background` attributes
to WinUI's `TextBlock` markup compiler.

MSL `text-align` on `HostButton` now maps to WinUI's native
`HorizontalContentAlignment` property instead of the unsupported
`Button.TextAlignment` attribute.

