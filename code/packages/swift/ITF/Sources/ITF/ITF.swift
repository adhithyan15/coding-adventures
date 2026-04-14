import BarcodeLayout1D
import PaintInstructions

public struct ITFPair: Equatable, Sendable {
    public let pair: String
    public let barPattern: String
    public let spacePattern: String
    public let binaryPattern: String
    public let sourceIndex: Int
}

public let defaultITFLayoutConfig = defaultBarcode1DLayoutConfig
public let defaultITFRenderConfig = defaultITFLayoutConfig

public enum ITFError: Error, Equatable {
    case invalidDigits
    case invalidLength
}

private let itfStartPattern = "1010"
private let itfStopPattern = "11101"
private let itfDigitPatterns = ["00110", "10001", "01001", "11000", "00101", "10100", "01100", "00011", "10010", "01010"]

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

public func normalizeITF(_ data: String) throws -> String {
    guard data.allSatisfy({ $0.isASCII && $0.isNumber }) else {
        throw ITFError.invalidDigits
    }
    guard !data.isEmpty, data.count.isMultiple(of: 2) else {
        throw ITFError.invalidLength
    }
    return data
}

public func encodeITF(_ data: String) throws -> [ITFPair] {
    let normalized = try normalizeITF(data)
    let digits = normalized.map(String.init)
    var encoded: [ITFPair] = []

    for pairIndex in stride(from: 0, to: digits.count, by: 2) {
        let left = digits[pairIndex]
        let right = digits[pairIndex + 1]
        let barPattern = itfDigitPatterns[Int(left)!]
        let spacePattern = itfDigitPatterns[Int(right)!]
        var binaryPattern = ""

        for offset in 0..<5 {
            let barMarker = Array(barPattern)[offset]
            let spaceMarker = Array(spacePattern)[offset]
            binaryPattern += (barMarker == "1" ? "111" : "1")
            binaryPattern += (spaceMarker == "1" ? "000" : "0")
        }

        encoded.append(
            ITFPair(
                pair: left + right,
                barPattern: barPattern,
                spacePattern: spacePattern,
                binaryPattern: binaryPattern,
                sourceIndex: pairIndex / 2
            )
        )
    }

    return encoded
}

public func expandITFRuns(_ data: String) throws -> [Barcode1DRun] {
    let encoded = try encodeITF(data)
    var runs: [Barcode1DRun] = []

    runs.append(contentsOf: retagRuns(try runsFromBinaryPattern(itfStartPattern, sourceCharacter: "start", sourceIndex: -1), role: "start"))
    for pair in encoded {
        runs.append(contentsOf: retagRuns(
            try runsFromBinaryPattern(pair.binaryPattern, sourceCharacter: pair.pair, sourceIndex: pair.sourceIndex),
            role: "data"
        ))
    }
    runs.append(contentsOf: retagRuns(try runsFromBinaryPattern(itfStopPattern, sourceCharacter: "stop", sourceIndex: -2), role: "stop"))
    return runs
}

public func layoutITF(
    _ data: String,
    config: Barcode1DLayoutConfig = defaultITFLayoutConfig
) throws -> PaintScene {
    let normalized = try normalizeITF(data)
    return try drawOneDimensionalBarcode(
        try expandITFRuns(normalized),
        config: config,
        options: PaintBarcode1DOptions(
            metadata: [
                "symbology": "itf",
                "pair_count": String(normalized.count / 2),
            ]
        )
    )
}

public func drawITF(
    _ data: String,
    config: Barcode1DLayoutConfig = defaultITFLayoutConfig
) throws -> PaintScene {
    try layoutITF(data, config: config)
}
