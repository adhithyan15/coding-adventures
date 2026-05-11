# mosaic-emit-html

Pure HTML static snapshot backend for the Mosaic compiler.

## What it does

`mosaic-emit-html` takes a Mosaic component IR (produced by `mosaic-analyzer`)
and emits a complete, standalone `<!DOCTYPE html>` file with no JavaScript,
no runtime dependency, and fully inlined CSS. Slot values are resolved from an
optional JSON fixture file provided at compile time.

## How it fits in the stack

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

## Usage

```rust
use mosaic_emit_html::HtmlRenderer;
use mosaic_vm::MosaicVM;
use mosaic_analyzer::analyze;
use serde_json::json;

// Fixture values for slots — resolved at compile time.
let fixtures = json!({
    "display-name": "Jane Doe",
    "avatar-url": "https://example.com/avatar.png",
    "visible": true,
    "items": ["Alpha", "Beta", "Gamma"]
}).as_object().cloned().unwrap();

// Optional CSS to inline in the <style> block.
let css = std::fs::read_to_string("styles.css").ok();

let renderer = HtmlRenderer::new(fixtures, css);

let file = analyze(std::fs::read_to_string("ProfileCard.mosaic").unwrap().as_str()).unwrap();
let vm = MosaicVM::new(file);
let result = vm.run(renderer).unwrap();

std::fs::write("ProfileCard.html", &result.output).unwrap();
println!("Written to ProfileCard.html");
```

## Fixture file format

```json
{
  "display-name": "Jane Doe",
  "count": 42,
  "visible": true,
  "items": ["Alpha", "Beta", "Gamma"]
}
```

| JSON type | Slot type                        |
|-----------|----------------------------------|
| `string`  | `text`, `image`, `color` slots   |
| `number`  | `number` slots                   |
| `boolean` | `bool` slots                     |
| `array`   | `list<T>` slots                  |

Slots absent from the fixture render as `[slot: name]` placeholders.

## Example output

```html
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <title>Profile Card</title>
  <style>
    *, *::before, *::after { box-sizing: border-box; }
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

## Primitive → HTML mapping

| Mosaic    | HTML element              |
|-----------|---------------------------|
| Box       | `<div>`                   |
| Column    | `<div style="display:flex;flex-direction:column">` |
| Row       | `<div style="display:flex;flex-direction:row">` |
| Text      | `<span>text content</span>` |
| Image     | `<img src="…" alt="…">`  |
| Spacer    | `<div style="flex:1">`    |
| Scroll    | `<div style="overflow:auto">` |
| Divider   | `<hr>`                    |
| Stack     | `<div style="position:relative">` |
| Icon      | `<span class="icon">`     |
| Grid      | `<table>` with fixture-driven `<thead>`/`<tbody>` |

## Use cases

- **Design reviews** — share rendered snapshots without a dev server.
- **Screenshot tests** — feed to headless Chrome / Playwright.
- **Static documentation** — embed in generated docs.
- **e2e test fixtures** — stable reference HTML for integration tests.
