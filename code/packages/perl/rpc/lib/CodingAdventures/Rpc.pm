package CodingAdventures::Rpc;

# ============================================================================
# CodingAdventures::Rpc — Codec-agnostic RPC primitive
# ============================================================================
#
# This is the abstract RPC layer that sits below codec-specific packages like
# CodingAdventures::JsonRpc. It captures the semantics of remote procedure
# calls — requests, responses, notifications, error codes, method dispatch,
# id correlation, handler panic recovery — without knowing anything about
# serialization formats or framing schemes.
#
# # Architecture
#
#   ┌─────────────────────────────────────────────────────────────┐
#   │  Application  (LSP server, custom tool, test client, …)     │
#   ├─────────────────────────────────────────────────────────────┤
#   │  CodingAdventures::Rpc                                      │
#   │  RpcServer / RpcClient                                      │
#   │  (method dispatch, id correlation, error handling,          │
#   │   handler registry, panic recovery)                         │
#   ├─────────────────────────────────────────────────────────────┤
#   │  RpcCodec                                                   │
#   │  (RpcMessage ↔ bytes)          JSON, MessagePack, Protobuf  │
#   ├─────────────────────────────────────────────────────────────┤
#   │  RpcFramer                                                  │
#   │  (byte stream ↔ chunks)   Content-Length, newlines, ws     │
#   ├─────────────────────────────────────────────────────────────┤
#   │  Transport (filehandle, socket, pipe)                       │
#   └─────────────────────────────────────────────────────────────┘
#
# # Sub-modules
#
# Modules are loaded in leaf-to-root order so that each module's dependencies
# are already in memory when the module itself is compiled:
#
#   Errors.pm      — error code constants (no deps)
#   Message.pm     — message constructors (no deps)
#   Codec.pm       — abstract codec interface (no deps)
#   Framer.pm      — abstract framer interface (no deps)
#   Server.pm      — dispatch server (depends on Message, Errors)
#   Client.pm      — synchronous client (depends on Message)
#
# # Usage
#
#   use CodingAdventures::Rpc;
#
# All sub-modules are loaded automatically. You can also load them
# individually if you only need part of the package.

use strict;
use warnings;

our $VERSION = '0.01';

# ---------------------------------------------------------------------------
# Load sub-modules in leaf-to-root order.
#
# "Leaf" = no dependencies on other Rpc sub-modules.
# "Root" = depends on leaves.
#
# This ordering ensures that when Server.pm says
#   use CodingAdventures::Rpc::Message qw(make_response make_error);
# the Message module is already compiled and in %INC.
# ---------------------------------------------------------------------------

use CodingAdventures::Rpc::Errors   ();   # leaf: error code constants
use CodingAdventures::Rpc::Message  ();   # leaf: message constructors
use CodingAdventures::Rpc::Codec    ();   # leaf: abstract codec interface
use CodingAdventures::Rpc::Framer   ();   # leaf: abstract framer interface
use CodingAdventures::Rpc::Server   ();   # root: depends on Message, Errors
use CodingAdventures::Rpc::Client   ();   # root: depends on Message

1;

__END__

=head1 NAME

CodingAdventures::Rpc — Codec-agnostic RPC primitive

=head1 SYNOPSIS

  use CodingAdventures::Rpc;

  # Build a server with your own codec and framer:
  my $server = CodingAdventures::Rpc::Server->new(
      codec  => MyJsonCodec->new,
      framer => MyContentLengthFramer->new(\*STDIN, \*STDOUT),
  );

  $server->on_request('ping', sub {
      my ($id, $params) = @_;
      return { pong => 1 };
  })->on_notification('log', sub {
      my ($params) = @_;
      warn $params->{message}, "\n";
  });

  $server->serve;   # blocks until stdin closes

=head1 DESCRIPTION

C<CodingAdventures::Rpc> is the abstract RPC layer that all codec-specific
packages (C<CodingAdventures::JsonRpc>, future C<MsgpackRpc>, etc.) build on.

It provides:

=over 4

=item *

Four message types: request, response, error, notification
(L<CodingAdventures::Rpc::Message>)

=item *

Standard error codes: PARSE_ERROR, INVALID_REQUEST, METHOD_NOT_FOUND,
INVALID_PARAMS, INTERNAL_ERROR (L<CodingAdventures::Rpc::Errors>)

=item *

A pluggable codec interface (L<CodingAdventures::Rpc::Codec>)

=item *

A pluggable framer interface (L<CodingAdventures::Rpc::Framer>)

=item *

A dispatch server with handler panic recovery (L<CodingAdventures::Rpc::Server>)

=item *

A synchronous client with server-push notification support
(L<CodingAdventures::Rpc::Client>)

=back

=head1 SUB-MODULES

=over 4

=item L<CodingAdventures::Rpc::Errors>

Integer constants for the standard RPC error codes.

=item L<CodingAdventures::Rpc::Message>

Constructor functions for all four message kinds.

=item L<CodingAdventures::Rpc::Codec>

Abstract base class documenting the codec interface contract.

=item L<CodingAdventures::Rpc::Framer>

Abstract base class documenting the framer interface contract.

=item L<CodingAdventures::Rpc::Server>

Codec-agnostic RPC server. Inject your codec and framer; register handlers.

=item L<CodingAdventures::Rpc::Client>

Synchronous RPC client. Inject your codec and framer; call request() and
notify().

=back

=head1 SEE ALSO

L<CodingAdventures::JsonRpc> — JSON + Content-Length instantiation of this layer.

=cut
