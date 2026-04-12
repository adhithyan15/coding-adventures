use strict;
use warnings;
use Test2::V0;

# ============================================================================
# CodingAdventures::Rpc — comprehensive test suite
# ============================================================================
#
# Test coverage:
#   1.  Module loads (all sub-modules)
#   2.  Error constants
#   3.  Message constructors: make_request, make_response, make_error, make_notification
#   4.  Message constructor edge cases (optional params, undef id)
#   5.  MockCodec: encode/decode round-trip
#   6.  MockFramer: read_frame/write_frame round-trip, EOF, error
#   7.  Server: dispatches request → result response
#   8.  Server: dispatches request → handler-returned error response
#   9.  Server: -32601 for unknown request method
#   10. Server: -32603 when handler dies (panic recovery)
#   11. Server: dispatches notification, no response written
#   12. Server: unknown notification silently dropped (no response)
#   13. Server: on_request and on_notification are chainable
#   14. Server: codec decode error → error response
#   15. Server: framing error → error response
#   16. Client: request() sends, receives result
#   17. Client: request() receives error response
#   18. Client: request() on EOF before response
#   19. Client: notify() sends without waiting
#   20. Client: on_notification() called for server-push during request()
#   21. Client: request ids are monotonically increasing
#   22. Client: on_notification is chainable
#
# Per lessons.md: Test2::V0 does not have use_ok.
# Use eval+require instead.

# ============================================================================
# MockCodec
# ============================================================================
#
# A minimal codec for testing that does not depend on any serialization
# library. It serializes RpcMessage blessed hashrefs to a simple
# pipe-delimited string and deserializes them back.
#
# Format:
#   request:      "req|$id|$method|$params_str"
#   response:     "resp|$id|$result_str"
#   error:        "err|$id|$code|$message|$data_str"
#   notification: "notif|$method|$params_str"
#
# $params_str and $result_str are serialized as key=value pairs joined by
# commas, or the literal string "undef" for undef.
#
# This is an intentionally simple format for tests only. Real codecs would
# use JSON, MessagePack, etc.

package MockCodec;
use parent 'CodingAdventures::Rpc::Codec';
use CodingAdventures::Rpc::Message qw(:all);

# Encode a scalar value to a compact string.
# hashref: "\x01"-separated "k\x02v" pairs  |  undef: "undef"  |  scalar: stringified
#
# We deliberately avoid comma and equals as separators — test values like
# "Hello, World" contain commas, which would break a comma-separated format.
# Non-printable ASCII \x01 (SOH) and \x02 (STX) never appear in test values.
sub _enc_val {
    my ($v) = @_;
    return 'undef' unless defined $v;
    if (ref($v) eq 'HASH') {
        return join("\x01", map { "$_\x02" . (defined $v->{$_} ? $v->{$_} : 'undef') }
                            sort keys %$v);
    }
    return "$v";
}

# Decode a compact string back to a value (hashref or undef).
sub _dec_val {
    my ($s) = @_;
    return undef if $s eq 'undef';
    if ($s =~ /\x02/) {
        my %h = map {
            my ($k, $v) = split /\x02/, $_, 2;
            ($k, (!defined $v || $v eq 'undef') ? undef : $v)
        } split /\x01/, $s;
        return \%h;
    }
    return $s;
}

sub encode {
    my ($self, $msg) = @_;
    my $kind = $msg->{kind};
    if ($kind eq 'request') {
        my $id     = defined $msg->{id} ? $msg->{id} : 'null';
        my $params = _enc_val($msg->{params});
        return "req|$id|$msg->{method}|$params";
    }
    if ($kind eq 'response') {
        my $id     = defined $msg->{id} ? $msg->{id} : 'null';
        my $result = _enc_val($msg->{result});
        return "resp|$id|$result";
    }
    if ($kind eq 'error') {
        my $id   = defined $msg->{id} ? $msg->{id} : 'null';
        my $data = _enc_val($msg->{data});
        return "err|$id|$msg->{code}|$msg->{message}|$data";
    }
    if ($kind eq 'notification') {
        my $params = _enc_val($msg->{params});
        return "notif|$msg->{method}|$params";
    }
    die "MockCodec: unknown kind '$kind'\n";
}

