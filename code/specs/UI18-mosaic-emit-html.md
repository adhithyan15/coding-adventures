# UI18 — mosaic-emit-html: Pure HTML Static Snapshot Backend

**Status:** Implemented  
**Layer:** UI  
**Depends on:** UI01 (mosaic-vm), UI00 (mosaic-analyzer)

---

## 1. Purpose

`mosaic-emit-html` is the **static HTML snapshot** backend for the Mosaic compiler.
It takes a `.mosaic` component plus an optional JSON fixture file that provides
concrete slot values, and emits a single complete `<!DOCTYPE html>` file with
no JavaScript, no runtime, and fully inlined styles.

Primary use cases:

- **Design reviews** — share a rendered snapshot without a dev server.
- **Screenshot tests** — feed to headless Chrome / Playwright for visual regression.
- **Static documentation** — embed rendered component previews in generated docs.
- **e2e test fixtures** — serve as stable reference HTML for integration tests.

```text
.mosaic source + optional fixture.json + optional styles.css
        │
        ▼
mosaic-analyzer  →  MosaicFile (IR)
        │
        ▼
MosaicVM  (drives MosaicRenderer callbacks)
        │
        ▼
HtmlRenderer  →  MyComponent.html
```

---

## 2. Output Structure

```html
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <title>ProfileCard</title>
  <style>
    /* CSS inlined from --css file, if provided */
    body { margin: 0; font-family: sans-serif; }
  </style>
</head>
<body>
  <div style="display:flex;flex-direction:column">
    <span>Jane Doe</span>
  </div>
</body>
</html>
```

---

## 3. Fixture File Format

The fixture file is a flat JSON object mapping slot names (kebab-case) to values:

```json
{
  "display-name": "Jane Doe",
  "avatar-url": "https://example.com/avatar.png",
  "count": 42,
  "visible": true,
  "items": ["Alpha", "Beta", "Gamma"],
  "column-headers": ["Name", "Score", "Rank"]
}
```

| JSON type | Used for slot type   |
|-----------|----------------------|
| `string`  | `text`, `image`, `color` slots |
| `number`  | `number` slots       |
| `boolean` | `bool` slots         |
| `array`   | `list<T>` slots      |

Slot values that are absent from the fixture are rendered as a visible placeholder:
`[slot: slot-name]`.

---

## 4. `HtmlRenderer` Struct

```rust
pub struct HtmlRenderer {
    component_name: String,
    slots: Vec<MosaicSlot>,
    /// Fixture values for slots provided at compile time.
    fixtures: serde_json::Map<String, serde_json::Value>,
    /// Stack of open element frames during depth-first traversal.
    stack: Vec<HtmlFrame>,
    /// HTML lines accumulated at the root level.
    root_lines: Vec<String>,
    /// Suppression depth: > 0 means we are inside a false `when` block.
    /// All content production is skipped while suppress > 0.
    suppress: usize,
    /// Optional CSS to inline in `<style>`.
    css: Option<String>,
}

struct HtmlFrame {
    close_tag: String,
    lines: Vec<String>,
}
```

The `suppress` counter enables nested `when` blocks inside a false outer `when`
to be cleanly skipped without complex stateful tracking.

---

## 5. Primitive → HTML Mapping (Static)

Same HTML elements as the web component backend, but with **inline `style=`
attributes** instead of JavaScript class methods:

| Mosaic element | HTML                                                     |
|----------------|----------------------------------------------------------|
| `Box`          | `<div style="…">`                                        |
| `Column`       | `<div style="display:flex;flex-direction:column;…">`     |
| `Row`          | `<div style="display:flex;flex-direction:row;…">`        |
| `Text`         | `<span style="…">slot-value-or-literal</span>`           |
| `Image`        | `<img src="fixture-value-or-placeholder" alt="…">`       |
| `Spacer`       | `<div style="flex:1;…">`                                 |
| `Scroll`       | `<div style="overflow:auto;…">`                          |
| `Divider`      | `<hr>`                                                   |
| `Stack`        | `<div style="position:relative;…">`                      |
| `Icon`         | `<span class="icon …">`                                  |
| `Grid`         | `<table>` with headers as `<th>` and rows as `<td>` (see §7) |

Slot references in properties are resolved by looking up the fixture. If absent:
dimension refs render as their raw string (e.g. `"16dp"` → `"16px"`), slot refs
render as `[slot: name]`.

---

## 6. `when` Blocks — Compile-time Suppression

```
when @visible {
  Text { content: "Hello"; }
}
```

1. Look up `fixtures["visible"]` as a boolean.
2. **`true` or missing** → render children normally (emit the body).  
   (Missing means "unknown at snapshot time" → show to aid design reviews.)
3. **`false`** → increment `suppress`. All `begin_node`, `render_slot_child`,
   `begin_when`, `begin_each` calls while `suppress > 0` produce no output.
4. `end_when` decrements `suppress` if it was incremented; otherwise no-op.

