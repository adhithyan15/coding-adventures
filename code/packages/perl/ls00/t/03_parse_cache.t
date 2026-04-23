#!/usr/bin/env perl

# 03_parse_cache.t -- ParseCache hit/miss and eviction tests

use strict;
use warnings;
use Test::More;

use CodingAdventures::Ls00::ParseCache;

# ── Mock Bridge ──────────────────────────────────────────────────────────────

{
    package MockBridge;

    my $parse_count = 0;

    sub new {
        $parse_count = 0;
        return bless {}, shift;
    }

    sub tokenize {
        my ($self, $source) = @_;
        return ([], undef);
    }

    sub parse {
        my ($self, $source) = @_;
        $parse_count++;
        my @diags;
        if ($source =~ /ERROR/) {
            push @diags, {
                range => {
                    start => { line => 0, character => 0 },
                    end   => { line => 0, character => 5 },
                },
                severity => 1,
                message  => 'syntax error',
            };
        }
        return ($source, \@diags, undef);
    }

    sub parse_count { return $parse_count }
}

# ── Cache hit and miss ──────────────────────────────────────────────────────

subtest "cache hit and miss" => sub {
    my $bridge = MockBridge->new();
    my $cache = CodingAdventures::Ls00::ParseCache->new();

    my $r1 = $cache->get_or_parse("file:///a.txt", 1, "hello", $bridge);
    ok($r1, "non-nil result");
    is(MockBridge::parse_count(), 1, "parse called once");

    my $r2 = $cache->get_or_parse("file:///a.txt", 1, "hello", $bridge);
    is($r1, $r2, "same reference on cache hit");
    is(MockBridge::parse_count(), 1, "parse not called again");

    my $r3 = $cache->get_or_parse("file:///a.txt", 2, "hello world", $bridge);
    isnt($r3, $r1, "different result for new version");
    is(MockBridge::parse_count(), 2, "parse called again for new version");
};

# ── Eviction ────────────────────────────────────────────────────────────────

subtest "eviction" => sub {
    my $bridge = MockBridge->new();
    my $cache = CodingAdventures::Ls00::ParseCache->new();

    my $r1 = $cache->get_or_parse("file:///a.txt", 1, "hello", $bridge);
    is(MockBridge::parse_count(), 1, "initial parse");

    $cache->evict("file:///a.txt");

    my $r2 = $cache->get_or_parse("file:///a.txt", 1, "hello", $bridge);
    isnt($r2, $r1, "new result after eviction");
    is(MockBridge::parse_count(), 2, "parse called after eviction");
};

# ── Diagnostics populated ──────────────────────────────────────────────────

subtest "diagnostics populated" => sub {
    my $bridge = MockBridge->new();
    my $cache = CodingAdventures::Ls00::ParseCache->new();

    my $result = $cache->get_or_parse("file:///a.txt", 1, "source with ERROR token", $bridge);
    ok(scalar @{$result->{diagnostics}} > 0, "diagnostics present for ERROR source");
};

subtest "empty diagnostics normalized" => sub {
    my $bridge = MockBridge->new();
    my $cache = CodingAdventures::Ls00::ParseCache->new();

    my $result = $cache->get_or_parse("file:///a.txt", 1, "clean source", $bridge);
    is(ref $result->{diagnostics}, 'ARRAY', "diagnostics is arrayref");
    is(scalar @{$result->{diagnostics}}, 0, "no diagnostics for clean source");
};

done_testing();
