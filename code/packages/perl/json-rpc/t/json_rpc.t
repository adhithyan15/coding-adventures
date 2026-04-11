use strict;
use warnings;
use Test2::V0;

# ============================================================================
# CodingAdventures::JsonRpc — comprehensive test suite
# ============================================================================
#
# Test coverage:
#   1. Module loads (all sub-modules)
#   2. Error constants have correct values
#   3. Message constructors: request, response, error_response, notification
#   4. classify_message: all four types + invalid cases
#   5. parse_message: valid messages, malformed JSON, invalid JSON-RPC
#   6. MessageWriter: Content-Length header, payload, byte length
#   7. MessageReader: single message, back-to-back, EOF, bad JSON,
#      valid JSON that is not JSON-RPC
#   8. Server dispatch: request → response, notification → no response,
#      unknown method → -32601, handler die → -32603,
#      handler returning ResponseError → error response
#   9. Round-trip: write → read → compare
#
# Per lessons.md: Test2::V0 does not have use_ok.
# Use eval+require instead.

# ============================================================================
# Helpers
# ============================================================================

# Build an in-memory filehandle wrapping a scalar buffer.
# `open $fh, '<', \$buffer` gives a readable handle backed by the string.
# `open $fh, '>', \$buffer` gives a writable handle that appends to $buffer.

# frame($json) — manually frame a JSON string the same way the writer does,
# for use as test input to the reader.
sub frame {
    my ($json) = @_;
    use bytes;
    my $len = length($json);
    return "Content-Length: $len\r\n\r\n$json";
}

# make_reader($data) — open an in-memory read handle on $data.
sub make_reader {
    my ($data) = @_;
    open my $fh, '<', \$data or die "Cannot open reader: $!";
    binmode($fh, ':raw');
    return $fh;
}

# make_writer() — open an in-memory write handle; returns ($fh, $buf_ref).
sub make_writer {
    my $buf = '';
    open my $fh, '>', \$buf or die "Cannot open writer: $!";
    binmode($fh, ':raw');
    return ($fh, \$buf);
}

# ============================================================================
# 1. Module loads
# ============================================================================

subtest 'modules load' => sub {
    ok( eval { require CodingAdventures::JsonRpc; 1 },
        'CodingAdventures::JsonRpc loads' );
    ok( eval { require CodingAdventures::JsonRpc::Errors; 1 },
        'Errors loads' );
    ok( eval { require CodingAdventures::JsonRpc::Message; 1 },
        'Message loads' );
    ok( eval { require CodingAdventures::JsonRpc::Reader; 1 },
        'Reader loads' );
    ok( eval { require CodingAdventures::JsonRpc::Writer; 1 },
        'Writer loads' );
    ok( eval { require CodingAdventures::JsonRpc::Server; 1 },
        'Server loads' );
};

# ============================================================================
# 2. Error constants
# ============================================================================

use CodingAdventures::JsonRpc::Errors qw(:all);

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

use CodingAdventures::JsonRpc::Message qw(:all);

subtest 'request constructor' => sub {
    my $r = request(1, 'ping', { x => 1 });
    is( $r->{jsonrpc}, '2.0',  'jsonrpc = 2.0'  );
    is( $r->{id},      1,      'id = 1'         );
    is( $r->{method},  'ping', 'method = ping'  );
    is( $r->{params}{x}, 1,   'params.x = 1'   );
};

subtest 'request without params' => sub {
    my $r = request(2, 'initialize');
    ok( !exists $r->{params}, 'params key absent when not supplied' );
};

subtest 'response constructor' => sub {
    my $r = response(1, { ok => 1 });
    is( $r->{jsonrpc},    '2.0', 'jsonrpc = 2.0' );
    is( $r->{id},         1,     'id = 1'        );
    is( $r->{result}{ok}, 1,     'result.ok = 1' );
};

subtest 'error_response constructor' => sub {
    my $e = { code => -32601, message => 'Method not found' };
    my $r = error_response(1, $e);
    is( $r->{jsonrpc},       '2.0',             'jsonrpc'        );
    is( $r->{id},            1,                 'id'             );
    is( $r->{error}{code},   -32601,            'error.code'     );
    is( $r->{error}{message}, 'Method not found', 'error.message' );
};

