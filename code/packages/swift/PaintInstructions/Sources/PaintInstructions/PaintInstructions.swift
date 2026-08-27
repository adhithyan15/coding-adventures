public typealias PaintMetadata = [String: String]

public struct PaintColorRGBA8: Equatable, Sendable {
    public let r: UInt8
    public let g: UInt8
    public let b: UInt8
    public let a: UInt8

    public init(r: UInt8, g: UInt8, b: UInt8, a: UInt8 = 255) {
        self.r = r
        self.g = g
        self.b = b
        self.a = a
    }

    public static let black = PaintColorRGBA8(r: 0, g: 0, b: 0, a: 255)
    public static let white = PaintColorRGBA8(r: 255, g: 255, b: 255, a: 255)
    public static let transparent = PaintColorRGBA8(r: 0, g: 0, b: 0, a: 0)
}

public func parsePaintColor(_ value: String) -> PaintColorRGBA8 {
    if value == "transparent" {
        return .transparent
    }

    var hex = value
    if hex.hasPrefix("#") {
        hex.removeFirst()
    }

    if hex.count == 3 {
        hex = hex.reduce(into: "") { result, char in
            result.append(char)
            result.append(char)
        }
    }

    func byte(_ start: String.Index, _ offset: Int) -> UInt8 {
        let end = hex.index(start, offsetBy: offset)
        return UInt8(hex[start..<end], radix: 16) ?? 0
    }

    guard hex.count == 6 || hex.count == 8 else {
        return .black
    }

    let start = hex.startIndex
    let r = byte(start, 2)
    let g = byte(hex.index(start, offsetBy: 2), 2)
    let b = byte(hex.index(start, offsetBy: 4), 2)
    let a = hex.count == 8 ? byte(hex.index(start, offsetBy: 6), 2) : UInt8(255)
    return PaintColorRGBA8(r: r, g: g, b: b, a: a)
}

// MARK: - Rect

public struct PaintRectInstruction: Equatable, Sendable {
    public let x: Int
    public let y: Int
    public let width: Int
    public let height: Int
    public let fill: String
    /// CSS hex stroke colour. Empty (the default) means no stroke.
    public let stroke: String
    /// Stroke width in pixels. Ignored when `stroke` is empty.
    public let strokeWidth: Double
    public let metadata: PaintMetadata

    public var kind: String { "rect" }

    public init(
        x: Int,
        y: Int,
        width: Int,
        height: Int,
        fill: String = "#000000",
        stroke: String = "",
        strokeWidth: Double = 0.0,
        metadata: PaintMetadata = [:]
    ) {
        self.x = x
        self.y = y
        self.width = width
        self.height = height
        self.fill = fill
        self.stroke = stroke
        self.strokeWidth = strokeWidth
        self.metadata = metadata
    }
}

// MARK: - Line

/// A stroked line segment between two points. A line with no stroke is
/// invisible, so unlike `PaintRectInstruction`, `stroke` is required here.
public struct PaintLineInstruction: Equatable, Sendable {
    public let x1: Double
    public let y1: Double
    public let x2: Double
    public let y2: Double
    public let stroke: String
    public let strokeWidth: Double
    public let metadata: PaintMetadata

    public var kind: String { "line" }

    public init(
        x1: Double,
        y1: Double,
        x2: Double,
        y2: Double,
        stroke: String,
        strokeWidth: Double,
        metadata: PaintMetadata = [:]
    ) {
        self.x1 = x1
        self.y1 = y1
        self.x2 = x2
        self.y2 = y2
        self.stroke = stroke
        self.strokeWidth = strokeWidth
        self.metadata = metadata
    }
}

// MARK: - Glyph run

/// One glyph's position within a `PaintGlyphRunInstruction`.
///
/// `glyphId` is a font-internal glyph index in the general contract, but
/// text-mode ("ascii") backends relax this to a literal Unicode code point
/// — see `P2D02-paint-vm-ascii.md` "Glyph runs" for the rationale.
public struct PaintGlyphPlacement: Equatable, Sendable {
    public let glyphId: Int
    public let x: Double
    public let y: Double

    public init(glyphId: Int, x: Double, y: Double) {
        self.glyphId = glyphId
        self.x = x
        self.y = y
    }
}

