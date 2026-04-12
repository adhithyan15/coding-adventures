package CodingAdventures::Rpc::Framer;

# ============================================================================
# CodingAdventures::Rpc::Framer — Interface contract for RPC framers
# ============================================================================
#
# A framer reads and writes discrete byte chunks from/to a raw byte stream.
# It answers the question: "where does one message end and the next begin?"
#
# # The Framing Problem
#
# Raw byte streams (TCP connections, stdin/stdout pipes) have no inherent
# message boundaries. If you write two JSON objects back-to-back to stdout:
#
#   {"method":"ping"}{"method":"pong"}
#
# A reader cannot tell where the first message ends and the second begins
# without additional framing. Framers solve this by adding envelope bytes:
#
#   Content-Length framing (used by LSP):
#     Content-Length: 17\r\n\r\n{"method":"ping"}Content-Length: 17\r\n\r\n{"method":"pong"}
#
#   Newline-delimited framing (NDJSON):
#     {"method":"ping"}\n{"method":"pong"}\n
#
#   4-byte length prefix framing:
#     \x00\x00\x00\x11{"method":"ping"}\x00\x00\x00\x11{"method":"pong"}
#
# The framer's job is to handle the envelope; the codec's job is to handle
# the content inside the envelope.
#
# # The Framer's role in the stack
#
#   Codec
#       │  raw payload bytes (no envelope)
#       ▼
#   Framer      ← this layer
#       │  framed byte stream (envelope + payload)
#       ▼
#   Transport (filehandle, socket, pipe)
#
# # Implementing a framer
#
# A framer is any Perl object with these two methods:
#
#   read_frame($self) → ($bytes, $err)
#
#     Read the next frame from the stream and return the payload bytes
#     (stripped of framing envelope).
#
#     Returns ($bytes, undef)  — a frame was read successfully.
#     Returns (undef, undef)   — clean EOF; the stream is finished.
#     Returns (undef, $err)    — framing error (malformed envelope).
#
#   write_frame($self, $bytes) → undef (or die on error)
#
#     Write a frame to the stream, wrapping $bytes in whatever envelope
#     the framing scheme requires.
#     May die on write error.
#
# # Example framers
#
#   ContentLengthFramer — "Content-Length: N\r\n\r\n" prefix; used by LSP
#   NewlineFramer       — appends "\n"; used by NDJSON
#   LengthPrefixFramer  — 4-byte big-endian length prefix; compact TCP variant
#   WebSocketFramer     — WebSocket data frames; used in browser clients
#   PassthroughFramer   — no framing; useful when HTTP handles framing

use strict;
use warnings;

our $VERSION = '0.01';

# ---------------------------------------------------------------------------
# new(%args) → Framer
#
# Base constructor. Subclasses accept transport-specific args like
# in_fh/out_fh filehandles or a socket.
# ---------------------------------------------------------------------------

sub new {
    my ($class, %args) = @_;
    return bless \%args, $class;
}

# ---------------------------------------------------------------------------
# read_frame() → ($bytes, $err)
#
# Base implementation — always dies. Subclasses MUST override this.
# ---------------------------------------------------------------------------

sub read_frame {
    my ($self) = @_;
    die ref($self) . "->read_frame() is not implemented. Override it in your framer subclass.\n";
}

# ---------------------------------------------------------------------------
# write_frame($bytes) → undef
#
# Base implementation — always dies. Subclasses MUST override this.
# ---------------------------------------------------------------------------

sub write_frame {
    my ($self, $bytes) = @_;
    die ref($self) . "->write_frame() is not implemented. Override it in your framer subclass.\n";
}

1;

__END__

=head1 NAME

CodingAdventures::Rpc::Framer — Interface contract for RPC framer objects

=head1 SYNOPSIS

  # Implement a framer (e.g., newline-delimited):
  package NewlineFramer;
  use parent 'CodingAdventures::Rpc::Framer';

  sub new {
      my ($class, %args) = @_;
      return $class->SUPER::new(%args);
      # expects: in_fh => $read_handle, out_fh => $write_handle
  }

  sub read_frame {
      my ($self) = @_;
      my $line = readline($self->{in_fh});
      return (undef, undef) unless defined $line;   # EOF
      chomp $line;
      return ($line, undef);
  }

  sub write_frame {
      my ($self, $bytes) = @_;
      print { $self->{out_fh} } $bytes . "\n";
  }

  # Use it with the RPC server:
  use CodingAdventures::Rpc::Server;
  my $server = CodingAdventures::Rpc::Server->new(
      codec  => $codec,
      framer => NewlineFramer->new(in_fh => \*STDIN, out_fh => \*STDOUT),
  );

=head1 DESCRIPTION

C<CodingAdventures::Rpc::Framer> is an abstract base class documenting the
interface contract for RPC framers. A framer splits a raw byte stream into
discrete payload chunks (frames) and reassembles them on the write side.

The framer operates on the byte stream below the codec. It handles
envelope/header bytes and delivers naked payload bytes to the codec.

=head1 METHODS

=over 4

=item new(%args)

Construct a framer. Subclasses typically accept C<in_fh> and C<out_fh>
filehandles or a single bidirectional socket handle.

=item read_frame()

Read the next frame from the transport. Return values:

=over 4

=item C<($bytes, undef)>

A frame was read. C<$bytes> contains the raw payload (no framing envelope).

=item C<(undef, undef)>

Clean EOF. The transport has been closed by the remote end.

=item C<(undef, $error_string)>

A framing error occurred (e.g., malformed Content-Length header, truncated
length prefix). The server will send an error response and continue.

=back

=item write_frame($bytes)

Write a frame to the transport. Wraps C<$bytes> in whatever envelope the
framing scheme requires and flushes the output. May C<die> on write error.

=back

=head1 SEE ALSO

L<CodingAdventures::Rpc>, L<CodingAdventures::Rpc::Codec>,
L<CodingAdventures::Rpc::Server>

=cut
