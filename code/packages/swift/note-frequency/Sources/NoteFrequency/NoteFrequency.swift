import Foundation

public enum NoteFrequencyError: Error, Equatable, CustomStringConvertible {
    case invalidNote(String)
    case unsupportedSpelling(String)

    public var description: String {
        switch self {
        case let .invalidNote(text):
            return "Invalid note \(String(reflecting: text)). Expected <letter><optional # or b><octave>, for example 'A4', 'C#5', or 'Db3'."
        case let .unsupportedSpelling(spelling):
            return "Unsupported note spelling \(String(reflecting: spelling)). Only natural notes plus single # or b accidentals are supported."
        }
    }
}

public struct Note: Equatable, CustomStringConvertible {
    public let letter: String
    public let accidental: String
    public let octave: Int

    private static let chromaticIndex: [String: Int] = [
        "C": 0,
        "C#": 1,
        "Db": 1,
        "D": 2,
        "D#": 3,
        "Eb": 3,
        "E": 4,
        "F": 5,
        "F#": 6,
        "Gb": 6,
        "G": 7,
        "G#": 8,
        "Ab": 8,
        "A": 9,
        "A#": 10,
        "Bb": 10,
        "B": 11,
    ]

    private static let referenceOctave = 4
    private static let referenceIndex = 9
    private static let referenceFrequencyHz = 440.0
    private static let semitonesPerOctave = 12

    public init(letter: String, accidental: String = "", octave: Int) throws {
        let canonicalLetter = letter.uppercased()
        let spelling = canonicalLetter + accidental
        guard Note.chromaticIndex[spelling] != nil else {
            throw NoteFrequencyError.unsupportedSpelling(spelling)
        }
        self.letter = canonicalLetter
        self.accidental = accidental
        self.octave = octave
    }

    public var spelling: String {
        letter + accidental
    }

    public var chromaticIndexValue: Int {
        Note.chromaticIndex[spelling]!
    }

    public func semitonesFromA4() -> Int {
        let octaveOffset = (octave - Note.referenceOctave) * Note.semitonesPerOctave
        let pitchOffset = chromaticIndexValue - Note.referenceIndex
        return octaveOffset + pitchOffset
    }

    public func frequency() -> Double {
        Note.referenceFrequencyHz * Foundation.pow(2.0, Double(semitonesFromA4()) / Double(Note.semitonesPerOctave))
    }

    public var description: String {
        "\(spelling)\(octave)"
    }
}

public func parseNote(_ text: String) throws -> Note {
    guard let first = text.first else {
        throw NoteFrequencyError.invalidNote(text)
    }

    let uppercaseLetter = String(first).uppercased()
    guard ["A", "B", "C", "D", "E", "F", "G"].contains(uppercaseLetter) else {
        throw NoteFrequencyError.invalidNote(text)
    }

    let rest = String(text.dropFirst())
    let accidental: String
    let octaveText: String
    if rest.hasPrefix("#") {
        accidental = "#"
        octaveText = String(rest.dropFirst())
    } else if rest.hasPrefix("b") {
        accidental = "b"
        octaveText = String(rest.dropFirst())
    } else {
        accidental = ""
        octaveText = rest
    }

    guard isCanonicalOctave(octaveText), let octave = Int(octaveText) else {
        throw NoteFrequencyError.invalidNote(text)
    }

    return try Note(letter: uppercaseLetter, accidental: accidental, octave: octave)
}

private func isCanonicalOctave(_ text: String) -> Bool {
    guard !text.isEmpty else {
        return false
    }

    let digits: Substring
    if text.first == "-" {
        digits = text.dropFirst()
    } else {
        digits = text[...]
    }

    return !digits.isEmpty && digits.allSatisfy { $0 >= "0" && $0 <= "9" }
}

public func noteToFrequency(_ text: String) throws -> Double {
    try parseNote(text).frequency()
}
