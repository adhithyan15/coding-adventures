# UI05 — mosaic-ir-to-layout: Mosaic IR → LayoutNode Tree

## Overview

`mosaic-ir-to-layout` is a compile-time converter. It takes a `MosaicComponent`
IR (the output of `mosaic-analyzer`) and a map of resolved slot values (the
component's props at render time) and produces a `LayoutNode` tree with
`FlexExt` and `PaintExt` fields populated, ready for `layout-flexbox`.

```
MosaicComponent IR + resolved slot values
    ↓  mosaic_ir_to_layout()
LayoutNode tree (with ext["flex"] and ext["paint"] populated)
    ↓  layout_flexbox()
PositionedNode tree
    ↓  layout_to_paint()
PaintScene
    ↓  paint-vm backend
pixels
```

This is the package that closes the Mosaic → Canvas loop. It bridges the
semantic Mosaic component model to the generic layout system.

---

## Package: `mosaic-ir-to-layout`

**Depends on:** `layout-ir`, `mosaic-analyzer` (for MosaicComponent IR types)

**Exports:** `mosaic_ir_to_layout`, `MosaicLayoutTheme`

---

## Function signature

```
mosaic_ir_to_layout(
  component: MosaicComponent,
  slots:     map<string, SlotValue>,
  theme:     MosaicLayoutTheme
) → LayoutNode
```

Returns a single root `LayoutNode` representing the component tree with all
slot references resolved to concrete values.

### `MosaicLayoutTheme`

Default visual style applied when a Mosaic property is not explicitly set:

```
MosaicLayoutTheme {
  defaultFont:       FontSpec    // font for Text nodes with no explicit style
  defaultTextColor:  Color       // text color for Text nodes with no explicit color
  baseFontSize:      float       // 1dp unit in logical units; default 1.0
}
```

A default theme is exported: `mosaic_default_theme()`.

---

## Slot value resolution

Before converting any node, all `@slotName` references in property values are
resolved to concrete values using the provided `slots` map:

- `@textSlot` → `string` value
- `@numberSlot` → `float` value
- `@boolSlot` → `bool` value
- `@imageSlot` → `string` (URL or data URI)
- `@colorSlot` → `Color`
- `@nodeSlot` → treated as an empty container (`LayoutNode` with no content);
  the actual child is injected by the caller at a higher level (outside this
  converter's scope)

If a slot reference is not found in `slots`, the slot's `default_value` from
the IR is used. If there is no default, a type-appropriate zero value is used
(empty string, 0, false, transparent color).

---

## Node mapping

Each Mosaic primitive maps to a `LayoutNode` with specific `ext["flex"]` and
`ext["paint"]` values:

### `Column`

```
LayoutNode {
  width:    size from "width" property, or size_fill()
  height:   size from "height" property, or size_wrap()
  padding:  from padding/padding-* properties
  ext["flex"] = {
    direction: "column",
    gap:       from "gap" property, default 0,
    alignItems:     from "align" property → align_to_items(v),
    justifyContent: from "justify" property, default "start"
  }
  ext["paint"] = paint_ext_from_props(props)
  children: [converted children]
}
```

### `Row`

Same as Column but `direction: "row"`. The `align` property maps to
`alignItems` (cross-axis = vertical). `justify` maps to `justifyContent`
(main-axis = horizontal).

### `Box`

A flex container with no default direction — children are absolutely
positioned within it:

```
LayoutNode {
  ext["flex"] = {
    direction: "column",
    wrap: "wrap"
  }
  ext["paint"] = paint_ext_from_props(props)
  children: [converted children]
}
```

### `Text`

A leaf node with `TextContent`:

```
LayoutNode {
  width:    size_wrap()
  height:   size_wrap()
  content: TextContent {
    kind:      "text",
    value:     resolved value of "content" property, or ""
    font:      font_from_props(props, theme),
    color:     color_from_props(props, theme.defaultTextColor),
    maxLines:  from "max-lines" property, or null,
    textAlign: from "text-align" property, or "start"
  }
}
```

`ext["flex"]["grow"]` = 0 by default; `ext["flex"]["shrink"]` = 1.

### `Image`

A leaf node with `ImageContent`:

```
LayoutNode {
  width:   size from "width" or "size" property, or size_wrap()
  height:  size from "height" or "size" property, or size_wrap()
  content: ImageContent {
    kind: "image",
    src:  resolved value of "source" property,
    fit:  from "fit" property, or "contain"
  }
  ext["paint"] = {
    cornerRadius: from "shape" property
                  → "circle" = node_width / 2 (computed at layout time)
                  → "rounded" = 8
  }
}
```

### `Spacer`

A flex-grow filler:

```
LayoutNode {
  width:  size_fill()
  height: size_fill()
  ext["flex"] = { grow: 1, shrink: 0 }
}
```

### `Scroll`

A container with overflow. `layout-flexbox` treats it as a standard flex
container. Scroll behavior is a paint-vm / renderer concern:

```
LayoutNode {
  ext["flex"] = { direction: "column", wrap: "nowrap" }
  ext["paint"] = { overflow: "scroll" }   // hint to renderer
  children: [converted children]
}
```

### `Divider`

A thin horizontal rule:

```
LayoutNode {
  width:   size_fill()
  height:  size_fixed(1)    // 1 logical unit
  ext["paint"] = {
    backgroundColor: theme.defaultTextColor (at 0.2 opacity)
  }
}
```

### Non-primitive component nodes

When a Mosaic component references another component by name (non-primitive
nodes), `mosaic-ir-to-layout` produces a placeholder container:

```
LayoutNode {
  id:       component name
  ext["flex"] = {}
  children: []    // empty; caller is responsible for injecting the subtree
}
```

The placeholder preserves the component identity so the caller can substitute
the referenced component's own `LayoutNode` tree.

---

## Property converters

### `align_to_items(v: string) → string`

Maps Mosaic `align` values to `alignItems` flex values:

| Mosaic `align` | `alignItems` |
|---|---|
| `"start"` | `"start"` |
| `"center"` | `"center"` |
| `"end"` | `"end"` |
| `"stretch"` | `"stretch"` |
| `"center-horizontal"` (in Column) | `"center"` |
| `"center-vertical"` (in Column) | ignored here; maps to `justifyContent: "center"` |

### `font_from_props(props, theme) → FontSpec`

1. Start with `theme.defaultFont`
2. If `font-weight` property is set: apply to `weight`
3. If `style: enum(heading, large)` etc.: map to predefined `FontSpec` sizes
   from theme. See `MosaicLayoutTheme.typescale` (optional extension).

### `color_from_props(props, default) → Color`

1. If `color` property is set: use it
2. Otherwise: return `default`

### `paint_ext_from_props(props) → PaintExt`

Reads visual decoration properties from the Mosaic node's property list:

| Mosaic property | `ext["paint"]` field |
|---|---|
| `background: color` | `backgroundColor` |
| `border-width: dim` | `borderWidth` |
| `border-color: color` | `borderColor` |
| `corner-radius: dim` | `cornerRadius` |
| `opacity: number` | `opacity` |
| `shadow: elevation.low/medium/high` | `shadowColor`, `shadowOffsetX/Y`, `shadowBlur` |

Shadow elevation values:

| `elevation` | `shadowColor` (rgba) | `offsetX` | `offsetY` | `blur` |
|---|---|---|---|---|
| `none` | transparent | 0 | 0 | 0 |
| `low` | rgba(0,0,0,31) | 0 | 1 | 3 |
| `medium` | rgba(0,0,0,38) | 0 | 4 | 12 |
| `high` | rgba(0,0,0,51) | 0 | 8 | 24 |

---

## When/each blocks

Mosaic `when` and `each` blocks are handled at this layer:

### `when` block

```
when @show {
  Text { content: "Hello"; }
}
```

If the resolved value of `show` is `true`: convert the inner nodes and include
them in the parent's children list normally.

If `false`: produce no children. The `LayoutNode` tree simply omits them.

This is correct because `mosaic-ir-to-layout` is a **runtime** converter (it
receives resolved slot values). Unlike the compile-time React/WebComponent
emitters which emit conditional expressions, this converter resolves conditions
immediately.

### `each` block

```
each item in @items {
  Text { content: @item; }
}
```

Iterate over the resolved `items` list. For each element, convert the inner
node with `@item` bound to the element value. Append all resulting `LayoutNode`
values to the parent's children list.

---

## What this package does NOT do

- Does not run any layout algorithm
- Does not produce `PositionedNode` — that is `layout-flexbox`'s job
- Does not produce paint instructions — that is `layout-to-paint`'s job
- Does not handle non-layout backends (React, Web Components) — those are
  `mosaic-emit-react` and `mosaic-emit-webcomponent`
- Does not validate that slot values are of the correct type (trust the
  analyzer's IR)
