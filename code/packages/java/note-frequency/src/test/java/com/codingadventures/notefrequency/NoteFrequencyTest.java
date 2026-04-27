package com.codingadventures.notefrequency;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;

import org.junit.jupiter.api.Test;

class NoteFrequencyTest {
    @Test
    void parseNoteExtractsFields() {
        Note note = NoteFrequency.parseNote("C#5");
        assertEquals("C", note.letter());
        assertEquals("#", note.accidental());
        assertEquals(5, note.octave());
    }

    @Test
    void lowercaseLettersAreNormalized() {
        assertEquals("G4", NoteFrequency.parseNote("g4").toString());
    }

    @Test
    void malformedNotesThrow() {
        for (String value : new String[] {"", "A", "H4", "#4", "4A", "A##4", "Bb"}) {
            assertThrows(IllegalArgumentException.class, () -> NoteFrequency.parseNote(value));
        }
    }

    @Test
    void unsupportedSpellingsThrow() {
        assertThrows(IllegalArgumentException.class, () -> new Note("E", "#", 4));
    }

    @Test
    void semitoneOffsetsMatchExamples() {
        assertEquals(0, NoteFrequency.parseNote("A4").semitonesFromA4());
        assertEquals(12, NoteFrequency.parseNote("A5").semitonesFromA4());
        assertEquals(-12, NoteFrequency.parseNote("A3").semitonesFromA4());
        assertEquals(-9, NoteFrequency.parseNote("C4").semitonesFromA4());
    }

    @Test
    void frequenciesMatchExamples() {
        assertEquals(440.0, NoteFrequency.parseNote("A4").frequency(), 1.0e-12);
        assertEquals(880.0, NoteFrequency.parseNote("A5").frequency(), 1.0e-12);
        assertEquals(220.0, NoteFrequency.parseNote("A3").frequency(), 1.0e-12);
        assertEquals(261.6255653005986, NoteFrequency.noteToFrequency("C4"), 1.0e-12);
        assertEquals(NoteFrequency.noteToFrequency("C#4"), NoteFrequency.noteToFrequency("Db4"), 1.0e-12);
    }
}
