package CodingAdventures::CompilerSourceMap::SourcePosition;

# ============================================================================
# SourcePosition — a span of characters in a source file
# ============================================================================
#
# Think of this as a "highlighter pen" marking a region of source code.
# The (line, column) pair marks the start; length tells you how many
# characters are highlighted.
#
# ## Brainfuck context
#
# Every Brainfuck command is exactly one character, so length = 1 always.
# For a future BASIC frontend, a keyword like "PRINT" would have length = 5.
#
# ## Fields
#
#   file    — source file path (e.g., "hello.bf")
#   line    — 1-based line number
#   column  — 1-based column number
#   length  — character span in source
#
# ## String representation
#
#   "hello.bf:1:3 (len=1)"
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

# new(%args) — create a source position.
sub new {
    my ($class, %args) = @_;
    return bless {
        file   => $args{file}   // '',
        line   => $args{line}   // 0,
        column => $args{column} // 0,
        length => $args{length} // 0,
    }, $class;
}

# to_string() — human-readable representation.
#
# Example: "hello.bf:1:3 (len=1)"
sub to_string {
    my ($self) = @_;
    return sprintf('%s:%d:%d (len=%d)',
        $self->{file}, $self->{line}, $self->{column}, $self->{length});
}

1;

__END__

=head1 NAME

CodingAdventures::CompilerSourceMap::SourcePosition - span of characters in a source file

=head1 SYNOPSIS

  my $pos = CodingAdventures::CompilerSourceMap::SourcePosition->new(
      file   => 'hello.bf',
      line   => 1,
      column => 3,
      length => 1,
  );
  print $pos->to_string;  # "hello.bf:1:3 (len=1)"

=head1 VERSION

0.01

=head1 LICENSE

MIT

=cut
