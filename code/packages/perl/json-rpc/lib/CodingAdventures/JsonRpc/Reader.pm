package CodingAdventures::JsonRpc::Reader;

# ============================================================================
# CodingAdventures::JsonRpc::Reader — Content-Length-framed message reader
# ============================================================================
#
# Reads one JSON-RPC 2.0 message at a time from a filehandle.
#
# # The framing protocol
#
# LSP (and JSON-RPC 2.0 over stdio) uses HTTP-style headers to delimit
# messages on the byte stream.  Each message looks like:
#
#     Content-Length: 97\r\n
#     \r\n
#     {"jsonrpc":"2.0","id":1,"method":"textDocument/hover","params":{...}}
#
# The reader:
#   1. Reads header lines until it finds a blank line (the end of headers).
#   2. Extracts the `Content-Length` value.
#   3. Reads exactly that many bytes as the JSON payload.
#   4. Decodes and validates the JSON.
#
# # Return convention for read_message / read_raw
#
# Functions return a two-element list:
#
#   ($value, undef)   — success
#   (undef, undef)    — clean EOF (no bytes read yet when the stream ended)
#   (undef, $error)   — error string describing what went wrong
#
# This two-value convention avoids exceptions for predictable conditions
# (EOF, bad input) while still propagating them as strings the caller can
# inspect and act on.
#
# # Filehandle requirements
#
# The filehandle must be opened in binary (`:raw`) mode so that Perl does
# not perform newline translation on Windows.  The header lines are
# terminated with \r\n per the spec; binary mode preserves the \r so we
# can strip it explicitly.

use strict;
use warnings;

use CodingAdventures::JsonRpc::Message qw(parse_message);
use CodingAdventures::JsonRpc::Errors  qw(:all);

our $VERSION = '0.01';

# ---------------------------------------------------------------------------
# new($fh) → MessageReader
#
# Create a new reader wrapping the given filehandle.
# The caller is responsible for opening the filehandle in binary mode:
#
#   binmode($fh, ':raw');
#   my $reader = CodingAdventures::JsonRpc::Reader->new($fh);
# ---------------------------------------------------------------------------

sub new {
    my ($class, $fh) = @_;
    return bless { fh => $fh }, $class;
}

# ---------------------------------------------------------------------------
# _read_content_length() → ($length, undef) | (undef, undef) | (undef, $err)
#
# Internal helper: read header lines until the blank-line separator, then
# return the Content-Length value.
#
# Header lines are separated from each other by \r\n, and the header block
# ends with a blank \r\n line (i.e., \r\n\r\n in total).  We use readline
# (the <> operator) to read one line at a time; because the filehandle is
# in binary mode, the \r is preserved and we strip it ourselves.
# ---------------------------------------------------------------------------

sub _read_content_length {
    my ($self) = @_;
    my $fh = $self->{fh};
    my $content_length;
    my $first_line = 1;    # track whether we've read any bytes at all

    while (1) {
        my $line = readline($fh);

        # readline returns undef on EOF.
        if (!defined $line) {
            if ($first_line) {
                # Clean EOF: the stream ended before any message started.
                return (undef, undef);
            }
            return (undef, "unexpected EOF while reading headers");
        }
        $first_line = 0;

        # Strip trailing \r\n or \n.  In binary mode readline stops at \n
        # but does NOT strip \r.
        $line =~ s/\r?\n$//;

        # An empty line marks the end of the header block.
        if ($line eq '') {
            if (defined $content_length) {
                return ($content_length, undef);
            }
            return (undef, "header block ended without Content-Length");
        }

        # Parse "Header-Name: value".  Only Content-Length matters to us;
        # other headers (e.g., Content-Type) are silently ignored.
        if ($line =~ /^Content-Length\s*:\s*(\d+)\s*$/i) {
            $content_length = int($1);
        }
        # Any other header line is simply discarded.
    }
}

# ---------------------------------------------------------------------------
# read_raw() → ($json_string, undef) | (undef, undef) | (undef, $error)
#
# Read one framed message and return the raw JSON payload as a string.
# Does NOT decode or validate the JSON.
# ---------------------------------------------------------------------------

sub read_raw {
    my ($self) = @_;
    my $fh = $self->{fh};

    my ($length, $err) = $self->_read_content_length;
    return (undef, $err) unless defined $length;

    # Edge case: a zero-byte payload is theoretically valid (though
    # meaningless for JSON-RPC).  read() with count 0 returns ''.
    if ($length == 0) {
        return ('', undef);
    }

    # Read exactly $length bytes.
    my $payload = '';
    my $remaining = $length;
    while ($remaining > 0) {
        my $chunk = '';
        my $n = read($fh, $chunk, $remaining);
        if (!defined $n) {
            return (undef, "read error: $!");
        }
        if ($n == 0) {
            return (undef, "unexpected EOF reading payload "
                . "(read " . ($length - $remaining) . " of $length bytes)");
        }
        $payload .= $chunk;
        $remaining -= $n;
    }

    return ($payload, undef);
}

# ---------------------------------------------------------------------------
# read_message() → ($hashref, undef) | (undef, undef) | (undef, $error)
#
# Read one framed message, decode the JSON, and validate it as a JSON-RPC
# 2.0 message.  Adds a `_type` key ('request', 'notification', 'response')
# to the returned hashref.
# ---------------------------------------------------------------------------

sub read_message {
    my ($self) = @_;

    my ($raw, $err) = $self->read_raw;
    return (undef, $err) if !defined $raw && defined $err;
    return (undef, undef) if !defined $raw;   # EOF

    # parse_message returns ($hashref, undef) or (undef, $error).
    return parse_message($raw);
}

1;

__END__

=head1 NAME

CodingAdventures::JsonRpc::Reader — Content-Length-framed JSON-RPC message reader

=head1 SYNOPSIS

  use CodingAdventures::JsonRpc::Reader;

  open my $fh, '<', \$buffer or die $!;
  binmode($fh, ':raw');
  my $reader = CodingAdventures::JsonRpc::Reader->new($fh);

  my ($msg, $err) = $reader->read_message;
  # $msg is a hashref with _type set to 'request', 'notification', or 'response'

=head1 DESCRIPTION

Reads one JSON-RPC 2.0 message per call from a binary filehandle.
Handles Content-Length header parsing and exact-byte payload reading.

=head1 METHODS

=over 4

=item new($fh)

Create a reader.  The filehandle should be opened with C<binmode($fh, ':raw')>.

=item read_message()

Returns C<($hashref, undef)> on success, C<(undef, undef)> on EOF,
C<(undef, $error)> on error.

=item read_raw()

Like C<read_message> but returns the raw JSON string without decoding.

=back

=cut