sub decode {
    my ($self, $bytes) = @_;
    return (undef, "parse error: empty input") unless defined $bytes && length $bytes;

    my @parts = split /\|/, $bytes, -1;
    my $kind = $parts[0];

    if ($kind eq 'req') {
        my ($k, $id, $method, $params_str) = @parts;
        $id = undef if $id eq 'null';
        my $msg = make_request($id, $method, _dec_val($params_str));
        return ($msg, undef);
    }
    if ($kind eq 'resp') {
        my ($k, $id, $result_str) = @parts;
        $id = undef if $id eq 'null';
        my $msg = make_response($id, _dec_val($result_str));
        return ($msg, undef);
    }
    if ($kind eq 'err') {
        my ($k, $id, $code, $message, $data_str) = @parts;
        $id = undef if $id eq 'null';
        my $msg = make_error($id, $code+0, $message, _dec_val($data_str));
        return ($msg, undef);
    }
    if ($kind eq 'notif') {
        my ($k, $method, $params_str) = @parts;
        my $msg = make_notification($method, _dec_val($params_str));
        return ($msg, undef);
    }

    # Simulate the two decode error cases:
    #   "BADPARSE" → parse error
    #   anything else unrecognized → invalid request
    if ($bytes =~ /BADPARSE/) {
        return (undef, "parse error: unrecognised input");
    }
    return (undef, "invalid request: unrecognised message kind '$kind'");
}

# ============================================================================
# MockFramer
# ============================================================================
#
# An in-memory framer backed by arrayrefs.
#
# Construction:
#   MockFramer->new(
#       frames  => \@frames_to_serve,    # arrayrefs of byte strings to return
#       written => \@output_accumulator, # arrayref; write_frame() appends here
#   )
#
# read_frame() pops from the front of @frames.
# A frame value of undef means "clean EOF".
# A frame value of \"error:..." means "return a framing error".

package MockFramer;
use parent 'CodingAdventures::Rpc::Framer';

sub new {
    my ($class, %args) = @_;
    return bless {
        frames  => $args{frames}  // [],
        written => $args{written} // [],
    }, $class;
}

sub read_frame {
    my ($self) = @_;
    my $frames = $self->{frames};
    return (undef, undef) unless @$frames;          # natural EOF

    my $f = shift @$frames;
    return (undef, undef) unless defined $f;         # sentinel EOF

    # A scalarref encodes a framing error string.
    if (ref($f) eq 'SCALAR') {
        return (undef, $$f);
    }

    return ($f, undef);
}

sub write_frame {
    my ($self, $bytes) = @_;
    push @{ $self->{written} }, $bytes;
}

# ============================================================================
# Back to the test script
# ============================================================================

package main;

# ============================================================================
# 1. Module loads
# ============================================================================

subtest 'modules load' => sub {
    ok( eval { require CodingAdventures::Rpc; 1 },
        'CodingAdventures::Rpc loads' )
        or diag $@;
    ok( eval { require CodingAdventures::Rpc::Errors; 1 },
        'Errors loads' );
    ok( eval { require CodingAdventures::Rpc::Message; 1 },
        'Message loads' );
    ok( eval { require CodingAdventures::Rpc::Codec; 1 },
        'Codec loads' );
    ok( eval { require CodingAdventures::Rpc::Framer; 1 },
        'Framer loads' );
    ok( eval { require CodingAdventures::Rpc::Server; 1 },
        'Server loads' );
    ok( eval { require CodingAdventures::Rpc::Client; 1 },
        'Client loads' );
};

# ============================================================================
# 2. Error constants
# ============================================================================

use CodingAdventures::Rpc::Errors qw(:all);

subtest 'error constants' => sub {
    is( PARSE_ERROR,      -32700, 'PARSE_ERROR      = -32700' );
    is( INVALID_REQUEST,  -32600, 'INVALID_REQUEST  = -32600' );
    is( METHOD_NOT_FOUND, -32601, 'METHOD_NOT_FOUND = -32601' );
    is( INVALID_PARAMS,   -32602, 'INVALID_PARAMS   = -32602' );
    is( INTERNAL_ERROR,   -32603, 'INTERNAL_ERROR   = -32603' );
};

# ============================================================================
# 3. Message constructors
# ============================================================================

use CodingAdventures::Rpc::Message qw(:all);

subtest 'make_request' => sub {
    my $r = make_request(1, 'ping', { x => 1 });
    is( $r->{kind},     'request', 'kind = request' );
    is( $r->{id},       1,         'id = 1'         );
    is( $r->{method},   'ping',    'method = ping'  );
    is( $r->{params}{x}, 1,        'params.x = 1'  );
    ok( ref($r) eq 'CodingAdventures::Rpc::Message', 'blessed correctly' );
};

