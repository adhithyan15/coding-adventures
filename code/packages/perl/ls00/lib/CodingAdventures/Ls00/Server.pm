package CodingAdventures::Ls00::Server;

# ============================================================================
# CodingAdventures::Ls00::Server -- Main LSP server coordinator
# ============================================================================
#
# The Server wires together:
#   - The bridge (language-specific logic)
#   - The DocumentManager (tracks open file contents)
#   - The ParseCache (avoids redundant parses)
#   - The JSON-RPC Server (protocol layer)
#
# It registers all LSP request and notification handlers, then calls
# serve() to start the blocking read-dispatch-write loop.
#
# # Server Lifecycle
#
#   Client (editor)              Server (us)
#     |                               |
#     |--initialize------------->     |  store clientInfo, return capabilities
#     |<-----------result--------     |
#     |--initialized (notif)----->    |  no-op (handshake complete)
#     |--textDocument/didOpen---->    |  open doc, parse, push diagnostics
#     |--textDocument/hover------>    |  get parse result, call bridge.hover
#     |<-----------result--------     |
#     |--shutdown---------------->    |  set shutdown flag, return undef
#     |--exit (notif)------------>    |  exit(0) or exit(1)
#
# # Sending Notifications to the Editor
#
# The JSON-RPC Server handles request/response pairs.  But the LSP server
# also needs to PUSH notifications to the editor (e.g., diagnostics).
# We do this by holding a reference to the JSON-RPC Writer and calling
# write_message() directly.

use strict;
use warnings;

use CodingAdventures::JsonRpc;
use CodingAdventures::JsonRpc::Message qw(notification);
use CodingAdventures::Ls00::DocumentManager;
use CodingAdventures::Ls00::ParseCache;
use CodingAdventures::Ls00::Handlers;

our $VERSION = '0.01';

# ---------------------------------------------------------------------------
# new($bridge, $in_fh, $out_fh) -> Server
#
# Create an LspServer wired to read from $in_fh and write to $out_fh.
#
# Typical usage:
#   my $server = CodingAdventures::Ls00::Server->new($bridge, \*STDIN, \*STDOUT);
#   $server->serve();
#
# For testing, pass in-memory filehandles (open on scalar references).
# ---------------------------------------------------------------------------

sub new {
    my ($class, $bridge, $in_fh, $out_fh) = @_;

    my $rpc_server = CodingAdventures::JsonRpc::Server->new($in_fh, $out_fh);
    my $writer     = CodingAdventures::JsonRpc::Writer->new($out_fh);

    my $self = bless {
        bridge      => $bridge,
        doc_manager => CodingAdventures::Ls00::DocumentManager->new(),
        parse_cache => CodingAdventures::Ls00::ParseCache->new(),
        rpc_server  => $rpc_server,
        writer      => $writer,
        shutdown    => 0,
        initialized => 0,
    }, $class;

    CodingAdventures::Ls00::Handlers::register_handlers($self);

    return $self;
}

# ---------------------------------------------------------------------------
# serve()
#
# Start the blocking JSON-RPC read-dispatch-write loop.  This call blocks
# until the editor closes the connection (EOF on stdin).
# ---------------------------------------------------------------------------

sub serve {
    my ($self) = @_;
    $self->{rpc_server}->serve();
}

# ---------------------------------------------------------------------------
# send_notification($method, \%params)
#
# Send a server-initiated notification to the editor.  Used for
# textDocument/publishDiagnostics and other push notifications.
# ---------------------------------------------------------------------------

sub send_notification {
    my ($self, $method, $params) = @_;
    my $notif = notification($method, $params);
    eval { $self->{writer}->write_message($notif) };
    # Best-effort: if the write fails, the editor will show stale data.
}

1;

__END__

=head1 NAME

CodingAdventures::Ls00::Server -- Main LSP server coordinator

=head1 SYNOPSIS

  use CodingAdventures::Ls00::Server;

  my $server = CodingAdventures::Ls00::Server->new($bridge, \*STDIN, \*STDOUT);
  $server->serve();

=head1 DESCRIPTION

Wires the bridge, document manager, parse cache, and JSON-RPC server
together.  Call C<serve()> to start the blocking event loop.

=head1 METHODS

=over 4

=item new($bridge, $in_fh, $out_fh)

Create a server.  Both filehandles should be in binary mode.

=item serve()

Blocking loop.  Returns when stdin closes.

=item send_notification($method, \%params)

Send a server-initiated notification (e.g., diagnostics).

=back

=cut
