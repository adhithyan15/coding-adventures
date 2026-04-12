package CodingAdventures::JsonRpc::Server;

# ============================================================================
# CodingAdventures::JsonRpc::Server — JSON-RPC 2.0 dispatch server
# ============================================================================
#
# The Server combines a MessageReader and MessageWriter with a handler
# dispatch table.  It drives the read-dispatch-write loop.
#
# # Handler contract
#
#   request handler:
#     sub { my ($id, $params) = @_; return $result_or_error }
#     If the return value is a hashref with `code` (integer) and `message`
#     (string), it is treated as a ResponseError and sent as an error response.
#     Otherwise the return value is used as the `result` field.
#
#   notification handler:
#     sub { my ($params) = @_ }
#     Return value is ignored.  Per spec, the server MUST NOT send a response.
#
# # Server loop
#
# `serve()` reads messages one at a time until EOF or a stream error.
# For each message:
#   - Request:      dispatch to handler → send response
#   - Notification: dispatch to handler if registered; ignore if not
#   - Response:     drop (pure-server mode; no pending requests)
#   - Parse error:  send PARSE_ERROR or INVALID_REQUEST response with null id
#
# # Usage example
#
#   my $server = CodingAdventures::JsonRpc::Server->new(\*STDIN, \*STDOUT);
#
#   $server->on_request('initialize', sub {
#       my ($id, $params) = @_;
#       return { capabilities => { hoverProvider => JSON::PP::true } };
#   });
#
#   $server->on_notification('textDocument/didOpen', sub {
#       my ($params) = @_;
#       # parse $params->{textDocument}{text} ...
#   });
#
#   $server->serve;    # blocks until stdin closes

use strict;
use warnings;

use CodingAdventures::JsonRpc::Reader  ();
use CodingAdventures::JsonRpc::Writer  ();
use CodingAdventures::JsonRpc::Message qw(response error_response);
use CodingAdventures::JsonRpc::Errors  qw(:all);

our $VERSION = '0.01';

# ---------------------------------------------------------------------------
# new($in_fh, $out_fh) → Server
#
# Create a new Server.  Both filehandles must be in binary mode (`:raw`).
# ---------------------------------------------------------------------------

sub new {
    my ($class, $in_fh, $out_fh) = @_;
    return bless {
        reader        => CodingAdventures::JsonRpc::Reader->new($in_fh),
        writer        => CodingAdventures::JsonRpc::Writer->new($out_fh),
        req_handlers  => {},   # method → coderef
        notif_handlers => {},  # method → coderef
    }, $class;
}

# ---------------------------------------------------------------------------
# on_request($method, $handler) → $self
#
# Register a handler for the given request method.  Returns $self for
# method chaining.
#
# The handler receives ($id, $params) and must return a result value or
# a ResponseError hashref { code => int, message => str }.
# ---------------------------------------------------------------------------

sub on_request {
    my ($self, $method, $handler) = @_;
    $self->{req_handlers}{$method} = $handler;
    return $self;
}

# ---------------------------------------------------------------------------
# on_notification($method, $handler) → $self
#
# Register a handler for the given notification method.  Returns $self.
# The handler receives ($params) and its return value is ignored.
# ---------------------------------------------------------------------------

sub on_notification {
    my ($self, $method, $handler) = @_;
    $self->{notif_handlers}{$method} = $handler;
    return $self;
}

# ---------------------------------------------------------------------------
# _is_response_error($value) → bool
#
# Return true when $value is a hashref with numeric `code` and string
# `message` keys — the ResponseError shape.
# ---------------------------------------------------------------------------

sub _is_response_error {
    my ($v) = @_;
    return 0 unless ref($v) eq 'HASH';
    return 0 unless defined $v->{code} && $v->{code} =~ /^-?\d+$/;
    return 0 unless defined $v->{message} && !ref($v->{message});
    return 1;
}

# ---------------------------------------------------------------------------
# dispatch($msg)
#
# Internal: process one decoded message and write any response.
# ---------------------------------------------------------------------------

