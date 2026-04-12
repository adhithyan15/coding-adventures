package CodingAdventures::Rpc::Errors;

# ============================================================================
# CodingAdventures::Rpc::Errors — Codec-agnostic RPC error code constants
# ============================================================================
#
# RPC error codes are integers defined independently of any serialization
# format. Whether the wire uses JSON, MessagePack, Protobuf, or a custom
# binary format, the semantic meaning of -32601 is always "Method not
# found".
#
# Think of these codes like HTTP status codes: 404 means "not found"
# regardless of whether the body is HTML, JSON, or plain text.
#
# # Standard error code table
#
# | Code    | Name              | When to use                                         |
# |---------|-------------------|-----------------------------------------------------|
# | -32700  | Parse error       | Framed bytes could not be decoded by the codec      |
# | -32600  | Invalid request   | Decoded OK but not a valid RPC message shape        |
# | -32601  | Method not found  | No handler registered for the method name           |
# | -32602  | Invalid params    | Handler rejected the params as malformed            |
# | -32603  | Internal error    | Unhandled exception inside a handler (panic)        |
#
# # Server-defined codes
#
# The range -32000 to -32099 is reserved for implementation-defined server
# errors. Use those when none of the standard codes apply.
#
# # What NOT to use
#
# The range -32800 to -32899 is reserved for LSP (Language Server Protocol)
# specific codes. Do not use that range in the rpc layer.
#
# # Usage
#
#   use CodingAdventures::Rpc::Errors qw(:all);
#   my $code = PARSE_ERROR;     # -32700
#
# Or import selectively:
#
#   use CodingAdventures::Rpc::Errors qw(PARSE_ERROR METHOD_NOT_FOUND);

use strict;
use warnings;

use Exporter 'import';

# Export all constants when the caller says `use ... qw(:all)`.
our @EXPORT_OK = qw(
    PARSE_ERROR
    INVALID_REQUEST
    METHOD_NOT_FOUND
    INVALID_PARAMS
    INTERNAL_ERROR
);
our %EXPORT_TAGS = ( all => \@EXPORT_OK );

our $VERSION = '0.01';

# ---------------------------------------------------------------------------
# Constant definitions
#
# We use the `constant` pragma (Perl core module) rather than the Readonly
# module from CPAN to keep the dependency list minimal. Each constant becomes
# a compile-time inlineable scalar — zero runtime overhead.
#
# Example: `PARSE_ERROR` compiles to the literal -32700 wherever it appears.
# ---------------------------------------------------------------------------

use constant PARSE_ERROR      => -32700;
use constant INVALID_REQUEST  => -32600;
use constant METHOD_NOT_FOUND => -32601;
use constant INVALID_PARAMS   => -32602;
use constant INTERNAL_ERROR   => -32603;

1;

__END__

=head1 NAME

CodingAdventures::Rpc::Errors — Codec-agnostic RPC error code constants

=head1 SYNOPSIS

  use CodingAdventures::Rpc::Errors qw(:all);
  print PARSE_ERROR;       # -32700
  print METHOD_NOT_FOUND;  # -32601

  # Or import selectively:
  use CodingAdventures::Rpc::Errors qw(PARSE_ERROR INTERNAL_ERROR);

=head1 DESCRIPTION

Provides compile-time integer constants for the standard RPC error codes.
These codes are codec-agnostic: the same numbers apply regardless of whether
the wire format is JSON, MessagePack, Protobuf, or anything else.

=head1 CONSTANTS

=over 4

=item PARSE_ERROR (-32700)

The framed bytes could not be decoded by the codec. For example, the bytes
are not valid JSON (when using a JSON codec) or not valid MessagePack (when
using a MessagePack codec).

=item INVALID_REQUEST (-32600)

The bytes were decoded successfully but the resulting data does not conform
to a valid RPC message shape (e.g., missing C<method> field in a request).

=item METHOD_NOT_FOUND (-32601)

The method named in the request has not been registered on this server.

=item INVALID_PARAMS (-32602)

The method was found but the handler rejected the parameters as malformed
or missing required fields.

=item INTERNAL_ERROR (-32603)

An unexpected exception (die, croak, panic) was thrown inside the handler.
The server recovered from it and is sending this error instead of crashing.

=back

=head1 SEE ALSO

L<CodingAdventures::Rpc>, L<CodingAdventures::Rpc::Server>

=cut
