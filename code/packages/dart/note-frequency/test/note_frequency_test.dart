import 'package:test/test.dart';
import 'package:coding_adventures_note_frequency/note_frequency.dart';

void main() {
  test('parseNote extracts fields', () {
    final note = parseNote('C#5');
    expect(note.letter, 'C');
    expect(note.accidental, '#');
    expect(note.octave, 5);
  });

  test('lowercase letters are normalized', () {
    expect(parseNote('g4').toString(), 'G4');
  });

  test('malformed notes throw', () {
    for (final value in ['', 'A', 'H4', '#4', '4A', 'A##4', 'Bb']) {
      expect(() => parseNote(value), throwsArgumentError);
    }
  });

  test('unsupported spellings throw', () {
    expect(() => Note('E', '#', 4), throwsArgumentError);
  });

  test('semitone offsets match examples', () {
    expect(parseNote('A4').semitonesFromA4(), 0);
    expect(parseNote('A5').semitonesFromA4(), 12);
    expect(parseNote('A3').semitonesFromA4(), -12);
    expect(parseNote('C4').semitonesFromA4(), -9);
  });

  test('frequencies match examples', () {
    expect(parseNote('A4').frequency(), closeTo(440.0, 1e-12));
    expect(parseNote('A5').frequency(), closeTo(880.0, 1e-12));
    expect(parseNote('A3').frequency(), closeTo(220.0, 1e-12));
    expect(noteToFrequency('C4'), closeTo(261.6255653005986, 1e-12));
    expect(noteToFrequency('C#4'), closeTo(noteToFrequency('Db4'), 1e-12));
  });
}
