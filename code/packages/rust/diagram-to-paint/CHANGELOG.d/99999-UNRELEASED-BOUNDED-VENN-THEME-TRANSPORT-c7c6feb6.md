## Unreleased — bound Venn theme origins and exact scene transport

Venn accepts root `themeVariables` front matter or a direct `themeVariables`
object in a closed single-line init directive after the leading YAML header.
The selected init object replaces front matter wholesale, including an empty
object. Only direct mapping fields supply values; nested maps/arrays, scalar
sections, sequence items and header literals cannot supply a theme. Specific
`vennTitleTextColor`/`vennSetTextColor` precede generic `titleColor`/`textColor`;
a present invalid specific value uses the default rather than the generic.
Active matching quotes preserve embedded other quotes and font-family lists;
only escaped active quotes/backslashes decode. Malformed present values default.

Defaults are `sans-serif`, `#0f172a`, `#0f172a`. Configured family replaces caller
option families, retaining size, weight, italic and stretch. Authored label
`color` overrides the configured set color. The checked geometric layout carries
all eight config fields; scenes expose all eight `venn.*` metadata entries.
Existing canvas/scalar/derived bounds and unrelated diagram readers remain intact.
Portable tests compare complete parsed config, geometry, paint primitives,
glyph positions, drawing order and actual ordered resolver queries in16 themed
source-to-scene cells, retaining the previous16 full-scene cells. This establishes
bounded transport, not physical-font fitting, rasterization or full CSS parity.
Actual affected14-package strict lint and925 unfiltered tests/40targets pass
at source05e61228. Miri **DEFERRED/PENDING**, never passed; hosted checks, current
main reconciliation, full Closure parity/lineage and delivery remain OPEN.
