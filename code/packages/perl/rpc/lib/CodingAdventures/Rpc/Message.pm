package CodingAdventures::Rpc::Message;

# ============================================================================
# CodingAdventures::Rpc::Message — Codec-agnostic RPC message constructors
# ============================================================================
#
# An RPC message is a data structure that flows between client and server.
# There are four kinds:
#
#   request      — client asks server to execute a named method and return a result
#   response     — server returns the successful result of a request
#   error        — server returns a failure result for a request
#   notification — client (or server) fires a one-way event; no response expected
#
# In statically-typed languages like Go and Rust, each kind is a distinct
# struct. In Perl, we represent all four as blessed hashrefs. The `kind` key
# distinguishes them:
#
#   request:      { kind=>'request',      id=>$id, method=>$method, params=>$params }
#   response:     { kind=>'response',     id=>$id, result=>$result }
#   error:        { kind=>'error',        id=>$id, code=>$code, message=>$msg, data=>$data }
#   notification: { kind=>'notification', method=>$method, params=>$params }
#
# # Why blessed hashrefs?
#
# Blessing the hashref into the package `CodingAdventures::Rpc::Message` lets
# callers use `ref($msg) eq 'CodingAdventures::Rpc::Message'` to distinguish
# RPC messages from plain hashes. The `kind` field then acts as a discriminant
# (like an enum variant) so the dispatcher can act on the correct branch.
#
# # The `id` field
#
# Request ids can be integers or strings. The client generates them; they are
# echoed back in the matching response so the client can correlate the reply.
# Notifications have no id (the server MUST NOT reply).
# Error responses with a null id are only used when the original request was
# so malformed that its id could not be extracted.
#
# # The `params` field
#
# `params` is whatever the codec decoded: a hashref, arrayref, or undef. The
# rpc layer never inspects params — it passes them straight to the handler.

use strict;
use warnings;

use Exporter 'import';

our @EXPORT_OK = qw(
    make_request
    make_response
    make_error
    make_notification
);
our %EXPORT_TAGS = ( all => \@EXPORT_OK );

our $VERSION = '0.01';

# ---------------------------------------------------------------------------
# make_request($id, $method, $params) → blessed hashref
#
# Construct a request message.  `$params` is optional and may be any value
# that the codec knows how to encode (hashref, arrayref, undef, etc.).
#
# The request carries an `id` so the server can include it in the response.
# When the server replies, the client matches the response id against the
# id it sent to find the right waiting request.
# ---------------------------------------------------------------------------

sub make_request {
    my ($id, $method, $params) = @_;
    my %msg = (
        kind   => 'request',
        id     => $id,
        method => $method,
    );
    $msg{params} = $params if @_ >= 3;
    return bless \%msg, __PACKAGE__;
}

# ---------------------------------------------------------------------------
# make_response($id, $result) → blessed hashref
#
# Construct a success response message.  `$result` is whatever the handler
# returned — a hashref, a string, a number, undef, etc.
#
# The `id` must echo the id from the corresponding request exactly.
# ---------------------------------------------------------------------------

sub make_response {
    my ($id, $result) = @_;
    return bless {
        kind   => 'response',
        id     => $id,
        result => $result,
    }, __PACKAGE__;
}

# ---------------------------------------------------------------------------
# make_error($id, $code, $message, $data) → blessed hashref
#
# Construct an error response message.  `$code` is one of the constants from
# CodingAdventures::Rpc::Errors (or a server-defined integer in -32000..-32099).
# `$message` is a human-readable string.  `$data` is optional extra context.
#
# Use `$id = undef` when the original request was so malformed that its id
# could not be extracted (e.g., codec parse failure).
# ---------------------------------------------------------------------------

sub make_error {
    my ($id, $code, $message, $data) = @_;
    my %msg = (
        kind    => 'error',
        id      => $id,
        code    => $code,
        message => $message,
    );
    $msg{data} = $data if @_ >= 4;
    return bless \%msg, __PACKAGE__;
}

# ---------------------------------------------------------------------------
# make_notification($method, $params) → blessed hashref
#
# Construct a notification message.  Notifications have no id and expect no
# response.  They are used for fire-and-forget events: the sender does not
# wait for a reply, and the receiver MUST NOT send one.
# ---------------------------------------------------------------------------

sub make_notification {
    my ($method, $params) = @_;
    my %msg = (
        kind   => 'notification',
        method => $method,
    );
    $msg{params} = $params if @_ >= 2;
    return bless \%msg, __PACKAGE__;
}

1;

__END__

=head1 NAME

CodingAdventures::Rpc::Message — Codec-agnostic RPC message constructors

=head1 SYNOPSIS

  use CodingAdventures::Rpc::Message qw(:all);

  # A client sending a request:
  my $req = make_request(1, 'ping', { echo => 'hello' });

  # A server sending back a success response:
  my $resp = make_response(1, { pong => 'hello' });

  # A server sending back an error response:
  use CodingAdventures::Rpc::Errors qw(METHOD_NOT_FOUND);
  my $err = make_error(1, METHOD_NOT_FOUND, 'Method not found');

  # A client (or server) sending a one-way event:
  my $notif = make_notification('log', { level => 'info', msg => 'started' });

  # Dispatching on kind:
  if ($msg->{kind} eq 'request') { ... }

=head1 DESCRIPTION

Provides constructor functions for the four RPC message kinds. Each message
is a blessed hashref in the C<CodingAdventures::Rpc::Message> package. The
C<kind> key acts as a discriminant.

=head1 FUNCTIONS

=over 4

=item make_request($id, $method, $params)

Construct a request message. C<$params> is optional.

=item make_response($id, $result)

Construct a success response. C<$result> is the handler's return value.

=item make_error($id, $code, $message [, $data])

Construct an error response. C<$id> may be C<undef> for codec-level errors.
C<$code> is an integer error code. C<$data> is optional extra detail.

=item make_notification($method, $params)

Construct a notification (fire-and-forget). C<$params> is optional.

=back

=head1 SEE ALSO

L<CodingAdventures::Rpc::Errors>, L<CodingAdventures::Rpc::Server>,
L<CodingAdventures::Rpc::Client>

=cut
