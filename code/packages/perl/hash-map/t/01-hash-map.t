use strict;
use warnings;
use utf8;
use Test::More;
use CodingAdventures::HashFunctions qw(fnv1a_32);
use CodingAdventures::HashMap qw(new_map from_entries merge);

sub dies_like {
    my ($code, $pattern, $name) = @_;
    my $ok = !eval { $code->(); 1 };
    ok($ok, $name);
    like($@, $pattern, "$name message") if $ok;
}

sub collision_pair {
    my ($capacity) = @_;
    my %first_by_bucket;
    for my $index (1 .. 1_000) {
        my $key = "collision-$index";
        my $bucket = fnv1a_32("scalar:$key") % $capacity;
        return ($first_by_bucket{$bucket}, $key) if exists $first_by_bucket{$bucket};
        $first_by_bucket{$bucket} = $key;
    }
    die "failed to find colliding keys";
}

subtest 'basic operations work with both strategies' => sub {
    for my $strategy (qw(chaining open_addressing)) {
        my $map = CodingAdventures::HashMap->new(capacity => 8, strategy => $strategy);
        is($map->set('hello', 42), $map, "$strategy set returns self");
        $map->set('world', 7)->set('hello', 99);
        is($map->size, 2, "$strategy size");
        is($map->get('hello'), 99, "$strategy overwrite");
        ok($map->has('world'), "$strategy membership");
        ok($map->delete('hello'), "$strategy delete");
        ok(!$map->delete('missing'), "$strategy missing delete");
        ok(!$map->has('hello'), "$strategy removed");
    }
};

subtest 'undef values preserve membership' => sub {
    for my $strategy (qw(chaining open_addressing)) {
        my $map = new_map(capacity => 8, strategy => $strategy);
        $map->set('undef-value', undef);
        ok($map->has('undef-value'), "$strategy has undef value");
        is($map->get('undef-value'), undef, "$strategy retrieves undef");
        is($map->size, 1, "$strategy counts undef value");
    }
};

subtest 'chaining handles collisions and resizes' => sub {
    my $map = CodingAdventures::HashMap->new(capacity => 2);
    $map->set('cat', 1)->set('car', 2)->set('cab', 3);
    is($map->capacity, 4, 'capacity doubled');
    is($map->get('cat'), 1, 'first key survives');
    is($map->get('car'), 2, 'second key survives');
    is($map->get('cab'), 3, 'third key survives');
    cmp_ok($map->load_factor, '<=', 1.0, 'load factor restored');
};

subtest 'open addressing preserves probe chains across tombstones' => sub {
    my ($first, $second) = collision_pair(8);
    my $map = CodingAdventures::HashMap->new(
        capacity => 8,
        strategy => 'open_addressing',
    );
    $map->set($first, 1)->set($second, 2);
    ok($map->delete($first), 'colliding first key deleted');
    is($map->get($second), 2, 'second key remains behind tombstone');
    $map->set('replacement', 3);
    is($map->get('replacement'), 3, 'insertion after tombstone works');
};

subtest 'open addressing resizes at 0.75 load' => sub {
    my $map = CodingAdventures::HashMap->new(capacity => 4, strategy => 'open');
    $map->set("key-$_", $_) for 1 .. 4;
    is($map->capacity, 8, 'capacity doubled');
    is($map->size, 4, 'size retained');
    is($map->get("key-$_"), $_, "key $_ retained") for 1 .. 4;
};

subtest 'all packaged hash functions and aliases work' => sub {
    for my $hash_fn (qw(fnv1a fnv1a_32 murmur3 murmur3_32 djb2)) {
        my $map = CodingAdventures::HashMap->new(
            capacity => 4,
            strategy => 'open-addressing',
            hash_fn  => $hash_fn,
        );
        $map->set('Ada', 'Lovelace');
        is($map->get('Ada'), 'Lovelace', "$hash_fn lookup");
    }
};

subtest 'bulk, merge, clone, clear, and reference keys work' => sub {
    my $left = from_entries([['a', 1], ['b', 2]]);
    my $right = from_entries([['b', 99], ['c', 3]]);
    my $merged = merge($left, $right);
    my %entries = map { $_->[0] => $_->[1] } @{$merged->entries};
    is_deeply(\%entries, {a => 1, b => 99, c => 3}, 'right map wins');
    is(scalar @{$merged->keys}, 3, 'keys enumerated');
    is(scalar @{$merged->values}, 3, 'values enumerated');
    my $clone = $merged->clone->set('d', 4);
    ok(!$merged->has('d'), 'clone does not mutate original');
    is($clone->clear->size, 0, 'clear empties clone');
    my $reference = [];
    $merged->set($reference, 'identity');
    is($merged->get($reference), 'identity', 'reference key found by identity');
    ok(!$merged->has([]), 'different reference is a different key');
};

subtest 'copy-on-write helpers leave inputs unchanged' => sub {
    my $empty = CodingAdventures::HashMap->new;
    my $filled = $empty->with_set('a', 1);
    my $removed = $filled->without('a');
    is($empty->size, 0, 'with_set leaves input empty');
    is($filled->get('a'), 1, 'with_set writes clone');
    ok(!$removed->has('a'), 'without removes from clone');
};

subtest 'invalid configurations and keys are rejected' => sub {
    dies_like(
        sub { CodingAdventures::HashMap->new(capacity => 0) },
        qr/capacity must be a positive integer/,
        'zero capacity',
    );
    dies_like(
        sub { CodingAdventures::HashMap->new(strategy => 'quadratic') },
        qr/strategy must be 'chaining' or 'open_addressing'/,
        'unknown strategy',
    );
    dies_like(
        sub { CodingAdventures::HashMap->new(hash_fn => 'sha256') },
        qr/hash_fn must be 'fnv1a', 'murmur3', or 'djb2'/,
        'unknown hash',
    );
    dies_like(
        sub { CodingAdventures::HashMap->new->set(undef, 1) },
        qr/key must be defined/,
        'undef key',
    );
    dies_like(
        sub { from_entries([[undef, 1]]) },
        qr/each entry must be a two-item array reference with a defined key/,
        'invalid entry',
    );
};

done_testing;
