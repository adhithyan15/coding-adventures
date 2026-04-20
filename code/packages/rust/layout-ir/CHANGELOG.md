# Changelog

## [0.1.0] — initial release

### Added
- All UI02 core types in Rust: `SizeValue`, `Edges`, `Color`, `FontSpec`, `TextAlign`, `ImageFit`, `TextContent`, `ImageContent`, `Content`, `LayoutNode`, `Constraints`, `PositionedNode`, `MeasureResult`.
- `TextMeasurer` trait — the contract layout algorithms use for text sizing.
- `ExtValue` / `Ext` — a zero-dependency typed extension-bag value set covering Str, Int, Float, Bool, List, Map. Layout algorithms namespace their keys under their algorithm name.
- Builder functions: `size_fixed`/`size_fill`/`size_wrap`, `edges_all`/`edges_xy`/`edges_zero`, `rgba`/`rgb`/`color_transparent`/`color_black`/`color_white`, `font_spec`/`font_bold`/`font_italic`, `constraints_fixed`/`constraints_width`/`constraints_unconstrained`/`constraints_shrink`, `LayoutNode::{empty, leaf_text, leaf_image, container}` + chainable `with_id`/`with_padding`/`with_margin`/`with_width`/`with_height`/`with_ext`.
- 9 unit tests covering all constructors, builder behavior, and the `TextMeasurer` trait with a synthetic fixed-width measurer (validates single-line and multi-line wrap paths).

### Design
- Zero external dependencies.
- `ExtValue` chosen over `Box<dyn Any>` to avoid pulling in the `any` trait object machinery while preserving enough type discrimination for extension packages to serialize structured data.
- All types derive `Clone + Debug + PartialEq` (with `Eq + Hash` on `Color`); `LayoutNode` and `Edges` also derive `Default` for ergonomic building.