subtest 'make_request without params' => sub {
    my $r = make_request(2, 'init');
    ok( !exists $r->{params}, 'params key absent when not given' );
};

subtest 'make_response' => sub {
    my $r = make_response(1, { ok => 1 });
    is( $r->{kind},       'response', 'kind = response' );
    is( $r->{id},         1,          'id = 1'          );
    is( $r->{result}{ok}, 1,          'result.ok = 1'   );
};

subtest 'make_error' => sub {
    my $e = make_error(1, -32601, 'Method not found', 'extra');
    is( $e->{kind},    'error',           'kind = error'    );
    is( $e->{id},      1,                 'id = 1'          );
    is( $e->{code},    -32601,            'code = -32601'   );
    is( $e->{message}, 'Method not found','message'         );
    is( $e->{data},    'extra',           'data = extra'    );
};

subtest 'make_error without data' => sub {
    my $e = make_error(1, -32600, 'Invalid Request');
    ok( !exists $e->{data}, 'data key absent when not given' );
};

subtest 'make_error with null id' => sub {
    my $e = make_error(undef, -32700, 'Parse error');
    ok( !defined $e->{id}, 'id is undef' );
};

subtest 'make_notification' => sub {
    my $n = make_notification('didChange', { uri => 'file:///a' });
    is( $n->{kind},        'notification', 'kind = notification'  );
    is( $n->{method},      'didChange',    'method = didChange'   );
    is( $n->{params}{uri}, 'file:///a',    'params.uri'           );
    ok( !exists $n->{id},  'no id key'                            );
};

subtest 'make_notification without params' => sub {
    my $n = make_notification('shutdown');
    ok( !exists $n->{params}, 'params key absent when not given' );
};

# ============================================================================
# 4. MockCodec round-trip
# ============================================================================

subtest 'MockCodec: request round-trip' => sub {
    my $codec = MockCodec->new;
    my $msg = make_request(7, 'add', { a => 3, b => 4 });
    my $bytes = $codec->encode($msg);
    my ($decoded, $err) = $codec->decode($bytes);
    ok( !defined $err,          'no error'               );
    is( $decoded->{kind},       'request', 'kind'        );
    is( $decoded->{id},         7,         'id'          );
    is( $decoded->{method},     'add',     'method'      );
    is( $decoded->{params}{a},  3,         'params.a'   );
};

subtest 'MockCodec: response round-trip' => sub {
    my $codec = MockCodec->new;
    my $msg = make_response(3, { sum => 7 });
    my $bytes = $codec->encode($msg);
    my ($decoded, $err) = $codec->decode($bytes);
    ok( !defined $err,              'no error'    );
    is( $decoded->{kind},           'response',   'kind'   );
    is( $decoded->{result}{sum},    7,            'result' );
};

subtest 'MockCodec: error round-trip' => sub {
    my $codec = MockCodec->new;
    my $msg = make_error(1, -32601, 'Method not found');
    my $bytes = $codec->encode($msg);
    my ($decoded, $err) = $codec->decode($bytes);
    ok( !defined $err,           'no error'    );
    is( $decoded->{kind},        'error',      'kind'    );
    is( $decoded->{code},        -32601,       'code'    );
    is( $decoded->{message},     'Method not found', 'message' );
};

subtest 'MockCodec: notification round-trip' => sub {
    my $codec = MockCodec->new;
    my $msg = make_notification('log', { level => 'info' });
    my $bytes = $codec->encode($msg);
    my ($decoded, $err) = $codec->decode($bytes);
    ok( !defined $err,             'no error'   );
    is( $decoded->{kind},          'notification', 'kind'  );
    is( $decoded->{method},        'log',          'method');
    is( $decoded->{params}{level}, 'info',         'params');
};

subtest 'MockCodec: decode parse error' => sub {
    my $codec = MockCodec->new;
    my ($msg, $err) = $codec->decode('BADPARSE_INPUT');
    ok( !defined $msg,                  'msg undef'     );
    ok( defined $err,                   'err defined'   );
    like( $err, qr/parse error/i,       'mentions parse error' );
};

