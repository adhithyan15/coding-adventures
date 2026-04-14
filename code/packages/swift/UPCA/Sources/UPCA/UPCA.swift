import BarcodeLayout1D
import PaintInstructions

public struct UPCADigit: Equatable, Sendable {
    public let digit: String
    public let encoding: String
    public let pattern: String
    public let sourceIndex: Int
    public let role: String
}

public let defaultUPCALayoutConfig = defaultBarcode1DLayoutConfig
public let defaultUPCARenderConfig = defaultUPCALayoutConfig

public enum UPCAError: Error, Equatable {
    case invalidDigits
    case invalidLength
    case invalidCheckDigit(expected: String, actual: String)
}

private let upcaSideGuard = "101"
private let upcaCenterGuard = "01010"
private let upcaDigitPatterns: [String: [String]] = [
    "L": ["0001101", "0011001", "0010011", "0111101", "0100011", "0110001", "0101111", "0111011", "0110111", "0001011"],
    "R": ["1110010", "1100110", "1101100", "1000010", "1011100", "1001110", "1010000", "1000100", "1001000", "1110100"],
]

private func assertUPCADigits(_ data: String, lengths: Set<Int>) throws {
    guard data.allSatisfy({ $0.isASCII && $0.isNumber }) else {
        throw UPCAError.invalidDigits
    }
    guard lengths.contains(data.count) else {
        throw UPCAError.invalidLength
    }
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

public func computeUPCACheckDigit(_ payload11: String) throws -> String {
    try assertUPCADigits(payload11, lengths: [11])
    let digits = payload11.map(String.init)
    var oddSum = 0
    var evenSum = 0

    for (index, digit) in digits.enumerated() {
        if index.isMultiple(of: 2) {
            oddSum += Int(digit) ?? 0
        } else {
            evenSum += Int(digit) ?? 0
        }
    }

    return String((10 - (((oddSum * 3) + evenSum) % 10)) % 10)
}

public func normalizeUPCA(_ data: String) throws -> String {
    try assertUPCADigits(data, lengths: [11, 12])
    if data.count == 11 {
        return data + (try computeUPCACheckDigit(data))
    }

    let expected = try computeUPCACheckDigit(String(data.prefix(11)))
    let actual = String(data.suffix(1))
    guard expected == actual else {
        throw UPCAError.invalidCheckDigit(expected: expected, actual: actual)
    }
    return data
}

public func encodeUPCA(_ data: String) throws -> [UPCADigit] {
    let normalized = try normalizeUPCA(data)
    return normalized.map(String.init).enumerated().map { index, digit in
        let encoding = index < 6 ? "L" : "R"
        return UPCADigit(
            digit: digit,
            encoding: encoding,
            pattern: upcaDigitPatterns[encoding]![Int(digit)!],
            sourceIndex: index,
            role: index == 11 ? "check" : "data"
        )
    }
}

public func expandUPCARuns(_ data: String) throws -> [Barcode1DRun] {
    let encoded = try encodeUPCA(data)
    var runs: [Barcode1DRun] = []

    runs.append(contentsOf: retagRuns(try runsFromBinaryPattern(upcaSideGuard, sourceCharacter: "start", sourceIndex: -1), role: "guard"))
    for entry in encoded.prefix(6) {
        runs.append(contentsOf: retagRuns(
            try runsFromBinaryPattern(entry.pattern, sourceCharacter: entry.digit, sourceIndex: entry.sourceIndex),
            role: entry.role
        ))
    }

    runs.append(contentsOf: retagRuns(try runsFromBinaryPattern(upcaCenterGuard, sourceCharacter: "center", sourceIndex: -2), role: "guard"))
    for entry in encoded.suffix(6) {
        runs.append(contentsOf: retagRuns(
            try runsFromBinaryPattern(entry.pattern, sourceCharacter: entry.digit, sourceIndex: entry.sourceIndex),
            role: entry.role
        ))
    }

    runs.append(contentsOf: retagRuns(try runsFromBinaryPattern(upcaSideGuard, sourceCharacter: "end", sourceIndex: -3), role: "guard"))
    return runs
}

public func layoutUPCA(
    _ data: String,
    config: Barcode1DLayoutConfig = defaultUPCALayoutConfig
) throws -> PaintScene {
    let normalized = try normalizeUPCA(data)
    return try drawOneDimensionalBarcode(
        try expandUPCARuns(normalized),
        config: config,
        options: PaintBarcode1DOptions(
            metadata: [
                "symbology": "upc-a",
                "content_modules": "95",
            ]
        )
    )
}

public func drawUPCA(
    _ data: String,
    config: Barcode1DLayoutConfig = defaultUPCALayoutConfig
) throws -> PaintScene {
    try layoutUPCA(data, config: config)
}