subtest 'notification constructor' => sub {
    my $n = notification('textDocument/didOpen', { uri => 'file:///a' });
    is( $n->{jsonrpc}, '2.0',                   'jsonrpc'       );
    is( $n->{method},  'textDocument/didOpen',  'method'        );
    is( $n->{params}{uri}, 'file:///a',         'params.uri'    );
    ok( !exists $n->{id}, 'no id key'                           );
};

# ============================================================================
# 4. classify_message
# ============================================================================

subtest 'classify_message' => sub {
    is( classify_message({ jsonrpc=>'2.0', id=>1, method=>'ping' }),
        'request',      'request' );

    is( classify_message({ jsonrpc=>'2.0', method=>'$/status' }),
        'notification', 'notification' );

    is( classify_message({ jsonrpc=>'2.0', id=>1, result=>{} }),
        'response',     'success response' );

    is( classify_message({ jsonrpc=>'2.0', id=>1, error=>{ code=>-32601, message=>'x' } }),
        'response',     'error response' );

    is( classify_message({ id=>1, method=>'ping' }),
        undef, 'missing jsonrpc → undef' );

    is( classify_message({ jsonrpc=>'1.0', id=>1, method=>'ping' }),
        undef, 'wrong version → undef' );

    is( classify_message('a string'),
        undef, 'non-hashref → undef' );

    is( classify_message(undef),
        undef, 'undef → undef' );
};

# ============================================================================
# 5. parse_message
# ============================================================================

subtest 'parse_message: valid request' => sub {
    my $json = '{"jsonrpc":"2.0","id":1,"method":"ping"}';
    my ($msg, $err) = parse_message($json);
    ok( defined $msg, 'msg defined'       );
    ok( !defined $err, 'no error'         );
    is( $msg->{id},     1,     'id'       );
    is( $msg->{method}, 'ping','method'   );
    is( $msg->{_type},  'request', '_type');
};

subtest 'parse_message: malformed JSON' => sub {
    my ($msg, $err) = parse_message('{bad}');
    ok( !defined $msg, 'msg undef'        );
    ok( defined $err,  'error defined'    );
    like( $err, qr/parse error/i, 'error mentions parse error' );
};

subtest 'parse_message: valid JSON but not JSON-RPC' => sub {
    my ($msg, $err) = parse_message('{"foo":"bar"}');
    ok( !defined $msg, 'msg undef'             );
    ok( defined $err,  'error defined'         );
    like( $err, qr/invalid request/i, 'mentions invalid request' );
};

# ============================================================================
# 6. MessageWriter
# ============================================================================

use CodingAdventures::JsonRpc::Writer;

subtest 'MessageWriter: write_raw produces correct header' => sub {
    my ($fh, $buf_ref) = make_writer();
    my $writer = CodingAdventures::JsonRpc::Writer->new($fh);
    my $payload = '{"jsonrpc":"2.0","id":1,"result":null}';
    $writer->write_raw($payload);
    close $fh;

    like( $$buf_ref, qr/^Content-Length: \d+\r\n\r\n/, 'header format' );
    my ($n) = $$buf_ref =~ /Content-Length: (\d+)/;
    use bytes;
    is( $n, length($payload), 'Content-Length equals byte length of payload' );
};

subtest 'MessageWriter: payload appears after header' => sub {
    my ($fh, $buf_ref) = make_writer();
    my $writer = CodingAdventures::JsonRpc::Writer->new($fh);
    my $payload = '{"jsonrpc":"2.0","method":"ping"}';
    $writer->write_raw($payload);
    close $fh;

    ok( index($$buf_ref, $payload) > 0, 'payload present after header' );
};

subtest 'MessageWriter: write_message encodes and frames message' => sub {
    my ($fh, $buf_ref) = make_writer();
    my $writer = CodingAdventures::JsonRpc::Writer->new($fh);
    $writer->write_message(response(1, { pong => 1 }));
    close $fh;

    like( $$buf_ref, qr/Content-Length:/, 'has Content-Length' );
    like( $$buf_ref, qr/"result"/,        'has result key'     );
};

# ============================================================================
# 7. MessageReader
# ============================================================================

use CodingAdventures::JsonRpc::Reader;

subtest 'MessageReader: reads a single framed message' => sub {
    my $json = '{"jsonrpc":"2.0","id":1,"method":"ping"}';
    my $fh = make_reader(frame($json));
    my $reader = CodingAdventures::JsonRpc::Reader->new($fh);
    my ($msg, $err) = $reader->read_message;
    ok( !defined $err, 'no error'        );
    ok( defined $msg,  'msg defined'     );
    is( $msg->{id},     1,     'id'      );
    is( $msg->{method}, 'ping','method'  );
};

