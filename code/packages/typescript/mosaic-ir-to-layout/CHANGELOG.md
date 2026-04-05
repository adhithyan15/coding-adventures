# Changelog

## 0.1.0 — Initial release

### Added
- `mosaic_ir_to_layout(component, slots, theme)` — converts a `MosaicComponent` IR
  tree to a `LayoutNode` tree ready for flexbox layout.
- `SlotValue` union type: `string | number | boolean | Color | LayoutNode | SlotValue[]`
- `SlotMap` — `Map<string, SlotValue>` for passing runtime slot values.
- `MosaicLayoutTheme` — configures default font, text colour, and base font size.
- `mosaic_default_theme()` — sensible defaults (16 px base, black text, system-ui).
- Primitive node support: `Column`, `Row`, `Box`, `Text`, `Image`, `Spacer`,
  `Divider`, `Scroll`.
- Non-primitive nodes (custom component references) produce a placeholder
  `LayoutNode` tagged with `_componentRef` in `ext`.
- Unknown primitive tags produce an empty container tagged with `_unknownTag`.
- Full `PaintExt` mapping: `background`, `border-color`, `border-width`,
  `corner-radius`, `opacity`, `shadow` (elevation table: none/low/medium/high).
- Font resolution via `style` shorthand and individual `font-size`/`font-weight`/
  `font-style` properties.
- Slot value resolution: scalar literals, ident references, hex colours,
  `dimension` values, slot refs (with zero defaults per slot type).
- `when` children — conditional subtrees; children omitted when condition is falsy.
- `each` children — iterative subtrees; expanded once per element in a list slot.
- Loop context propagates the current element so nested `slot_ref` values resolve
  to the loop variable.
- 64 unit tests, 100 % statement coverage, 82 % branch coverage.
