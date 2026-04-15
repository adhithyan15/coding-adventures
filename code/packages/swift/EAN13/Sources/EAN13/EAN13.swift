import BarcodeLayout1D
import PaintInstructions

public struct EAN13Digit: Equatable, Sendable {
    public let digit: String
    public let encoding: String
    public let pattern: String
    public let sourceIndex: Int
    public let role: String
}

public let defaultEAN13LayoutConfig = defaultBarcode1DLayoutConfig
public let defaultEAN13RenderConfig = defaultEAN13LayoutConfig

public enum EAN13Error: Error, Equatable {
    case invalidDigits
    case invalidLength
    case invalidCheckDigit(expected: String, actual: String)
}

private let ean13SideGuard = "101"
private let ean13CenterGuard = "01010"

private let ean13DigitPatterns: [String: [String]] = [
    "L": ["0001101", "0011001", "0010011", "0111101", "0100011", "0110001", "0101111", "0111011", "0110111", "0001011"],
    "G": ["0100111", "0110011", "0011011", "0100001", "0011101", "0111001", "0000101", "0010001", "0001001", "0010111"],
    "R": ["1110010", "1100110", "1101100", "1000010", "1011100", "1001110", "1010000", "1000100", "1001000", "1110100"],
]

private let ean13LeftParityPatterns = [
    "LLLLLL", "LLGLGG", "LLGGLG", "LLGGGL", "LGLLGG",
    "LGGLLG", "LGGGLL", "LGLGLG", "LGLGGL", "LGGLGL",
]

private func assertEAN13Digits(_ data: String, lengths: Set<Int>) throws {
    guard data.allSatisfy({ $0.isASCII && $0.isNumber }) else {
        throw EAN13Error.invalidDigits
    }
    guard lengths.contains(data.count) else {
        throw EAN13Error.invalidLength
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

public func computeEAN13CheckDigit(_ payload12: String) throws -> String {
    try assertEAN13Digits(payload12, lengths: [12])
    let digits = payload12.map(String.init).reversed()
    var total = 0
    for (index, digit) in digits.enumerated() {
        total += (Int(digit) ?? 0) * (index.isMultiple(of: 2) ? 3 : 1)
    }
    return String((10 - (total % 10)) % 10)
}

public func normalizeEAN13(_ data: String) throws -> String {
    try assertEAN13Digits(data, lengths: [12, 13])
    if data.count == 12 {
        return data + (try computeEAN13CheckDigit(data))
    }

    let expected = try computeEAN13CheckDigit(String(data.prefix(12)))
    let actual = String(data.suffix(1))
    guard expected == actual else {
        throw EAN13Error.invalidCheckDigit(expected: expected, actual: actual)
    }
    return data
}

public func leftParityPattern(_ data: String) throws -> String {
    let normalized = try normalizeEAN13(data)
    let leadingDigit = Int(String(normalized.prefix(1))) ?? 0
    return ean13LeftParityPatterns[leadingDigit]
}

public func encodeEAN13(_ data: String) throws -> [EAN13Digit] {
    let normalized = try normalizeEAN13(data)
    let parity = try leftParityPattern(normalized)
    let digits = normalized.map(String.init)
    var encoded: [EAN13Digit] = []

    for offset in 0..<6 {
        let digit = digits[offset + 1]
        let encoding = String(Array(parity)[offset])
        encoded.append(
            EAN13Digit(
                digit: digit,
                encoding: encoding,
                pattern: ean13DigitPatterns[encoding]![Int(digit)!],
                sourceIndex: offset + 1,
                role: "data"
            )
        )
    }

    for offset in 0..<6 {
        let digit = digits[offset + 7]
        encoded.append(
            EAN13Digit(
                digit: digit,
                encoding: "R",
                pattern: ean13DigitPatterns["R"]![Int(digit)!],
                sourceIndex: offset + 7,
                role: offset == 5 ? "check" : "data"
            )
        )
    }

    return encoded
}

public func expandEAN13Runs(_ data: String) throws -> [Barcode1DRun] {
    let encoded = try encodeEAN13(data)
    var runs: [Barcode1DRun] = []

    runs.append(contentsOf: retagRuns(try runsFromBinaryPattern(ean13SideGuard, sourceCharacter: "start", sourceIndex: -1), role: "guard"))
    for entry in encoded.prefix(6) {
        runs.append(contentsOf: retagRuns(
            try runsFromBinaryPattern(entry.pattern, sourceCharacter: entry.digit, sourceIndex: entry.sourceIndex),
            role: entry.role
        ))
    }

    runs.append(contentsOf: retagRuns(try runsFromBinaryPattern(ean13CenterGuard, sourceCharacter: "center", sourceIndex: -2), role: "guard"))
    for entry in encoded.suffix(6) {
        runs.append(contentsOf: retagRuns(
            try runsFromBinaryPattern(entry.pattern, sourceCharacter: entry.digit, sourceIndex: entry.sourceIndex),
            role: entry.role
        ))
    }

    runs.append(contentsOf: retagRuns(try runsFromBinaryPattern(ean13SideGuard, sourceCharacter: "end", sourceIndex: -3), role: "guard"))
    return runs
}

public func layoutEAN13(
    _ data: String,
    config: Barcode1DLayoutConfig = defaultEAN13LayoutConfig
) throws -> PaintScene {
    let normalized = try normalizeEAN13(data)
    return try drawOneDimensionalBarcode(
        try expandEAN13Runs(normalized),
        config: config,
        options: PaintBarcode1DOptions(
            metadata: [
                "symbology": "ean-13",
                "leading_digit": String(normalized.prefix(1)),
                "left_parity": try leftParityPattern(normalized),
                "content_modules": "95",
            ]
        )
    )
}

public func drawEAN13(
    _ data: String,
    config: Barcode1DLayoutConfig = defaultEAN13LayoutConfig
) throws -> PaintScene {
    try layoutEAN13(data, config: config)
}
