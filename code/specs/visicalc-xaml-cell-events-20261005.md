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

## Native evidence, 2026-10-05

Generated WinUI build: zero errors, ten existing WMC1506 immutable-row warnings.
The standard Rust MosaicApp DLL was bundled, with a copied isolated state fixture.

Actual Windows interaction: clicking B2 changes address/selection to B2=14 and
status to onGridNavigate. Entering =6*7 in the formula field and pressing Return
changes B2 to 42, E2 to 79, B5 to 65 and E5 to 225, while A1 stays 43.
Clicking empty F6 moves selection and highlight to F6, proving the hit area does
not depend on visible text. No generated host code was edited.

Keyboard navigation, cell automation semantics, formula-field sizing and numeric
text alignment remain separate acceptance work; this does not close #14276.
