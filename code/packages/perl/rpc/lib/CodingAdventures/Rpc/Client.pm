package CodingAdventures::Rpc::Client;

# ============================================================================
# CodingAdventures::Rpc::Client — Codec-agnostic RPC client
# ============================================================================
#
# The Client sends requests and notifications to a remote RPC server and
# receives responses. It operates synchronously (blocking): request() sends
# a message and waits for the matching response before returning.
#
# # Id management
#
# The client maintains a monotonically increasing integer counter starting
# at 1. Each call to request() increments the counter and uses the new value
# as the request id. The server echoes this id back in the response, allowing
# the client to match the response to the original request.
#
#   Client                         Server
#   ──────                         ──────
#   request(1, 'ping', undef) ──►  dispatch to handler
#                              ◄── response(1, { pong => 1 })
#   request(2, 'add', {a=>1}) ──►  dispatch to handler
#                              ◄── response(2, { sum => 1 })
#
# # Blocking request flow
#
# request() sends the message then enters a reading loop. It reads frames
# until it finds a response with a matching id:
#
#   send request with id = N
#   loop:
#     bytes = framer.read_frame()
#     if EOF: return error
#     msg = codec.decode(bytes)
#     if response with id == N: return result
#     if error with id == N:    return error
#     if notification: call handler if registered, continue
#     otherwise: ignore and continue
#
# # Server-push notifications
#
# While the client is blocked in request(), the server may push notifications
# (e.g., "diagnostics/publish" in LSP). The client handles these by looking
# up the notification method in its notif_handlers table and calling the
# appropriate handler, then continuing to wait for the response.
#
# # Fire-and-forget notifications
#
# notify() sends a notification without waiting for any response. Useful for
# events like "textDocument/didChange" where the client does not need a reply.
#
# # Usage example
#
#   my $client = CodingAdventures::Rpc::Client->new(
#       codec  => $codec,
#       framer => $framer,
#   );
#
#   my ($result, $err) = $client->request('add', { a => 1, b => 2 });
#   if (defined $err) {
#       warn "error: $err->{message}\n";
#   } else {
#       print "sum = $result->{sum}\n";
#   }
#
#   $client->notify('log', { message => 'hello from client' });

use strict;
use warnings;

use CodingAdventures::Rpc::Message qw(make_request make_notification);

our $VERSION = '0.01';

# ---------------------------------------------------------------------------
# new(codec => $codec, framer => $framer) → Client
#
#   codec  — any object with encode($msg) and decode($bytes) methods
#   framer — any object with read_frame() and write_frame($bytes) methods
# ---------------------------------------------------------------------------

sub new {
    my ($class, %args) = @_;
    die "CodingAdventures::Rpc::Client->new requires 'codec'\n"
        unless defined $args{codec};
    die "CodingAdventures::Rpc::Client->new requires 'framer'\n"
        unless defined $args{framer};
    return bless {
        codec          => $args{codec},
        framer         => $args{framer},
        next_id        => 1,         # monotonic request id counter
        notif_handlers => {},        # method name → coderef for server push
    }, $class;
}

# ---------------------------------------------------------------------------
# on_notification($method, $handler) → $self
#
# Register a handler for server-initiated notifications. When the client is
# blocked inside request() and the server sends a notification, this handler
# is called before the client resumes waiting for the response.
#
# Returns $self for method chaining.
# ---------------------------------------------------------------------------

sub on_notification {
    my ($self, $method, $handler) = @_;
    $self->{notif_handlers}{$method} = $handler;
    return $self;
}

# ---------------------------------------------------------------------------
# request($method, $params) → ($result, $err)
#
# Send a request to the server and wait (blocking) for the response.
#
# Returns ($result_value, undef) on success.
# Returns (undef, $err_hashref) on error, where $err_hashref has:
#   { code => $int, message => $str, data => $optional }
#
# Any server-push notifications received while waiting are dispatched to
# registered notification handlers.
#
# $params is optional and can be any value the codec supports (hashref,
# arrayref, string, number, undef).
# ---------------------------------------------------------------------------

