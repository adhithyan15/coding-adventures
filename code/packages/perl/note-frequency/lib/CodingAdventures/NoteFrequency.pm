package CodingAdventures::NoteFrequency;

use strict;
use warnings;
use Exporter 'import';

our @EXPORT_OK = qw(parse_note note_to_frequency);
our $VERSION = '0.1.0';

my %CHROMATIC_INDEX = (
    'C' => 0,
    'C#' => 1,
    'Db' => 1,
    'D' => 2,
    'D#' => 3,
    'Eb' => 3,
    'E' => 4,
    'F' => 5,
    'F#' => 6,
    'Gb' => 6,
    'G' => 7,
    'G#' => 8,
    'Ab' => 8,
    'A' => 9,
    'A#' => 10,
    'Bb' => 10,
    'B' => 11,
);

use constant REFERENCE_OCTAVE => 4;
use constant REFERENCE_INDEX => 9;
use constant REFERENCE_FREQUENCY_HZ => 440.0;
use constant SEMITONES_PER_OCTAVE => 12;

sub parse_note {
    my ($text) = @_;
    if (!defined $text || $text !~ /\A([A-Ga-g])([#b]?)(-?\d+)\z/) {
        die "Invalid note '" . (defined $text ? $text : 'undef') . "'. Expected <letter><optional # or b><octave>, for example 'A4', 'C#5', or 'Db3'.";
    }

    return CodingAdventures::NoteFrequency::Note->new(
        letter => $1,
        accidental => $2,
        octave => $3,
    );
}

sub note_to_frequency {
    my ($text) = @_;
    return parse_note($text)->frequency;
}

package CodingAdventures::NoteFrequency::Note;

use strict;
use warnings;

sub new {
    my ($class, %args) = @_;
    my $letter = uc($args{letter});
    my $accidental = $args{accidental} // '';
    my $octave = int($args{octave});
    my $spelling = $letter . $accidental;

    if (!exists $CHROMATIC_INDEX{$spelling}) {
        die "Unsupported note spelling '$spelling'. Only natural notes plus single # or b accidentals are supported.";
    }

    return bless {
        letter => $letter,
        accidental => $accidental,
        octave => $octave,
    }, $class;
}

sub letter { return $_[0]->{letter}; }
sub accidental { return $_[0]->{accidental}; }
sub octave { return $_[0]->{octave}; }
sub spelling { return $_[0]->{letter} . $_[0]->{accidental}; }
sub chromatic_index { return $CHROMATIC_INDEX{ $_[0]->spelling }; }

sub semitones_from_a4 {
    my ($self) = @_;
    my $octave_offset = ($self->octave - CodingAdventures::NoteFrequency::REFERENCE_OCTAVE()) *
        CodingAdventures::NoteFrequency::SEMITONES_PER_OCTAVE();
    my $pitch_offset = $self->chromatic_index - CodingAdventures::NoteFrequency::REFERENCE_INDEX();
    return $octave_offset + $pitch_offset;
}

sub frequency {
    my ($self) = @_;
    return CodingAdventures::NoteFrequency::REFERENCE_FREQUENCY_HZ() *
        (2 ** ($self->semitones_from_a4 / CodingAdventures::NoteFrequency::SEMITONES_PER_OCTAVE()));
}

1;
