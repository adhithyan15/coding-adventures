package CodingAdventures::Rpc::Server;

# ============================================================================
# CodingAdventures::Rpc::Server — Codec-agnostic RPC dispatch server
# ============================================================================
#
# The Server owns a codec, a framer, and two dispatch tables (one for
# requests, one for notifications). Its C<serve()> method drives the
# read-dispatch-write loop until EOF or an unrecoverable I/O error.
#
# # How it works
#
# The server loop is:
#
#   1. Call framer->read_frame() to get the next raw payload bytes.
#   2. Call codec->decode($bytes) to turn bytes into an RpcMessage.
#   3. Dispatch based on message kind:
#      - request:      call handler → send result or error response
#      - notification: call handler (no response written)
#      - response:     drop silently (pure-server mode)
#   4. Repeat until EOF.
#
# # Handler contract
#
#   Request handler:
#     sub { my ($id, $params) = @_; return $result }
#     The handler returns a result value (any Perl value). If it returns
#     a hashref with `code` (integer) and `message` (string) keys, it is
#     treated as an error response.  Otherwise the value is a success result.
#
#   Notification handler:
#     sub { my ($params) = @_ }
#     Return value is ignored. The server MUST NOT write a response.
#
# # Panic safety
#
# Every handler call is wrapped in eval{}. If the handler dies, the server
# catches the exception and sends a -32603 Internal error response (for
# requests) or silently swallows it (for notifications). The server keeps
# running — one bad handler cannot kill the whole process.
#
# This is the Perl equivalent of Go's `defer recover()` or Python's
# `try/except BaseException`.
#
# # Usage example
#
#   my $server = CodingAdventures::Rpc::Server->new(
#       codec  => $codec,
#       framer => $framer,
#   );
#
#   $server->on_request('ping', sub {
#       my ($id, $params) = @_;
#       return { pong => 1 };
#   });
#
#   $server->on_notification('log', sub {
#       my ($params) = @_;
#       warn $params->{message};
#   });
#
#   $server->serve;  # blocks until EOF

use strict;
use warnings;

use CodingAdventures::Rpc::Message qw(make_response make_error);
use CodingAdventures::Rpc::Errors  qw(:all);

our $VERSION = '0.01';

# ---------------------------------------------------------------------------
# new(codec => $codec, framer => $framer) → Server
#
# Create a new Server.
#
#   codec  — any object with encode($msg) and decode($bytes) methods
#   framer — any object with read_frame() and write_frame($bytes) methods
#
# The server retains references to both objects for the duration of its
# lifetime. They are not thread-safe by default — use separate instances
# per thread if needed.
# ---------------------------------------------------------------------------

sub new {
    my ($class, %args) = @_;
    die "CodingAdventures::Rpc::Server->new requires 'codec'\n"
        unless defined $args{codec};
    die "CodingAdventures::Rpc::Server->new requires 'framer'\n"
        unless defined $args{framer};
    return bless {
        codec          => $args{codec},
        framer         => $args{framer},
        req_handlers   => {},   # method name → coderef
        notif_handlers => {},   # method name → coderef
    }, $class;
}

# ---------------------------------------------------------------------------
# on_request($method, $handler) → $self
#
# Register a handler for the given request method name.  Calling this a
# second time with the same method name replaces the earlier handler.
#
# Returns $self so calls can be chained:
#
#   $server->on_request('a', sub { ... })
#          ->on_request('b', sub { ... });
# ---------------------------------------------------------------------------

sub on_request {
    my ($self, $method, $handler) = @_;
    $self->{req_handlers}{$method} = $handler;
    return $self;
}

# ---------------------------------------------------------------------------
# on_notification($method, $handler) → $self
#
# Register a handler for the given notification method name.
# Returns $self for chaining.
# ---------------------------------------------------------------------------

sub on_notification {
    my ($self, $method, $handler) = @_;
    $self->{notif_handlers}{$method} = $handler;
    return $self;
}

# ---------------------------------------------------------------------------
# _is_error_shape($value) → bool
#
# Return true when $value looks like a handler-returned error descriptor:
# a hashref with a numeric `code` and a string `message`.
#
# This lets handlers signal errors by returning
#   { code => INVALID_PARAMS, message => 'bad params' }
# without having to construct a full RpcMessage.
# ---------------------------------------------------------------------------

sub _is_error_shape {
    my ($v) = @_;
    return 0 unless ref($v) eq 'HASH';
    return 0 unless defined $v->{code} && $v->{code} =~ /^-?\d+$/;
    return 0 unless defined $v->{message} && !ref($v->{message});
    return 1;
}

# ---------------------------------------------------------------------------
# _dispatch($msg)
#
# Internal: process one decoded RpcMessage and write any response.
# Called by serve() for each message.
# ---------------------------------------------------------------------------

