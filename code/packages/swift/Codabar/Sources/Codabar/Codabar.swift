import BarcodeLayout1D
import PaintInstructions

public struct CodabarSymbol: Equatable, Sendable {
    public let character: String
    public let pattern: String
    public let sourceIndex: Int
    public let role: String

    public init(character: String, pattern: String, sourceIndex: Int, role: String) {
        self.character = character
        self.pattern = pattern
        self.sourceIndex = sourceIndex
        self.role = role
    }
}

public let defaultCodabarLayoutConfig = defaultBarcode1DLayoutConfig
public let defaultCodabarRenderConfig = defaultCodabarLayoutConfig

public enum CodabarError: Error, Equatable {
    case invalidBodyCharacter(String)
    case invalidGuard(String)
}

private let codabarPatterns: [String: String] = [
    "0": "101010011", "1": "101011001", "2": "101001011", "3": "110010101",
    "4": "101101001", "5": "110101001", "6": "100101011", "7": "100101101",
    "8": "100110101", "9": "110100101", "-": "101001101", "$": "101100101",
    ":": "1101011011", "/": "1101101011", ".": "1101101101", "+": "1011011011",
    "A": "1011001001", "B": "1001001011", "C": "1010010011", "D": "1010011001",
]

private let codabarGuards: Set<String> = ["A", "B", "C", "D"]

private func isGuard(_ value: String) -> Bool {
    codabarGuards.contains(value)
}

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

private func assertBodyCharacters(_ body: String) throws {
    for character in body {
        let token = String(character)
        if codabarPatterns[token] == nil || isGuard(token) {
            throw CodabarError.invalidBodyCharacter(token)
        }
    }
}

public func normalizeCodabar(
    _ data: String,
    start: String = "A",
    stop: String = "A"
) throws -> String {
    let normalized = data.uppercased()
    let startGuard = start.uppercased()
    let stopGuard = stop.uppercased()
    let symbols = normalized.map(String.init)

    if symbols.count >= 2,
       let first = symbols.first,
       let last = symbols.last,
       isGuard(first),
       isGuard(last) {
        try assertBodyCharacters(symbols.dropFirst().dropLast().joined())
        return normalized
    }

    guard isGuard(startGuard), isGuard(stopGuard) else {
        throw CodabarError.invalidGuard("Codabar guards must be one of A, B, C, or D")
    }

    try assertBodyCharacters(normalized)
    return startGuard + normalized + stopGuard
}

public func encodeCodabar(
    _ data: String,
    start: String = "A",
    stop: String = "A"
) throws -> [CodabarSymbol] {
    let normalized = try normalizeCodabar(data, start: start, stop: stop)
    return try normalized.map(String.init).enumerated().map { index, character in
        guard let pattern = codabarPatterns[character] else {
            throw CodabarError.invalidBodyCharacter(character)
        }

        let role: String
        if index == 0 {
            role = "start"
        } else if index == normalized.count - 1 {
            role = "stop"
        } else {
            role = "data"
        }

        return CodabarSymbol(character: character, pattern: pattern, sourceIndex: index, role: role)
    }
}

public func expandCodabarRuns(
    _ data: String,
    start: String = "A",
    stop: String = "A"
) throws -> [Barcode1DRun] {
    let encoded = try encodeCodabar(data, start: start, stop: stop)
    var runs: [Barcode1DRun] = []

    for (index, symbol) in encoded.enumerated() {
        runs.append(
            contentsOf: retagRuns(
                try runsFromBinaryPattern(
                    symbol.pattern,
                    sourceCharacter: symbol.character,
                    sourceIndex: symbol.sourceIndex
                ),
                role: symbol.role
            )
        )

        if index < encoded.count - 1 {
            runs.append(
                Barcode1DRun(
                    color: "space",
                    modules: 1,
                    sourceCharacter: symbol.character,
                    sourceIndex: symbol.sourceIndex,
                    role: "inter-character-gap"
                )
            )
        }
    }

    return runs
}

public func layoutCodabar(
    _ data: String,
    config: Barcode1DLayoutConfig = defaultCodabarLayoutConfig,
    start: String = "A",
    stop: String = "A"
) throws -> PaintScene {
    let normalized = try normalizeCodabar(data, start: start, stop: stop)
    return try drawOneDimensionalBarcode(
        try expandCodabarRuns(normalized),
        config: config,
        options: PaintBarcode1DOptions(
            metadata: [
                "symbology": "codabar",
                "start": String(normalized.prefix(1)),
                "stop": String(normalized.suffix(1)),
            ]
        )
    )
}

public func drawCodabar(
    _ data: String,
    config: Barcode1DLayoutConfig = defaultCodabarLayoutConfig,
    start: String = "A",
    stop: String = "A"
) throws -> PaintScene {
    try layoutCodabar(data, config: config, start: start, stop: stop)
}
