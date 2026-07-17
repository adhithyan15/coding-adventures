use strict;
use warnings;
use Test::More;
use CodingAdventures::InMemoryDataStoreEngine;
use CodingAdventures::InMemoryDataStoreProtocol;

sub execute {
    my ($engine, @parts) = @_;
    return $engine->execute_parts(\@parts);
}

sub response_values {
    my ($response) = @_;
    is($response->kind, 'array', 'array response');
    return [map { $_->value } @{$response->value}];
}

sub error_like {
    my ($response, $text, $name) = @_;
    is($response->kind, 'error', "$name kind");
    like($response->value, qr/\Q$text\E/, $name);
}

subtest 'strings integers and keyspace commands' => sub {
    my $engine = CodingAdventures::InMemoryDataStoreEngine->new;
    is(execute($engine, 'PING')->value, 'PONG', 'ping');
    is(execute($engine, 'PING', 'hello')->value, 'hello', 'ping payload');
    is(execute($engine, 'ECHO', "\0binary")->value, "\0binary", 'binary echo');
    is(execute($engine, 'SET', 'user:1', '40')->value, 'OK', 'set');
    is(execute($engine, 'GET', 'user:1')->value, '40', 'get');
    is(execute($engine, 'EXISTS', 'user:1', 'missing')->value, 1, 'exists');
    is(execute($engine, 'TYPE', 'user:1')->value, 'string', 'type');
    is(execute($engine, 'TYPE', 'missing')->value, 'none', 'missing type');
    is(execute($engine, 'INCR', 'user:1')->value, 41, 'incr');
    is(execute($engine, 'INCRBY', 'user:1', '2')->value, 43, 'incrby');
    is(execute($engine, 'DECR', 'user:1')->value, 42, 'decr');
    is(execute($engine, 'DECRBY', 'user:1', '2')->value, 40, 'decrby');
    is(execute($engine, 'APPEND', 'user:1', '!')->value, 3, 'append');
    is(execute($engine, 'APPEND', 'new', 'abc')->value, 3, 'append new');
    execute($engine, 'SET', 'user:2', 'Lin');
    is_deeply(response_values(execute($engine, 'KEYS', 'user:*')), ['user:1', 'user:2'], 'keys');
    is(execute($engine, 'RENAME', 'user:2', 'user:two')->value, 'OK', 'rename');
    is(execute($engine, 'GET', 'user:two')->value, 'Lin', 'renamed value');
    execute($engine, 'SET', 'literal[1]', 'yes');
    is_deeply(response_values(execute($engine, 'KEYS', 'literal[1]')), ['literal[1]'], 'literal glob');
    is(execute($engine, 'DEL', 'user:1', 'missing')->value, 1, 'delete');
    ok(!defined(execute($engine, 'GET', 'user:1')->value), 'deleted value');
};

subtest 'hashes and lists' => sub {
    my $engine = CodingAdventures::InMemoryDataStoreEngine->new;
    is(execute($engine, 'HSET', 'user', 'name', 'Ada', 'city', 'London')->value, 2, 'hset');
    is(execute($engine, 'HSET', 'user', 'name', 'Augusta')->value, 0, 'hset update');
    is(execute($engine, 'HGET', 'user', 'name')->value, 'Augusta', 'hget');
    is(execute($engine, 'HEXISTS', 'user', 'city')->value, 1, 'hexists');
    is(execute($engine, 'HLEN', 'user')->value, 2, 'hlen');
    is_deeply(response_values(execute($engine, 'HKEYS', 'user')), ['city', 'name'], 'hkeys');
    is_deeply(response_values(execute($engine, 'HVALS', 'user')), ['London', 'Augusta'], 'hvals');
    is_deeply(response_values(execute($engine, 'HGETALL', 'user')), ['city', 'London', 'name', 'Augusta'], 'hgetall');
    is(execute($engine, 'HDEL', 'user', 'city', 'missing')->value, 1, 'hdel');
    is(execute($engine, 'HDEL', 'user', 'name')->value, 1, 'hdel final');
    is(execute($engine, 'HLEN', 'user')->value, 0, 'empty hash removed');
    is(execute($engine, 'LPUSH', 'queue', 'b', 'a')->value, 2, 'lpush');
    is(execute($engine, 'RPUSH', 'queue', 'c')->value, 3, 'rpush');
    is(execute($engine, 'LLEN', 'queue')->value, 3, 'llen');
    is(execute($engine, 'LINDEX', 'queue', '-1')->value, 'c', 'lindex');
    is_deeply(response_values(execute($engine, 'LRANGE', 'queue', '0', '-1')), ['a', 'b', 'c'], 'lrange');
    is(execute($engine, 'LPOP', 'queue')->value, 'a', 'lpop');
    is(execute($engine, 'RPOP', 'queue')->value, 'c', 'rpop');
    is(execute($engine, 'RPOP', 'queue')->value, 'b', 'rpop final');
    ok(!defined(execute($engine, 'LPOP', 'queue')->value), 'empty pop');
};

