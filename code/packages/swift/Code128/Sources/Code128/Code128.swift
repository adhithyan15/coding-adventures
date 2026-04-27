import BarcodeLayout1D
import PaintInstructions

public struct Code128Symbol: Equatable, Sendable {
    public let label: String
    public let value: Int
    public let pattern: String
    public let sourceIndex: Int
    public let role: String

    public init(label: String, value: Int, pattern: String, sourceIndex: Int, role: String) {
        self.label = label
        self.value = value
        self.pattern = pattern
        self.sourceIndex = sourceIndex
        self.role = role
    }
}

public let defaultCode128LayoutConfig = defaultBarcode1DLayoutConfig
public let defaultCode128RenderConfig = defaultCode128LayoutConfig

public enum Code128Error: Error, Equatable {
    case nonPrintableCharacter
}

private let code128StartB = 104
private let code128Stop = 106

private let code128Patterns = [
    "11011001100", "11001101100", "11001100110", "10010011000", "10010001100", "10001001100", "10011001000", "10011000100",
    "10001100100", "11001001000", "11001000100", "11000100100", "10110011100", "10011011100", "10011001110", "10111001100",
    "10011101100", "10011100110", "11001110010", "11001011100", "11001001110", "11011100100", "11001110100", "11101101110",
    "11101001100", "11100101100", "11100100110", "11101100100", "11100110100", "11100110010", "11011011000", "11011000110",
    "11000110110", "10100011000", "10001011000", "10001000110", "10110001000", "10001101000", "10001100010", "11010001000",
    "11000101000", "11000100010", "10110111000", "10110001110", "10001101110", "10111011000", "10111000110", "10001110110",
    "11101110110", "11010001110", "11000101110", "11011101000", "11011100010", "11011101110", "11101011000", "11101000110",
    "11100010110", "11101101000", "11101100010", "11100011010", "11101111010", "11001000010", "11110001010", "10100110000",
    "10100001100", "10010110000", "10010000110", "10000101100", "10000100110", "10110010000", "10110000100", "10011010000",
    "10011000010", "10000110100", "10000110010", "11000010010", "11001010000", "11110111010", "11000010100", "10001111010",
    "10100111100", "10010111100", "10010011110", "10111100100", "10011110100", "10011110010", "11110100100", "11110010100",
    "11110010010", "11011011110", "11011110110", "11110110110", "10101111000", "10100011110", "10001011110", "10111101000",
    "10111100010", "11110101000", "11110100010", "10111011110", "10111101110", "11101011110", "11110101110", "11010000100",
    "11010010000", "11010011100", "1100011101011",
]

private func retagRuns(_ runs: [Barcode1DRun], role: String) -> [Barcode1DRun] {
    runs.map {
        Barcode1DRun(
            color: $0.color,
            modules: $0.modules,
            sourceCharacter: $0.sourceCharacter,
            sourceIndex: $0.sourceIndex,
            role: role,
            metadata: $0.metadata
        )
    }
}

public func normalizeCode128B(_ data: String) throws -> String {
    for scalar in data.unicodeScalars {
        if scalar.value < 32 || scalar.value > 126 {
            throw Code128Error.nonPrintableCharacter
        }
    }
    return data
}

public func valueForCode128BCharacter(_ character: String) -> Int {
    Int(character.unicodeScalars.first?.value ?? 32) - 32
}

public func computeCode128Checksum(_ values: [Int]) -> Int {
    (code128StartB + values.enumerated().reduce(0) { partial, pair in
        partial + (pair.element * (pair.offset + 1))
    }) % 103
}

public func encodeCode128B(_ data: String) throws -> [Code128Symbol] {
    let normalized = try normalizeCode128B(data)
    let dataSymbols = normalized.map(String.init).enumerated().map { index, character in
        let value = valueForCode128BCharacter(character)
        return Code128Symbol(
            label: character,
            value: value,
            pattern: code128Patterns[value],
            sourceIndex: index,
            role: "data"
        )
    }
    let checksum = computeCode128Checksum(dataSymbols.map(\.value))

    return [
        Code128Symbol(label: "Start B", value: code128StartB, pattern: code128Patterns[code128StartB], sourceIndex: -1, role: "start"),
        ] + dataSymbols + [
        Code128Symbol(label: "Checksum \(checksum)", value: checksum, pattern: code128Patterns[checksum], sourceIndex: normalized.count, role: "check"),
        Code128Symbol(label: "Stop", value: code128Stop, pattern: code128Patterns[code128Stop], sourceIndex: normalized.count + 1, role: "stop"),
    ]
}

public func expandCode128Runs(_ data: String) throws -> [Barcode1DRun] {
    try encodeCode128B(data).flatMap { symbol in
        retagRuns(
            try runsFromBinaryPattern(
                symbol.pattern,
                sourceCharacter: symbol.label,
                sourceIndex: symbol.sourceIndex
            ),
            role: symbol.role
        )
    }
}

public func layoutCode128(
    _ data: String,
    config: Barcode1DLayoutConfig = defaultCode128LayoutConfig
) throws -> PaintScene {
    let normalized = try normalizeCode128B(data)
    let checksum = try encodeCode128B(normalized)[normalizeCode128B(normalized).count + 1].value

    return try drawOneDimensionalBarcode(
        try expandCode128Runs(normalized),
        config: config,
        options: PaintBarcode1DOptions(
            metadata: [
                "symbology": "code128",
                "code_set": "B",
                "checksum": String(checksum),
            ]
        )
    )
}

public func drawCode128(
    _ data: String,
    config: Barcode1DLayoutConfig = defaultCode128LayoutConfig
) throws -> PaintScene {
    try layoutCode128(data, config: config)
}