subtest 'MockCodec: decode invalid request' => sub {
    my $codec = MockCodec->new;
    my ($msg, $err) = $codec->decode('unknown|data|here');
    ok( !defined $msg,                  'msg undef'     );
    ok( defined $err,                   'err defined'   );
    like( $err, qr/invalid request/i,   'mentions invalid request' );
};

# ============================================================================
# 5. MockFramer
# ============================================================================

subtest 'MockFramer: read_frame returns frames in order' => sub {
    my @written;
    my $framer = MockFramer->new(
        frames  => ['hello', 'world'],
        written => \@written,
    );
    my ($f1, $e1) = $framer->read_frame;
    my ($f2, $e2) = $framer->read_frame;
    my ($f3, $e3) = $framer->read_frame;
    is( $f1, 'hello',  'first frame'     );
    is( $f2, 'world',  'second frame'    );
    ok( !defined $f3 && !defined $e3, 'EOF after last frame' );
};

subtest 'MockFramer: write_frame accumulates output' => sub {
    my @written;
    my $framer = MockFramer->new(frames => [], written => \@written);
    $framer->write_frame('abc');
    $framer->write_frame('def');
    is( $written[0], 'abc', 'first write' );
    is( $written[1], 'def', 'second write');
};

subtest 'MockFramer: framing error sentinel' => sub {
    my $framer = MockFramer->new(
        frames => [\ 'bad framing header'],
    );
    my ($bytes, $err) = $framer->read_frame;
    ok( !defined $bytes, 'bytes undef on error' );
    ok( defined $err,    'err defined'          );
    is( $err, 'bad framing header', 'error message' );
};

# ============================================================================
# Helpers for server / client tests
# ============================================================================

# build_server($frames_aref, $written_aref) → server
# Creates a server with MockCodec and MockFramer, registering no handlers.
sub build_server {
    my ($frames, $written) = @_;
    return CodingAdventures::Rpc::Server->new(
        codec  => MockCodec->new,
        framer => MockFramer->new(frames => $frames, written => $written),
    );
}

# encode($msg) → string — shorthand using the MockCodec
my $_codec = MockCodec->new;
sub enc { $_codec->encode($_[0]) }

# ============================================================================
# 6. Server: request → result response
# ============================================================================

subtest 'Server: dispatches request and writes result response' => sub {
    my @written;
    my $req = enc(make_request(1, 'greet', { name => 'World' }));
    my $server = build_server([$req], \@written);
    $server->on_request('greet', sub {
        my ($id, $params) = @_;
        return { greeting => "Hello, $params->{name}" };
    });
    $server->serve;

    is( scalar @written, 1, 'one response written' );
    my ($resp, $err) = $_codec->decode($written[0]);
    ok( !defined $err,            'response decoded ok' );
    is( $resp->{kind},  'response', 'kind = response'   );
    is( $resp->{id},    1,          'id echoed'         );
    like( $resp->{result}{greeting}, qr/Hello, World/, 'greeting in result' );
};

# ============================================================================
# 7. Server: request → handler-returned error response
# ============================================================================

subtest 'Server: handler-returned error shape becomes error response' => sub {
    my @written;
    my $req = enc(make_request(2, 'validate', {}));
    my $server = build_server([$req], \@written);
    $server->on_request('validate', sub {
        return { code => INVALID_PARAMS, message => 'bad params' };
    });
    $server->serve;

    is( scalar @written, 1, 'one response' );
    my ($msg, $err) = $_codec->decode($written[0]);
    ok( !defined $err,          'decoded ok'       );
    is( $msg->{kind},  'error', 'kind = error'     );
    is( $msg->{code},  -32602,  'code = -32602'    );
    is( $msg->{id},    2,       'id echoed'        );
};

# ============================================================================
# 8. Server: -32601 for unknown method
# ============================================================================

subtest 'Server: sends -32601 for unknown request method' => sub {
    my @written;
    my $req = enc(make_request(3, 'nosuchmethod'));
    my $server = build_server([$req], \@written);
    # No handlers registered
    $server->serve;

    is( scalar @written, 1, 'one response' );
    my ($msg, $err) = $_codec->decode($written[0]);
    ok( !defined $err,          'decoded ok'     );
    is( $msg->{kind},  'error', 'kind = error'   );
    is( $msg->{code},  -32601,  'code = -32601'  );
};

# ============================================================================
# 9. Server: -32603 when handler dies (panic recovery)
# ============================================================================