subtest 'sets sorted sets and HyperLogLog' => sub {
    my $engine = CodingAdventures::InMemoryDataStoreEngine->new;
    is(execute($engine, 'SADD', 'left', qw(a b c a))->value, 3, 'sadd left');
    is(execute($engine, 'SADD', 'right', qw(b c d))->value, 3, 'sadd right');
    is(execute($engine, 'SISMEMBER', 'left', 'b')->value, 1, 'sismember');
    is(execute($engine, 'SCARD', 'left')->value, 3, 'scard');
    is_deeply(response_values(execute($engine, 'SMEMBERS', 'left')), [qw(a b c)], 'smembers');
    is_deeply(response_values(execute($engine, 'SUNION', 'left', 'right')), [qw(a b c d)], 'sunion');
    is_deeply(response_values(execute($engine, 'SINTER', 'left', 'right')), [qw(b c)], 'sinter');
    is_deeply(response_values(execute($engine, 'SDIFF', 'left', 'right')), ['a'], 'sdiff');
    is_deeply(response_values(execute($engine, 'SINTER', 'left', 'missing')), [], 'missing intersection');
    is(execute($engine, 'SREM', 'left', 'a', 'missing')->value, 1, 'srem');
    is(execute($engine, 'ZADD', 'scores', '1', 'alice', '2', 'bob', '1.5', 'cara')->value, 3, 'zadd');
    is(execute($engine, 'ZADD', 'scores', '3', 'alice')->value, 0, 'zadd update');
    is_deeply(response_values(execute($engine, 'ZRANGE', 'scores', '0', '-1')), [qw(cara bob alice)], 'zrange');
    is_deeply(response_values(execute($engine, 'ZRANGE', 'scores', '0', '1', 'WITHSCORES')), ['cara', '1.5', 'bob', '2'], 'zrange scores');
    is_deeply(response_values(execute($engine, 'ZRANGEBYSCORE', 'scores', '1', '2')), [qw(cara bob)], 'zrangebyscore');
    is(execute($engine, 'ZRANK', 'scores', 'bob')->value, 1, 'zrank');
    is(execute($engine, 'ZSCORE', 'scores', 'cara')->value, '1.5', 'zscore');
    is(execute($engine, 'ZCARD', 'scores')->value, 3, 'zcard');
    is(execute($engine, 'ZREM', 'scores', 'bob', 'missing')->value, 1, 'zrem');
    is(execute($engine, 'PFADD', 'visitors', qw(alice bob))->value, 1, 'pfadd');
    is(execute($engine, 'PFADD', 'visitors', 'alice')->value, 0, 'pfadd no change');
    is(execute($engine, 'PFADD', 'other', 'cara')->value, 1, 'pfadd other');
    cmp_ok(execute($engine, 'PFCOUNT', 'visitors')->value, '>=', 2, 'pfcount');
    cmp_ok(execute($engine, 'PFCOUNT', 'visitors', 'other')->value, '>=', 3, 'pfcount merge');
    is(execute($engine, 'PFMERGE', 'all', 'visitors', 'other')->value, 'OK', 'pfmerge');
    cmp_ok(execute($engine, 'PFCOUNT', 'all')->value, '>=', 3, 'merged count');
};

