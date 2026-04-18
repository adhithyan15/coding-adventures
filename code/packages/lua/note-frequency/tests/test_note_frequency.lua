local note_frequency = require("coding_adventures.note_frequency")

describe("note_frequency", function()
    it("parses note fields", function()
        local note = note_frequency.parse_note("C#5")
        assert.are.equal("C", note.letter)
        assert.are.equal("#", note.accidental)
        assert.are.equal(5, note.octave)
    end)

    it("normalizes lowercase note letters", function()
        assert.are.equal("G4", tostring(note_frequency.parse_note("g4")))
    end)

    it("rejects malformed note strings", function()
        for _, value in ipairs({"", "A", "H4", "#4", "4A", "A##4", "Bb"}) do
            local ok, err = pcall(function()
                note_frequency.parse_note(value)
            end)

            assert.is_false(ok)
            assert.is_truthy(string.find(err, "Invalid note", 1, true))
        end
    end)

    it("rejects unsupported spellings", function()
        local ok, err = pcall(function()
            note_frequency.Note.new("E", "#", 4)
        end)

        assert.is_false(ok)
        assert.is_truthy(string.find(err, "Unsupported note spelling", 1, true))
    end)

    it("matches semitone reference examples", function()
        assert.are.equal(0, note_frequency.parse_note("A4"):semitones_from_a4())
        assert.are.equal(12, note_frequency.parse_note("A5"):semitones_from_a4())
        assert.are.equal(-12, note_frequency.parse_note("A3"):semitones_from_a4())
        assert.are.equal(-9, note_frequency.parse_note("C4"):semitones_from_a4())
    end)

    it("matches frequency reference examples", function()
        assert.is_true(math.abs(note_frequency.parse_note("A4"):frequency() - 440.0) < 1e-12)
        assert.is_true(math.abs(note_frequency.parse_note("A5"):frequency() - 880.0) < 1e-12)
        assert.is_true(math.abs(note_frequency.parse_note("A3"):frequency() - 220.0) < 1e-12)
        assert.is_true(math.abs(note_frequency.note_to_frequency("C4") - 261.6255653005986) < 1e-12)
        assert.is_true(math.abs(note_frequency.note_to_frequency("C#4") - note_frequency.note_to_frequency("Db4")) < 1e-12)
    end)
end)
