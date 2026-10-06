#!/usr/bin/env perl

# t/02-hasher.t -- Tests for CodingAdventures::BuildTool::Hasher
# ==============================================================
#
# 11 test cases covering determinism, extension allowlists, and
# the special filenames allowlist.

use strict;
use warnings;
use FindBin qw($Bin);
use lib "$Bin/../lib";

use Test2::V0;
use File::Temp qw(tempdir);
use File::Path qw(make_path);
use File::Spec ();
use JSON::PP qw(decode_json);

use CodingAdventures::BuildTool::Hasher;
use CodingAdventures::BuildTool::Discovery;

my $h = CodingAdventures::BuildTool::Hasher->new();

sub make_pkg { my ($path) = @_; return { name => 'perl/demo', language => 'perl', path => $path } }

sub write_file {
    my ($path, $content) = @_;
    open(my $fh, '>', $path) or die "Cannot write $path: $!";
    print $fh $content;
    close $fh;
}

sub load_source_fixture {
    my ($name) = @_;
    my $path = "$Bin/../../../../specs/fixtures/build-tool-v1/cases/$name.json";
    open(my $fh, '<:raw', $path) or die "Cannot read $path: $!";
    local $/;
    my $fixture = decode_json(<$fh>);
    close $fh;
    return $fixture;
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
    ok($h->is_source_extension('.py', 'python'),   '.py included for Python');
    ok(!$h->is_source_extension('.py', 'perl'),    '.py excluded for Perl');
    ok(!$h->is_source_extension('.log', 'python'), '.log excluded');
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
    ok($h->is_special_filename('go.mod', 'go'), 'go.mod included for Go');
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

# ---------------------------------------------------------------------------
# Test 11: exact lowercase blib is generated while near names remain source
# ---------------------------------------------------------------------------
subtest 'blib pruning is exact and case-sensitive' => sub {
    my $dir = tempdir(CLEANUP => 1);
    for my $relative (qw(excluded/blib near-case/Blib near-name/blib-example)) {
        make_path("$dir/$relative");
        write_file("$dir/$relative/source.pm", "package Source;\n1;\n");
    }

    my @relative = map {
        my $path = File::Spec->abs2rel($_, $dir);
        $path =~ s{\\}{/}g;
        $path;
    } $h->collect_source_files(make_pkg($dir));

    is(
        \@relative,
        [qw(near-case/Blib/source.pm near-name/blib-example/source.pm)],
        'only exact lowercase blib is pruned',
    );
};

subtest 'neutral source fixtures project exact Dune pruning to Perl files' => sub {
    for my $name (qw(source-collection-extension source-collection-declared)) {
        my $fixture = load_source_fixture($name);
        my $dir = tempdir(CLEANUP => 1);
        my @dune = grep {
            $_->{kind} eq 'file' &&
            $_->{path} =~ m{(?:^|/)(?:_build|_Build|_build-example)/}
        } @{ $fixture->{input}{options}{candidates} };
        is(scalar @dune, 3, "$name has three Dune path components");
        for my $candidate (@dune) {
            (my $relative = $candidate->{path}) =~ s/\.ml$/.pm/;
            my $path = "$dir/$relative";
            (my $parent = $path) =~ s{/[^/]+$}{};
            make_path($parent);
            write_file($path, pack('H*', $candidate->{content_hex}));
        }
        my @relative = map {
            my $path = File::Spec->abs2rel($_, $dir);
            $path =~ s{\\}{/}g;
            $path;
        } $h->collect_source_files(make_pkg($dir));
        is(
            \@relative,
            [qw(case/_Build/generated.pm near/_build-example/generated.pm)],
            "$name prunes only the exact lowercase Dune directory",
        );
    }
};

subtest 'discovered declarations select non-source files and the selected BUILD' => sub {
    my $dir = tempdir(CLEANUP => 1);
    make_path("$dir/code/packages/perl/demo/assets", "$dir/code/packages/perl/demo/lib", "$dir/code/packages/perl/demo/_build");
    my $pkg_dir = "$dir/code/packages/perl/demo";
    write_file("$pkg_dir/BUILD", "perl_library(name = \"demo\", srcs = glob([\"assets/*.bin\"]))\n");
    write_file("$pkg_dir/assets/data.bin", "original\n");
    write_file("$pkg_dir/lib/undeclared.pm", "original\n");
    write_file("$pkg_dir/_build/decoy.bin", "original\n");
    my $discovery = CodingAdventures::BuildTool::Discovery->new(root => $dir);
    $discovery->discover();
    my ($pkg) = @{ $discovery->packages };
    my @relative = map {
        my $path = File::Spec->abs2rel($_, $pkg_dir);
        $path =~ s{\\}{/}g;
        $path;
    } $h->collect_source_files($pkg);
    is(\@relative, [qw(BUILD assets/data.bin)], 'declared mode includes only BUILD and matching retained files');

    my $first = $h->hash_package($pkg);
    write_file("$pkg_dir/lib/undeclared.pm", "changed\n");
    is($h->hash_package($pkg), $first, 'undeclared source does not change digest');
    write_file("$pkg_dir/_build/decoy.bin", "changed\n");
    is($h->hash_package($pkg), $first, 'generated tree does not change digest');
    write_file("$pkg_dir/assets/data.bin", "changed\n");
    isnt($h->hash_package($pkg), $first, 'declared non-source bytes change digest');
    my $second = $h->hash_package($pkg);
    write_file("$pkg_dir/BUILD", "perl_library(name = \"demo\", srcs = glob([\"assets/*.bin\"])) # changed\n");
    isnt($h->hash_package($pkg), $second, 'selected BUILD bytes change digest');
};

subtest 'explicit empty declaration does not fall back to extension mode' => sub {
    my $dir = tempdir(CLEANUP => 1);
    write_file("$dir/BUILD", "perl_library(name = \"empty\", srcs = [])\n");
    write_file("$dir/ignored.pm", "original\n");
    my $pkg = { name => 'perl/demo', path => $dir, language => 'perl', source_mode => 'declared_sources', declared_srcs => [], build_file => 'BUILD' };
    my @relative = map { File::Spec->abs2rel($_, $dir) } $h->collect_source_files($pkg);
    is(\@relative, ['BUILD'], 'only selected BUILD is retained');
    my $first = $h->hash_package($pkg);
    write_file("$dir/ignored.pm", "changed\n");
    is($h->hash_package($pkg), $first, 'ignored extension does not change digest');
};

subtest 'declared globs are validated before traversal' => sub {
    my $dir = tempdir(CLEANUP => 1);
    write_file("$dir/BUILD", "perl_library(name = \"bad\", srcs = [])\n");
    for my $invalid ('../outside.pm', '/absolute.pm', 'lib\\*.pm', 'lib/[z-a].pm') {
        my $pkg = { path => $dir, language => 'perl', source_mode => 'declared_sources', declared_srcs => ['safe/*.pm', $invalid], build_file => 'BUILD' };
        like(dies { $h->collect_source_files($pkg) }, qr/DECLARED_GLOB_INVALID/, "$invalid rejected");
    }
};

done_testing();
