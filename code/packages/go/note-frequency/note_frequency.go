package notefrequency

import (
	"fmt"
	"math"
	"regexp"
	"strconv"
	"strings"
)

var notePattern = regexp.MustCompile(`^([A-Ga-g])([#b]?)(-?\d+)$`)

var chromaticIndex = map[string]int{
	"C":  0,
	"C#": 1,
	"Db": 1,
	"D":  2,
	"D#": 3,
	"Eb": 3,
	"E":  4,
	"F":  5,
	"F#": 6,
	"Gb": 6,
	"G":  7,
	"G#": 8,
	"Ab": 8,
	"A":  9,
	"A#": 10,
	"Bb": 10,
	"B":  11,
}

const (
	referenceOctave      = 4
	referenceFrequencyHz = 440.0
	semitonesPerOctave   = 12
)

var referenceIndex = chromaticIndex["A"]

type Note struct {
	Letter     string
	Accidental string
	Octave     int
}

func NewNote(letter string, accidental string, octave int) (Note, error) {
	canonicalLetter := strings.ToUpper(letter)
	spelling := canonicalLetter + accidental
	if _, ok := chromaticIndex[spelling]; !ok {
		return Note{}, fmt.Errorf(
			"unsupported note spelling %q. only natural notes plus single # or b accidentals are supported",
			spelling,
		)
	}

	return Note{Letter: canonicalLetter, Accidental: accidental, Octave: octave}, nil
}

func (n Note) Spelling() string    { return n.Letter + n.Accidental }
func (n Note) ChromaticIndex() int { return chromaticIndex[n.Spelling()] }

func (n Note) SemitonesFromA4() int {
	octaveOffset := (n.Octave - referenceOctave) * semitonesPerOctave
	pitchOffset := n.ChromaticIndex() - referenceIndex
	return octaveOffset + pitchOffset
}

func (n Note) Frequency() float64 {
	return referenceFrequencyHz * math.Pow(2, float64(n.SemitonesFromA4())/semitonesPerOctave)
}

func (n Note) String() string { return fmt.Sprintf("%s%d", n.Spelling(), n.Octave) }

func ParseNote(text string) (Note, error) {
	match := notePattern.FindStringSubmatch(text)
	if match == nil {
		return Note{}, fmt.Errorf(
			"invalid note %q. expected <letter><optional # or b><octave>, for example 'A4', 'C#5', or 'Db3'",
			text,
		)
	}

	octave, err := strconv.Atoi(match[3])
	if err != nil {
		return Note{}, fmt.Errorf("invalid note %q. octave must be an integer", text)
	}

	return NewNote(match[1], match[2], octave)
}

func NoteToFrequency(text string) (float64, error) {
	note, err := ParseNote(text)
	if err != nil {
		return 0, err
	}
	return note.Frequency(), nil
}
