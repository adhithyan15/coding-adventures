#!/usr/bin/env perl

# t/02-hasher.t -- Tests for CodingAdventures::BuildTool::Hasher
# ==============================================================
#
# 10 test cases covering determinism, extension allowlists, and
# the special filenames allowlist.

use strict;
use warnings;
use FindBin qw($Bin);
use lib "$Bin/../lib";

use Test2::V0;
use File::Temp qw(tempdir);
use File::Path qw(make_path);

use CodingAdventures::BuildTool::Hasher;

my $h = CodingAdventures::BuildTool::Hasher->new();

sub make_pkg { my ($path) = @_; return { name => 'test/pkg', path => $path } }

sub write_file {
    my ($path, $content) = @_;
    open(my $fh, '>', $path) or die "Cannot write $path: $!";
    print $fh $content;
    close $fh;
}

# ---------------------------------------------------------------------------
# Test 1: Hash is deterministic
# ---------------------------------------------------------------------------
subtest 'hash is deterministic' => sub {
    my $dir = tempdir(CLEANUP => 1);
    write_file("$dir/lib.pm", "package Foo;\n1;\n");
    write_file("$dir/BUILD",  "prove -l -v t/\n");

    my $h1 = $h->hash_package(make_pkg($dir));
    my $h2 = $h->hash_package(make_pkg($dir));

    is($h1, $h2, 'same hash on repeated calls');
    like($h1, qr/^[0-9a-f]{64}$/, 'hash is 64-char hex');
};

# ---------------------------------------------------------------------------
# Test 2: Hash changes with content modification
# ---------------------------------------------------------------------------
subtest 'hash changes when file content changes' => sub {
    my $dir = tempdir(CLEANUP => 1);
    write_file("$dir/lib.pm", "package Foo;\n1;\n");

    my $h1 = $h->hash_package(make_pkg($dir));

    write_file("$dir/lib.pm", "package Foo;\nsub new { }\n1;\n");

    my $h2 = $h->hash_package(make_pkg($dir));

    isnt($h1, $h2, 'hash changes after content modification');
};

# ---------------------------------------------------------------------------
# Test 3: Hash includes BUILD file
# ---------------------------------------------------------------------------
subtest 'hash includes BUILD file' => sub {
    my $dir = tempdir(CLEANUP => 1);
    write_file("$dir/lib.pm", "package Foo;\n1;\n");
    write_file("$dir/BUILD", "prove -l -v t/\n");

    my $h1 = $h->hash_package(make_pkg($dir));

    write_file("$dir/BUILD", "prove -l -v t/ --formatter TAP::Formatter::JUnit\n");

    my $h2 = $h->hash_package(make_pkg($dir));

    isnt($h1, $h2, 'hash changes when BUILD changes');
};

# ---------------------------------------------------------------------------
# Test 4: Python extensions included
# ---------------------------------------------------------------------------
subtest 'python extensions are included' => sub {
    ok($h->is_source_extension('.py'),   '.py included');
    ok($h->is_source_extension('.toml'), '.toml included');
    ok(!$h->is_source_extension('.log'), '.log excluded');
};

# ---------------------------------------------------------------------------
# Test 5: Perl extensions included
# ---------------------------------------------------------------------------
subtest 'perl extensions are included' => sub {
    ok($h->is_source_extension('.pm'), '.pm included');
    ok($h->is_source_extension('.pl'), '.pl included');
    ok($h->is_source_extension('.t'),  '.t  included');
    ok($h->is_source_extension('.xs'), '.xs included');
};

# ---------------------------------------------------------------------------
# Test 6: Non-source files excluded
# ---------------------------------------------------------------------------
subtest 'non-source extensions excluded' => sub {
    ok(!$h->is_source_extension('.bak'), '.bak excluded');
    ok(!$h->is_source_extension('.log'), '.log excluded');
    ok(!$h->is_source_extension('.swp'), '.swp excluded');
    ok(!$h->is_source_extension('.DS_Store'), '.DS_Store excluded');
};

# ---------------------------------------------------------------------------
# Test 7: Special filenames included
# ---------------------------------------------------------------------------
subtest 'special filenames are included' => sub {
    ok($h->is_special_filename('cpanfile'),   'cpanfile included');
    ok($h->is_special_filename('Makefile.PL'), 'Makefile.PL included');
    ok($h->is_special_filename('BUILD'),       'BUILD included');
    ok($h->is_special_filename('go.mod'),      'go.mod included');
    ok(!$h->is_special_filename('random.txt'), 'random.txt not special');
};

# ---------------------------------------------------------------------------
# Test 8: Hash order is deterministic (files sorted)
# ---------------------------------------------------------------------------
subtest 'hash is independent of file creation order' => sub {
    my $dir1 = tempdir(CLEANUP => 1);
    my $dir2 = tempdir(CLEANUP => 1);

    # Create files in different orders.
    write_file("$dir1/a.pm", "package A;\n1;\n");
    write_file("$dir1/b.pm", "package B;\n1;\n");
    write_file("$dir2/b.pm", "package B;\n1;\n");
    write_file("$dir2/a.pm", "package A;\n1;\n");

    my $h1 = $h->hash_package(make_pkg($dir1));
    my $h2 = $h->hash_package(make_pkg($dir2));

    is($h1, $h2, 'same hash regardless of creation order');
};

# ---------------------------------------------------------------------------
# Test 9: Empty package has consistent hash
# ---------------------------------------------------------------------------
subtest 'empty package has consistent hash' => sub {
    my $dir = tempdir(CLEANUP => 1);
    my $h1  = $h->hash_package(make_pkg($dir));
    my $h2  = $h->hash_package(make_pkg($dir));

    is($h1, $h2, 'empty package hash is stable');
    like($h1, qr/^[0-9a-f]{64}$/, 'valid hex string even for empty package');
};

# ---------------------------------------------------------------------------
# Test 10: Subdirectory files included
# ---------------------------------------------------------------------------
subtest 'nested .pm files in subdirectory are included' => sub {
    my $dir = tempdir(CLEANUP => 1);
    make_path("$dir/lib/CodingAdventures");
    write_file("$dir/lib/CodingAdventures/Foo.pm", "package CodingAdventures::Foo;\n1;\n");

    my @files = $h->collect_source_files(make_pkg($dir));
    my @pm_files = grep { /\.pm$/ } @files;

    ok(scalar @pm_files >= 1, 'found at least one .pm in subdirectory');
    ok(grep { /Foo\.pm$/ } @pm_files, 'Foo.pm is in the list');
};

done_testing();
