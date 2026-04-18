package com.codingadventures.notefrequency

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith

class NoteFrequencyTest {
    @Test
    fun `parseNote extracts fields`() {
        val note = NoteFrequency.parseNote("C#5")
        assertEquals("C", note.letter)
        assertEquals("#", note.accidental)
        assertEquals(5, note.octave)
    }

    @Test
    fun `lowercase letters are normalized`() {
        assertEquals("G4", NoteFrequency.parseNote("g4").toString())
    }

    @Test
    fun `malformed notes throw`() {
        listOf("", "A", "H4", "#4", "4A", "A##4", "Bb").forEach { value ->
            assertFailsWith<IllegalArgumentException> { NoteFrequency.parseNote(value) }
        }
    }

    @Test
    fun `unsupported spellings throw`() {
        assertFailsWith<IllegalArgumentException> { Note("E", "#", 4) }
    }

    @Test
    fun `semitone offsets match examples`() {
        assertEquals(0, NoteFrequency.parseNote("A4").semitonesFromA4())
        assertEquals(12, NoteFrequency.parseNote("A5").semitonesFromA4())
        assertEquals(-12, NoteFrequency.parseNote("A3").semitonesFromA4())
        assertEquals(-9, NoteFrequency.parseNote("C4").semitonesFromA4())
    }

    @Test
    fun `frequencies match examples`() {
        assertEquals(440.0, NoteFrequency.parseNote("A4").frequency(), 1.0e-12)
        assertEquals(880.0, NoteFrequency.parseNote("A5").frequency(), 1.0e-12)
        assertEquals(220.0, NoteFrequency.parseNote("A3").frequency(), 1.0e-12)
        assertEquals(261.6255653005986, NoteFrequency.noteToFrequency("C4"), 1.0e-12)
        assertEquals(NoteFrequency.noteToFrequency("C#4"), NoteFrequency.noteToFrequency("Db4"), 1.0e-12)
    }
}
