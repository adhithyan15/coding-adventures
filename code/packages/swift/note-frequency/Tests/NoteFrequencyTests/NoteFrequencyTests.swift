import XCTest
@testable import NoteFrequency

final class NoteFrequencyTests: XCTestCase {
    func testParseNote() throws {
        let note = try parseNote("C#5")
        XCTAssertEqual(note.letter, "C")
        XCTAssertEqual(note.accidental, "#")
        XCTAssertEqual(note.octave, 5)
    }

    func testLowercaseNormalization() throws {
        XCTAssertEqual(try parseNote("g4").description, "G4")
    }

    func testMalformedNotesThrow() {
        for value in ["", "A", "H4", "#4", "4A", "A##4", "Bb"] {
            XCTAssertThrowsError(try parseNote(value))
        }
    }

    func testUnsupportedSpellingsThrow() {
        XCTAssertThrowsError(try Note(letter: "E", accidental: "#", octave: 4))
    }

    func testSemitoneOffsets() throws {
        XCTAssertEqual(try parseNote("A4").semitonesFromA4(), 0)
        XCTAssertEqual(try parseNote("A5").semitonesFromA4(), 12)
        XCTAssertEqual(try parseNote("A3").semitonesFromA4(), -12)
        XCTAssertEqual(try parseNote("C4").semitonesFromA4(), -9)
    }

    func testFrequencies() throws {
        XCTAssertEqual(try parseNote("A4").frequency(), 440.0, accuracy: 1e-12)
        XCTAssertEqual(try parseNote("A5").frequency(), 880.0, accuracy: 1e-12)
        XCTAssertEqual(try parseNote("A3").frequency(), 220.0, accuracy: 1e-12)
        XCTAssertEqual(try noteToFrequency("C4"), 261.6255653005986, accuracy: 1e-12)
        XCTAssertEqual(try noteToFrequency("C#4"), try noteToFrequency("Db4"), accuracy: 1e-12)
    }
}
