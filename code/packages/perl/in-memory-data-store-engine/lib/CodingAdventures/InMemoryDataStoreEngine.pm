package CodingAdventures::InMemoryDataStoreEngine;

use strict;
use warnings;
use POSIX qw(isfinite);
use Time::HiRes qw(time);
use CodingAdventures::HyperLogLog;
use CodingAdventures::InMemoryDataStoreProtocol;

our $VERSION = '0.1.0';

my $RESPONSE = 'CodingAdventures::InMemoryDataStoreProtocol::EngineResponse';
my $FRAME = 'CodingAdventures::InMemoryDataStoreProtocol::CommandFrame';
my $I64_MAX = 9223372036854775807;
my $I64_MIN = -9223372036854775807 - 1;

sub new {
    my ($class, @arguments) = @_;
    die "constructor expects key/value options\n" if @arguments % 2;
    my %options = @arguments;
    my $now_provider = $options{time_provider} || sub { int(time() * 1000) };
    my $store = $options{store} || CodingAdventures::InMemoryDataStoreEngine::Store->new(
        database_count => $options{database_count},
        time_provider  => $now_provider,
    );
    return bless {store => $store, time_provider => $now_provider}, $class;
}

sub store { return $_[0]->{store}; }
sub current_time_ms { return $_[0]->{time_provider}->(); }

sub execute_frame {
    my ($self, $frame) = @_;
    return _error('ERR protocol error: expected array of bulk strings') if !defined $frame;
    $self->{store}->active_database->active_expire;
    my $command = uc($frame->command);
    my $method = '_cmd_' . lc($command);
    return _error("ERR unknown command '" . lc($frame->command) . "'") if !$self->can($method);
    return $self->$method($frame->args);
}

sub execute_parts {
    my ($self, $parts) = @_;
    return $self->execute_frame($FRAME->from_parts($parts));
}

sub _entry {
    my ($self, $key) = @_;
    return $self->{store}->active_database->get($key);
}

sub _ensure_collection {
    my ($self, $key, $type, $factory) = @_;
    my $entry = $self->_entry($key);
    if (!defined $entry) {
        $entry = CodingAdventures::InMemoryDataStoreEngine::Entry->new($type, $factory->());
        $self->{store}->active_database->set($key, $entry);
    }
    return $entry->{type} eq $type ? $entry : undef;
}

sub _cmd_ping {
    my ($self, $args) = @_;
    return $RESPONSE->simple_string('PONG') if !@$args;
    return _bulk($args->[0]) if @$args == 1;
    return _wrong_arity('ping');
}

sub _cmd_echo {
    my ($self, $args) = @_;
    return @$args == 1 ? _bulk($args->[0]) : _wrong_arity('echo');
}

sub _cmd_set {
    my ($self, $args) = @_;
    return _wrong_arity('set') if @$args != 2;
    $self->{store}->active_database->set(
        $args->[0],
        CodingAdventures::InMemoryDataStoreEngine::Entry->new('string', $args->[1]),
    );
    return $RESPONSE->ok;
}

sub _cmd_get {
    my ($self, $args) = @_;
    return _wrong_arity('get') if @$args != 1;
    my $entry = $self->_entry($args->[0]);
    return $RESPONSE->null if !defined $entry;
    return _wrong_type() if $entry->{type} ne 'string';
    return _bulk($entry->{value});
}

sub _cmd_del {
    my ($self, $args) = @_;
    return _wrong_arity('del') if !@$args;
    my $removed = 0;
    $removed += $self->{store}->active_database->delete($_) for @$args;
    return _integer($removed);
}

sub _cmd_exists {
    my ($self, $args) = @_;
    return _wrong_arity('exists') if !@$args;
    my $found = 0;
    $found++ for grep { defined $self->_entry($_) } @$args;
    return _integer($found);
}

sub _cmd_keys {
    my ($self, $args) = @_;
    return _wrong_arity('keys') if @$args != 1;
    return _array([map { _bulk($_) } @{$self->{store}->active_database->keys($args->[0])}]);
}

sub _cmd_type {
    my ($self, $args) = @_;
    return _wrong_arity('type') if @$args != 1;
    my $entry = $self->_entry($args->[0]);
    return $RESPONSE->simple_string(defined($entry) ? $entry->{type} : 'none');
}