subtest 'MessageReader: reads two back-to-back messages' => sub {
    my $j1 = '{"jsonrpc":"2.0","id":1,"method":"a"}';
    my $j2 = '{"jsonrpc":"2.0","method":"b"}';
    my $fh = make_reader(frame($j1) . frame($j2));
    my $reader = CodingAdventures::JsonRpc::Reader->new($fh);

    my ($m1, $e1) = $reader->read_message;
    ok( !defined $e1 && defined $m1, 'first message ok' );
    is( $m1->{method}, 'a', 'first method = a' );

    my ($m2, $e2) = $reader->read_message;
    ok( !defined $e2 && defined $m2, 'second message ok' );
    is( $m2->{method}, 'b', 'second method = b' );
};

subtest 'MessageReader: returns (undef, undef) on clean EOF' => sub {
    my $fh = make_reader('');
    my $reader = CodingAdventures::JsonRpc::Reader->new($fh);
    my ($msg, $err) = $reader->read_message;
    ok( !defined $msg, 'msg undef on EOF' );
    ok( !defined $err, 'err undef on EOF' );
};

subtest 'MessageReader: returns (undef, $err) on malformed JSON' => sub {
    my $framed = "Content-Length: 5\r\n\r\n{bad}";
    my $fh = make_reader($framed);
    my $reader = CodingAdventures::JsonRpc::Reader->new($fh);
    my ($msg, $err) = $reader->read_message;
    ok( !defined $msg, 'msg undef'          );
    ok( defined $err,  'err defined'        );
    like( $err, qr/parse error/i, 'mentions parse error' );
};

subtest 'MessageReader: returns (undef, $err) for valid JSON that is not JSON-RPC' => sub {
    my $json   = '{"foo":"bar"}';
    my $framed = frame($json);
    my $fh = make_reader($framed);
    my $reader = CodingAdventures::JsonRpc::Reader->new($fh);
    my ($msg, $err) = $reader->read_message;
    ok( !defined $msg, 'msg undef'               );
    ok( defined $err,  'err defined'             );
    like( $err, qr/invalid request/i, 'mentions invalid request' );
};

subtest 'MessageReader: read_raw returns raw JSON string' => sub {
    my $json = '{"jsonrpc":"2.0","id":7,"method":"test"}';
    my $fh = make_reader(frame($json));
    my $reader = CodingAdventures::JsonRpc::Reader->new($fh);
    my ($raw, $err) = $reader->read_raw;
    ok( !defined $err, 'no error'        );
    is( $raw, $json,   'raw == original' );
};

# ============================================================================
# 8. Server dispatch
# ============================================================================

use CodingAdventures::JsonRpc::Server;

# Helper: run the server with a single input message and return the output.
sub run_server_once {
    my ($input_json, $setup_fn) = @_;
    my $in_fh = make_reader(frame($input_json));

    my ($out_fh, $out_buf) = make_writer();
    my $server = CodingAdventures::JsonRpc::Server->new($in_fh, $out_fh);
    $setup_fn->($server) if $setup_fn;
    $server->serve;
    close $out_fh;
    return $$out_buf;
}

subtest 'Server: dispatches request and writes response' => sub {
    my $input = '{"jsonrpc":"2.0","id":1,"method":"greet","params":{"name":"World"}}';
    my $out = run_server_once($input, sub {
        my ($srv) = @_;
        $srv->on_request('greet', sub {
            my ($id, $params) = @_;
            return { greeting => "Hello, $params->{name}" };
        });
    });
    like( $out, qr/"result"/, 'response has result'     );
    like( $out, qr/Hello, World/, 'greeting in output'  );
};

subtest 'Server: dispatches notification without writing a response' => sub {
    my $notif = '{"jsonrpc":"2.0","method":"didChange","params":{"x":1}}';
    my $called = 0;
    my $out = run_server_once($notif, sub {
        my ($srv) = @_;
        $srv->on_notification('didChange', sub { $called++ });
    });
    is( $called, 1,  'notification handler called once' );
    is( $out,   '',  'no output for notification'       );
};

subtest 'Server: sends -32601 for unknown request method' => sub {
    my $input = '{"jsonrpc":"2.0","id":1,"method":"unknown"}';
    my $out = run_server_once($input);
    like( $out, qr/-32601/,   '-32601 in output'  );
    like( $out, qr/"error"/,  'error key present' );
};

