package CodingAdventures::Ls00::LspErrors;

# ============================================================================
# CodingAdventures::Ls00::LspErrors -- LSP-specific error codes
# ============================================================================
#
# The JSON-RPC 2.0 specification reserves error codes [-32768, -32000].
# The LSP specification further reserves [-32899, -32800] for LSP
# protocol-level errors.
#
# Standard JSON-RPC error codes (from CodingAdventures::JsonRpc::Errors):
#   -32700  ParseError
#   -32600  InvalidRequest
#   -32601  MethodNotFound
#   -32602  InvalidParams
#   -32603  InternalError
#
# LSP-specific error codes are listed below.
#
# Reference:
# https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#errorCodes

use strict;
use warnings;

use Exporter 'import';
our @EXPORT_OK = qw(
    SERVER_NOT_INITIALIZED
    UNKNOWN_ERROR_CODE
    REQUEST_FAILED
    SERVER_CANCELLED
    CONTENT_MODIFIED
    REQUEST_CANCELLED
);
our %EXPORT_TAGS = ( all => \@EXPORT_OK );

our $VERSION = '0.01';

# ServerNotInitialized (-32002): the server received a request before the
# initialize handshake was completed.
use constant SERVER_NOT_INITIALIZED => -32002;

# UnknownErrorCode (-32001): a generic error code for unknown errors.
use constant UNKNOWN_ERROR_CODE => -32001;

# RequestFailed (-32803): a request failed but not due to a protocol problem.
use constant REQUEST_FAILED => -32803;

# ServerCancelled (-32802): the server cancelled the request.
use constant SERVER_CANCELLED => -32802;

# ContentModified (-32801): the document content was modified before the
# request completed.
use constant CONTENT_MODIFIED => -32801;

# RequestCancelled (-32800): the client cancelled the request.
use constant REQUEST_CANCELLED => -32800;

1;

__END__

=head1 NAME

CodingAdventures::Ls00::LspErrors -- LSP-specific error code constants

=head1 SYNOPSIS

  use CodingAdventures::Ls00::LspErrors qw(:all);

  my $err = { code => REQUEST_FAILED, message => 'document not open' };

=head1 DESCRIPTION

Provides compile-time constants for LSP-specific error codes.

=cut
