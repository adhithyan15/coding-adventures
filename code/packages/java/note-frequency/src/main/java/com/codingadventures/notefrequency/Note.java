package com.codingadventures.notefrequency;

import java.util.Map;

public final class Note {
    private static final Map<String, Integer> CHROMATIC_INDEX = Map.ofEntries(
        Map.entry("C", 0),
        Map.entry("C#", 1),
        Map.entry("Db", 1),
        Map.entry("D", 2),
        Map.entry("D#", 3),
        Map.entry("Eb", 3),
        Map.entry("E", 4),
        Map.entry("F", 5),
        Map.entry("F#", 6),
        Map.entry("Gb", 6),
        Map.entry("G", 7),
        Map.entry("G#", 8),
        Map.entry("Ab", 8),
        Map.entry("A", 9),
        Map.entry("A#", 10),
        Map.entry("Bb", 10),
        Map.entry("B", 11)
    );

    private static final int REFERENCE_OCTAVE = 4;
    private static final int REFERENCE_INDEX = 9;
    private static final double REFERENCE_FREQUENCY_HZ = 440.0;
    private static final int SEMITONES_PER_OCTAVE = 12;

    private final String letter;
    private final String accidental;
    private final int octave;

    public Note(String letter, String accidental, int octave) {
        String canonicalLetter = letter.toUpperCase();
        String spelling = canonicalLetter + accidental;
        if (!CHROMATIC_INDEX.containsKey(spelling)) {
            throw new IllegalArgumentException(
                "Unsupported note spelling " + spelling + ". Only natural notes plus single # or b accidentals are supported."
            );
        }
        this.letter = canonicalLetter;
        this.accidental = accidental;
        this.octave = octave;
    }

    public String letter() { return letter; }
    public String accidental() { return accidental; }
    public int octave() { return octave; }
    public String spelling() { return letter + accidental; }
    public int chromaticIndex() { return CHROMATIC_INDEX.get(spelling()); }

    public int semitonesFromA4() {
        int octaveOffset = (octave - REFERENCE_OCTAVE) * SEMITONES_PER_OCTAVE;
        int pitchOffset = chromaticIndex() - REFERENCE_INDEX;
        return octaveOffset + pitchOffset;
    }

    public double frequency() {
        return REFERENCE_FREQUENCY_HZ * Math.pow(2.0, semitonesFromA4() / (double) SEMITONES_PER_OCTAVE);
    }

    @Override
    public String toString() { return spelling() + octave; }
}
