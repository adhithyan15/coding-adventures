use strict;
use warnings;
use Test::More;
use CodingAdventures::InMemoryDataStoreProtocol;

my $CommandFrame = 'CodingAdventures::InMemoryDataStoreProtocol::CommandFrame';
my $EngineResponse = 'CodingAdventures::InMemoryDataStoreProtocol::EngineResponse';

sub dies_like {
    my ($code, $pattern, $name) = @_;
    my $ok = !eval { $code->(); 1 };
    ok($ok, $name);
    like($@, $pattern, "$name message") if $ok;
}

subtest 'ASCII command normalization' => sub {
    is(CodingAdventures::InMemoryDataStoreProtocol::ascii_upper('get'), 'GET', 'lowercase');
    is(CodingAdventures::InMemoryDataStoreProtocol::ascii_upper('mSeT'), 'MSET', 'mixed case');
    is(CodingAdventures::InMemoryDataStoreProtocol::ascii_upper('ping-2'), 'PING-2', 'punctuation');
    dies_like(sub { CodingAdventures::InMemoryDataStoreProtocol::ascii_upper(undef) }, qr/data must be a string/, 'undefined rejected');
    dies_like(sub { CodingAdventures::InMemoryDataStoreProtocol::ascii_upper("\xFF") }, qr/only ASCII bytes/, 'non-ASCII rejected');
};

subtest 'command frames' => sub {
    my $frame = $CommandFrame->from_parts(['set', 'key', 'value']);
    is($frame->command, 'SET', 'command normalized');
    is_deeply($frame->args, ['key', 'value'], 'args');
    is_deeply($frame->to_parts, ['SET', 'key', 'value'], 'parts');
    ok(!defined $CommandFrame->from_parts([]), 'empty parts return undef');

    my $args = ['key'];
    my $get = $CommandFrame->new('GET', $args);
    $args->[0] = 'changed';
    is_deeply($get->args, ['key'], 'constructor copies args');
    my $parts = $get->to_parts;
    $parts->[1] = 'changed';
    is_deeply($get->to_parts, ['GET', 'key'], 'to_parts is defensive');
};

subtest 'command validation' => sub {
    dies_like(sub { $CommandFrame->new(undef) }, qr/command must be a string/, 'undefined command');
    dies_like(sub { $CommandFrame->new('GET', 'key') }, qr/args must be an array reference/, 'invalid args');
    dies_like(sub { $CommandFrame->new('GET', [undef]) }, qr/args\[0\] must be a string/, 'invalid arg');
    dies_like(sub { $CommandFrame->from_parts('GET') }, qr/parts must be an array reference/, 'invalid parts');
    dies_like(sub { $CommandFrame->from_parts(['GET', undef]) }, qr/parts\[1\] must be a string/, 'invalid part');
};

subtest 'scalar and convenience responses' => sub {
    is_deeply($EngineResponse->simple_string('PONG'), $EngineResponse->new('simple_string', 'PONG'), 'simple string');
    is_deeply($EngineResponse->error('ERR'), $EngineResponse->new('error', 'ERR'), 'error');
    is_deeply($EngineResponse->integer(42), $EngineResponse->new('integer', 42), 'integer');
    is_deeply($EngineResponse->bulk_string('value'), $EngineResponse->new('bulk_string', 'value'), 'bulk string');
    is_deeply($EngineResponse->ok, $EngineResponse->new('simple_string', 'OK'), 'OK');
    is_deeply($EngineResponse->null, $EngineResponse->new('bulk_string', undef), 'null');
    is_deeply($EngineResponse->zero, $EngineResponse->new('integer', 0), 'zero');
    is_deeply($EngineResponse->one, $EngineResponse->new('integer', 1), 'one');
};

subtest 'array responses' => sub {
    my $values = [$EngineResponse->ok, $EngineResponse->integer(3)];
    my $response = $EngineResponse->array($values);
    $values->[0] = $EngineResponse->error('changed');
    is($response->kind, 'array', 'array kind');
    is($response->value->[0]->kind, 'simple_string', 'input array copied');
    is($response->value->[1]->value, 3, 'nested value');
    is_deeply($EngineResponse->array(undef), $EngineResponse->new('array', undef), 'null array');
};

subtest 'response validation' => sub {
    dies_like(sub { $EngineResponse->new('unknown', undef) }, qr/invalid response kind: unknown/, 'invalid kind');
    dies_like(sub { $EngineResponse->simple_string(undef) }, qr/simple_string value must be a string/, 'invalid string');
    dies_like(sub { $EngineResponse->integer(1.5) }, qr/integer value must be an integer/, 'invalid integer');
    dies_like(sub { $EngineResponse->bulk_string([]) }, qr/bulk_string value must be a string or undef/, 'invalid bulk string');
    dies_like(sub { $EngineResponse->array(['not a response']) }, qr/value\[0\] must be an EngineResponse/, 'invalid array item');
};

done_testing;
