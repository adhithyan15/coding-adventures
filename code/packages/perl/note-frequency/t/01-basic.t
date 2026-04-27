use strict;
use warnings;
use Test::More;

use CodingAdventures::NoteFrequency qw(parse_note note_to_frequency);

my $note = parse_note('C#5');
is($note->letter, 'C', 'letter parsed');
is($note->accidental, '#', 'accidental parsed');
is($note->octave, 5, 'octave parsed');
is(parse_note('g4')->spelling . parse_note('g4')->octave, 'G4', 'lowercase normalized');

for my $value ('', 'A', 'H4', '#4', '4A', 'A##4', 'Bb') {
    my $ok = eval { parse_note($value); 1 };
    my $error = $@;
    ok(!$ok, "rejects malformed note $value");
    like($error, qr/Invalid note/, "malformed note $value explains the shape");
}

my $unsupported_ok = eval {
    CodingAdventures::NoteFrequency::Note->new(letter => 'E', accidental => '#', octave => 4);
    1;
};
my $unsupported_error = $@;
ok(!$unsupported_ok, 'rejects unsupported spellings');
like($unsupported_error, qr/Unsupported note spelling/, 'unsupported spelling explains the failure');

is(parse_note('A4')->semitones_from_a4, 0, 'A4 offset');
is(parse_note('A5')->semitones_from_a4, 12, 'A5 offset');
is(parse_note('A3')->semitones_from_a4, -12, 'A3 offset');
is(parse_note('C4')->semitones_from_a4, -9, 'C4 offset');
ok(abs(parse_note('A4')->frequency - 440.0) < 1e-12, 'A4 frequency');
ok(abs(parse_note('A5')->frequency - 880.0) < 1e-12, 'A5 frequency');
ok(abs(parse_note('A3')->frequency - 220.0) < 1e-12, 'A3 frequency');
ok(abs(note_to_frequency('C4') - 261.6255653005986) < 1e-12, 'middle C frequency');
ok(abs(note_to_frequency('C#4') - note_to_frequency('Db4')) < 1e-12, 'enharmonic spellings');

done_testing;
