import BarcodeLayout1D
import PaintInstructions

public struct EncodedCharacter: Equatable, Sendable {
    public let character: String
    public let isStartStop: Bool
    public let pattern: String

    public init(character: String, isStartStop: Bool, pattern: String) {
        self.character = character
        self.isStartStop = isStartStop
        self.pattern = pattern
    }
}

public let defaultCode39LayoutConfig = defaultBarcode1DLayoutConfig
public let defaultCode39RenderConfig = defaultCode39LayoutConfig

public enum Code39Error: Error, Equatable {
    case invalidCharacter(String)
    case reservedStartStop
}

private let code39BarSpacePatterns: [String: String] = [
    "0": "bwbWBwBwb", "1": "BwbWbwbwB", "2": "bwBWbwbwB", "3": "BwBWbwbwb",
    "4": "bwbWBwbwB", "5": "BwbWBwbwb", "6": "bwBWBwbwb", "7": "bwbWbwBwB",
    "8": "BwbWbwBwb", "9": "bwBWbwBwb", "A": "BwbwbWbwB", "B": "bwBwbWbwB",
    "C": "BwBwbWbwb", "D": "bwbwBWbwB", "E": "BwbwBWbwb", "F": "bwBwBWbwb",
    "G": "bwbwbWBwB", "H": "BwbwbWBwb", "I": "bwBwbWBwb", "J": "bwbwBWBwb",
    "K": "BwbwbwbWB", "L": "bwBwbwbWB", "M": "BwBwbwbWb", "N": "bwbwBwbWB",
    "O": "BwbwBwbWb", "P": "bwBwBwbWb", "Q": "bwbwbwBWB", "R": "BwbwbwBWb",
    "S": "bwBwbwBWb", "T": "bwbwBwBWb", "U": "BWbwbwbwB", "V": "bWBwbwbwB",
    "W": "BWBwbwbwb", "X": "bWbwBwbwB", "Y": "BWbwBwbwb", "Z": "bWBwBwbwb",
    "-": "bWbwbwBwB", ".": "BWbwbwBwb", " ": "bWBwbwBwb", "$": "bWbWbWbwb",
    "/": "bWbWbwbWb", "+": "bWbwbWbWb", "%": "bwbWbWbWb", "*": "bWbwBwBwb",
]

private let barSpaceColors = [
    "bar", "space", "bar", "space", "bar", "space", "bar", "space", "bar",
]

private func widthPattern(_ pattern: String) -> String {
    pattern.map { $0.isUppercase ? "W" : "N" }.joined()
}

public func normalizeCode39(_ data: String) throws -> String {
    let normalized = data.uppercased()
    for character in normalized {
        if character == "*" {
            throw Code39Error.reservedStartStop
        }
        if code39BarSpacePatterns[String(character)] == nil {
            throw Code39Error.invalidCharacter(String(character))
        }
    }
    return normalized
}

public func encodeCode39Character(_ character: String) throws -> EncodedCharacter {
    guard let pattern = code39BarSpacePatterns[character] else {
        throw Code39Error.invalidCharacter(character)
    }
    return EncodedCharacter(
        character: character,
        isStartStop: character == "*",
        pattern: widthPattern(pattern)
    )
}

public func encodeCode39(_ data: String) throws -> [EncodedCharacter] {
    let normalized = try normalizeCode39(data)
    let symbols = ["*"] + normalized.map { String($0) } + ["*"]
    return try symbols.map { try encodeCode39Character($0) }
}

public func expandCode39Runs(_ data: String) throws -> [Barcode1DRun] {
    let encoded = try encodeCode39(data)
    var runs: [Barcode1DRun] = []

    for (index, character) in encoded.enumerated() {
        runs.append(
            contentsOf: try runsFromWidthPattern(
                character.pattern,
                colors: barSpaceColors,
                sourceCharacter: character.character,
                sourceIndex: index
            )
        )

        if index < encoded.count - 1 {
            runs.append(
                Barcode1DRun(
                    color: "space",
                    modules: 1,
                    sourceCharacter: character.character,
                    sourceIndex: index,
                    role: "inter-character-gap"
                )
            )
        }
    }

    return runs
}

public func layoutCode39(
    _ data: String,
    config: Barcode1DLayoutConfig = defaultCode39LayoutConfig
) throws -> PaintScene {
    let normalized = try normalizeCode39(data)
    return try drawOneDimensionalBarcode(
        try expandCode39Runs(normalized),
        config: config,
        options: PaintBarcode1DOptions(
            metadata: [
                "symbology": "code39",
                "data": normalized,
            ]
        )
    )
}

public func drawCode39(
    _ data: String,
    config: Barcode1DLayoutConfig = defaultCode39LayoutConfig
) throws -> PaintScene {
    try layoutCode39(data, config: config)
}
