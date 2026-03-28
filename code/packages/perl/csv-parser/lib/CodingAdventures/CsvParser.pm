package CodingAdventures::CsvParser;

# ============================================================================
# CodingAdventures::CsvParser — RFC 4180 state-machine CSV parser
# ============================================================================
#
# This module implements a CSV parser based on the four-state machine
# described in RFC 4180. CSV (Comma-Separated Values) is deceptively
# complex: fields may be quoted, quotes may be escaped by doubling them,
# and line endings may be \r\n, \n, or bare \r.
#
# The four states of the machine:
#
#   FIELD_START       — we are at the beginning of a new field
#   IN_UNQUOTED_FIELD — we are inside a plain field (no quotes)
#   IN_QUOTED_FIELD   — we are inside a "double-quoted" field
#   IN_QUOTED_MAYBE_END — we just saw a closing " inside a quoted field;
#                         is it the end, or is the next char another " (escape)?
#
# State-transition diagram:
#
#   FIELD_START ──────── " ──────────────► IN_QUOTED_FIELD
#   FIELD_START ──────── delimiter ──────► FIELD_START         (emit empty field)
#   FIELD_START ──────── \n / \r\n ───────► FIELD_START         (emit row)
#   FIELD_START ──────── other char ─────► IN_UNQUOTED_FIELD
#
#   IN_UNQUOTED_FIELD ── delimiter ──────► FIELD_START         (emit field)
#   IN_UNQUOTED_FIELD ── \n / \r\n ───────► FIELD_START         (emit field+row)
#   IN_UNQUOTED_FIELD ── other char ─────► IN_UNQUOTED_FIELD   (accumulate)
#
#   IN_QUOTED_FIELD ──── " ──────────────► IN_QUOTED_MAYBE_END
#   IN_QUOTED_FIELD ──── other char ─────► IN_QUOTED_FIELD     (accumulate)
#
#   IN_QUOTED_MAYBE_END ─ " ─────────────► IN_QUOTED_FIELD     (emit literal ")
#   IN_QUOTED_MAYBE_END ─ delimiter ─────► FIELD_START         (emit field)
#   IN_QUOTED_MAYBE_END ─ \n / \r\n ──────► FIELD_START         (emit field+row)
#   IN_QUOTED_MAYBE_END ─ EOF ───────────► done                (emit field+row)
#
# This module is part of the coding-adventures project, an educational
# computing stack built from logic gates up through interpreters and
# compilers.

use strict;
use warnings;

our $VERSION = '0.01';

# ---------------------------------------------------------------------------
# State constants — using plain integers for clarity
# ---------------------------------------------------------------------------
use constant {
    FIELD_START         => 0,  # Waiting for the first character of a new field
    IN_UNQUOTED_FIELD   => 1,  # Inside a field that did NOT start with a quote
    IN_QUOTED_FIELD     => 2,  # Inside a "quoted" field
    IN_QUOTED_MAYBE_END => 3,  # Saw a closing quote; is this the end or ""?
};

# ---------------------------------------------------------------------------
# new(\%opts)
#
# Constructor. Accepts an optional hashref with keys:
#   delimiter => ","   (default is comma)
#
# Example:
#   my $p = CodingAdventures::CsvParser->new();
#   my $p = CodingAdventures::CsvParser->new({ delimiter => ";" });
# ---------------------------------------------------------------------------
sub new {
    my ($class, $opts) = @_;
    $opts //= {};
    my $self = {
        delimiter => $opts->{delimiter} // ',',
    };
    return bless $self, $class;
}

