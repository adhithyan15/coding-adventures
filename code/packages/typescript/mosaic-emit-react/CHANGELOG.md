# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-04

### Added

- `MosaicReactRenderer` — implements `MosaicRenderer` from `mosaic-vm`
- Emits typed React functional components (`.tsx`) from `MosaicIR`
- Full JSX output: `<div>`, `<span>`, `<img>`, `<button>` mapped from Mosaic
  node types `Box`, `Text`, `Image`, `Button`
- Inline `style` objects with camelCase CSS property names
- Props interface generation: slot types mapped to TypeScript types
  (`text→string`, `number→number`, `bool→boolean`, `node→React.ReactNode`,
  `image→string`, `list<T>→T[]`, named component slots → `React.ReactElement<Props>`)
- Slot reference interpolation: `@name` → `{name}` JSX expression
- `when @flag { ... }` → `{flag && (...)}` conditional rendering
- `each @items as item { ... }` → `{items.map((item, _index) => (...))}` iteration
  with `React.Fragment` wrapper keyed by `_index`
- Accessibility: `a11y-role: button` → `role="button"`, `a11y-role: heading` →
  `<h2>`, `a11y-role: image` → `role="img"`, `a11y-hidden: true` → `aria-hidden="true"`
- Color format: `#rrggbbaa` hex → `rgba(r, g, b, alpha)` with 3-decimal alpha
- Dimension format: `dp`/`sp` → `px`, `%` → `%`, fill → `"100%"`, wrap → `"auto"`
- Text alignment: `text-align: center` → `className="text-center"`
- CSS class passthrough: `class: foo` → `className="foo"`
- Image `src` slot reference support
- Non-primitive component imports: components that reference other Mosaic
  components generate `import { Foo } from "./Foo.js"` and pass `foo` slot as prop
- 117 tests, 100% coverage
