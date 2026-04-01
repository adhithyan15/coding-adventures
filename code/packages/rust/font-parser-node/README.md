# font-parser-node (Rust)

Node.js N-API addon wrapping the Rust `font-parser` core. Part of the FNT series.

Exposes `load`, `fontMetrics`, `glyphId`, `glyphMetrics`, and `kerning`
on the addon's `exports` object using Node.js's stable N-API interface.

## Quick start

```javascript
const fp = require("./font_parser_native.node");
const fs = require("fs");

const data = fs.readFileSync("Inter-Regular.ttf");
const font = fp.load(data);

const m = fp.fontMetrics(font);
console.log(m.unitsPerEm);   // 2048
console.log(m.familyName);   // "Inter"

const gidA = fp.glyphId(font, 0x0041);
const gm   = fp.glyphMetrics(font, gidA);
console.log(gm.advanceWidth);        // e.g. 1401
console.log(gm.leftSideBearing);     // e.g. 7

const k = fp.kerning(font, gidA, fp.glyphId(font, 0x0056));
// 0 — Inter v4.0 uses GPOS, not the legacy kern table
```

## How it works

`fp.load()` creates an instance of the `FontFile` JS class and wraps a
`Box<FontFile>` in it via `napi_wrap` with a GC finalizer that calls
`Box::from_raw` when the JS object is collected.

The addon targets N-API version 4 (Node.js 10.16+) for ABI stability.

## Building

```bash
cargo build -p font-parser-node --release
cp target/release/libfont_parser_native.dylib font_parser_native.node  # macOS
# or .so on Linux, .dll on Windows
```

## License

MIT
