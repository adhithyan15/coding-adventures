local note_frequency = {}
local Note = {}
Note.__index = Note

local NOTE_PATTERN = "^([A-Ga-g])([#b]?)(%-?%d+)$"
local CHROMATIC_INDEX = {
    ["C"] = 0,
    ["C#"] = 1,
    ["Db"] = 1,
    ["D"] = 2,
    ["D#"] = 3,
    ["Eb"] = 3,
    ["E"] = 4,
    ["F"] = 5,
    ["F#"] = 6,
    ["Gb"] = 6,
    ["G"] = 7,
    ["G#"] = 8,
    ["Ab"] = 8,
    ["A"] = 9,
    ["A#"] = 10,
    ["Bb"] = 10,
    ["B"] = 11,
}

local REFERENCE_OCTAVE = 4
local REFERENCE_INDEX = CHROMATIC_INDEX["A"]
local REFERENCE_FREQUENCY_HZ = 440.0
local SEMITONES_PER_OCTAVE = 12

function Note.new(letter, accidental, octave)
    local canonical_letter = string.upper(letter)
    local spelling = canonical_letter .. accidental
    if CHROMATIC_INDEX[spelling] == nil then
        error("Unsupported note spelling " .. string.format("%q", spelling) .. ". Only natural notes plus single # or b accidentals are supported.")
    end

    return setmetatable({
        letter = canonical_letter,
        accidental = accidental,
        octave = octave,
    }, Note)
end

function Note:spelling()
    return self.letter .. self.accidental
end

function Note:chromatic_index()
    return CHROMATIC_INDEX[self:spelling()]
end

function Note:semitones_from_a4()
    local octave_offset = (self.octave - REFERENCE_OCTAVE) * SEMITONES_PER_OCTAVE
    local pitch_offset = self:chromatic_index() - REFERENCE_INDEX
    return octave_offset + pitch_offset
end

function Note:frequency()
    return REFERENCE_FREQUENCY_HZ * (2 ^ (self:semitones_from_a4() / SEMITONES_PER_OCTAVE))
end

function Note:__tostring()
    return self:spelling() .. tostring(self.octave)
end

function note_frequency.parse_note(text)
    local letter, accidental, octave_text = string.match(text, NOTE_PATTERN)
    if letter == nil then
        error("Invalid note " .. string.format("%q", text) .. ". Expected <letter><optional # or b><octave>, for example 'A4', 'C#5', or 'Db3'.")
    end

    return Note.new(letter, accidental, tonumber(octave_text))
end

function note_frequency.note_to_frequency(text)
    return note_frequency.parse_note(text):frequency()
end

note_frequency.Note = Note

return note_frequency