# ---------------------------------------------------------------------------
# parse($text, \%opts) -> \@rows
#
# Parse a complete CSV text and return an arrayref of arrayrefs. Each inner
# arrayref is one row; each element of that arrayref is one field (string).
#
# $opts is optional and may override the instance delimiter:
#   delimiter => ";"
#
# Handles:
#   - \r\n  (Windows line endings)
#   - \n    (Unix line endings)
#   - \r    (old Mac line endings)
#   - Quoted fields: "hello, world" => one field
#   - Doubled quotes inside quoted fields: "say ""hi"" now" => say "hi" now
#   - Empty fields: a,,b => ("a", "", "b")
#   - Empty quoted fields: "","" => ("", "")
# ---------------------------------------------------------------------------
sub parse {
    my ($self, $text, $opts) = @_;
    $opts //= {};

    # Allow per-call delimiter override
    my $delim = $opts->{delimiter} // $self->{delimiter};

    # The result: an arrayref of rows; each row is an arrayref of fields
    my @rows;

    # The current row being assembled
    my @current_row;

    # The current field being assembled (as a list of characters, joined at emit)
    my $field = '';

    # Current state of the state machine
    my $state = FIELD_START;

    # Split input into individual characters. We process one character at a
    # time so the state machine is explicit and easy to follow.
    my @chars = split //, $text;
    my $i     = 0;
    my $len   = scalar @chars;

    while ( $i < $len ) {
        my $ch = $chars[$i];

        # ------------------------------------------------------------------
        # Handle \r\n as a single logical newline — look ahead one character
        # when we see \r.
        # ------------------------------------------------------------------
        my $is_newline = 0;
        if ( $ch eq "\r" ) {
            $is_newline = 1;
            # Consume the following \n if present
            if ( $i + 1 < $len && $chars[ $i + 1 ] eq "\n" ) {
                $i++;
            }
        }
        elsif ( $ch eq "\n" ) {
            $is_newline = 1;
        }

        # ------------------------------------------------------------------
        # State machine transitions
        # ------------------------------------------------------------------
        if ( $state == FIELD_START ) {
            if ($is_newline) {
                # An empty line or the end of a row that ended with a delimiter.
                # Emit the (empty) field and the row.
                push @current_row, $field;
                push @rows, [@current_row];
                @current_row = ();
                $field       = '';
                # Stay in FIELD_START
            }
            elsif ( $ch eq $delim ) {
                # Empty field before this delimiter
                push @current_row, $field;
                $field = '';
                # Stay in FIELD_START
            }
            elsif ( $ch eq '"' ) {
                # Start of a quoted field
                $state = IN_QUOTED_FIELD;
            }
            else {
                # First character of an unquoted field
                $field .= $ch;
                $state = IN_UNQUOTED_FIELD;
            }
        }
        elsif ( $state == IN_UNQUOTED_FIELD ) {
            if ($is_newline) {
                push @current_row, $field;
                push @rows, [@current_row];
                @current_row = ();
                $field       = '';
                $state       = FIELD_START;
            }
            elsif ( $ch eq $delim ) {
                push @current_row, $field;
                $field = '';
                $state = FIELD_START;
            }
            else {
                $field .= $ch;
                # Stay in IN_UNQUOTED_FIELD
            }
        }
        elsif ( $state == IN_QUOTED_FIELD ) {
            if ( $ch eq '"' ) {
                # Might be end of field, or might be "" escape
                $state = IN_QUOTED_MAYBE_END;
            }
            else {
                # Any other character, including newlines, is literal inside quotes
                if ($is_newline) {
                    # Preserve the newline as-is inside a quoted field.
                    # We already consumed any \r above; emit \n.
                    $field .= "\n";
                }
                else {
                    $field .= $ch;
                }
                # Stay in IN_QUOTED_FIELD
            }
        }
        elsif ( $state == IN_QUOTED_MAYBE_END ) {
            if ( $ch eq '"' ) {
                # "" inside a quoted field means a literal double-quote
                $field .= '"';
                $state = IN_QUOTED_FIELD;
            }
            elsif ($is_newline) {
                # The closing quote was genuine; this newline ends the row
                push @current_row, $field;
                push @rows, [@current_row];
                @current_row = ();
                $field       = '';
                $state       = FIELD_START;
            }
            elsif ( $ch eq $delim ) {
                # The closing quote was genuine; this delimiter starts the next field
                push @current_row, $field;
                $field = '';
                $state = FIELD_START;
            }
            else {
                # Technically malformed CSV (quote not at end), but we are
                # permissive: treat the quote as part of the field value.
                $field .= '"' . $ch;
                $state = IN_UNQUOTED_FIELD;
            }
        }

        $i++;
    }

    # ------------------------------------------------------------------
    # End-of-input: flush whatever is in the current field/row
    # ------------------------------------------------------------------
    # A file may or may not end with a newline. If there is content in the
    # current row (or even just one field started), emit the last row.
    if ( $state == IN_QUOTED_MAYBE_END ) {
        # The final " closed a quoted field; flush
        push @current_row, $field;
        push @rows, [@current_row];
    }
    elsif ( @current_row || length($field) || $state == IN_UNQUOTED_FIELD ) {
        # There is an unterminated final row
        push @current_row, $field;
        push @rows, [@current_row];
    }

    return \@rows;
}

1;

__END__

=head1 NAME

CodingAdventures::CsvParser - RFC 4180 state-machine CSV parser with quoted field support

=head1 SYNOPSIS

    use CodingAdventures::CsvParser;

    my $parser = CodingAdventures::CsvParser->new();
    my $rows   = $parser->parse("a,b,c\n1,2,3");
    # $rows = [["a","b","c"],["1","2","3"]]

    # Custom delimiter
    my $tsv = CodingAdventures::CsvParser->new({ delimiter => "\t" });
    my $rows = $tsv->parse("a\tb\tc");

    # Per-call delimiter override
    my $rows = $parser->parse("a;b;c", { delimiter => ";" });

=head1 DESCRIPTION

A pure-Perl, dependency-free CSV parser that implements the RFC 4180
standard using an explicit four-state finite automaton. It correctly
handles quoted fields, doubled-quote escapes, CRLF/LF/CR line endings,
empty fields, and newlines embedded inside quoted fields.

=head1 METHODS

=head2 new(\%opts)

Create a new parser instance. Optional keys:

=over 4

=item delimiter

The field separator character. Defaults to C<",">.

=back

=head2 parse($text, \%opts) -> \@rows

Parse CSV text and return an arrayref of arrayrefs. Each inner arrayref
is one row. Optional C<\%opts> hash may specify C<delimiter> to override
the instance default for this call.

=head1 VERSION

Version 0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