subtest 'Server: -32603 when handler dies' => sub {
    my @written;
    my $req = enc(make_request(4, 'boom'));
    my $server = build_server([$req], \@written);
    $server->on_request('boom', sub { die "intentional test panic\n" });
    $server->serve;

    is( scalar @written, 1, 'one response despite panic' );
    my ($msg, $err) = $_codec->decode($written[0]);
    ok( !defined $err,          'decoded ok'     );
    is( $msg->{kind},  'error', 'kind = error'   );
    is( $msg->{code},  -32603,  'code = -32603'  );
};

# ============================================================================
# 10. Server: notification → handler called, no response written
# ============================================================================

subtest 'Server: dispatches notification, no response written' => sub {
    my @written;
    my $called = 0;
    my $notif = enc(make_notification('didChange', { uri => 'file:///a' }));
    my $server = build_server([$notif], \@written);
    $server->on_notification('didChange', sub { $called++ });
    $server->serve;

    is( $called,         1,   'handler called once'         );
    is( scalar @written, 0,   'no output for notification'  );
};

# ============================================================================
# 11. Server: unknown notification silently dropped
# ============================================================================

subtest 'Server: unknown notification silently dropped (no response)' => sub {
    my @written;
    my $notif = enc(make_notification('unregistered'));
    my $server = build_server([$notif], \@written);
    $server->serve;

    is( scalar @written, 0, 'no output for unregistered notification' );
};

# ============================================================================
# 12. Server: on_request and on_notification are chainable
# ============================================================================

subtest 'Server: on_request is chainable' => sub {
    my @written;
    my $server = build_server([], \@written);
    my $chain = $server
        ->on_request('a', sub {})
        ->on_request('b', sub {});
    is( $chain, $server, 'chaining returns self' );
};

subtest 'Server: on_notification is chainable' => sub {
    my @written;
    my $server = build_server([], \@written);
    my $chain = $server
        ->on_notification('x', sub {})
        ->on_notification('y', sub {});
    is( $chain, $server, 'chaining returns self' );
};

# ============================================================================
# 13. Server: codec decode error → error response with null id
# ============================================================================

subtest 'Server: codec parse error → error response null id' => sub {
    my @written;
    # Feed the server a raw string that MockCodec will reject as "parse error"
    my $server = build_server(['BADPARSE_GARBAGE'], \@written);
    $server->serve;

    is( scalar @written, 1, 'one error response written' );
    my ($msg, $err) = $_codec->decode($written[0]);
    ok( !defined $err,           'response decoded ok'  );
    is( $msg->{kind},   'error', 'kind = error'         );
    is( $msg->{code},   -32700,  'code = PARSE_ERROR'   );
    ok( !defined $msg->{id},     'id is null/undef'     );
};

subtest 'Server: codec invalid-request error → -32600 null id' => sub {
    my @written;
    my $server = build_server(['unknown|junk|here'], \@written);
    $server->serve;

    is( scalar @written, 1, 'one error response written' );
    my ($msg, $err) = $_codec->decode($written[0]);
    ok( !defined $err,           'response decoded ok'  );
    is( $msg->{kind},   'error', 'kind = error'         );
    is( $msg->{code},   -32600,  'code = INVALID_REQUEST');
    ok( !defined $msg->{id},     'id is null/undef'     );
};

# ============================================================================
# 14. Server: framing error → error response
# ============================================================================

subtest 'Server: framing error → -32600 error response' => sub {
    my @written;
    # The scalarref sentinel in MockFramer signals a framing error.
    my $server = build_server([\ 'malformed frame header'], \@written);
    $server->serve;

    is( scalar @written, 1, 'one error response written' );
    my ($msg, $err) = $_codec->decode($written[0]);
    ok( !defined $err,           'decoded ok'          );
    is( $msg->{kind},   'error', 'kind = error'        );
    is( $msg->{code},   -32600,  'code = INVALID_REQUEST');
};

# ============================================================================
# 15. Server: serve() handles multiple messages in sequence
# ============================================================================

subtest 'Server: processes multiple messages in sequence' => sub {
    my @written;
    my $req1 = enc(make_request(10, 'echo', { v => 'A' }));
    my $req2 = enc(make_request(11, 'echo', { v => 'B' }));
    my $server = build_server([$req1, $req2], \@written);
    $server->on_request('echo', sub {
        my ($id, $params) = @_;
        return { echoed => $params->{v} };
    });
    $server->serve;

    is( scalar @written, 2, 'two responses written' );
    my ($r1) = $_codec->decode($written[0]);
    my ($r2) = $_codec->decode($written[1]);
    is( $r1->{result}{echoed}, 'A', 'first echoed A' );
    is( $r2->{result}{echoed}, 'B', 'second echoed B' );
};