subtest 'Server: sends -32603 when handler dies' => sub {
    my $input = '{"jsonrpc":"2.0","id":2,"method":"boom"}';
    my $out = run_server_once($input, sub {
        my ($srv) = @_;
        $srv->on_request('boom', sub { die "intentional test error\n" });
    });
    like( $out, qr/-32603/,  '-32603 in output'  );
    like( $out, qr/"error"/, 'error key present' );
};

subtest 'Server: sends error response when handler returns ResponseError' => sub {
    my $input = '{"jsonrpc":"2.0","id":3,"method":"validate","params":{}}';
    my $out = run_server_once($input, sub {
        my ($srv) = @_;
        $srv->on_request('validate', sub {
            return { code => INVALID_PARAMS, message => 'bad params' };
        });
    });
    like( $out, qr/-32602/,  '-32602 in output'      );
    like( $out, qr/"error"/, 'error key present'     );
};

subtest 'Server: ignores unregistered notifications (no output)' => sub {
    my $notif = '{"jsonrpc":"2.0","method":"unregistered"}';
    my $out = run_server_once($notif);
    is( $out, '', 'no output for unregistered notification' );
};

subtest 'Server: on_request is chainable' => sub {
    my $in_fh  = make_reader('');
    my ($out_fh) = make_writer();
    my $server = CodingAdventures::JsonRpc::Server->new($in_fh, $out_fh);
    my $chain = $server
        ->on_request('a', sub {})
        ->on_request('b', sub {});
    is( $chain, $server, 'chaining returns self' );
};

subtest 'Server: on_notification is chainable' => sub {
    my $in_fh  = make_reader('');
    my ($out_fh) = make_writer();
    my $server = CodingAdventures::JsonRpc::Server->new($in_fh, $out_fh);
    my $chain = $server
        ->on_notification('x', sub {})
        ->on_notification('y', sub {});
    is( $chain, $server, 'chaining returns self' );
};

# ============================================================================
# 9. Round-trip tests
# ============================================================================

subtest 'round-trip: Request write → read → compare' => sub {
    my ($out_fh, $out_buf) = make_writer();
    my $writer = CodingAdventures::JsonRpc::Writer->new($out_fh);
    $writer->write_message(request(42, 'textDocument/hover',
        { textDocument => { uri => 'file:///main.bf' }, position => { line => 0, character => 3 } }
    ));
    close $out_fh;

    my $in_fh = make_reader($$out_buf);
    my $reader = CodingAdventures::JsonRpc::Reader->new($in_fh);
    my ($msg, $err) = $reader->read_message;
    ok( !defined $err, 'no error'      );
    is( $msg->{id},     42,                    'id'     );
    is( $msg->{method}, 'textDocument/hover',  'method' );
    is( $msg->{params}{textDocument}{uri}, 'file:///main.bf', 'uri' );
};

subtest 'round-trip: Notification write → read → compare' => sub {
    my ($out_fh, $out_buf) = make_writer();
    my $writer = CodingAdventures::JsonRpc::Writer->new($out_fh);
    $writer->write_message(notification('textDocument/didOpen',
        { textDocument => { uri => 'file:///a.bf', text => '+[>+<-].' } }
    ));
    close $out_fh;

    my $in_fh = make_reader($$out_buf);
    my $reader = CodingAdventures::JsonRpc::Reader->new($in_fh);
    my ($msg, $err) = $reader->read_message;
    ok( !defined $err, 'no error'              );
    is( $msg->{method}, 'textDocument/didOpen','method'        );
    is( $msg->{params}{textDocument}{text}, '+[>+<-].', 'text' );
};

subtest 'round-trip: error response write → read → compare' => sub {
    my ($out_fh, $out_buf) = make_writer();
    my $writer = CodingAdventures::JsonRpc::Writer->new($out_fh);
    $writer->write_message(error_response(1, {
        code    => METHOD_NOT_FOUND,
        message => 'Method not found',
        data    => 'no handler for foo',
    }));
    close $out_fh;

    my $in_fh = make_reader($$out_buf);
    my $reader = CodingAdventures::JsonRpc::Reader->new($in_fh);
    my ($msg, $err) = $reader->read_message;
    ok( !defined $err, 'no error'          );
    is( $msg->{id},           1,            'id'    );
    is( $msg->{error}{code},  -32601,       'code'  );
    is( $msg->{error}{message}, 'Method not found', 'message' );
};

done_testing;