/// Pre-positioned glyphs, each already placed in scene coordinates.
/// `fontRef`, `fontSize`, and `fill` are required fields but are ignored by
/// text-mode (ASCII) backends.
public struct PaintGlyphRunInstruction: Equatable, Sendable {
    public let glyphs: [PaintGlyphPlacement]
    public let fontRef: String
    public let fontSize: Double
    public let fill: String
    public let metadata: PaintMetadata

    public var kind: String { "glyph_run" }

    public init(
        glyphs: [PaintGlyphPlacement],
        fontRef: String,
        fontSize: Double,
        fill: String,
        metadata: PaintMetadata = [:]
    ) {
        self.glyphs = glyphs
        self.fontRef = fontRef
        self.fontSize = fontSize
        self.fill = fill
        self.metadata = metadata
    }
}

// MARK: - Transform2D

/// A six-value affine transform, matching the Canvas/SVG convention:
/// `x' = a*x + c*y + e`, `y' = b*x + d*y + f`.
public struct Transform2D: Equatable, Sendable {
    public let a: Double
    public let b: Double
    public let c: Double
    public let d: Double
    public let e: Double
    public let f: Double

    public init(a: Double, b: Double, c: Double, d: Double, e: Double, f: Double) {
        self.a = a
        self.b = b
        self.c = c
        self.d = d
        self.e = e
        self.f = f
    }

    public static let identity = Transform2D(a: 1, b: 0, c: 0, d: 1, e: 0, f: 0)

    public var isIdentity: Bool {
        a == 1 && b == 0 && c == 0 && d == 1 && e == 0 && f == 0
    }
}

// MARK: - Group / Clip / Layer

/// A child list with an optional `Transform2D` and opacity.
public struct PaintGroupInstruction: Equatable, Sendable {
    public let children: [PaintInstruction]
    public let transform: Transform2D?
    public let opacity: Double?
    public let metadata: PaintMetadata

    public var kind: String { "group" }

    public init(
        children: [PaintInstruction],
        transform: Transform2D? = nil,
        opacity: Double? = nil,
        metadata: PaintMetadata = [:]
    ) {
        self.children = children
        self.transform = transform
        self.opacity = opacity
        self.metadata = metadata
    }
}

/// A rectangular clip region wrapping a child list.
public struct PaintClipInstruction: Equatable, Sendable {
    public let x: Double
    public let y: Double
    public let width: Double
    public let height: Double
    public let children: [PaintInstruction]
    public let metadata: PaintMetadata

    public var kind: String { "clip" }

    public init(
        x: Double,
        y: Double,
        width: Double,
        height: Double,
        children: [PaintInstruction],
        metadata: PaintMetadata = [:]
    ) {
        self.x = x
        self.y = y
        self.width = width
        self.height = height
        self.children = children
        self.metadata = metadata
    }
}

/// A child list with a filter flag, blend mode, opacity, and transform.
/// `hasFilters` is a simplified stand-in for the full filter-effect union —
/// no backend in this repository's Swift port implements pixel-level
/// filters, so all that matters for dispatch is whether to reject the layer.
public struct PaintLayerInstruction: Equatable, Sendable {
    public let children: [PaintInstruction]
    public let hasFilters: Bool
    public let blendMode: String?
    public let opacity: Double?
    public let transform: Transform2D?
    public let metadata: PaintMetadata

    public var kind: String { "layer" }

    public init(
        children: [PaintInstruction],
        hasFilters: Bool = false,
        blendMode: String? = nil,
        opacity: Double? = nil,
        transform: Transform2D? = nil,
        metadata: PaintMetadata = [:]
    ) {
        self.children = children
        self.hasFilters = hasFilters
        self.blendMode = blendMode
        self.opacity = opacity
        self.transform = transform
        self.metadata = metadata
    }
}

// MARK: - PaintInstruction (closed sum type)

/// Every renderable paint instruction. Swift's idiomatic closed sum type is
/// an `enum` with associated values — the equivalent of the sealed
/// class/interface hierarchy used by this same package in Kotlin, Java, and
/// Dart. Switching over a `PaintInstruction` without a `default:` case
/// produces a compiler error if a new case is ever added, the same
/// exhaustiveness safety net those languages get from `sealed`.
public enum PaintInstruction: Equatable, Sendable {
    case rect(PaintRectInstruction)
    case line(PaintLineInstruction)
    case glyphRun(PaintGlyphRunInstruction)
    case group(PaintGroupInstruction)
    case clip(PaintClipInstruction)
    case layer(PaintLayerInstruction)

