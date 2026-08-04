use strict;
use warnings;
use utf8;
use Test::More;
use CodingAdventures::RadixTree;

sub make_tree {
    my $tree = CodingAdventures::RadixTree->new;
    my $index = 0;
    $tree->insert($_, ++$index) for @_;
    return $tree;
}

sub dies_like {
    my ($code, $pattern, $name) = @_;
    my $ok = !eval { $code->(); 1 };
    ok($ok, $name);
    like($@, $pattern, "$name message") if $ok;
}

subtest 'empty tree' => sub {
    my $tree = CodingAdventures::RadixTree->new;
    is($tree->size, 0, 'size');
    is($tree->length, 0, 'length');
    ok($tree->is_empty, 'empty');
    is($tree->node_count, 1, 'root only');
    is($tree->search('anything'), undef, 'missing search');
    ok(!$tree->starts_with(''), 'empty prefix does not match empty tree');
    ok($tree->is_valid, 'valid');
};

subtest 'compressed-edge insertion cases and updates' => sub {
    my $tree = CodingAdventures::RadixTree->new;
    $tree->insert('application', 1);
    $tree->insert('apple', 2);
    $tree->insert('app', 3);
    $tree->insert('apt', 4);

    is($tree->search('application'), 1, 'extension');
    is($tree->search('apple'), 2, 'partial split');
    is($tree->search('app'), 3, 'prefix split');
    is($tree->search('apt'), 4, 'divergent split');
    is($tree->search('appl'), undef, 'prefix-only key absent');

    $tree->insert('app', 99);
    $tree->insert('undefined', undef);
    is($tree->search('app'), 99, 'value updated');
    ok($tree->contains_key('undefined'), 'terminal key may store undef');
    is($tree->size, 5, 'update did not grow size');
    ok($tree->is_valid, 'valid after splits');
};

subtest 'mid-edge prefix queries and sorted keys' => sub {
    my $tree = make_tree(qw(search searcher searching banana));
    ok($tree->starts_with('sear'), 'mid-edge prefix');
    ok(!$tree->starts_with('seek'), 'missing prefix');
    is_deeply(
        $tree->words_with_prefix('sear'),
        [qw(search searcher searching)],
        'mid-edge prefix results',
    );
    is_deeply(
        $tree->words_with_prefix('search'),
        [qw(search searcher searching)],
        'node-aligned prefix results',
    );
    is_deeply($tree->words_with_prefix('xyz'), [], 'missing prefix results');
    is_deeply($tree->keys, [qw(banana search searcher searching)], 'sorted keys');
    is($tree->node_count, 5, 'compressed node count');
};

subtest 'delete merges compressed edges' => sub {
    my $tree = make_tree(qw(app apple apt));
    ok($tree->delete('app'), 'delete shared prefix');
    ok(!$tree->contains('app'), 'deleted key absent');
    is($tree->search('apple'), 2, 'longer key preserved');
    is($tree->search('apt'), 3, 'sibling preserved');
    ok(!$tree->delete('missing'), 'missing delete');
    ok(!$tree->delete('ap'), 'prefix-only delete');
    ok($tree->delete('apple'), 'delete leaf');
    is($tree->node_count, 2, 'redundant path merged');
    ok($tree->is_valid, 'valid after merge');
};

subtest 'longest prefix and empty-string keys' => sub {
    my $tree = CodingAdventures::RadixTree->new([
        ['', 'root'], ['a', 1], ['ab', 2], ['abc', 3], ['application', 4],
    ]);
    is($tree->longest_prefix_match('abcdef'), 'abc', 'longest match');
    is($tree->longest_prefix_match('application/json'), 'application', 'long edge match');
    is($tree->longest_prefix_match('xyz'), '', 'empty key is a prefix');
    is($tree->search(''), 'root', 'empty key value');
    ok($tree->starts_with(''), 'empty prefix matches non-empty tree');
    ok($tree->delete(''), 'delete empty key');
    is($tree->longest_prefix_match('xyz'), undef, 'no match after deletion');
};

subtest 'Unicode labels survive splits and merges' => sub {
    my $tree = CodingAdventures::RadixTree->new;
    $tree->insert("caf\x{00e9}", 'single');
    $tree->insert("cafe\x{0301}", 'combining');
    $tree->insert('cafeteria', 'food');
    $tree->insert("\x{732b}", 'cat');

    is($tree->search("caf\x{00e9}"), 'single', 'single-codepoint accent');
    is_deeply($tree->words_with_prefix('cafet'), ['cafeteria'], 'Unicode branch prefix');
    is(
        $tree->longest_prefix_match("cafe\x{0301}-au-lait"),
        "cafe\x{0301}",
        'Unicode longest prefix',
    );
    ok($tree->delete("cafe\x{0301}"), 'delete Unicode key');
    is($tree->search('cafeteria'), 'food', 'sibling remains');
    ok($tree->is_valid, 'valid Unicode structure');
};

subtest 'deterministic mixed mutations agree with a hash' => sub {
    my $tree = CodingAdventures::RadixTree->new;
    my %expected;
    for my $index (1 .. 200) {
        my $key = sprintf('route/%02d/%03d', $index % 17, ($index * 37) % 211);
        $tree->insert($key, $index);
        $expected{$key} = $index;
    }

    is($tree->size, scalar(keys %expected), 'unique size');
    is($tree->search($_), $expected{$_}, "lookup $_") for sort keys %expected;
    is_deeply($tree->keys, [sort keys %expected], 'keys match hash');

    my @to_delete = grep { $expected{$_} % 2 == 0 } keys %expected;
    for my $key (@to_delete) {
        ok($tree->delete($key), "delete $key");
        delete $expected{$key};
    }

    my $prefix = 'route/03';
    my @prefix_keys = sort grep { index($_, $prefix) == 0 } keys %expected;
    is_deeply($tree->words_with_prefix($prefix), \@prefix_keys, 'prefix keys match hash');
    ok($tree->is_valid, 'valid after mixed mutations');
};

subtest 'export, iteration, and input validation' => sub {
    my $tree = CodingAdventures::RadixTree->new([['b', 2], ['a', 1]]);
    is_deeply($tree->entries, [['a', 1], ['b', 2]], 'sorted entries');
    is_deeply($tree->to_hash, {a => 1, b => 2}, 'hash export');

    my @seen;
    $tree->each(sub { push @seen, [@_] });
    is_deeply(\@seen, $tree->entries, 'sorted callback iteration');
    like("$tree", qr/2 keys/, 'string rendering');

    dies_like(
        sub { CodingAdventures::RadixTree->new('not-an-array') },
        qr/entries must be an array reference/,
        'constructor validation',
    );
    dies_like(
        sub { CodingAdventures::RadixTree->new([[]]) },
        qr/entry at index 0 must contain a key/,
        'entry validation',
    );
    dies_like(sub { $tree->insert([]) }, qr/key must be a string/, 'key validation');
    dies_like(
        sub { $tree->starts_with({}) },
        qr/prefix must be a string/,
        'prefix validation',
    );
    dies_like(
        sub { $tree->longest_prefix_match([]) },
        qr/input must be a string/,
        'input validation',
    );
    dies_like(
        sub { $tree->each('not-code') },
        qr/callback must be a code reference/,
        'callback validation',
    );
};

done_testing;
