use strict;
use warnings;
use utf8;
use Test::More;
use CodingAdventures::RespProtocol qw(
    encode encode_many encode_bulk_string decode decode_all equal
);

my $Value = 'CodingAdventures::RespProtocol::Value';
my $Decoder = 'CodingAdventures::RespProtocol::Decoder';

sub dies_like {
    my ($code, $pattern, $name) = @_;
    my $ok = !eval { $code->(); 1 };
    ok($ok, $name);
    like($@, $pattern, "$name message") if $ok;
}

sub round_trip {
    my ($value) = @_;
    my $wire = encode($value);
    my $decoded = decode($wire);
    ok(defined($decoded), 'round trip decoded');
    is($decoded->[1], length($wire), 'round trip consumed all bytes');
    ok(equal($value, $decoded->[0]), 'round trip value matches');
}

subtest 'typed value construction and validation' => sub {
    is($Value->simple_string('OK')->value, 'OK', 'simple string');
    my $error = $Value->error('WRONGTYPE bad value');
    is($error->error_type, 'WRONGTYPE', 'error type');
    is($error->detail, 'bad value', 'error detail');
    is($Value->integer(42)->value, 42, 'integer');
    ok($Value->null_bulk_string->is_null, 'null bulk');
    ok($Value->null_array->is_null, 'null array');
    like('' . $Value->array([]), qr/0 values/, 'string rendering');

    dies_like(sub { $Value->new('unknown') }, qr/invalid RESP kind: unknown/, 'invalid kind');
    dies_like(sub { $Value->simple_string("bad\rline") }, qr/must not contain CR or LF/, 'invalid line');
    dies_like(sub { $Value->integer(1.5) }, qr/must be an integer/, 'invalid integer');
    dies_like(sub { $Value->array(['not-a-value']) }, qr/array value\[0\] must be a RESP Value/, 'invalid array');
};

subtest 'exact scalar and null encoding' => sub {
    is(encode($Value->simple_string('OK')), "+OK\r\n", 'simple');
    is(encode($Value->error('ERR boom')), "-ERR boom\r\n", 'error');
    is(encode($Value->integer(-42)), ":-42\r\n", 'integer');
    is(encode($Value->bulk_string('hello')), "\$5\r\nhello\r\n", 'bulk');
    is(encode($Value->bulk_string('')), "\$0\r\n\r\n", 'empty bulk');
    is(encode($Value->null_bulk_string), "\$-1\r\n", 'null bulk');
    is(encode($Value->null_array), "*-1\r\n", 'null array');
};

subtest 'binary and nested array encoding' => sub {
    my $binary = "\0foo\r\nbar\xFF";
    is(encode_bulk_string($binary), "\$10\r\n" . $binary . "\r\n", 'binary bulk');
    my $command = $Value->array([
        $Value->bulk_string('SET'),
        $Value->bulk_string('key'),
        $Value->array([$Value->integer(1), $Value->null_bulk_string]),
    ]);
    is(
        encode($command),
        "*3\r\n\$3\r\nSET\r\n\$3\r\nkey\r\n*2\r\n:1\r\n\$-1\r\n",
        'nested array',
    );
    is(encode_many([$Value->integer(1), $Value->integer(0), 'x']), ":1\r\n:0\r\n\$1\r\nx\r\n", 'many');
};

subtest 'typed decoding and offsets' => sub {
    my $wire = "+OK\r\n-ERR boom\r\n:42\r\n\$3\r\nfoo\r\n\$-1\r\n*-1\r\n";
    my ($values, $next) = @{decode_all($wire)};
    is(@$values, 6, 'value count');
    is($next, length($wire), 'consumed all');
    is($values->[0]->kind, 'simple_string', 'simple kind');
    is($values->[1]->kind, 'error', 'error kind');
    is($values->[2]->value, 42, 'integer value');
    is($values->[3]->value, 'foo', 'bulk value');
    ok($values->[4]->is_null, 'null bulk decoded');
    ok($values->[5]->is_null && $values->[5]->kind eq 'array', 'null array decoded');
};

subtest 'every incomplete prefix consumes nothing' => sub {
    my $full = "*3\r\n\$3\r\nSET\r\n\$3\r\nfoo\r\n\$3\r\nbar\r\n";
    for my $length (0 .. length($full) - 1) {
        is(decode(substr($full, 0, $length)), undef, "incomplete prefix $length");
    }
    my $decoded = decode($full);
    is($decoded->[1], length($full), 'full frame consumed');
    is(@{$decoded->[0]->value}, 3, 'three elements');
};

subtest 'malformed frames are rejected' => sub {
    dies_like(sub { decode(":abc\r\n") }, qr/invalid RESP integer: abc/, 'invalid integer');
    dies_like(sub { decode("\$-2\r\n") }, qr/invalid negative bulk length: -2/, 'negative bulk');
    dies_like(sub { decode("*-2\r\n") }, qr/invalid negative array length: -2/, 'negative array');
    dies_like(sub { decode("\$3\r\nfooXX") }, qr/bulk string missing trailing CRLF/, 'bad terminator');
    dies_like(sub { decode("+OK\r\n", 99) }, qr/offset is outside data/, 'bad offset');
};

subtest 'inline commands and incomplete tails' => sub {
    my $inline = decode("SET key value\r\n");
    is(@{$inline->[0]->value}, 3, 'three inline tokens');
    is($inline->[0]->value->[0]->value, 'SET', 'first token');
    is($inline->[1], 15, 'inline consumed');

    my ($values, $next) = @{decode_all("+OK\r\n:1\r\n\$5\r\nhel")};
    is(@$values, 2, 'two complete values');
    is($next, 9, 'tail begins at third frame');
};

subtest 'round trips nested and all-byte values' => sub {
    round_trip($Value->bulk_string(join('', map { chr($_) } 0 .. 255)));
    round_trip($Value->array([
        $Value->simple_string('OK'),
        $Value->error('ERR boom'),
        $Value->integer(-1),
        $Value->bulk_string("payload\0"),
        $Value->null_bulk_string,
        $Value->array([$Value->integer(2)]),
        $Value->null_array,
    ]));
};

subtest 'streaming handles byte fragmentation and multiple messages' => sub {
    my @expected = (
        $Value->array([$Value->bulk_string('PING')]),
        $Value->integer(42),
        $Value->error('ERR bad'),
    );
    my $wire = encode_many(\@expected);
    my $decoder = $Decoder->new;
    $decoder->feed(substr($wire, $_, 1)) for 0 .. length($wire) - 1;
    is($decoder->pending_bytes, 0, 'no pending bytes');
    for my $value (@expected) {
        ok($decoder->has_message, 'message available');
        ok(equal($value, $decoder->get_message), 'message matches');
    }
    ok(!$decoder->has_message, 'queue empty');
    dies_like(sub { $decoder->get_message }, qr/no decoded message is available/, 'empty queue');
};

done_testing;