sub _cmd_rename {
    my ($self, $args) = @_;
    return _wrong_arity('rename') if @$args != 2;
    my $entry = $self->_entry($args->[0]);
    return _error('ERR no such key') if !defined $entry;
    if ($args->[0] ne $args->[1]) {
        $self->{store}->active_database->delete($args->[0]);
        $self->{store}->active_database->set($args->[1], $entry);
    }
    return $RESPONSE->ok;
}

sub _cmd_append {
    my ($self, $args) = @_;
    return _wrong_arity('append') if @$args != 2;
    my $entry = $self->_entry($args->[0]);
    if (!defined $entry) {
        $self->{store}->active_database->set(
            $args->[0],
            CodingAdventures::InMemoryDataStoreEngine::Entry->new('string', $args->[1]),
        );
        return _integer(length($args->[1]));
    }
    return _wrong_type() if $entry->{type} ne 'string';
    $entry->{value} .= $args->[1];
    return _integer(length($entry->{value}));
}

sub _incr_by {
    my ($self, $args, $fixed_delta, $command) = @_;
    my $expected = defined($fixed_delta) ? 1 : 2;
    return _wrong_arity($command) if @$args != $expected;
    my $delta = defined($fixed_delta) ? $fixed_delta : _parse_i64($args->[1]);
    return _integer_error() if !defined $delta;
    my $entry = $self->_entry($args->[0]);
    return _wrong_type() if defined($entry) && $entry->{type} ne 'string';
    my $current = defined($entry) ? _parse_i64($entry->{value}) : 0;
    return _integer_error() if !defined $current;
    return _integer_error()
        if ($delta > 0 && $current > $I64_MAX - $delta)
        || ($delta < 0 && $current < $I64_MIN - $delta);
    my $result = $current + $delta;
    $self->{store}->active_database->set(
        $args->[0],
        CodingAdventures::InMemoryDataStoreEngine::Entry->new(
            'string', "$result", defined($entry) ? $entry->{expires_at_ms} : undef,
        ),
    );
    return _integer($result);
}

sub _cmd_incr { return $_[0]->_incr_by($_[1], 1, 'incr'); }
sub _cmd_decr { return $_[0]->_incr_by($_[1], -1, 'decr'); }
sub _cmd_incrby { return $_[0]->_incr_by($_[1], undef, 'incrby'); }

sub _cmd_decrby {
    my ($self, $args) = @_;
    return _wrong_arity('decrby') if @$args != 2;
    my $delta = _parse_i64($args->[1]);
    return _integer_error() if !defined($delta) || $delta == $I64_MIN;
    return $self->_incr_by([$args->[0], '' . -$delta], undef, 'decrby');
}

sub _cmd_hset {
    my ($self, $args) = @_;
    return _wrong_arity('hset') if @$args < 3 || @$args % 2 == 0;
    my $entry = $self->_ensure_collection($args->[0], 'hash', sub { {} });
    return _wrong_type() if !defined $entry;
    my $added = 0;
    for (my $index = 1; $index < @$args; $index += 2) {
        $added++ if !exists $entry->{value}{$args->[$index]};
        $entry->{value}{$args->[$index]} = $args->[$index + 1];
    }
    return _integer($added);
}

sub _cmd_hget {
    my ($self, $args) = @_;
    return _wrong_arity('hget') if @$args != 2;
    my $entry = $self->_entry($args->[0]);
    return $RESPONSE->null if !defined $entry;
    return _wrong_type() if $entry->{type} ne 'hash';
    return _bulk($entry->{value}{$args->[1]});
}

