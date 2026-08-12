# mosaic-compile

CLI that compiles `.mosaic` component files to a target output format.

## What it does

`mosaic-compile` is the System A CLI for the Mosaic compiler. It reads a unified
`.mosaic` source file and compiles it to one of four backends:

| Backend | Output | Use case |
|---------|--------|----------|
| `webcomponent` | Custom Element JS | Production browser components |
| `html` | Static HTML snapshot | Server-side rendering, e-mail |
| `react` | React JSX functional component | React applications |
| `paint` | Raster PNG image | Preview thumbnails, headless rendering |

## How it fits in the stack

```text
.mosaic source
     │
     ▼
mosaic-analyzer  →  MosaicFile (IR)
     │
     ▼
MosaicVM  (drives MosaicRenderer callbacks)
     │
     ├── --backend webcomponent  →  MyComponent.js   (Custom Element)
     ├── --backend html          →  MyComponent.html (static snapshot)
     ├── --backend react         →  MyComponent.jsx  (React functional component)
     └── --backend paint         →  MyComponent.png  (raster PNG via Paint VM)
```

> **Note:** The `paint` backend bypasses `MosaicVM` and calls `mosaic-emit-paint`
> directly, which walks the `MosaicFile` IR and drives a naive box-model layout
> engine to produce a `PaintScene`, then renders it to PNG via the platform's
> graphics backend (Cairo on Linux, Skia on macOS/Windows, Metal on Apple Silicon).

## Installation

```bash
cargo install --path code/packages/rust/mosaic-compile
```

Or build directly:

```bash
cargo build -p mosaic-compile
./target/debug/mosaic-compile --help
```

## Usage

The CLI has two modes — pick one:

```text
Legacy single-file mode (.mosaic):
    mosaic-compile --backend <BACKEND> [OPTIONS] <SOURCE>

Three-file pipeline mode (.mil + .mll + .msl, UI23/UI24):
    mosaic-compile --backend react --interface <I.mil> --layout <L.mll> --style <S.msl> [-o <OUT>]

ARGUMENTS:
    SOURCE                   Path to the .mosaic source file (legacy mode only)

FLAGS:
    -b, --backend <name>     webcomponent | html | react | paint  [required]
    -o, --output <path>      Output file path
                             Default: <ComponentName>.js / .html / .jsx / .png / .tsx
    -f, --fixtures <path>    JSON fixture file for slot values (html only)
    -c, --css <path>         CSS file to inline (html only)
        --interface <path>   .mil mosmodel interface file (pipeline mode)
        --layout <path>      .mll moslayout file (pipeline mode)
        --style <path>       .msl mosstyle file (pipeline mode)
    -h, --help               Show help
    -V, --version            Print version
```

Pipeline mode supports the text/native emitter family (`react`, `html`,
`webcomponent`, `swiftui`, `qt`, `xaml`, `compose`, and `flutter`). The `paint` backend
remains legacy single-file only.

When a layout contains `pkg::package::Component`, pipeline mode resolves that
component and merges its MSL defaults before the consumer's own styles. This is
the same composition path used by package mode, so reusable controls keep their
authored appearance in every backend.

Package mode compiles a Mosaic package directory that contains
`mosaic-package.toml` plus `src/*.mil`, `src/*.mll`, and optional `src/*.msl`
files:

```text
mosaic-compile pkg <PACKAGE_ROOT> --backend <BACKEND> --output <DIR> [--emit-project] [--profile permissive|native-complete] [--runtime-library <CDYLIB>] [--token-palette <JSON>]

BACKEND: react | swiftui | qt | xaml | compose | webcomponent | html | flutter
```

`--emit-project` asks the package builder to write the selected backend's
runnable shell next to the component artifacts, such as a WinUI/XAML project or
a Qt/CMake project.

For Compose, Flutter, Qt, SwiftUI, and XAML distributions, `--runtime-library`
selects an already-built target Rust application library. Compose, Flutter, and
Qt accept `.dylib`, `.so`, or `.dll`; SwiftUI requires `.dylib`; XAML requires
`.dll`. Mosaic copies it into the generated project's native application
resources and the standard binding resolves it relative to the installed app.
Flutter uses Dart's stable build-hook/code-asset packaging contract and therefore
requires Flutter 3.38+ and Dart 3.10+ when a runtime is bundled. The option
requires `--emit-project`; strict project builds on all five native backends
require it.

