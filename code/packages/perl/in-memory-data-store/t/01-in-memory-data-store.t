use strict;
use warnings;
use Test::More;
use CodingAdventures::InMemoryDataStore qw(
    command_from_resp response_to_resp_value command_to_frame
    frame_to_response_text
);
use CodingAdventures::InMemoryDataStoreEngine;
use CodingAdventures::InMemoryDataStoreProtocol;
use CodingAdventures::RespProtocol qw(encode);

my $Value = 'CodingAdventures::RespProtocol::Value';
my $Response = 'CodingAdventures::InMemoryDataStoreProtocol::EngineResponse';

sub command {
    return $Value->array([map { $Value->bulk_string($_) } @_]);
}

subtest 'executes RESP frames end to end' => sub {
    my $store = CodingAdventures::InMemoryDataStore->new;
    my $response = $store->execute_frame(command('PING'));
    is($response->kind, 'simple_string', 'simple string response');
    is($response->value, 'PONG', 'PONG response');
};

subtest 'handles incremental and pipelined RESP input' => sub {
    my $store = CodingAdventures::InMemoryDataStore->new;
    my $set_wire = encode(command('SET', 'counter', '1'));
    my $get_wire = encode(command('GET', 'counter'));
    is_deeply($store->process(substr($set_wire, 0, 5)), [], 'incomplete frame buffered');
    is($store->handle(substr($set_wire, 5) . $get_wire), "+OK\r\n\$1\r\n1\r\n", 'pipeline encoded');
};

subtest 'preserves binary-safe values' => sub {
    my $store = CodingAdventures::InMemoryDataStore->new;
    my $binary = "a\0b\xFF";
    is($store->execute_frame(command('SET', 'binary', $binary))->value, 'OK', 'SET succeeds');
    is($store->execute_frame(command('GET', 'binary'))->value, $binary, 'binary value round trips');
};

subtest 'rejects invalid frames and ignores blank arrays' => sub {
    my $store = CodingAdventures::InMemoryDataStore->new;
    is($store->execute_frame($Value->simple_string('PING'))->kind, 'error', 'scalar rejected');
    is($store->execute_frame($Value->array([])), undef, 'blank array ignored');
    is($store->execute_frame($Value->array([$Value->null_bulk_string]))->kind, 'error', 'null part rejected');
};

subtest 'converts command and response IR values' => sub {
    my $frame = CodingAdventures::InMemoryDataStoreProtocol::CommandFrame->new('ECHO', ['hello']);
    is(command_from_resp(command_to_frame($frame))->command, 'ECHO', 'command round trip');
    my $nested = $Response->array([
        $Response->integer(2),
        $Response->bulk_string(undef),
    ]);
    my $converted = response_to_resp_value($nested);
    is($converted->value->[0]->kind, 'integer', 'integer converted');
    ok($converted->value->[1]->is_null, 'null bulk converted');
    is(frame_to_response_text($converted->value->[1]), '(nil)', 'null display');
};

subtest 'accepts injected engines and resets to a fresh store' => sub {
    my $engine = CodingAdventures::InMemoryDataStoreEngine->new;
    my $store = CodingAdventures::InMemoryDataStore->new(engine => $engine);
    $store->execute_parts(['SET', 'name', 'Ada']);
    is($store->execute_parts(['GET', 'name'])->value, 'Ada', 'injected engine used');
    $store->reset;
    ok($store->execute_parts(['GET', 'name'])->is_null, 'reset clears state');
};

done_testing;
