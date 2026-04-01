# FontParser (Swift)

Metrics-only OpenType/TrueType font parser in pure Swift with zero external
dependencies. Part of the FNT series.

## Installation (Swift Package Manager)

```swift
// Package.swift
.package(path: "../font-parser")
// target:
.product(name: "FontParser", package: "font-parser")
```

## Quick start

```swift
import FontParser
import Foundation

let data = try Data(contentsOf: URL(fileURLWithPath: "Inter-Regular.ttf"))
let font = try load(data)

let m = fontMetrics(font)
print(m.unitsPerEm)     // 2048
print(m.familyName)     // "Inter"
print(m.ascender)       // 1984
print(m.xHeight!)       // 1082

let gidA = glyphId(font, codepoint: 0x0041)!   // 'A'
let gm   = glyphMetrics(font, glyphId: Int(gidA))!
print(gm.advanceWidth)          // e.g. 1401
print(gm.leftSideBearing)       // e.g. 7

print(kerning(font, left: Int(gidA), right: Int(glyphId(font, codepoint: 0x0056)!)))
// 0 — Inter v4.0 uses GPOS, not the legacy kern table
```

## Error handling

```swift
do {
    let font = try FontParser.load(data)
} catch FontError.bufferTooShort {
    print("too short")
} catch FontError.invalidMagic {
    print("bad magic")
} catch FontError.tableNotFound(let tag) {
    print("missing table: \(tag)")
} catch FontError.parseError(let msg) {
    print("parse error: \(msg)")
}
```

## Development

```
swift test
```

## License

MIT