# ============================================================================
# 16. Server: notification handler die is silently swallowed
# ============================================================================

subtest 'Server: notification handler die is swallowed (no crash, no response)' => sub {
    my @written;
    my $notif = enc(make_notification('boom'));
    my $server = build_server([$notif], \@written);
    $server->on_notification('boom', sub { die "notification handler panic\n" });
    # Must not die
    ok( eval { $server->serve; 1 }, 'serve did not die' );
    is( scalar @written, 0, 'no response for notification even after panic' );
};

# ============================================================================
# 17. Client: request() round-trip
# ============================================================================

subtest 'Client: request() sends and receives result' => sub {
    # Pre-encoded response frame that the framer will return.
    my $resp_bytes = enc(make_response(1, { sum => 7 }));
    my @written;
    my $framer = MockFramer->new(frames => [$resp_bytes], written => \@written);
    my $client = CodingAdventures::Rpc::Client->new(
        codec  => MockCodec->new,
        framer => $framer,
    );

    my ($result, $err) = $client->request('add', { a => 3, b => 4 });

    ok( !defined $err,           'no error'         );
    ok( defined $result,         'result defined'   );
    is( $result->{sum}, 7,       'result.sum = 7'   );

    # Verify the encoded request was sent
    is( scalar @written, 1, 'one frame written' );
    my ($req, $req_err) = MockCodec->new->decode($written[0]);
    ok( !defined $req_err,     'request decoded ok' );
    is( $req->{kind},  'request', 'sent a request'  );
    is( $req->{method}, 'add',    'method = add'     );
    is( $req->{id},     1,        'id = 1'           );
};

# ============================================================================
# 18. Client: request() receives error response
# ============================================================================

subtest 'Client: request() receives error response' => sub {
    my $err_bytes = enc(make_error(1, METHOD_NOT_FOUND, 'Method not found'));
    my @written;
    my $framer = MockFramer->new(frames => [$err_bytes], written => \@written);
    my $client = CodingAdventures::Rpc::Client->new(
        codec  => MockCodec->new,
        framer => $framer,
    );

    my ($result, $err) = $client->request('nosuch');

    ok( !defined $result,        'result undef on error'    );
    ok( defined $err,            'err defined'              );
    is( $err->{code}, -32601,    'error code = -32601'      );
    like( $err->{message}, qr/Method not found/, 'error message' );
};

# ============================================================================
# 19. Client: request() on EOF before response
# ============================================================================

subtest 'Client: request() returns error on EOF before response' => sub {
    my @written;
    # Empty frames list = immediate EOF
    my $framer = MockFramer->new(frames => [], written => \@written);
    my $client = CodingAdventures::Rpc::Client->new(
        codec  => MockCodec->new,
        framer => $framer,
    );

    my ($result, $err) = $client->request('ping');

    ok( !defined $result,  'result undef'   );
    ok( defined $err,      'err defined'    );
    like( $err->{message}, qr/closed/i, 'error mentions connection closed' );
};

# ============================================================================
# 20. Client: notify() sends without waiting
# ============================================================================

subtest 'Client: notify() encodes and sends without reading response' => sub {
    my @written;
    my $framer = MockFramer->new(frames => [], written => \@written);
    my $client = CodingAdventures::Rpc::Client->new(
        codec  => MockCodec->new,
        framer => $framer,
    );

    $client->notify('log', { message => 'hello' });

    is( scalar @written, 1, 'one frame written' );
    my ($notif, $err) = MockCodec->new->decode($written[0]);
    ok( !defined $err,              'decoded ok'            );
    is( $notif->{kind},  'notification', 'kind = notification' );
    is( $notif->{method}, 'log',         'method = log'      );
};

# ============================================================================
# 21. Client: on_notification() dispatched during request()
# ============================================================================