sub request {
    my ($self, $method, $params) = @_;
    my $codec  = $self->{codec};
    my $framer = $self->{framer};

    # Assign the next monotonically increasing id.
    my $id = $self->{next_id}++;

    # Build and send the request.
    my $req = (@_ >= 3)
        ? make_request($id, $method, $params)
        : make_request($id, $method);
    $framer->write_frame($codec->encode($req));

    # Wait for the matching response. While waiting, dispatch any
    # server-push notifications we receive.
    while (1) {
        my ($bytes, $frame_err) = $framer->read_frame;

        # Clean EOF before we got a response — connection was closed.
        if (!defined $bytes && !defined $frame_err) {
            return (undef, {
                code    => -32603,
                message => 'Connection closed before response received',
            });
        }

        if (defined $frame_err) {
            return (undef, {
                code    => -32600,
                message => 'Framing error while waiting for response',
                data    => $frame_err,
            });
        }

        my ($msg, $decode_err) = $codec->decode($bytes);
        if (defined $decode_err) {
            # Garbled response — treat as internal error and keep waiting.
            # (In practice, a garbled response means the stream is corrupt;
            # returning an error here is safer than looping forever.)
            return (undef, {
                code    => -32700,
                message => 'Codec decode error while waiting for response',
                data    => $decode_err,
            });
        }

        my $kind = $msg->{kind};

        if ($kind eq 'response' && defined $msg->{id} && $msg->{id} == $id) {
            # This is the response we are waiting for — success.
            return ($msg->{result}, undef);
        }

        if ($kind eq 'error' && defined $msg->{id} && $msg->{id} == $id) {
            # This is the error response for our request.
            return (undef, {
                code    => $msg->{code},
                message => $msg->{message},
                defined($msg->{data}) ? (data => $msg->{data}) : (),
            });
        }

        if ($kind eq 'notification') {
            # Server-push notification received while waiting. Dispatch it
            # to the registered handler (if any) and continue waiting.
            my $handler = $self->{notif_handlers}{$msg->{method} // ''};
            if (defined $handler) {
                eval { $handler->($msg->{params}) };
                # Ignore handler die — don't let it abort the request.
            }
            next;
        }

        # Any other message (response for a different id, etc.) — ignore.
    }
}

# ---------------------------------------------------------------------------
# notify($method, $params) → undef
#
# Send a notification to the server. No response is expected or waited for.
# This is fire-and-forget.
#
# $params is optional.
# ---------------------------------------------------------------------------

sub notify {
    my ($self, $method, $params) = @_;
    my $codec  = $self->{codec};
    my $framer = $self->{framer};
    my $notif = (@_ >= 3)
        ? make_notification($method, $params)
        : make_notification($method);
    $framer->write_frame($codec->encode($notif));
    return;
}

1;

__END__

=head1 NAME

CodingAdventures::Rpc::Client — Codec-agnostic RPC client

=head1 SYNOPSIS

  use CodingAdventures::Rpc::Client;

  my $client = CodingAdventures::Rpc::Client->new(
      codec  => $my_codec,
      framer => $my_framer,
  );

  # Register a handler for server-push notifications:
  $client->on_notification('diagnostics', sub {
      my ($params) = @_;
      print "Diagnostic: $params->{message}\n";
  });

  # Send a request and wait for the response:
  my ($result, $err) = $client->request('add', { a => 3, b => 4 });
  if (defined $err) {
      warn "RPC error $err->{code}: $err->{message}\n";
  } else {
      print "sum = $result->{sum}\n";
  }

  # Fire-and-forget notification:
  $client->notify('log', { level => 'info', message => 'ready' });

=head1 DESCRIPTION

A synchronous (blocking) RPC client. Sends requests and waits for the
matching response. Dispatches server-push notifications received while
waiting. Sends fire-and-forget notifications.

=head1 METHODS

=over 4

=item new(codec => $codec, framer => $framer)

Construct a client. Both C<codec> and C<framer> are required.

=item request($method, $params)

Send a request and block until the response arrives. Returns
C<($result, undef)> on success or C<(undef, $err_hashref)> on error.
C<$params> is optional.

=item notify($method, $params)

Send a fire-and-forget notification. C<$params> is optional.

=item on_notification($method, $coderef)

Register a handler for server-initiated notifications. Called while the
client is blocked in C<request()>. Returns C<$self> for chaining.

=back

=head1 SEE ALSO

L<CodingAdventures::Rpc>, L<CodingAdventures::Rpc::Server>

=cut
