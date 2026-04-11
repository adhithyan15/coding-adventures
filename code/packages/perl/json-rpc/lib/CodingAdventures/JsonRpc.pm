package CodingAdventures::JsonRpc;

# ============================================================================
# CodingAdventures::JsonRpc — JSON-RPC 2.0 transport library (umbrella module)
# ============================================================================
#
# JSON-RPC 2.0 is the wire protocol underlying the Language Server Protocol
# (LSP).  This umbrella module loads all sub-modules and re-exports the
# most commonly used symbols so callers only need a single `use` statement.
#
# Sub-modules:
#   CodingAdventures::JsonRpc::Errors   — standard error-code constants
#   CodingAdventures::JsonRpc::Message  — message constructors + parse_message
#   CodingAdventures::JsonRpc::Reader   — MessageReader class
#   CodingAdventures::JsonRpc::Writer   — MessageWriter class
#   CodingAdventures::JsonRpc::Server   — Server dispatch loop
#
# # No external dependencies
#
# The only non-core module used is JSON::PP, which has been bundled with
# Perl distributions since 5.14 (released 2011).  No CPAN install required.
#
# # Typical usage
#
#   use CodingAdventures::JsonRpc;
#
#   my $server = CodingAdventures::JsonRpc::Server->new(\*STDIN, \*STDOUT);
#   $server
#     ->on_request('initialize', sub { ... })
#     ->on_notification('textDocument/didOpen', sub { ... })
#     ->serve;

use strict;
use warnings;

our $VERSION = '0.01';

# Load all sub-modules eagerly so callers get everything with one `use`.
use CodingAdventures::JsonRpc::Errors  ();
use CodingAdventures::JsonRpc::Message ();
use CodingAdventures::JsonRpc::Reader  ();
use CodingAdventures::JsonRpc::Writer  ();
use CodingAdventures::JsonRpc::Server  ();

1;

__END__

=head1 NAME

CodingAdventures::JsonRpc — JSON-RPC 2.0 over stdin/stdout (umbrella module)

=head1 SYNOPSIS

  use CodingAdventures::JsonRpc;

  # Build and run a server
  my $server = CodingAdventures::JsonRpc::Server->new(\*STDIN, \*STDOUT);
  $server->on_request('initialize', sub {
      my ($id, $params) = @_;
      return { capabilities => {} };
  });
  $server->serve;

  # Low-level: read a message from a buffer
  use CodingAdventures::JsonRpc::Reader;
  open my $fh, '<', \$buffer;
  binmode($fh, ':raw');
  my $reader = CodingAdventures::JsonRpc::Reader->new($fh);
  my ($msg, $err) = $reader->read_message;

=head1 DESCRIPTION

JSON-RPC 2.0 transport library using Content-Length framing over stdin/stdout.
Suitable for building Language Server Protocol (LSP) servers.

See the individual sub-module documentation for full API details.

=head1 MODULES

=over 4

=item L<CodingAdventures::JsonRpc::Errors>

Standard error code constants.

=item L<CodingAdventures::JsonRpc::Message>

Message constructors and C<parse_message> / C<classify_message>.

=item L<CodingAdventures::JsonRpc::Reader>

C<MessageReader> — reads Content-Length-framed messages from a filehandle.

=item L<CodingAdventures::JsonRpc::Writer>

C<MessageWriter> — writes Content-Length-framed messages to a filehandle.

=item L<CodingAdventures::JsonRpc::Server>

C<Server> — registers handlers and drives the read-dispatch-write loop.

=back

=cut