sub _dispatch {
    my ($self, $msg) = @_;
    my $kind   = $msg->{kind};
    my $codec  = $self->{codec};
    my $framer = $self->{framer};

    if ($kind eq 'request') {
        my $id     = $msg->{id};
        my $method = $msg->{method};
        my $params = $msg->{params};

        # Look up the handler. -32601 if not found.
        my $handler = $self->{req_handlers}{$method};
        unless (defined $handler) {
            my $err = make_error($id, METHOD_NOT_FOUND,
                "Method not found: $method");
            $framer->write_frame($codec->encode($err));
            return;
        }

        # Call the handler inside eval{} for panic safety.
        # If the handler calls die() or croak(), we catch it here and
        # send -32603 Internal error instead of crashing the process.
        my $result = eval { $handler->($id, $params) };
        if ($@) {
            my $errmsg = "$@";
            $errmsg =~ s/\s+$//;   # trim trailing newline from die("msg\n")
            my $err = make_error($id, INTERNAL_ERROR, 'Internal error', $errmsg);
            $framer->write_frame($codec->encode($err));
            return;
        }

        # If the handler returned a { code => ..., message => ... } hashref,
        # treat it as an application-level error response.
        if (_is_error_shape($result)) {
            my $err = make_error($id, $result->{code}, $result->{message},
                $result->{data});
            $framer->write_frame($codec->encode($err));
        } else {
            my $resp = make_response($id, $result);
            $framer->write_frame($codec->encode($resp));
        }

    } elsif ($kind eq 'notification') {
        # Per spec: the server MUST NOT send any response to a notification.
        # Unknown notification methods are silently dropped.
        my $handler = $self->{notif_handlers}{$msg->{method} // ''};
        if (defined $handler) {
            # Swallow any handler die — notifications get no error response.
            eval { $handler->($msg->{params}) };
        }

    } else {
        # response / error response coming back to a pure server — drop.
        # A bidirectional peer would route these to its pending-request table.
    }
}

# ---------------------------------------------------------------------------
# serve()
#
# Blocking read-dispatch-write loop. Returns when the transport reaches
# clean EOF. Dies on unrecoverable I/O errors.
#
# The loop:
#   1. Read next frame.
#   2. On EOF (undef, undef): exit loop.
#   3. On framing error (undef, $err): send error response, continue.
#   4. Decode the frame via the codec.
#   5. On decode error: send error response, continue.
#   6. Dispatch the message.
# ---------------------------------------------------------------------------

sub serve {
    my ($self) = @_;
    my $codec  = $self->{codec};
    my $framer = $self->{framer};

    while (1) {
        # Step 1-3: read from framer
        my ($bytes, $frame_err) = $framer->read_frame;

        # Clean EOF: remote end closed the connection
        last if !defined $bytes && !defined $frame_err;

        if (defined $frame_err) {
            # Framing error — we cannot recover the message id, so use undef.
            my $err = make_error(undef, INVALID_REQUEST,
                'Invalid Request', $frame_err);
            $framer->write_frame($codec->encode($err));
            next;
        }

        # Step 4-5: decode via codec
        my ($msg, $decode_err) = $codec->decode($bytes);

        if (defined $decode_err) {
            # Choose PARSE_ERROR vs INVALID_REQUEST based on the error string.
            # Codec implementations should include "parse error" in the string
            # for serialization failures and "invalid request" for shape failures.
            my $code;
            if ($decode_err =~ /parse error/i) {
                $code = PARSE_ERROR;
            } else {
                $code = INVALID_REQUEST;
            }
            my $label = ($code == PARSE_ERROR) ? 'Parse error' : 'Invalid Request';
            my $err = make_error(undef, $code, $label, $decode_err);
            $framer->write_frame($codec->encode($err));
            next;
        }

        # Step 6: dispatch
        $self->_dispatch($msg);
    }
}

1;

__END__

=head1 NAME

CodingAdventures::Rpc::Server — Codec-agnostic RPC request/notification dispatch server

=head1 SYNOPSIS

  use CodingAdventures::Rpc::Server;

  my $server = CodingAdventures::Rpc::Server->new(
      codec  => $my_codec,    # anything with encode() and decode()
      framer => $my_framer,   # anything with read_frame() and write_frame()
  );

  $server->on_request('ping', sub {
      my ($id, $params) = @_;
      return { pong => 1 };
  });

  $server->on_notification('log', sub {
      my ($params) = @_;
      warn $params->{message};
  });

  $server->serve;   # blocks until EOF

=head1 DESCRIPTION

Drives a codec-agnostic read-dispatch-write loop. It knows nothing about
JSON, Content-Length headers, or any specific wire format. Those concerns
belong to the codec and framer objects injected at construction time.

=head1 METHODS

=over 4

=item new(codec => $codec, framer => $framer)

Construct a server. Both C<codec> and C<framer> are required.

=item on_request($method, $coderef)

Register a handler for the named request method. The coderef receives
C<($id, $params)> and should return either a result value or a hashref
C<{ code => $int, message => $str }> to signal an error.
Returns C<$self> for method chaining.

=item on_notification($method, $coderef)

Register a handler for the named notification method. The coderef receives
C<($params)>. Return value is ignored. Returns C<$self> for chaining.

=item serve()

Blocking loop. Reads frames, decodes messages, dispatches to handlers, and
writes responses. Returns on clean EOF. Handler exceptions are caught and
converted to C<-32603 Internal error> responses; they never kill the server.

=back

=head1 SEE ALSO

L<CodingAdventures::Rpc>, L<CodingAdventures::Rpc::Client>,
L<CodingAdventures::Rpc::Codec>, L<CodingAdventures::Rpc::Framer>

=cut