subtest 'Client: server-push notification dispatched during request()' => sub {
    my $server_push = enc(make_notification('diagnostics', { count => 3 }));
    my $resp        = enc(make_response(1, { ok => 1 }));

    # Framer returns: server-push notification first, then the actual response.
    my @written;
    my $framer = MockFramer->new(
        frames  => [$server_push, $resp],
        written => \@written,
    );
    my $client = CodingAdventures::Rpc::Client->new(
        codec  => MockCodec->new,
        framer => $framer,
    );

    my $received_count;
    $client->on_notification('diagnostics', sub {
        my ($params) = @_;
        $received_count = $params->{count};
    });

    my ($result, $err) = $client->request('check');

    ok( !defined $err,                    'request succeeded'          );
    is( $received_count, 3,               'notification handler called' );
    is( $result->{ok},   1,               'result from response'        );
};

# ============================================================================
# 22. Client: request ids are monotonically increasing
# ============================================================================

subtest 'Client: request ids monotonically increase from 1' => sub {
    my @frames = (
        enc(make_response(1, { n => 'a' })),
        enc(make_response(2, { n => 'b' })),
        enc(make_response(3, { n => 'c' })),
    );
    my @written;
    my $framer = MockFramer->new(frames => \@frames, written => \@written);
    my $client = CodingAdventures::Rpc::Client->new(
        codec  => MockCodec->new,
        framer => $framer,
    );

    $client->request('a');
    $client->request('b');
    $client->request('c');

    # decode() returns a list ($msg, $err).  Use list-context slice [0] to get
    # the message (not the last element which decode() would return in scalar context).
    my @ids = map { (MockCodec->new->decode($_))[0]->{id} } @written;
    is( $ids[0], 1, 'first request id = 1' );
    is( $ids[1], 2, 'second request id = 2');
    is( $ids[2], 3, 'third request id = 3' );
};

# ============================================================================
# 23. Client: on_notification is chainable
# ============================================================================

subtest 'Client: on_notification is chainable' => sub {
    my @written;
    my $framer = MockFramer->new(frames => [], written => \@written);
    my $client = CodingAdventures::Rpc::Client->new(
        codec  => MockCodec->new,
        framer => $framer,
    );
    my $chain = $client
        ->on_notification('a', sub {})
        ->on_notification('b', sub {});
    is( $chain, $client, 'chaining returns self' );
};

# ============================================================================
# 24. Abstract base class die messages
# ============================================================================

subtest 'Codec base class encode dies with helpful message' => sub {
    my $codec = CodingAdventures::Rpc::Codec->new;
    ok( !eval { $codec->encode({}); 1 }, 'encode dies' );
    like( $@, qr/not implemented/i, 'message mentions not implemented' );
};

subtest 'Codec base class decode dies with helpful message' => sub {
    my $codec = CodingAdventures::Rpc::Codec->new;
    ok( !eval { $codec->decode('x'); 1 }, 'decode dies' );
    like( $@, qr/not implemented/i, 'message mentions not implemented' );
};

subtest 'Framer base class read_frame dies with helpful message' => sub {
    my $framer = CodingAdventures::Rpc::Framer->new;
    ok( !eval { $framer->read_frame; 1 }, 'read_frame dies' );
    like( $@, qr/not implemented/i, 'message mentions not implemented' );
};

subtest 'Framer base class write_frame dies with helpful message' => sub {
    my $framer = CodingAdventures::Rpc::Framer->new;
    ok( !eval { $framer->write_frame('x'); 1 }, 'write_frame dies' );
    like( $@, qr/not implemented/i, 'message mentions not implemented' );
};

# ============================================================================
# 25. Server constructor validates args
# ============================================================================

subtest 'Server->new dies without codec' => sub {
    ok( !eval {
        CodingAdventures::Rpc::Server->new(framer => MockFramer->new);
        1
    }, 'dies without codec' );
    like( $@, qr/codec/i, 'message mentions codec' );
};

subtest 'Server->new dies without framer' => sub {
    ok( !eval {
        CodingAdventures::Rpc::Server->new(codec => MockCodec->new);
        1
    }, 'dies without framer' );
    like( $@, qr/framer/i, 'message mentions framer' );
};

subtest 'Client->new dies without codec' => sub {
    ok( !eval {
        CodingAdventures::Rpc::Client->new(framer => MockFramer->new);
        1
    }, 'dies without codec' );
};

subtest 'Client->new dies without framer' => sub {
    ok( !eval {
        CodingAdventures::Rpc::Client->new(codec => MockCodec->new);
        1
    }, 'dies without framer' );
};

done_testing;