subtest 'expiry logical databases and admin commands' => sub {
    my $now = 2_000_000;
    my $engine = CodingAdventures::InMemoryDataStoreEngine->new(time_provider => sub { $now });
    execute($engine, 'SET', 'temporary', 'value');
    is(execute($engine, 'TTL', 'temporary')->value, -1, 'ttl persistent');
    is(execute($engine, 'PERSIST', 'temporary')->value, 0, 'persist none');
    is(execute($engine, 'EXPIRE', 'temporary', '10')->value, 1, 'expire');
    is(execute($engine, 'TTL', 'temporary')->value, 10, 'ttl');
    is(execute($engine, 'PTTL', 'temporary')->value, 10_000, 'pttl');
    is(execute($engine, 'PERSIST', 'temporary')->value, 1, 'persist');
    is(execute($engine, 'EXPIREAT', 'temporary', '1999')->value, 1, 'expireat');
    ok(!defined(execute($engine, 'GET', 'temporary')->value), 'expired');
    is(execute($engine, 'TTL', 'temporary')->value, -2, 'missing ttl');
    execute($engine, 'SET', 'db0', 'zero');
    is(execute($engine, 'SELECT', '1')->value, 'OK', 'select');
    execute($engine, 'SET', 'db1', 'one');
    is(execute($engine, 'DBSIZE')->value, 1, 'dbsize');
    like(execute($engine, 'INFO')->value, qr/active_db:1/, 'info');
    is(execute($engine, 'FLUSHDB')->value, 'OK', 'flushdb');
    is(execute($engine, 'DBSIZE')->value, 0, 'empty db');
    execute($engine, 'SET', 'again', 'one');
    is(execute($engine, 'FLUSHALL')->value, 'OK', 'flushall');
    execute($engine, 'SELECT', '0');
    is(execute($engine, 'DBSIZE')->value, 0, 'all empty');
};

subtest 'protocol arity type and parse errors' => sub {
    my $engine = CodingAdventures::InMemoryDataStoreEngine->new;
    error_like($engine->execute_frame(undef), 'protocol error', 'protocol');
    error_like(execute($engine, 'NOPE'), 'unknown command', 'unknown');
    for my $parts (
        ['PING', 'a', 'b'], ['ECHO'], ['SET', 'a'], ['GET'], ['DEL'],
        ['HSET', 'a', 'b'], ['LPUSH', 'a'], ['SADD', 'a'],
        ['ZADD', 'a', '1'], ['PFADD', 'a'], ['EXPIRE', 'a'],
        ['SELECT'], ['FLUSHDB', 'x'], ['INFO', 'x'],
    ) {
        error_like($engine->execute_parts($parts), 'wrong number', join(' ', @$parts));
    }
    error_like(execute($engine, 'RENAME', 'missing', 'other'), 'no such key', 'rename missing');
    error_like(execute($engine, 'SELECT', '99'), 'DB index', 'select range');
    execute($engine, 'SET', 'string', 'value');
    for my $parts (
        ['HGET', 'string', 'field'], ['LPUSH', 'string', 'value'],
        ['SADD', 'string', 'value'], ['ZADD', 'string', '1', 'value'],
        ['PFADD', 'string', 'value'], ['SUNION', 'string'],
    ) {
        error_like($engine->execute_parts($parts), 'WRONGTYPE', join(' ', @$parts));
    }
    error_like(execute($engine, 'INCR', 'string'), 'integer', 'incr text');
    error_like(execute($engine, 'INCRBY', 'n', 'bad'), 'integer', 'incrby text');
    error_like(execute($engine, 'DECRBY', 'n', '-9223372036854775808'), 'integer', 'decrby minimum');
    execute($engine, 'SET', 'max', '9223372036854775807');
    error_like(execute($engine, 'INCR', 'max'), 'integer', 'overflow');
    error_like(execute($engine, 'ZADD', 'z', 'nan', 'a'), 'float', 'nan');
};

