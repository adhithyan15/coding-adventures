package com.codingadventures.notefrequency

private val chromaticIndexMap = mapOf(
    "C" to 0,
    "C#" to 1,
    "Db" to 1,
    "D" to 2,
    "D#" to 3,
    "Eb" to 3,
    "E" to 4,
    "F" to 5,
    "F#" to 6,
    "Gb" to 6,
    "G" to 7,
    "G#" to 8,
    "Ab" to 8,
    "A" to 9,
    "A#" to 10,
    "Bb" to 10,
    "B" to 11,
)

private const val REFERENCE_OCTAVE = 4
private const val REFERENCE_INDEX = 9
private const val REFERENCE_FREQUENCY_HZ = 440.0
private const val SEMITONES_PER_OCTAVE = 12

data class Note(private val rawLetter: String, val accidental: String = "", val octave: Int) {
    val letter: String = rawLetter.uppercase()
    val spelling: String = letter + accidental

    init {
        require(chromaticIndexMap.containsKey(spelling)) {
            "Unsupported note spelling $spelling. Only natural notes plus single # or b accidentals are supported."
        }
    }

    fun chromaticIndex(): Int = chromaticIndexMap.getValue(spelling)

    fun semitonesFromA4(): Int {
        val octaveOffset = (octave - REFERENCE_OCTAVE) * SEMITONES_PER_OCTAVE
        val pitchOffset = chromaticIndex() - REFERENCE_INDEX
        return octaveOffset + pitchOffset
    }

    fun frequency(): Double = REFERENCE_FREQUENCY_HZ * Math.pow(2.0, semitonesFromA4().toDouble() / SEMITONES_PER_OCTAVE)

    override fun toString(): String = "$spelling$octave"
}

object NoteFrequency {
    private val notePattern = Regex("^([A-Ga-g])([#b]?)(-?\\d+)$")

    fun parseNote(text: String): Note {
        val match = notePattern.matchEntire(text)
            ?: throw IllegalArgumentException(
                "Invalid note $text. Expected <letter><optional # or b><octave>, for example 'A4', 'C#5', or 'Db3'."
            )
        val (letter, accidental, octaveText) = match.destructured
        return Note(letter, accidental, octaveText.toInt())
    }

    fun noteToFrequency(text: String): Double = parseNote(text).frequency()
}
