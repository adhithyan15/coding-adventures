# Native XAML container click payloads

Related: #14274, #14276, epic #14267.

Grid Cell exposes onClick with row/col props on its Box. XAML currently emits
a Border without a tap handler, so native pointer selection never reaches the
Rust adapter. Add reusable container onClick/onTap lowering to Tapped. Bind
the sender's typed row view model and resolve explicit numeric payload props
through the existing expression lowerer, preserving captured outer indices.
Never infer coordinates from displayed text or splice raw expressions into C#.
Unsupported payloads must report an error rather than dispatch invented values.

Transparent containers must be hit-testable across the full cell area. Preserve
existing visual-state names and styling. Do not change the shared app adapter
or hand-edit generated hosts. Add nested-row regression coverage, build the
generated WinUI app and click non-origin cells, then edit a formula there.
Retain full keyboard/a11y/persistence acceptance as separate work.