subtest 'storage and sorted set helpers' => sub {
    is(CodingAdventures::InMemoryDataStoreEngine::EntryType::STRING(), 'string', 'entry type constant');
    my $error = !eval { CodingAdventures::InMemoryDataStoreEngine::Store->new(database_count => 0); 1 };
    ok($error, 'rejects zero databases');
    my $store = CodingAdventures::InMemoryDataStoreEngine::Store->new(database_count => 2);
    $store->select(1);
    is($store->active_db, 1, 'active database');
    my $now = 50_000;
    my $database = CodingAdventures::InMemoryDataStoreEngine::Database->new(time_provider => sub { $now });
    $database->set('live', CodingAdventures::InMemoryDataStoreEngine::Entry->new('string', 'yes'));
    $database->set('old', CodingAdventures::InMemoryDataStoreEngine::Entry->new('string', 'no', $now - 1));
    ok(!defined($database->get('old')), 'lazy expiry');
    is_deeply($database->keys('l?ve'), ['live'], 'glob');
    $database->clear;
    is_deeply($database->{entries}, {}, 'clear');
    my $sorted = CodingAdventures::InMemoryDataStoreEngine::SortedSet->new;
    ok($sorted->insert(1, 'b'), 'new b');
    ok($sorted->insert(1, 'a'), 'new a');
    ok(!$sorted->insert(2, 'b'), 'update b');
    is($sorted->rank('a'), 0, 'rank');
    is_deeply($sorted->range_by_score(0, 1.5), [['a', 1]], 'score range');
    ok($sorted->remove('a'), 'remove');
};

subtest 'public frames and binary strings' => sub {
    my $engine = CodingAdventures::InMemoryDataStoreEngine->new(time_provider => sub { 1234 });
    is($engine->current_time_ms, 1234, 'clock');
    my $frame = CodingAdventures::InMemoryDataStoreProtocol::CommandFrame->new('ping');
    is($engine->execute_frame($frame)->value, 'PONG', 'frame');
    my $binary = "\0\xff\1";
    execute($engine, 'SET', $binary, $binary);
    is(execute($engine, 'GET', $binary)->value, $binary, 'binary round trip');
};

subtest 'deterministic randomized string reference model' => sub {
    my $engine = CodingAdventures::InMemoryDataStoreEngine->new;
    my %model;
    my $state = 20260716;
    my $next_random = sub {
        my ($limit) = @_;
        $state = ($state * 1103515245 + 12345) & 0x7fffffff;
        return $state % $limit;
    };
    for (1 .. 5000) {
        my $key = 'key:' . $next_random->(31);
        my $choice = $next_random->(6);
        if ($choice == 0) {
            my $value = '' . $next_random->(10000);
            is(execute($engine, 'SET', $key, $value)->value, 'OK', 'random set');
            $model{$key} = $value;
        } elsif ($choice == 1) {
            is(execute($engine, 'GET', $key)->value, $model{$key}, 'random get');
        } elsif ($choice == 2) {
            my $expected = exists($model{$key}) ? 1 : 0;
            is(execute($engine, 'DEL', $key)->value, $expected, 'random delete');
            delete $model{$key};
        } elsif ($choice == 3) {
            is(execute($engine, 'EXISTS', $key)->value, exists($model{$key}) ? 1 : 0, 'random exists');
        } elsif ($choice == 4) {
            my $suffix = chr(97 + $next_random->(26));
            $model{$key} = (defined($model{$key}) ? $model{$key} : '') . $suffix;
            is(execute($engine, 'APPEND', $key, $suffix)->value, length($model{$key}), 'random append');
        } else {
            my $current = $model{$key};
            if (!defined($current) || $current =~ /\A[+-]?\d+\z/) {
                my $expected = (defined($current) ? $current : 0) + 1;
                is(execute($engine, 'INCR', $key)->value, $expected, 'random incr');
                $model{$key} = "$expected";
            } else {
                error_like(execute($engine, 'INCR', $key), 'integer', 'random invalid incr');
            }
        }
    }
};

done_testing;
