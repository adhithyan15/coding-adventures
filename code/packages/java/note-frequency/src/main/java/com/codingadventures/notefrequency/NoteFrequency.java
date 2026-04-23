package com.codingadventures.notefrequency;

import java.util.regex.Matcher;
import java.util.regex.Pattern;

public final class NoteFrequency {
    private static final Pattern NOTE_PATTERN = Pattern.compile("^([A-Ga-g])([#b]?)(-?\\d+)$");

    private NoteFrequency() {
    }

    public static Note parseNote(String text) {
        Matcher matcher = NOTE_PATTERN.matcher(text);
        if (!matcher.matches()) {
            throw new IllegalArgumentException(
                "Invalid note " + text + ". Expected <letter><optional # or b><octave>, for example 'A4', 'C#5', or 'Db3'."
            );
        }
        return new Note(matcher.group(1), matcher.group(2), Integer.parseInt(matcher.group(3)));
    }

    public static double noteToFrequency(String text) {
        return parseNote(text).frequency();
    }
}
