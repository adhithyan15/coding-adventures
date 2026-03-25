#!/usr/bin/env perl

# t/08-glob-match.t -- Tests for CodingAdventures::BuildTool::GlobMatch
# ========================================================================
#
# 8 test cases for glob pattern matching.

use strict;
use warnings;
use FindBin qw($Bin);
use lib "$Bin/../lib";

use Test2::V0;

use CodingAdventures::BuildTool::GlobMatch;

my $gm = CodingAdventures::BuildTool::GlobMatch->new();

# ---------------------------------------------------------------------------
# Test 1: *.pm matches .pm files at one level
# ---------------------------------------------------------------------------
subtest '*.pm matches .pm files one level' => sub {
    ok($gm->matches('lib/*.pm',  'lib/Foo.pm'),   'lib/Foo.pm matches lib/*.pm');
    ok(!$gm->matches('lib/*.pm', 'lib/a/Foo.pm'), 'lib/a/Foo.pm does not match lib/*.pm');
};

# ---------------------------------------------------------------------------
# Test 2: ** matches any depth
# ---------------------------------------------------------------------------
subtest '** matches across directories' => sub {
    ok($gm->matches('lib/**/*.pm', 'lib/Foo.pm'),     'lib/Foo.pm matches lib/**/*.pm');
    ok($gm->matches('lib/**/*.pm', 'lib/a/b/Foo.pm'), 'lib/a/b/Foo.pm matches lib/**/*.pm');
};

# ---------------------------------------------------------------------------
# Test 3: ? matches one character except /
# ---------------------------------------------------------------------------
subtest '? matches one character' => sub {
    ok($gm->matches('t/??.t', 't/00.t'), 't/00.t matches t/??.t');
    ok($gm->matches('t/??.t', 't/99.t'), 't/99.t matches t/??.t');
    ok(!$gm->matches('t/??.t', 't/000.t'), 't/000.t does not match t/??.t');
};

# ---------------------------------------------------------------------------
# Test 4: Literal match
# ---------------------------------------------------------------------------
subtest 'literal pattern matches exact path' => sub {
    ok($gm->matches('cpanfile', 'cpanfile'), 'cpanfile matches cpanfile');
    ok(!$gm->matches('cpanfile', 'Makefile.PL'), 'Makefile.PL does not match cpanfile');
};

# ---------------------------------------------------------------------------
# Test 5: Dot in pattern is literal
# ---------------------------------------------------------------------------
subtest 'dot in pattern is literal' => sub {
    ok($gm->matches('*.pm', 'Foo.pm'), 'Foo.pm matches *.pm');
    ok(!$gm->matches('*.pm', 'Foopm'), 'Foopm does not match *.pm (no dot)');
};

# ---------------------------------------------------------------------------
# Test 6: Character class [abc]
# ---------------------------------------------------------------------------
subtest 'character class matches one of listed chars' => sub {
    ok($gm->matches('[abc].pm', 'a.pm'), 'a.pm matches [abc].pm');
    ok($gm->matches('[abc].pm', 'b.pm'), 'b.pm matches [abc].pm');
    ok(!$gm->matches('[abc].pm', 'd.pm'), 'd.pm does not match [abc].pm');
};

# ---------------------------------------------------------------------------
# Test 7: filter_files returns matching subset
# ---------------------------------------------------------------------------
subtest 'filter_files returns matching files' => sub {
    my @files = qw(
        lib/Foo.pm
        lib/Bar.pm
        t/00-foo.t
        README.md
        cpanfile
    );

    my @pm_files = $gm->filter_files(['lib/*.pm'], \@files);
    is(scalar @pm_files, 2, '2 .pm files found');

    my @t_files = $gm->filter_files(['t/*.t'], \@files);
    is(scalar @t_files, 1, '1 .t file found');
};

# ---------------------------------------------------------------------------
# Test 8: Multiple patterns in filter_files
# ---------------------------------------------------------------------------
subtest 'filter_files with multiple patterns' => sub {
    my @files = qw(lib/Foo.pm t/00.t README.md);
    my @matching = $gm->filter_files(['lib/*.pm', 't/*.t'], \@files);
    is(scalar @matching, 2, '2 files match combined patterns');
};

done_testing();