sub dispatch {
    my ($self, $msg) = @_;
    my $type   = $msg->{_type};
    my $writer = $self->{writer};

    if ($type eq 'request') {
        my $id     = $msg->{id};
        my $method = $msg->{method};
        my $params = $msg->{params};

        my $handler = $self->{req_handlers}{$method};
        unless (defined $handler) {
            # -32601 Method not found
            $writer->write_message(
                error_response($id, {
                    code    => METHOD_NOT_FOUND,
                    message => "Method not found: $method",
                })
            );
            return;
        }

        # Call the handler, catching any Perl die/croak.
        my $result = eval { $handler->($id, $params) };
        if ($@) {
            my $errmsg = $@;
            $errmsg =~ s/\s+$//;   # strip trailing newline from die()
            $writer->write_message(
                error_response($id, {
                    code    => INTERNAL_ERROR,
                    message => 'Internal error',
                    data    => $errmsg,
                })
            );
            return;
        }

        # If the handler returned a ResponseError shape, send an error response.
        if (_is_response_error($result)) {
            $writer->write_message(error_response($id, $result));
        } else {
            $writer->write_message(response($id, $result));
        }

    } elsif ($type eq 'notification') {
        # The server MUST NOT send a response to a Notification.
        # Silently ignore unregistered notification methods.
        my $handler = $self->{notif_handlers}{$msg->{method}};
        if (defined $handler) {
            eval { $handler->($msg->{params}) };
            # Errors in notification handlers are silently swallowed.
        }

    } elsif ($type eq 'response') {
        # A Response to a request we sent (client-side usage).
        # In a pure server implementation there are no pending requests,
        # so we silently drop incoming Responses.
        return;
    }
}

# ---------------------------------------------------------------------------
# serve()
#
# Blocking read-dispatch-write loop.  Returns when the input stream reaches
# EOF (client closed stdin).
# ---------------------------------------------------------------------------

sub serve {
    my ($self) = @_;
    my $reader = $self->{reader};
    my $writer = $self->{writer};

    while (1) {
        my ($msg, $err) = $reader->read_message;

        # Clean EOF: client closed stdin.
        last if !defined $msg && !defined $err;

        if (defined $err) {
            # Framing or JSON error: send an error response with null id,
            # then continue reading (the stream may still be valid).
            my $code = ($err =~ /parse error/) ? PARSE_ERROR : INVALID_REQUEST;
            my $label = ($code == PARSE_ERROR) ? 'Parse error' : 'Invalid Request';
            $writer->write_message(
                error_response(undef, {
                    code    => $code,
                    message => $label,
                    data    => $err,
                })
            );
            next;
        }

        $self->dispatch($msg);
    }
}

1;

__END__

=head1 NAME

CodingAdventures::JsonRpc::Server — JSON-RPC 2.0 request/notification dispatch server

=head1 SYNOPSIS

  use CodingAdventures::JsonRpc::Server;

  my $server = CodingAdventures::JsonRpc::Server->new(\*STDIN, \*STDOUT);

  $server->on_request('ping', sub {
      my ($id, $params) = @_;
      return { pong => 1 };
  });

  $server->on_notification('log', sub {
      my ($params) = @_;
      warn $params->{message};
  });

  $server->serve;

=head1 DESCRIPTION

Combines a C<MessageReader> and C<MessageWriter> with a handler dispatch table.
Runs a blocking loop until EOF.

=head1 METHODS

=over 4

=item new($in_fh, $out_fh)

Create a server.  Both handles should be in binary mode.

=item on_request($method, $coderef)

Register a request handler.  The coderef receives C<($id, $params)> and must
return either a result value or a ResponseError hashref
C<{ code => $int, message => $str }>.  Returns C<$self> for chaining.

=item on_notification($method, $coderef)

Register a notification handler.  The coderef receives C<($params)>.
Return value is ignored.  Returns C<$self> for chaining.

=item serve()

Blocking loop.  Reads, dispatches, and writes until EOF.

=back

=cut