sub _cmd_hdel {
    my ($self, $args) = @_;
    return _wrong_arity('hdel') if @$args < 2;
    my $entry = $self->_entry($args->[0]);
    return $RESPONSE->zero if !defined $entry;
    return _wrong_type() if $entry->{type} ne 'hash';
    my $removed = 0;
    for my $field (@$args[1 .. $#$args]) {
        if (exists $entry->{value}{$field}) {
            delete $entry->{value}{$field};
            $removed++;
        }
    }
    $self->{store}->active_database->delete($args->[0]) if !keys %{$entry->{value}};
    return _integer($removed);
}

sub _hash_array {
    my ($self, $args, $command, $mode) = @_;
    return _wrong_arity($command) if @$args != 1;
    my $entry = $self->_entry($args->[0]);
    return _array([]) if !defined $entry;
    return _wrong_type() if $entry->{type} ne 'hash';
    my @values;
    for my $field (sort keys %{$entry->{value}}) {
        push @values, _bulk($field) if $mode ne 'values';
        push @values, _bulk($entry->{value}{$field}) if $mode ne 'keys';
    }
    return _array(\@values);
}

sub _cmd_hgetall { return $_[0]->_hash_array($_[1], 'hgetall', 'all'); }
sub _cmd_hkeys { return $_[0]->_hash_array($_[1], 'hkeys', 'keys'); }
sub _cmd_hvals { return $_[0]->_hash_array($_[1], 'hvals', 'values'); }

sub _cmd_hlen {
    my ($self, $args) = @_;
    return _wrong_arity('hlen') if @$args != 1;
    my $entry = $self->_entry($args->[0]);
    return $RESPONSE->zero if !defined $entry;
    return _wrong_type() if $entry->{type} ne 'hash';
    return _integer(scalar keys %{$entry->{value}});
}

sub _cmd_hexists {
    my ($self, $args) = @_;
    return _wrong_arity('hexists') if @$args != 2;
    my $entry = $self->_entry($args->[0]);
    return $RESPONSE->zero if !defined $entry;
    return _wrong_type() if $entry->{type} ne 'hash';
    return _integer(exists($entry->{value}{$args->[1]}) ? 1 : 0);
}

sub _push_list {
    my ($self, $args, $left) = @_;
    my $command = $left ? 'lpush' : 'rpush';
    return _wrong_arity($command) if @$args < 2;
    my $entry = $self->_ensure_collection($args->[0], 'list', sub { [] });
    return _wrong_type() if !defined $entry;
    for my $value (@$args[1 .. $#$args]) {
        $left ? unshift(@{$entry->{value}}, $value) : push(@{$entry->{value}}, $value);
    }
    return _integer(scalar @{$entry->{value}});
}

sub _cmd_lpush { return $_[0]->_push_list($_[1], 1); }
sub _cmd_rpush { return $_[0]->_push_list($_[1], 0); }

sub _pop_list {
    my ($self, $args, $left) = @_;
    my $command = $left ? 'lpop' : 'rpop';
    return _wrong_arity($command) if @$args != 1;
    my $entry = $self->_entry($args->[0]);
    return $RESPONSE->null if !defined $entry;
    return _wrong_type() if $entry->{type} ne 'list';
    my $value = $left ? shift(@{$entry->{value}}) : pop(@{$entry->{value}});
    $self->{store}->active_database->delete($args->[0]) if !@{$entry->{value}};
    return _bulk($value);
}

sub _cmd_lpop { return $_[0]->_pop_list($_[1], 1); }
sub _cmd_rpop { return $_[0]->_pop_list($_[1], 0); }

sub _cmd_llen {
    my ($self, $args) = @_;
    return _wrong_arity('llen') if @$args != 1;
    my $entry = $self->_entry($args->[0]);
    return $RESPONSE->zero if !defined $entry;
    return _wrong_type() if $entry->{type} ne 'list';
    return _integer(scalar @{$entry->{value}});
}

sub _cmd_lindex {
    my ($self, $args) = @_;
    return _wrong_arity('lindex') if @$args != 2;
    my $entry = $self->_entry($args->[0]);
    return $RESPONSE->null if !defined $entry;
    return _wrong_type() if $entry->{type} ne 'list';
    my $index = _parse_i64($args->[1]);
    return _integer_error() if !defined $index;
    $index += @{$entry->{value}} if $index < 0;
    return $RESPONSE->null if $index < 0 || $index >= @{$entry->{value}};
    return _bulk($entry->{value}[$index]);
}

sub _cmd_lrange {
    my ($self, $args) = @_;
    return _wrong_arity('lrange') if @$args != 3;
    my $entry = $self->_entry($args->[0]);
    return _array([]) if !defined $entry;
    return _wrong_type() if $entry->{type} ne 'list';
    my $start = _parse_i64($args->[1]);
    my $stop = _parse_i64($args->[2]);
    return _integer_error() if !defined($start) || !defined($stop);
    my $length = @{$entry->{value}};
    $start += $length if $start < 0;
    $stop += $length if $stop < 0;
    $start = 0 if $start < 0;
    $stop = $length - 1 if $stop >= $length;
    return _array([]) if !$length || $start > $stop || $start >= $length;
    return _array([map { _bulk($entry->{value}[$_]) } $start .. $stop]);
}

sub _cmd_sadd {
    my ($self, $args) = @_;
    return _wrong_arity('sadd') if @$args < 2;
    my $entry = $self->_ensure_collection($args->[0], 'set', sub { {} });
    return _wrong_type() if !defined $entry;
    my $added = 0;
    for my $value (@$args[1 .. $#$args]) {
        $added++ if !exists $entry->{value}{$value};
        $entry->{value}{$value} = 1;
    }
    return _integer($added);
}

sub _cmd_srem {
    my ($self, $args) = @_;
    return _wrong_arity('srem') if @$args < 2;
    my $entry = $self->_entry($args->[0]);
    return $RESPONSE->zero if !defined $entry;
    return _wrong_type() if $entry->{type} ne 'set';
    my $removed = 0;
    for my $value (@$args[1 .. $#$args]) {
        if (exists $entry->{value}{$value}) {
            delete $entry->{value}{$value};
            $removed++;
        }
    }
    $self->{store}->active_database->delete($args->[0]) if !keys %{$entry->{value}};
    return _integer($removed);
}

sub _cmd_sismember {
    my ($self, $args) = @_;
    return _wrong_arity('sismember') if @$args != 2;
    my $entry = $self->_entry($args->[0]);
    return $RESPONSE->zero if !defined $entry;
    return _wrong_type() if $entry->{type} ne 'set';
    return _integer(exists($entry->{value}{$args->[1]}) ? 1 : 0);
}

sub _cmd_smembers {
    my ($self, $args) = @_;
    return _wrong_arity('smembers') if @$args != 1;
    my $entry = $self->_entry($args->[0]);
    return _array([]) if !defined $entry;
    return _wrong_type() if $entry->{type} ne 'set';
    return _array([map { _bulk($_) } sort keys %{$entry->{value}}]);
}

sub _cmd_scard {
    my ($self, $args) = @_;
    return _wrong_arity('scard') if @$args != 1;
    my $entry = $self->_entry($args->[0]);
    return $RESPONSE->zero if !defined $entry;
    return _wrong_type() if $entry->{type} ne 'set';
    return _integer(scalar keys %{$entry->{value}});
}

sub _set_operation {
    my ($self, $args, $command, $operation) = @_;
    return _wrong_arity($command) if !@$args;
    my $first = $self->_entry($args->[0]);
    return _wrong_type() if defined($first) && $first->{type} ne 'set';
    my %result = defined($first) ? %{$first->{value}} : ();
    for my $key (@$args[1 .. $#$args]) {
        my $entry = $self->_entry($key);
        return _wrong_type() if defined($entry) && $entry->{type} ne 'set';
        if ($operation eq 'union') {
            $result{$_} = 1 for defined($entry) ? keys %{$entry->{value}} : ();
        } elsif ($operation eq 'intersection') {
            delete $result{$_} for grep { !defined($entry) || !exists($entry->{value}{$_}) } keys %result;
        } elsif (defined $entry) {
            delete $result{$_} for keys %{$entry->{value}};
        }
    }
    return _array([map { _bulk($_) } sort keys %result]);
}

sub _cmd_sunion { return $_[0]->_set_operation($_[1], 'sunion', 'union'); }
sub _cmd_sinter { return $_[0]->_set_operation($_[1], 'sinter', 'intersection'); }
sub _cmd_sdiff { return $_[0]->_set_operation($_[1], 'sdiff', 'difference'); }

sub _cmd_zadd {
    my ($self, $args) = @_;
    return _wrong_arity('zadd') if @$args < 3 || @$args % 2 == 0;
    my @parsed;
    for (my $index = 1; $index < @$args; $index += 2) {
        my $score = _parse_float($args->[$index]);
        return _float_error() if !defined $score;
        push @parsed, [$score, $args->[$index + 1]];
    }
    my $entry = $self->_ensure_collection(
        $args->[0], 'zset', sub { CodingAdventures::InMemoryDataStoreEngine::SortedSet->new },
    );
    return _wrong_type() if !defined $entry;
    my $added = 0;
    $added += $entry->{value}->insert(@$_) for @parsed;
    return _integer($added);
}

sub _flatten_zset {
    my ($values, $with_scores) = @_;
    my @result;
    for my $item (@$values) {
        push @result, _bulk($item->[0]);
        push @result, _bulk(_format_score($item->[1])) if $with_scores;
    }
    return \@result;
}

sub _cmd_zrange {
    my ($self, $args) = @_;
    return _wrong_arity('zrange') if @$args != 3 && @$args != 4;
    my $start = _parse_i64($args->[1]);
    my $end = _parse_i64($args->[2]);
    return _integer_error() if !defined($start) || !defined($end);
    my $entry = $self->_entry($args->[0]);
    return _array([]) if !defined $entry;
    return _wrong_type() if $entry->{type} ne 'zset';
    my $with_scores = @$args == 4 && uc($args->[3]) eq 'WITHSCORES';
    return _array(_flatten_zset($entry->{value}->range_by_index($start, $end), $with_scores));
}

sub _cmd_zrangebyscore {
    my ($self, $args) = @_;
    return _wrong_arity('zrangebyscore') if @$args != 3 && @$args != 4;
    my $minimum = _parse_float($args->[1]);
    my $maximum = _parse_float($args->[2]);
    return _float_error() if !defined($minimum) || !defined($maximum);
    my $entry = $self->_entry($args->[0]);
    return _array([]) if !defined $entry;
    return _wrong_type() if $entry->{type} ne 'zset';
    my $with_scores = @$args == 4 && uc($args->[3]) eq 'WITHSCORES';
    return _array(_flatten_zset($entry->{value}->range_by_score($minimum, $maximum), $with_scores));
}

sub _cmd_zrank {
    my ($self, $args) = @_;
    return _wrong_arity('zrank') if @$args != 2;
    my $entry = $self->_entry($args->[0]);
    return $RESPONSE->null if !defined $entry;
    return _wrong_type() if $entry->{type} ne 'zset';
    my $rank = $entry->{value}->rank($args->[1]);
    return defined($rank) ? _integer($rank) : $RESPONSE->null;
}

sub _cmd_zscore {
    my ($self, $args) = @_;
    return _wrong_arity('zscore') if @$args != 2;
    my $entry = $self->_entry($args->[0]);
    return $RESPONSE->null if !defined $entry;
    return _wrong_type() if $entry->{type} ne 'zset';
    my $score = $entry->{value}->score($args->[1]);
    return defined($score) ? _bulk(_format_score($score)) : $RESPONSE->null;
}

sub _cmd_zcard {
    my ($self, $args) = @_;
    return _wrong_arity('zcard') if @$args != 1;
    my $entry = $self->_entry($args->[0]);
    return $RESPONSE->zero if !defined $entry;
    return _wrong_type() if $entry->{type} ne 'zset';
    return _integer($entry->{value}->size);
}

sub _cmd_zrem {
    my ($self, $args) = @_;
    return _wrong_arity('zrem') if @$args < 2;
    my $entry = $self->_entry($args->[0]);
    return $RESPONSE->zero if !defined $entry;
    return _wrong_type() if $entry->{type} ne 'zset';
    my $removed = 0;
    $removed += $entry->{value}->remove($_) for @$args[1 .. $#$args];
    $self->{store}->active_database->delete($args->[0]) if !$entry->{value}->size;
    return _integer($removed);
}

sub _cmd_pfadd {
    my ($self, $args) = @_;
    return _wrong_arity('pfadd') if @$args < 2;
    my $entry = $self->_ensure_collection(
        $args->[0], 'hll', sub { CodingAdventures::HyperLogLog->new },
    );
    return _wrong_type() if !defined $entry;
    my $before = join(',', @{$entry->{value}->registers});
    $entry->{value}->add($_) for @$args[1 .. $#$args];
    return _integer($before ne join(',', @{$entry->{value}->registers}) ? 1 : 0);
}

sub _cmd_pfcount {
    my ($self, $args) = @_;
    return _wrong_arity('pfcount') if !@$args;
    my $aggregate;
    for my $key (@$args) {
        my $entry = $self->_entry($key);
        next if !defined $entry;
        return _wrong_type() if $entry->{type} ne 'hll';
        $aggregate = defined($aggregate) ? $aggregate->merge($entry->{value}) : $entry->{value};
    }
    return _integer(defined($aggregate) ? $aggregate->count : 0);
}

sub _cmd_pfmerge {
    my ($self, $args) = @_;
    return _wrong_arity('pfmerge') if @$args < 2;
    my $aggregate;
    for my $key (@$args[1 .. $#$args]) {
        my $entry = $self->_entry($key);
        next if !defined $entry;
        return _wrong_type() if $entry->{type} ne 'hll';
        $aggregate = defined($aggregate) ? $aggregate->merge($entry->{value}) : $entry->{value};
    }
    my $destination = $self->_entry($args->[0]);
    $self->{store}->active_database->set(
        $args->[0],
        CodingAdventures::InMemoryDataStoreEngine::Entry->new(
            'hll',
            defined($aggregate) ? $aggregate : CodingAdventures::HyperLogLog->new,
            defined($destination) ? $destination->{expires_at_ms} : undef,
        ),
    );
    return $RESPONSE->ok;
}

sub _expire {
    my ($self, $args, $absolute) = @_;
    my $command = $absolute ? 'expireat' : 'expire';
    return _wrong_arity($command) if @$args != 2;
    my $entry = $self->_entry($args->[0]);
    return $RESPONSE->zero if !defined $entry;
    my $seconds = _parse_i64($args->[1]);
    return _integer_error() if !defined $seconds;
    $entry->{expires_at_ms} = $absolute
        ? $seconds * 1000
        : $self->{time_provider}->() + $seconds * 1000;
    return $RESPONSE->one;
}

sub _cmd_expire { return $_[0]->_expire($_[1], 0); }
sub _cmd_expireat { return $_[0]->_expire($_[1], 1); }

sub _cmd_ttl {
    my ($self, $args) = @_;
    return _wrong_arity('ttl') if @$args != 1;
    my $entry = $self->_entry($args->[0]);
    return _integer(-2) if !defined $entry;
    return _integer(-1) if !defined $entry->{expires_at_ms};
    my $remaining = int(($entry->{expires_at_ms} - $self->{time_provider}->()) / 1000);
    return _integer($remaining < -2 ? -2 : $remaining);
}

sub _cmd_pttl {
    my ($self, $args) = @_;
    return _wrong_arity('pttl') if @$args != 1;
    my $entry = $self->_entry($args->[0]);
    return _integer(-2) if !defined $entry;
    return _integer(-1) if !defined $entry->{expires_at_ms};
    my $remaining = $entry->{expires_at_ms} - $self->{time_provider}->();
    return _integer($remaining < -1 ? -1 : $remaining);
}

sub _cmd_persist {
    my ($self, $args) = @_;
    return _wrong_arity('persist') if @$args != 1;
    my $entry = $self->_entry($args->[0]);
    return $RESPONSE->zero if !defined($entry) || !defined($entry->{expires_at_ms});
    $entry->{expires_at_ms} = undef;
    return $RESPONSE->one;
}

sub _cmd_select {
    my ($self, $args) = @_;
    return _wrong_arity('select') if @$args != 1;
    my $index = _parse_i64($args->[0]);
    return _error('ERR DB index is out of range')
        if !defined($index) || $index < 0 || $index >= @{$self->{store}->{databases}};
    $self->{store}->select($index);
    return $RESPONSE->ok;
}

sub _cmd_flushdb {
    my ($self, $args) = @_;
    return _wrong_arity('flushdb') if @$args;
    $self->{store}->flushdb;
    return $RESPONSE->ok;
}

sub _cmd_flushall {
    my ($self, $args) = @_;
    return _wrong_arity('flushall') if @$args;
    $self->{store}->flushall;
    return $RESPONSE->ok;
}

sub _cmd_dbsize {
    my ($self, $args) = @_;
    return _wrong_arity('dbsize') if @$args;
    $self->{store}->active_database->active_expire;
    return _integer(scalar keys %{$self->{store}->active_database->{entries}});
}

sub _cmd_info {
    my ($self, $args) = @_;
    return _wrong_arity('info') if @$args;
    my $size = scalar keys %{$self->{store}->active_database->{entries}};
    return _bulk(
        "# Server\r\nin_memory_data_store_version:0.1.0\r\n"
        . "active_db:$self->{store}->{active_db}\r\ndbsize:$size\r\n"
    );
}

sub _bulk { return $RESPONSE->bulk_string($_[0]); }
sub _integer { return $RESPONSE->integer(0 + $_[0]); }
sub _array { return $RESPONSE->array($_[0]); }
sub _error { return $RESPONSE->error($_[0]); }
sub _wrong_arity { return _error("ERR wrong number of arguments for '$_[0]' command"); }
sub _wrong_type { return _error('WRONGTYPE Operation against a key holding the wrong kind of value'); }
sub _integer_error { return _error('ERR value is not an integer or out of range'); }
sub _float_error { return _error('ERR value is not a valid float'); }

sub _parse_i64 {
    my ($value) = @_;
    return undef if !defined($value) || ref($value) || $value !~ /\A([+-]?)(\d+)\z/;
    my ($sign, $digits) = ($1, $2);
    $digits =~ s/\A0+(?=\d)//;
    my $limit = $sign eq '-' ? '9223372036854775808' : '9223372036854775807';
    return undef if length($digits) > length($limit)
        || (length($digits) == length($limit) && $digits gt $limit);
    return 0 + $value;
}

sub _parse_float {
    my ($value) = @_;
    return undef if !defined($value) || ref($value)
        || $value !~ /\A[+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?\z/;
    my $parsed = 0.0 + $value;
    return isfinite($parsed) ? $parsed : undef;
}

sub _format_score {
    my ($score) = @_;
    return sprintf('%.0f', $score) if $score == int($score);
    my $text = sprintf('%.15f', $score);
    $text =~ s/0+\z//;
    $text =~ s/\.\z//;
    return $text;
}

package CodingAdventures::InMemoryDataStoreEngine::EntryType;

use strict;
use warnings;
use constant {
    STRING => 'string',
    HASH   => 'hash',
    LIST   => 'list',
    SET    => 'set',
    ZSET   => 'zset',
    HLL    => 'hll',
};

package CodingAdventures::InMemoryDataStoreEngine::Entry;

use strict;
use warnings;

sub new {
    my ($class, $type, $value, $expires_at_ms) = @_;
    return bless {type => $type, value => $value, expires_at_ms => $expires_at_ms}, $class;
}

sub type { return $_[0]->{type}; }
sub value { return $_[0]->{value}; }
sub expires_at_ms { return $_[0]->{expires_at_ms}; }

package CodingAdventures::InMemoryDataStoreEngine::SortedSet;

use strict;
use warnings;

sub new { return bless {scores => {}}, $_[0]; }

sub insert {
    my ($self, $score, $member) = @_;
    my $is_new = !exists $self->{scores}{$member};
    $self->{scores}{$member} = $score;
    return $is_new ? 1 : 0;
}

sub remove {
    my ($self, $member) = @_;
    return 0 if !exists $self->{scores}{$member};
    delete $self->{scores}{$member};
    return 1;
}

sub ordered_entries {
    my ($self) = @_;
    return [map { [$_, $self->{scores}{$_}] }
        sort { $self->{scores}{$a} <=> $self->{scores}{$b} || $a cmp $b }
        keys %{$self->{scores}}];
}

sub rank {
    my ($self, $member) = @_;
    my $entries = $self->ordered_entries;
    for my $index (0 .. $#$entries) {
        return $index if $entries->[$index][0] eq $member;
    }
    return undef;
}

sub score {
    my ($self, $member) = @_;
    return $self->{scores}{$member};
}

sub size { return scalar keys %{$_[0]->{scores}}; }

sub range_by_index {
    my ($self, $start, $end) = @_;
    my $entries = $self->ordered_entries;
    my $length = @$entries;
    return [] if !$length;
    $start += $length if $start < 0;
    $end += $length if $end < 0;
    return [] if $start < 0 || $end < 0 || $start >= $length || $start > $end;
    $end = $length - 1 if $end >= $length;
    return [@$entries[$start .. $end]];
}

sub range_by_score {
    my ($self, $minimum, $maximum) = @_;
    return [grep { $_->[1] >= $minimum && $_->[1] <= $maximum } @{$self->ordered_entries}];
}

package CodingAdventures::InMemoryDataStoreEngine::Database;

use strict;
use warnings;
use Time::HiRes qw(time);

sub new {
    my ($class, %options) = @_;
    return bless {
        entries       => {},
        time_provider => $options{time_provider} || sub { int(time() * 1000) },
    }, $class;
}

sub get {
    my ($self, $key) = @_;
    my $entry = $self->{entries}{$key};
    if (defined($entry) && defined($entry->{expires_at_ms})
        && $entry->{expires_at_ms} <= $self->{time_provider}->()) {
        delete $self->{entries}{$key};
        return undef;
    }
    return $entry;
}

sub set { $_[0]->{entries}{$_[1]} = $_[2]; return $_[2]; }

sub delete {
    my ($self, $key) = @_;
    return 0 if !exists $self->{entries}{$key};
    delete $self->{entries}{$key};
    return 1;
}

sub expire_lazy { $_[0]->get($_[1]); return; }

sub active_expire {
    my ($self) = @_;
    my $now = $self->{time_provider}->();
    for my $key (keys %{$self->{entries}}) {
        my $entry = $self->{entries}{$key};
        delete $self->{entries}{$key}
            if defined($entry->{expires_at_ms}) && $entry->{expires_at_ms} <= $now;
    }
    return;
}

sub keys {
    my ($self, $pattern) = @_;
    $self->active_expire;
    return [sort grep { _glob_match($pattern, $_) } keys %{$self->{entries}}];
}

sub clear { $_[0]->{entries} = {}; return; }
sub entries { return $_[0]->{entries}; }

sub _glob_match {
    my ($pattern, $value) = @_;
    my ($pi, $vi) = (0, 0);
    my ($star, $retry) = (-1, 0);
    while ($vi < length($value)) {
        if ($pi < length($pattern)
            && (substr($pattern, $pi, 1) eq '?' || substr($pattern, $pi, 1) eq substr($value, $vi, 1))) {
            $pi++; $vi++;
        } elsif ($pi < length($pattern) && substr($pattern, $pi, 1) eq '*') {
            $star = $pi; $retry = $vi; $pi++;
        } elsif ($star >= 0) {
            $retry++; $vi = $retry; $pi = $star + 1;
        } else {
            return 0;
        }
    }
    $pi++ while $pi < length($pattern) && substr($pattern, $pi, 1) eq '*';
    return $pi == length($pattern) ? 1 : 0;
}

package CodingAdventures::InMemoryDataStoreEngine::Store;

use strict;
use warnings;
use Time::HiRes qw(time);

sub new {
    my ($class, %options) = @_;
    my $count = defined($options{database_count}) ? $options{database_count} : 16;
    die "database_count must be positive\n"
        if $count !~ /\A\d+\z/ || $count <= 0;
    my $provider = $options{time_provider} || sub { int(time() * 1000) };
    my @databases = map {
        CodingAdventures::InMemoryDataStoreEngine::Database->new(time_provider => $provider)
    } 1 .. $count;
    return bless {databases => \@databases, active_db => 0}, $class;
}

sub active_db { return $_[0]->{active_db}; }
sub databases { return [@{$_[0]->{databases}}]; }
sub active_database { return $_[0]->{databases}[$_[0]->{active_db}]; }
sub select { $_[0]->{active_db} = $_[1]; return; }
sub flushdb { $_[0]->active_database->clear; return; }
sub flushall { $_->clear for @{$_[0]->{databases}}; return; }

package CodingAdventures::InMemoryDataStoreEngine::DataStoreEngine;

use strict;
use warnings;
our @ISA = ('CodingAdventures::InMemoryDataStoreEngine');

1;
