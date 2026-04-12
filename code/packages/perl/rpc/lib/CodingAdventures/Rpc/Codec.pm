package CodingAdventures::Rpc::Codec;

# ============================================================================
# CodingAdventures::Rpc::Codec — Interface contract for RPC codecs
# ============================================================================
#
# A codec translates between RPC message objects and byte strings.
# It answers the question: "how do we serialize this message to bytes?"
#
# # The Codec's role in the stack
#
#   Application
#       │  RpcMessage objects (blessed hashrefs)
#       ▼
#   Codec       ← this layer
#       │  raw bytes (a Perl string with :raw bytes)
#       ▼
#   Framer
#       │  framed byte stream (with length headers, delimiters, etc.)
#       ▼
#   Transport (filehandle, socket, pipe)
#
# The codec never touches framing. It receives exactly the payload bytes
# from the framer (no Content-Length header, no WebSocket envelope) and
# returns exactly the payload bytes.
#
# # Implementing a codec
#
# A codec is any Perl object with these two methods:
#
#   encode($self, $msg) → $bytes_string
#
#     Convert an RpcMessage blessed hashref into a byte string.
#     May die on encode failure.
#
#   decode($self, $bytes) → ($msg, $err)
#
#     Convert a byte string back into an RpcMessage blessed hashref.
#     Returns ($msg, undef) on success.
#     Returns (undef, $err_string) on failure (bad encoding or invalid shape).
#
# # Why duck typing?
#
# Perl does not have formal interface types. Instead we use duck typing: any
# object with the right methods "implements" the codec interface. This module
# documents the contract in POD and provides a base class that dies with a
# helpful message if a subclass forgets to implement a method.
#
# # Example codecs
#
#   CodingAdventures::JsonRpc::JsonCodec  — JSON over Content-Length framing
#   MsgpackCodec (future)                 — MessagePack over length-prefix framing
#   ProtobufCodec (future)                — Protobuf over length-prefix framing
#
# # Usage (implementing a codec)
#
#   package MyCodec;
#   use parent 'CodingAdventures::Rpc::Codec';
#
#   sub encode {
#       my ($self, $msg) = @_;
#       # serialize $msg to bytes and return
#   }
#
#   sub decode {
#       my ($self, $bytes) = @_;
#       # parse $bytes; return ($msg, undef) or (undef, $err)
#   }

use strict;
use warnings;

our $VERSION = '0.01';

# ---------------------------------------------------------------------------
# new(%args) → Codec
#
# Base constructor. Subclasses may override this to accept codec-specific
# configuration (character encoding, schema registry, etc.).
# ---------------------------------------------------------------------------

sub new {
    my ($class, %args) = @_;
    return bless \%args, $class;
}

# ---------------------------------------------------------------------------
# encode($msg) → $bytes
#
# Base implementation — always dies. Subclasses MUST override this.
#
# The error message is intentionally explicit so a developer who forgets
# to override sees exactly what they need to do.
# ---------------------------------------------------------------------------

sub encode {
    my ($self, $msg) = @_;
    die ref($self) . "->encode() is not implemented. Override it in your codec subclass.\n";
}

# ---------------------------------------------------------------------------
# decode($bytes) → ($msg, $err)
#
# Base implementation — always dies. Subclasses MUST override this.
# ---------------------------------------------------------------------------

sub decode {
    my ($self, $bytes) = @_;
    die ref($self) . "->decode() is not implemented. Override it in your codec subclass.\n";
}

1;

__END__

=head1 NAME

CodingAdventures::Rpc::Codec — Interface contract for RPC codec objects

=head1 SYNOPSIS

  # Implement a codec:
  package MyCodec;
  use parent 'CodingAdventures::Rpc::Codec';

  sub encode {
      my ($self, $msg) = @_;
      # serialize the CodingAdventures::Rpc::Message blessed hashref to bytes
      return $bytes_string;
  }

  sub decode {
      my ($self, $bytes) = @_;
      # parse bytes; on success return ($msg_hashref, undef)
      # on failure return (undef, "error description string")
  }

  # Use it with the RPC server:
  use CodingAdventures::Rpc::Server;
  my $server = CodingAdventures::Rpc::Server->new(
      codec  => MyCodec->new,
      framer => $framer,
  );

=head1 DESCRIPTION

C<CodingAdventures::Rpc::Codec> is an abstract base class that documents the
interface contract for RPC codecs. A codec translates between
C<CodingAdventures::Rpc::Message> blessed hashrefs and raw byte strings.

The codec operates on payload bytes only. It never sees framing overhead
(Content-Length headers, length prefixes, WebSocket envelopes). The framer
strips those before calling C<decode> and adds them after C<encode> returns.

=head1 METHODS

=over 4

=item new(%args)

Construct a codec object. Subclasses may accept configuration parameters.

=item encode($msg)

Convert a C<CodingAdventures::Rpc::Message> hashref to a byte string.
The returned string should be treated as raw bytes (C<:raw> / C<:bytes>).
May C<die> on encode failure.

=item decode($bytes)

Convert a raw byte string to a C<CodingAdventures::Rpc::Message> hashref.
Returns C<($msg, undef)> on success.
Returns C<(undef, $error_string)> on failure (malformed bytes or invalid
message shape).

=back

=head1 IMPLEMENTING A CODEC

Subclass C<CodingAdventures::Rpc::Codec> and override both C<encode> and
C<decode>. The base class methods die with a helpful message if called
directly, so you will catch missing implementations immediately in tests.

The C<decode> method should distinguish two failure modes:

=over 4

=item Parse error (PARSE_ERROR = -32700)

The bytes could not be deserialized at all (e.g., invalid JSON syntax,
truncated MessagePack frame). Return C<(undef, "parse error: ...")>.

=item Invalid request (INVALID_REQUEST = -32600)

The bytes deserialized successfully but the resulting data does not look
like a valid RPC message (e.g., missing C<method> field). Return
C<(undef, "invalid request: ...")>.

=back

The C<RpcServer> uses these strings to choose the right error code when
sending an error response to the client.

=head1 SEE ALSO

L<CodingAdventures::Rpc>, L<CodingAdventures::Rpc::Framer>,
L<CodingAdventures::Rpc::Server>

=cut
