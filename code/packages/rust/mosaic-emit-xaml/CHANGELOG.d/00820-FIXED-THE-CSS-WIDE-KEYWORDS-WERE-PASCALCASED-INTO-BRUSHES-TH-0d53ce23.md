### Fixed -- the CSS-wide keywords were PascalCased into brushes that do not exist (#15141)

`normalize_xaml_color_value` already dropped `currentColor` as a cascade
keyword rather than a colour. `inherit`, `initial`, `unset` and `revert`
are the same kind of thing and were not in that guard, so they fell to the
all-lowercase branch and came out as `Foreground="Inherit"`.

The fallback justified itself with "the markup compiler will reject
anything that isn't a real named color, and over-pascalCasing just shifts
which compiler complains". **That is not true**, and believing it is how
this shipped: the markup compiler does not validate brush literals, so an
unconvertible one builds clean and throws `E_XAMLPARSEFAILED` at runtime,
when the page is shown. The comment has been corrected to say so.

A genuinely misspelt colour name still slips through the same branch. That
is a narrower gap needing a real named-colour set, and is left open rather
than half-closed.

**Not verified on a running app.** WinUI 3 does not build on macOS, so this
is verified by unit test and by inspecting emitted XAML only. Emitted output
for task-app, visicalc and engram-app is byte-identical before and after --
none of them authored a CSS-wide keyword in a colour position, so this is a
latent trap closed, not a live bug fixed.

