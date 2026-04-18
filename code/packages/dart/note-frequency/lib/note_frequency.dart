import 'dart:math' as math;

const Map<String, int> _chromaticIndex = {
  'C': 0,
  'C#': 1,
  'Db': 1,
  'D': 2,
  'D#': 3,
  'Eb': 3,
  'E': 4,
  'F': 5,
  'F#': 6,
  'Gb': 6,
  'G': 7,
  'G#': 8,
  'Ab': 8,
  'A': 9,
  'A#': 10,
  'Bb': 10,
  'B': 11,
};

const int _referenceOctave = 4;
const int _referenceIndex = 9;
const double _referenceFrequencyHz = 440.0;
const int _semitonesPerOctave = 12;
final RegExp _notePattern = RegExp(r'^([A-Ga-g])([#b]?)(-?\d+)$');

class Note {
  Note(String letter, String accidental, int octave)
      : letter = letter.toUpperCase(),
        accidental = accidental,
        octave = octave {
    if (!_chromaticIndex.containsKey(spelling)) {
      throw ArgumentError(
        'Unsupported note spelling $spelling. Only natural notes plus single # or b accidentals are supported.',
      );
    }
  }

  final String letter;
  final String accidental;
  final int octave;

  String get spelling => '$letter$accidental';

  int chromaticIndex() => _chromaticIndex[spelling]!;

  int semitonesFromA4() {
    final octaveOffset = (octave - _referenceOctave) * _semitonesPerOctave;
    final pitchOffset = chromaticIndex() - _referenceIndex;
    return octaveOffset + pitchOffset;
  }

  double frequency() =>
      _referenceFrequencyHz * math.pow(2.0, semitonesFromA4() / _semitonesPerOctave).toDouble();

  @override
  String toString() => '$spelling$octave';
}

Note parseNote(String text) {
  final match = _notePattern.firstMatch(text);
  if (match == null) {
    throw ArgumentError(
      "Invalid note $text. Expected <letter><optional # or b><octave>, for example 'A4', 'C#5', or 'Db3'.",
    );
  }

  return Note(
    match.group(1)!,
    match.group(2)!,
    int.parse(match.group(3)!),
  );
}

double noteToFrequency(String text) => parseNote(text).frequency();
