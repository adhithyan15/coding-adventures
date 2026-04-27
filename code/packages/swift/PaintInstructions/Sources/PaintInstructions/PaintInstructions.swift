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

public struct PaintRectInstruction: Equatable, Sendable {
    public let x: Int
    public let y: Int
    public let width: Int
    public let height: Int
    public let fill: String
    public let metadata: PaintMetadata

    public var kind: String { "rect" }

    public init(
        x: Int,
        y: Int,
        width: Int,
        height: Int,
        fill: String = "#000000",
        metadata: PaintMetadata = [:]
    ) {
        self.x = x
        self.y = y
        self.width = width
        self.height = height
        self.fill = fill
        self.metadata = metadata
    }
}

public typealias PaintInstruction = PaintRectInstruction

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

public func paintRect(
    x: Int,
    y: Int,
    width: Int,
    height: Int,
    fill: String = "#000000",
    metadata: PaintMetadata = [:]
) -> PaintRectInstruction {
    PaintRectInstruction(
        x: x,
        y: y,
        width: width,
        height: height,
        fill: fill,
        metadata: metadata
    )
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
