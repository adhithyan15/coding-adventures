package notefrequency

import "testing"

func approxEqual(got float64, want float64) bool {
	difference := got - want
	if difference < 0 {
		difference = -difference
	}
	return difference < 1e-12
}

func TestParseNote(t *testing.T) {
	note, err := ParseNote("C#5")
	if err != nil {
		t.Fatalf("ParseNote returned error: %v", err)
	}
	if note.Letter != "C" || note.Accidental != "#" || note.Octave != 5 {
		t.Fatalf("unexpected note %#v", note)
	}
}

func TestParseNoteNormalizesLowercase(t *testing.T) {
	note, err := ParseNote("g4")
	if err != nil {
		t.Fatalf("ParseNote returned error: %v", err)
	}
	if note.String() != "G4" {
		t.Fatalf("expected G4, got %s", note.String())
	}
}

func TestParseNoteRejectsMalformedInput(t *testing.T) {
	for _, value := range []string{"", "A", "H4", "#4", "4A", "A##4", "Bb"} {
		if _, err := ParseNote(value); err == nil {
			t.Fatalf("expected ParseNote(%q) to fail", value)
		}
	}
}

func TestNewNoteRejectsUnsupportedSpelling(t *testing.T) {
	if _, err := NewNote("E", "#", 4); err == nil {
		t.Fatal("expected unsupported spelling to fail")
	}
}

func TestSemitonesFromA4(t *testing.T) {
	cases := map[string]int{"A4": 0, "A5": 12, "A3": -12, "C4": -9}
	for input, want := range cases {
		note, err := ParseNote(input)
		if err != nil {
			t.Fatalf("ParseNote(%q) returned error: %v", input, err)
		}
		if got := note.SemitonesFromA4(); got != want {
			t.Fatalf("expected %q to be %d semitones from A4, got %d", input, want, got)
		}
	}
}

func TestFrequencyMapping(t *testing.T) {
	cases := map[string]float64{"A4": 440.0, "A5": 880.0, "A3": 220.0}
	for input, want := range cases {
		note, err := ParseNote(input)
		if err != nil {
			t.Fatalf("ParseNote(%q) returned error: %v", input, err)
		}
		if got := note.Frequency(); !approxEqual(got, want) {
			t.Fatalf("expected %q to map to %.12f Hz, got %.12f Hz", input, want, got)
		}
	}
}

func TestMiddleCFrequency(t *testing.T) {
	got, err := NoteToFrequency("C4")
	if err != nil {
		t.Fatalf("NoteToFrequency returned error: %v", err)
	}
	want := 261.6255653005986
	if !approxEqual(got, want) {
		t.Fatalf("expected C4 to map to %.12f Hz, got %.12f Hz", want, got)
	}
}

func TestEnharmonicSpellingsMatch(t *testing.T) {
	sharp, err := NoteToFrequency("C#4")
	if err != nil {
		t.Fatalf("sharp spelling failed: %v", err)
	}
	flat, err := NoteToFrequency("Db4")
	if err != nil {
		t.Fatalf("flat spelling failed: %v", err)
	}
	if !approxEqual(sharp, flat) {
		t.Fatalf("expected enharmonic spellings to match, got %.12f and %.12f", sharp, flat)
	}
}
