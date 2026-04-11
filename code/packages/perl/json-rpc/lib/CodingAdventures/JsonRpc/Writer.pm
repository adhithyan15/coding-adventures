package CodingAdventures::JsonRpc::Writer;

# ============================================================================
# CodingAdventures::JsonRpc::Writer — Content-Length-framed message writer
# ============================================================================
#
# Writes one JSON-RPC 2.0 message at a time to a filehandle.
#
# # The framing format
#
# Every message is preceded by a header block followed by a blank line:
#
#     Content-Length: <n>\r\n
#     \r\n
#     <UTF-8 JSON payload, exactly n bytes>
#
# where `n` is the **byte** length of the UTF-8-encoded JSON string.
#
# This is different from the character count when the JSON contains multi-byte
# UTF-8 sequences (e.g., non-ASCII identifiers or string values).  In Perl,
# `length($str)` returns the byte count for a byte string (not upgraded to
# UTF-8), and `bytes::length($str)` gives bytes regardless of the internal
# representation.  We use `use bytes` locally to guarantee byte counting.
#
# # Why \r\n?
#
# The LSP spec mandates \r\n line endings in the header, following HTTP/1.1.
# Some clients also accept \n alone, but we always send \r\n for
# interoperability.
#
# # Flushing
#
# After writing each message we call C<$fh->flush()> if the handle supports
# it.  This ensures that buffered I/O does not hold back a response after a
# request has been processed.  For C<\*STDOUT> the caller should also run
# C<binmode(STDOUT, ':raw')> and optionally C<$| = 1> for auto-flush.

use strict;
use warnings;

use CodingAdventures::JsonRpc::Message qw(message_to_json);

our $VERSION = '0.01';

# ---------------------------------------------------------------------------
# new($fh) → MessageWriter
#
# Create a new writer wrapping the given filehandle.
# ---------------------------------------------------------------------------

sub new {
    my ($class, $fh) = @_;
    return bless { fh => $fh }, $class;
}

# ---------------------------------------------------------------------------
# write_raw($json_string)
#
# Frame the given JSON string with a Content-Length header and write
# everything to the filehandle.
#
# The Content-Length value is the byte count of $json_string.  We compute
# this with `use bytes` in a lexical scope so we do not change the global
# character-vs-byte semantics for the rest of the program.
# ---------------------------------------------------------------------------

sub write_raw {
    my ($self, $json_str) = @_;
    my $fh = $self->{fh};

    # Compute the byte length of the payload.
    # `use bytes` makes length() count raw bytes instead of characters.
    my $byte_len;
    {
        use bytes;
        $byte_len = length($json_str);
    }

    # Write the header.  The separator between header and body is \r\n\r\n:
    # one \r\n at the end of the Content-Length line plus one blank \r\n line.
    print $fh "Content-Length: $byte_len\r\n\r\n$json_str";

    # Flush so the client receives the full message immediately.
    # We use eval to silently ignore flush failures on handles that do not
    # support it (e.g., in-memory scalar references used in tests).
    eval { $fh->flush } if ref($fh) && $fh->can('flush');
}

# ---------------------------------------------------------------------------
# write_message($hashref)
#
# Encode a message hashref to JSON and write it with Content-Length framing.
# Strips the internal `_type` key before encoding.
# ---------------------------------------------------------------------------

sub write_message {
    my ($self, $msg) = @_;
    my $json_str = message_to_json($msg);
    $self->write_raw($json_str);
}

1;

__END__

=head1 NAME

CodingAdventures::JsonRpc::Writer — Content-Length-framed JSON-RPC message writer

=head1 SYNOPSIS

  use CodingAdventures::JsonRpc::Writer;

  open my $fh, '>', \my $buffer;
  binmode($fh, ':raw');
  my $writer = CodingAdventures::JsonRpc::Writer->new($fh);

  $writer->write_message({ jsonrpc => '2.0', id => 1, result => {} });
  $writer->write_raw('{"jsonrpc":"2.0","id":1,"result":{}}');

=head1 DESCRIPTION

Writes Content-Length-framed JSON-RPC 2.0 messages to a binary filehandle.

=head1 METHODS

=over 4

=item new($fh)

Create a writer.  The filehandle should be in binary mode.

=item write_message($hashref)

Encode C<$hashref> as JSON, add Content-Length framing, and write to the handle.
The internal C<_type> key (if present) is stripped before encoding.

=item write_raw($json_string)

Write a pre-encoded JSON string with Content-Length framing.

=back

=cut