Nested `when` inside a false outer `when` also increments `suppress`, so the
counter correctly handles arbitrary nesting depth.

---

## 7. `each` Blocks — Fixture-driven Repetition

```
each @items as item {
  Text { content: @item; }
}
```

The VM calls `begin_each` once, then traverses the body children once, then
calls `end_each`. The HTML backend handles `each` using a fixture array:

1. Look up `fixtures["items"]` as a JSON array.
2. If the array is **absent or empty**, emit the body once with the loop variable
   resolving to `[item]` (a visible placeholder).
3. If the array has elements, repeat the body once for each element, substituting
   the element's value wherever `@item` (the loop variable) appears in props.

Implementation note: because the VM traverses the body only once, v1 uses the
first fixture element for the first repetition. Full multi-repetition requires
the VM to support re-traversal or the backend to buffer the body instructions —
this is a v2 concern. For v1, a single iteration with the first fixture value
is emitted when a non-empty array fixture is provided.

---

## 8. Slot Reference Resolution in Properties

When a `ResolvedValue::SlotRef { name, .. }` appears in a node's property (e.g.
`content: @title`), the renderer:

1. Looks up `fixtures[name]`.
2. If found: uses the JSON value as a string.
3. If absent: emits `[slot: name]` as a visible placeholder.

All slot values embedded in HTML attributes are run through `html_escape()`:

```rust
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
     .replace('<', "&lt;")
     .replace('>', "&gt;")
     .replace('"', "&quot;")
}
```

---

## 9. Grid — Static Table Rendering

When `Grid` is encountered with `headers` and `rows` props pointing to list
fixture values:

```html
<table>
  <thead>
    <tr><th>Name</th><th>Score</th><th>Rank</th></tr>
  </thead>
  <tbody>
    <tr><td>Alice</td></tr>
    <tr><td>Bob</td></tr>
  </tbody>
</table>
```

If the fixture is absent, a single-column placeholder table is emitted:

```html
<table>
  <thead><tr><th>[headers]</th></tr></thead>
  <tbody><tr><td>[rows]</td></tr></tbody>
</table>
```

---

## 10. CSS Inlining

When the `--css` flag points to a CSS file, the content is read and placed
verbatim inside a `<style>` block in `<head>`:

```html
<head>
  …
  <style>
    /* content of the provided CSS file */
  </style>
</head>
```

This is intended to be used with CSS output from `mosstyle-compiler`. If no
`--css` flag is given, a minimal reset is emitted:

```css
*, *::before, *::after { box-sizing: border-box; }
body { margin: 0; font-family: sans-serif; }
```

---

## 11. `emit()` — Final HTML Document

```rust
fn emit(self) -> EmitResult {
    let html_body = self.root_lines.join("\n");
    let style_block = self.css.unwrap_or_else(|| minimal_reset().to_string());

    let output = format!(r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <title>{name}</title>
  <style>
{css}
  </style>
</head>
<body>
{body}
</body>
</html>
"#, name = self.component_name, css = style_block, body = html_body);

    EmitResult { output, component_name: self.component_name }
}
```

---

## 12. `HtmlRenderer::new(fixtures, css)`

```rust
impl HtmlRenderer {
    pub fn new(
        fixtures: serde_json::Map<String, serde_json::Value>,
        css: Option<String>,
    ) -> Self { … }
}
```

---

## 13. Testing Strategy

| Test | What it covers |
|------|----------------|
| `test_html_document_structure` | Output starts with `<!DOCTYPE html>` and has `<html>`, `<head>`, `<body>` |
| `test_component_title` | `<title>` tag contains component name |
| `test_box_renders_div` | Box → `<div>` |
| `test_column_flex_style` | Column → `display:flex;flex-direction:column` |
| `test_row_flex_style` | Row → `display:flex;flex-direction:row` |
| `test_text_content_from_fixture` | Text `content: @title` resolved from fixture |
| `test_text_content_placeholder` | Text `content: @title` without fixture → `[slot: title]` |
| `test_when_true_renders` | `when @show` with fixture `"show": true` → body present |
| `test_when_false_suppresses` | `when @show` with fixture `"show": false` → body absent |
| `test_when_missing_renders` | `when @show` with no fixture → body present (design-review default) |
| `test_each_with_fixture` | `each @items` with array fixture → first item rendered |
| `test_each_without_fixture` | `each @items` with no fixture → placeholder rendered |
| `test_css_inlined` | CSS string appears inside `<style>` tag |
| `test_slot_value_html_escaped` | `<script>` in fixture value → `&lt;script&gt;` |
| `test_divider_hr` | Divider → `<hr>` |

---

## 14. v2 Roadmap

- `--serve` mode: watch `.mosaic` and fixture file for changes, serve via a
  local HTTP server, reload the browser on save.
- Full `each` repetition for multi-element fixture arrays (requires VM body
  re-traversal or backend-side instruction buffering).
- `--watch` without `--serve` (just rewrite the file on change).
- Source maps from HTML elements back to `.mosaic` source lines.
