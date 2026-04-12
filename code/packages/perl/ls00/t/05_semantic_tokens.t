#!/usr/bin/env perl

# 05_semantic_tokens.t -- Semantic token encoding tests

use strict;
use warnings;
use Test::More;

use CodingAdventures::Ls00::Capabilities qw(:all);

# ── Empty input ──────────────────────────────────────────────────────────────

subtest "empty tokens" => sub {
    my $data = encode_semantic_tokens([]);
    is(scalar @$data, 0, "empty data for empty tokens");
};

subtest "undef tokens" => sub {
    my $data = encode_semantic_tokens(undef);
    is(scalar @$data, 0, "empty data for undef");
};

# ── Single token ────────────────────────────────────────────────────────────

subtest "single keyword token" => sub {
    my $tokens = [
        { line => 0, character => 0, length => 5, token_type => 'keyword', modifiers => [] },
    ];
    my $data = encode_semantic_tokens($tokens);

    is(scalar @$data, 5, "5 ints for one token");
    is($data->[0], 0,  "deltaLine = 0");
    is($data->[1], 0,  "deltaChar = 0");
    is($data->[2], 5,  "length = 5");
    is($data->[3], 15, "tokenTypeIndex = 15 (keyword)");
    is($data->[4], 0,  "modifiers = 0");
};

# ── Multiple tokens on same line ────────────────────────────────────────────

subtest "two tokens same line" => sub {
    my $tokens = [
        { line => 0, character => 0, length => 3, token_type => 'keyword', modifiers => [] },
        { line => 0, character => 4, length => 4, token_type => 'function', modifiers => ['declaration'] },
    ];
    my $data = encode_semantic_tokens($tokens);

    is(scalar @$data, 10, "10 ints for 2 tokens");

    is($data->[0], 0,  "A: deltaLine = 0");
    is($data->[1], 0,  "A: deltaChar = 0");
    is($data->[2], 3,  "A: length = 3");
    is($data->[3], 15, "A: keyword = 15");
    is($data->[4], 0,  "A: mods = 0");

    is($data->[5], 0,  "B: deltaLine = 0 (same line)");
    is($data->[6], 4,  "B: deltaChar = 4 (relative to A)");
    is($data->[7], 4,  "B: length = 4");
    is($data->[8], 12, "B: function = 12");
    is($data->[9], 1,  "B: declaration = bit 0 = 1");
};

# ── Tokens on different lines ───────────────────────────────────────────────

subtest "tokens on different lines" => sub {
    my $tokens = [
        { line => 0, character => 0, length => 3, token_type => 'keyword', modifiers => [] },
        { line => 2, character => 4, length => 5, token_type => 'number', modifiers => [] },
    ];
    my $data = encode_semantic_tokens($tokens);

    is(scalar @$data, 10, "10 ints for 2 tokens");
    is($data->[5], 2,  "B: deltaLine = 2");
    is($data->[6], 4,  "B: deltaChar = 4 (absolute on new line)");
    is($data->[8], 19, "B: number = 19");
};

# ── Unsorted input ──────────────────────────────────────────────────────────

subtest "unsorted input gets sorted" => sub {
    my $tokens = [
        { line => 1, character => 0, length => 2, token_type => 'number', modifiers => [] },
        { line => 0, character => 0, length => 3, token_type => 'keyword', modifiers => [] },
    ];
    my $data = encode_semantic_tokens($tokens);

    is(scalar @$data, 10, "10 ints for 2 tokens");
    is($data->[3], 15, "first token is keyword (15)");
    is($data->[8], 19, "second token is number (19)");
};

# ── Unknown token type skipped ──────────────────────────────────────────────

subtest "unknown token type skipped" => sub {
    my $tokens = [
        { line => 0, character => 0, length => 3, token_type => 'unknownType', modifiers => [] },
        { line => 0, character => 4, length => 2, token_type => 'keyword', modifiers => [] },
    ];
    my $data = encode_semantic_tokens($tokens);

    is(scalar @$data, 5, "unknown type skipped, only 5 ints");
};

# ── Modifier bitmask ────────────────────────────────────────────────────────

subtest "modifier bitmask: readonly" => sub {
    my $tokens = [
        { line => 0, character => 0, length => 3, token_type => 'variable', modifiers => ['readonly'] },
    ];
    my $data = encode_semantic_tokens($tokens);

    is($data->[4], 4, "readonly = bit 2 = value 4");
};

subtest "modifier bitmask: multiple modifiers" => sub {
    my $tokens = [
        { line => 0, character => 0, length => 3, token_type => 'variable', modifiers => ['declaration', 'readonly'] },
    ];
    my $data = encode_semantic_tokens($tokens);

    is($data->[4], 5, "declaration + readonly = 1 | 4 = 5");
};

# ── SemanticTokenLegend consistency ──────────────────────────────────────────

subtest "legend has required types" => sub {
    my $legend = semantic_token_legend();

    ok(scalar @{$legend->{tokenTypes}} > 0, "non-empty tokenTypes");
    ok(scalar @{$legend->{tokenModifiers}} > 0, "non-empty tokenModifiers");

    my @required = qw(keyword string number variable function);
    for my $rt (@required) {
        my $found = grep { $_ eq $rt } @{$legend->{tokenTypes}};
        ok($found, "legend contains '$rt'");
    }
};

subtest "token_type_index for known types" => sub {
    is(token_type_index('keyword'),  15, "keyword is at index 15");
    is(token_type_index('function'), 12, "function is at index 12");
    is(token_type_index('number'),   19, "number is at index 19");
    is(token_type_index('variable'),  8, "variable is at index 8");
};

subtest "token_type_index for unknown type" => sub {
    is(token_type_index('nonexistent'), -1, "unknown type returns -1");
};

subtest "token_modifier_mask" => sub {
    is(token_modifier_mask([]),              0, "empty modifiers = 0");
    is(token_modifier_mask(['declaration']), 1, "declaration = bit 0 = 1");
    is(token_modifier_mask(['definition']),  2, "definition = bit 1 = 2");
    is(token_modifier_mask(['readonly']),    4, "readonly = bit 2 = 4");
    is(token_modifier_mask(['declaration', 'readonly']), 5, "declaration + readonly = 5");
};

done_testing();
