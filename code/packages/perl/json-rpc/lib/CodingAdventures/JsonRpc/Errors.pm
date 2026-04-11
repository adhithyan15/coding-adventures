package CodingAdventures::JsonRpc::Errors;

# ============================================================================
# CodingAdventures::JsonRpc::Errors — Standard JSON-RPC 2.0 error codes
# ============================================================================
#
# JSON-RPC 2.0 reserves a set of integer error codes for well-known failure
# modes.  Every error response sent by a server must carry one of these
# codes (or a server-defined code in the range -32099 to -32000).
#
# | Code    | Name              | When to use                                |
# |---------|-------------------|--------------------------------------------|
# | -32700  | Parse error       | Message body is not valid JSON             |
# | -32600  | Invalid Request   | JSON parsed but not a valid Request object |
# | -32601  | Method not found  | Method not registered on the server        |
# | -32602  | Invalid params    | Invalid method parameters                  |
# | -32603  | Internal error    | Unhandled exception inside a handler       |
#
# The range -32099 to -32000 is reserved for implementation-defined server
# errors.  The range -32899 to -32800 is reserved for LSP-specific errors.
#
# Usage:
#
#   use CodingAdventures::JsonRpc::Errors qw(:all);
#   my $code = PARSE_ERROR;     # -32700
#
# Or import selectively:
#
#   use CodingAdventures::JsonRpc::Errors qw(PARSE_ERROR METHOD_NOT_FOUND);

use strict;
use warnings;

use Exporter 'import';

# Export all constants by default when the caller says `use ... qw(:all)`.
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
# We use constant pragma (Perl core) rather than Readonly to avoid CPAN deps.
# Each constant is a compile-time scalar substitution — zero overhead.
# ---------------------------------------------------------------------------

use constant PARSE_ERROR      => -32700;
use constant INVALID_REQUEST  => -32600;
use constant METHOD_NOT_FOUND => -32601;
use constant INVALID_PARAMS   => -32602;
use constant INTERNAL_ERROR   => -32603;

1;

__END__

=head1 NAME

CodingAdventures::JsonRpc::Errors — Standard JSON-RPC 2.0 error code constants

=head1 SYNOPSIS

  use CodingAdventures::JsonRpc::Errors qw(:all);
  print PARSE_ERROR;       # -32700
  print METHOD_NOT_FOUND;  # -32601

=head1 DESCRIPTION

Provides compile-time constants for the reserved JSON-RPC 2.0 error codes.

=head1 CONSTANTS

=over 4

=item PARSE_ERROR (-32700)

The message body is not valid JSON.

=item INVALID_REQUEST (-32600)

The JSON was parsed but does not conform to the JSON-RPC 2.0 Request spec.

=item METHOD_NOT_FOUND (-32601)

The requested method has not been registered on this server.

=item INVALID_PARAMS (-32602)

Invalid method parameters.

=item INTERNAL_ERROR (-32603)

An internal server error occurred while handling the request.

=back

=cut
