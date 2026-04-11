package CodingAdventures::JsonRpc::Message;

# ============================================================================
# CodingAdventures::JsonRpc::Message — JSON-RPC 2.0 message constructors
# ============================================================================
#
# JSON-RPC 2.0 defines four message types.  All carry `"jsonrpc": "2.0"`:
#
#   Request      — has `id` and `method`; expects a response
#   Response     — has `id` and `result` (or `error`)
#   Notification — has `method` but no `id`; server must NOT respond
#
# Example JSON for each type:
#
#   Request:
#     {"jsonrpc":"2.0","id":1,"method":"textDocument/hover","params":{...}}
#
#   Response (success):
#     {"jsonrpc":"2.0","id":1,"result":{"contents":"..."}}
#
#   Response (error):
#     {"jsonrpc":"2.0","id":1,"error":{"code":-32601,"message":"Method not found"}}
#
#   Notification:
#     {"jsonrpc":"2.0","method":"textDocument/didOpen","params":{...}}
#
# Discrimination rule:
#   has id  AND method              → request
#   has method, no id               → notification
#   has id  AND (result OR error)   → response
#
# This module provides plain-hashref constructors.  Hashrefs are idiomatic
# Perl data structures; they are cheap to create and easy to inspect.

use strict;
use warnings;
use JSON::PP qw(encode_json decode_json);

use Exporter 'import';

our @EXPORT_OK = qw(
    request
    response
    error_response
    notification
    parse_message
    message_to_json
    classify_message
);
our %EXPORT_TAGS = ( all => \@EXPORT_OK );

our $VERSION = '0.01';

# ---------------------------------------------------------------------------
# Message constructors
# ---------------------------------------------------------------------------

# request($id, $method, $params)
#
# Build a Request hashref.  `$params` is optional (omit or pass undef).
#
# Example:
#   my $req = request(1, 'textDocument/hover', { position => {line=>0} });
#   # {"jsonrpc":"2.0","id":1,"method":"textDocument/hover","params":{...}}

sub request {
    my ($id, $method, $params) = @_;
    my $msg = {
        jsonrpc => '2.0',
        id      => $id,
        method  => $method,
    };
    $msg->{params} = $params if defined $params;
    return $msg;
}

# response($id, $result)
#
# Build a success Response hashref.
#
# Example:
#   my $resp = response(1, { capabilities => {} });
#   # {"jsonrpc":"2.0","id":1,"result":{"capabilities":{}}}

sub response {
    my ($id, $result) = @_;
    return {
        jsonrpc => '2.0',
        id      => $id,
        result  => $result,
    };
}

# error_response($id, $error_hashref)
#
# Build an error Response hashref.  `$error_hashref` must have `code`
# (integer) and `message` (string); `data` is optional.
#
# Example:
#   my $err = error_response(1, { code => -32601, message => 'Method not found' });
#   # {"jsonrpc":"2.0","id":1,"error":{"code":-32601,"message":"Method not found"}}

sub error_response {
    my ($id, $error) = @_;
    return {
        jsonrpc => '2.0',
        id      => $id,
        error   => $error,
    };
}

# notification($method, $params)
#
# Build a Notification hashref (no `id`).  `$params` is optional.
#
# Notifications are one-way: the server must NOT send a response.
#
# Example:
#   my $notif = notification('textDocument/didOpen', { textDocument => {...} });

sub notification {
    my ($method, $params) = @_;
    my $msg = {
        jsonrpc => '2.0',
        method  => $method,
    };
    $msg->{params} = $params if defined $params;
    return $msg;
}

# ---------------------------------------------------------------------------
# classify_message($hashref) → string | undef
#
# Examine a decoded hashref and return the message type:
#   'request'      — has id and method
#   'notification' — has method, no id
#   'response'     — has id and (result or error key present)
#   undef          — does not look like a valid JSON-RPC 2.0 message
#
# We check for the explicit presence of the 'error' key using exists() rather
# than truthiness, because an error value of 0 (unlikely but possible in
# custom extensions) would be falsely missed by a simple `if $msg->{error}`.
# ---------------------------------------------------------------------------

sub classify_message {
    my ($msg) = @_;
    return undef unless ref($msg) eq 'HASH';
    return undef unless ($msg->{jsonrpc} // '') eq '2.0';

    my $has_id     = exists $msg->{id};
    my $has_method = exists $msg->{method};
    my $has_result = exists $msg->{result};
    my $has_error  = exists $msg->{error};

    if ($has_id && $has_method) {
        return 'request';
    } elsif ($has_method && !$has_id) {
        return 'notification';
    } elsif ($has_id && ($has_result || $has_error)) {
        return 'response';
    }
    return undef;
}

# ---------------------------------------------------------------------------
# parse_message($json_string) → ($hashref, undef) | (undef, $error)
#
# Decode a JSON string and validate it as a JSON-RPC 2.0 message.
# Returns a two-element list:
#   - On success: ($hashref, undef)   — hashref has a `_type` key added
#   - On JSON error: (undef, $error)  — $error starts with "parse error:"
#   - On invalid message: (undef, $error) — $error starts with "invalid request:"
# ---------------------------------------------------------------------------

sub parse_message {
    my ($json_str) = @_;

    # Attempt to decode the JSON.
    my $decoded = eval { decode_json($json_str) };
    if ($@) {
        # $@ contains the JSON::PP error string.
        return (undef, "parse error: $@");
    }

    # Validate the decoded value is a JSON-RPC 2.0 message.
    my $type = classify_message($decoded);
    unless (defined $type) {
        return (undef, "invalid request: not a JSON-RPC 2.0 message");
    }

    # Annotate with the classified type for the caller's convenience.
    $decoded->{_type} = $type;
    return ($decoded, undef);
}

# ---------------------------------------------------------------------------
# message_to_json($hashref) → $json_string
#
# Encode a message hashref to a compact JSON string.
# Strips the internal `_type` key before encoding so it does not appear
# on the wire.
# ---------------------------------------------------------------------------

sub message_to_json {
    my ($msg) = @_;

    # Build a copy without the internal _type annotation.
    my %copy = %$msg;
    delete $copy{_type};

    return encode_json(\%copy);
}

1;

__END__

=head1 NAME

CodingAdventures::JsonRpc::Message — JSON-RPC 2.0 message constructors and utilities

=head1 SYNOPSIS

  use CodingAdventures::JsonRpc::Message qw(:all);

  my $req   = request(1, 'textDocument/hover', { position => {line=>0} });
  my $resp  = response(1, { contents => '...' });
  my $err   = error_response(1, { code => -32601, message => 'Not found' });
  my $notif = notification('textDocument/didChange', { text => '...' });

  my ($msg, $error) = parse_message($json_string);
  my $json = message_to_json($hashref);
  my $type = classify_message($hashref);   # 'request' | 'notification' | 'response' | undef

=cut