    public var kind: String {
        switch self {
        case .rect(let r): return r.kind
        case .line(let l): return l.kind
        case .glyphRun(let g): return g.kind
        case .group(let g): return g.kind
        case .clip(let c): return c.kind
        case .layer(let l): return l.kind
        }
    }
}

// MARK: - PaintScene

public struct PaintScene: Equatable, Sendable {
    public let width: Int
    public let height: Int
    public let instructions: [PaintInstruction]
    public let background: String
    public let metadata: PaintMetadata

    public init(
        width: Int,
        height: Int,
        instructions: [PaintInstruction],
        background: String = "#ffffff",
        metadata: PaintMetadata = [:]
    ) {
        self.width = width
        self.height = height
        self.instructions = instructions
        self.background = background
        self.metadata = metadata
    }
}

// MARK: - Helper factory functions

public func paintRect(
    x: Int,
    y: Int,
    width: Int,
    height: Int,
    fill: String = "#000000",
    stroke: String = "",
    strokeWidth: Double = 0.0,
    metadata: PaintMetadata = [:]
) -> PaintInstruction {
    .rect(PaintRectInstruction(
        x: x,
        y: y,
        width: width,
        height: height,
        fill: fill,
        stroke: stroke,
        strokeWidth: strokeWidth,
        metadata: metadata
    ))
}

public func paintLine(
    x1: Double,
    y1: Double,
    x2: Double,
    y2: Double,
    stroke: String,
    strokeWidth: Double,
    metadata: PaintMetadata = [:]
) -> PaintInstruction {
    .line(PaintLineInstruction(
        x1: x1, y1: y1, x2: x2, y2: y2,
        stroke: stroke, strokeWidth: strokeWidth, metadata: metadata
    ))
}

public func paintGlyphRun(
    glyphs: [PaintGlyphPlacement],
    fontRef: String,
    fontSize: Double,
    fill: String,
    metadata: PaintMetadata = [:]
) -> PaintInstruction {
    .glyphRun(PaintGlyphRunInstruction(
        glyphs: glyphs, fontRef: fontRef, fontSize: fontSize, fill: fill, metadata: metadata
    ))
}

public func paintGroup(
    children: [PaintInstruction],
    transform: Transform2D? = nil,
    opacity: Double? = nil,
    metadata: PaintMetadata = [:]
) -> PaintInstruction {
    .group(PaintGroupInstruction(
        children: children, transform: transform, opacity: opacity, metadata: metadata
    ))
}

public func paintClip(
    x: Double,
    y: Double,
    width: Double,
    height: Double,
    children: [PaintInstruction],
    metadata: PaintMetadata = [:]
) -> PaintInstruction {
    .clip(PaintClipInstruction(
        x: x, y: y, width: width, height: height, children: children, metadata: metadata
    ))
}

public func paintLayer(
    children: [PaintInstruction],
    hasFilters: Bool = false,
    blendMode: String? = nil,
    opacity: Double? = nil,
    transform: Transform2D? = nil,
    metadata: PaintMetadata = [:]
) -> PaintInstruction {
    .layer(PaintLayerInstruction(
        children: children, hasFilters: hasFilters, blendMode: blendMode,
        opacity: opacity, transform: transform, metadata: metadata
    ))
}

public func paintScene(
    width: Int,
    height: Int,
    instructions: [PaintInstruction],
    background: String = "#ffffff",
    metadata: PaintMetadata = [:]
) -> PaintScene {
    PaintScene(
        width: width,
        height: height,
        instructions: instructions,
        background: background,
        metadata: metadata
    )
}

public func createScene(
    width: Int,
    height: Int,
    instructions: [PaintInstruction],
    background: String = "#ffffff",
    metadata: PaintMetadata = [:]
) -> PaintScene {
    paintScene(
        width: width,
        height: height,
        instructions: instructions,
        background: background,
        metadata: metadata
    )
}