`--token-palette` applies one versioned palette to the app package and every
referenced Mosaic package. Global values can be refined per generated backend:

```json
{
  "schema_version": 1,
  "tokens": {
    "color-accent": "#5b5bd6",
    "brand-action": "$color-accent",
    "radius-md": "10px"
  },
  "backends": {
    "swiftui": { "radius-md": "12px" },
    "xaml": { "radius-md": "8px" }
  }
}
```

Token names use lowercase kebab-case. Values are single safe declarations;
single-token aliases are supported. Unknown schema versions, misspelled backend
names, unsafe values, missing references, and cycles fail the build.

Package mode defaults to `--profile permissive`, which emits the package plus a
machine-readable `<backend>/mosaic-degradations.json`. Use
`--profile native-complete` in CI to reject known interactive, accessibility,
effect-host, or placeholder behavior before any application artifacts are
emitted. The first inventory is intentionally conservative and will grow as
backend property/event coverage is audited. Compose, Flutter, and SwiftUI project shells
that pass the strict profile require the standard Rust application library at
startup and do not include optional-host or generated sample-prop fallbacks.

## Examples

### Compile to a Web Component

```bash
mosaic-compile --backend webcomponent ProfileCard.mosaic
# Writes: ProfileCard.js
```

### Compile to a React component

```bash
mosaic-compile --backend react ProfileCard.mosaic
# Writes: ProfileCard.jsx
```

### Compile to a PNG thumbnail

```bash
mosaic-compile --backend paint ProfileCard.mosaic
# Writes: ProfileCard.png  (800×600 by default)
```

### Compile to static HTML with fixtures

```bash
# Create a fixture file
cat > data.json << 'EOF'
{
  "display-name": "Jane Doe",
  "avatar-url": "https://example.com/avatar.png",
  "visible": true,
  "items": ["Alpha", "Beta", "Gamma"]
}
EOF

mosaic-compile --backend html --fixtures data.json ProfileCard.mosaic
# Writes: ProfileCard.html
```

### Compile to HTML with inlined CSS

```bash
mosaic-compile --backend html --css styles.css -o preview.html ProfileCard.mosaic
# Writes: preview.html with inlined CSS
```

## Output examples

### Web Component

```javascript
// Auto-generated by mosaic-emit-webcomponent. Do not edit.

class ProfileCard extends HTMLElement {
  static get observedAttributes() { return ['display-name', 'avatar-url']; }

  constructor() {
    super();
    this.attachShadow({ mode: 'open' });
    this._displayName = '';
    this._avatarUrl = '';
  }

  connectedCallback() { this._render(); }
  attributeChangedCallback(name, _oldVal, newVal) {
    this[`_${this._toCamel(name)}`] = newVal ?? '';
    this._render();
  }

  _render() {
    this.shadowRoot.innerHTML = `<div style="display:flex;flex-direction:column">
      <span>${this._displayName}</span>
    </div>`;
  }

  _escape(s) { /* … */ }
  _toCamel(s) { /* … */ }
}
customElements.define('profile-card', ProfileCard);
```

### Static HTML

```html
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <title>Profile Card</title>
  <style>/* inlined CSS or minimal reset */</style>
</head>
<body>
  <div style="display:flex;flex-direction:column">
    <span>Jane Doe</span>
  </div>
</body>
</html>
```

### React JSX

```jsx
// Auto-generated by mosaic-emit-react. Do not edit.
import React from 'react';

interface ProfileCardProps {
  displayName?: string;
  avatarUrl?: string;
}

export default function ProfileCard({ displayName = '', avatarUrl = '' }: ProfileCardProps) {
  return (
    <div style={{ display: 'flex', flexDirection: 'column' }}>
      <span>{displayName}</span>
    </div>
  );
}
```

### PNG thumbnail

The `paint` backend produces a binary PNG file. The default canvas is 800×600 px.
Use `--output` to override the file path:

```bash
mosaic-compile --backend paint -o thumb.png ProfileCard.mosaic
```
