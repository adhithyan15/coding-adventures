#!/usr/bin/env perl

# t/00-discovery.t -- Tests for CodingAdventures::BuildTool::Discovery
# ======================================================================
#
# We test 15 cases as specified in the spec:
#   1.  Discover single package
#   2.  Discover nested packages with language inference
#   3.  Infer Perl language
#   4.  Infer Python language
#   5.  Skip .git directories
#   6.  Skip node_modules
#   7.  Skip __pycache__
#   8.  Package name format
#   9.  Platform BUILD precedence (mac)
#   10. Platform BUILD fallback
#   11. No BUILD file → not registered
#   12. Empty directory → no packages
#   13. Multiple languages
#   14. Unknown language
#   15. BUILD file content extracted correctly

use strict;
use warnings;
use FindBin qw($Bin);
use lib "$Bin/../lib";

use Test2::V0;
use File::Temp qw(tempdir);
use File::Path qw(make_path);
use File::Spec ();
use JSON::PP qw(decode_json);

use CodingAdventures::BuildTool::Discovery;

# Helper: create a package directory with a BUILD file.
sub make_pkg {
    my ($root, $path, $content) = @_;
    $content //= "prove -l -v t/\n";
    make_path("$root/$path");
    open(my $fh, '>', "$root/$path/BUILD") or die "Cannot create BUILD: $!";
    print $fh $content;
    close $fh;
}

sub load_registry_fixture {
    my $path = "$Bin/../../../../specs/fixtures/build-tool-v1/cases/discovery-language-registry.json";
    open(my $fh, '<:raw', $path) or die "Cannot read $path: $!";
    local $/;
    my $fixture = decode_json(<$fh>);
    close $fh;
    return $fixture;
}

# ---------------------------------------------------------------------------
# Test 1: Discover single package
# ---------------------------------------------------------------------------
subtest 'discover single package' => sub {
    my $root = tempdir(CLEANUP => 1);
    make_pkg($root, 'perl/logic-gates');

    my $d = CodingAdventures::BuildTool::Discovery->new(root => $root);
    $d->discover();
    my @pkgs = @{ $d->packages() };

    is(scalar @pkgs, 1, 'found exactly 1 package');
    is($pkgs[0]{name}, 'perl/logic-gates', 'name is correct');
};

# ---------------------------------------------------------------------------
# Test 2: Discover nested packages with language inference
# ---------------------------------------------------------------------------
subtest 'nested packages language inference' => sub {
    my $root = tempdir(CLEANUP => 1);
    make_pkg($root, 'code/packages/python/logic-gates');

    my $d = CodingAdventures::BuildTool::Discovery->new(root => $root);
    $d->discover();
    my @pkgs = @{ $d->packages() };

    is(scalar @pkgs, 1, 'found 1 package');
    is($pkgs[0]{language}, 'python', 'language inferred as python');
};

# ---------------------------------------------------------------------------
# Test 3: Infer Perl language
# ---------------------------------------------------------------------------
subtest 'infer perl language' => sub {
    my $root = tempdir(CLEANUP => 1);
    make_pkg($root, 'packages/perl/logic-gates');

    my $d = CodingAdventures::BuildTool::Discovery->new(root => $root);
    $d->discover();
    my @pkgs = @{ $d->packages() };

    is($pkgs[0]{language}, 'perl', 'language is perl');
};

# ---------------------------------------------------------------------------
# Test 4: Infer Python language
# ---------------------------------------------------------------------------
subtest 'infer python language' => sub {
    my $root = tempdir(CLEANUP => 1);
    make_pkg($root, 'packages/python/arithmetic');

    my $d = CodingAdventures::BuildTool::Discovery->new(root => $root);
    $d->discover();
    my @pkgs = @{ $d->packages() };

    is($pkgs[0]{language}, 'python', 'language is python');
};

# ---------------------------------------------------------------------------
# Test 4b: Infer C# language
# ---------------------------------------------------------------------------
subtest 'infer csharp language' => sub {
    my $root = tempdir(CLEANUP => 1);
    make_pkg($root, 'packages/csharp/graph');

    my $d = CodingAdventures::BuildTool::Discovery->new(root => $root);
    $d->discover();
    my @pkgs = @{ $d->packages() };

    is($pkgs[0]{language}, 'csharp', 'language is csharp');
};

# ---------------------------------------------------------------------------
# Test 4c: Infer wasm language
# ---------------------------------------------------------------------------
subtest 'infer wasm language' => sub {
    my $root = tempdir(CLEANUP => 1);
    make_pkg($root, 'packages/wasm/graph');

    my $d = CodingAdventures::BuildTool::Discovery->new(root => $root);
    $d->discover();
    my @pkgs = @{ $d->packages() };

    is($pkgs[0]{language}, 'wasm', 'language is wasm');
};

# ---------------------------------------------------------------------------
# Test 5: Skip .git directories
# ---------------------------------------------------------------------------
subtest 'skip .git directories' => sub {
    my $root = tempdir(CLEANUP => 1);
    make_pkg($root, 'perl/logic-gates');
    # Put a BUILD file inside .git — it should never be found.
    make_path("$root/.git/hooks");
    open(my $fh, '>', "$root/.git/BUILD") or die;
    print $fh "echo should not run\n";
    close $fh;

    my $d = CodingAdventures::BuildTool::Discovery->new(root => $root);
    $d->discover();
    my @pkgs = @{ $d->packages() };

    is(scalar @pkgs, 1, 'only 1 package (not .git/BUILD)');
    unlike($pkgs[0]{path}, qr/\.git/, 'path does not contain .git');
};

# ---------------------------------------------------------------------------
# Test 6: Skip node_modules
# ---------------------------------------------------------------------------
subtest 'skip node_modules' => sub {
    my $root = tempdir(CLEANUP => 1);
    make_pkg($root, 'typescript/app');
    make_path("$root/typescript/app/node_modules/some-pkg");
    open(my $fh, '>', "$root/typescript/app/node_modules/some-pkg/BUILD") or die;
    print $fh "echo inner\n";
    close $fh;

    my $d = CodingAdventures::BuildTool::Discovery->new(root => $root);
    $d->discover();
    my @pkgs = @{ $d->packages() };

    is(scalar @pkgs, 1, 'only top-level package found');
};

# ---------------------------------------------------------------------------
# Test 7: Skip __pycache__
# ---------------------------------------------------------------------------
subtest 'skip __pycache__' => sub {
    my $root = tempdir(CLEANUP => 1);
    make_pkg($root, 'python/foo');
    make_path("$root/python/foo/__pycache__");
    open(my $fh, '>', "$root/python/foo/__pycache__/BUILD") or die;
    print $fh "echo ignore\n";
    close $fh;

    my $d = CodingAdventures::BuildTool::Discovery->new(root => $root);
    $d->discover();
    my @pkgs = @{ $d->packages() };

    is(scalar @pkgs, 1, '__pycache__ not traversed');
};

# ---------------------------------------------------------------------------
# Test 8: Package name format
# ---------------------------------------------------------------------------
subtest 'package name format' => sub {
    my $root = tempdir(CLEANUP => 1);
    make_pkg($root, 'code/packages/perl/bitset');

    my $d = CodingAdventures::BuildTool::Discovery->new(root => $root);
    $d->discover();
    my @pkgs = @{ $d->packages() };

    is($pkgs[0]{name}, 'perl/bitset', 'name is language/dirname');
};

# ---------------------------------------------------------------------------
# Test 9: Platform BUILD precedence — BUILD_mac on Darwin
# ---------------------------------------------------------------------------
subtest 'platform BUILD_mac preferred on darwin' => sub {
    my $root = tempdir(CLEANUP => 1);
    my $dir  = "$root/perl/pkg";
    make_path($dir);
    open(my $fh1, '>', "$dir/BUILD") or die;
    print $fh1 "generic\n"; close $fh1;
    open(my $fh2, '>', "$dir/BUILD_mac") or die;
    print $fh2 "mac-specific\n"; close $fh2;

    my $result = CodingAdventures::BuildTool::Discovery::choose_build_file_for_platform(
        $dir, 'darwin'
    );
    is($result, 'BUILD_mac', 'BUILD_mac chosen on darwin');
};

# ---------------------------------------------------------------------------
# Test 10: Platform BUILD fallback — no platform-specific file
# ---------------------------------------------------------------------------
subtest 'fallback to BUILD when no platform file' => sub {
    my $root = tempdir(CLEANUP => 1);
    my $dir  = "$root/perl/pkg";
    make_path($dir);
    open(my $fh, '>', "$dir/BUILD") or die;
    print $fh "generic\n"; close $fh;

    my $result = CodingAdventures::BuildTool::Discovery::choose_build_file_for_platform(
        $dir, 'darwin'
    );
    is($result, 'BUILD', 'falls back to BUILD');
};

# ---------------------------------------------------------------------------
# Test 11: Directory without BUILD file is not a package
# ---------------------------------------------------------------------------
subtest 'no BUILD file means no package' => sub {
    my $root = tempdir(CLEANUP => 1);
    make_path("$root/perl/logic-gates");
    # No BUILD file created.

    my $d = CodingAdventures::BuildTool::Discovery->new(root => $root);
    $d->discover();
    my @pkgs = @{ $d->packages() };

    is(scalar @pkgs, 0, 'no packages found without BUILD');
};

# ---------------------------------------------------------------------------
# Test 12: Empty directory → no packages
# ---------------------------------------------------------------------------
subtest 'empty directory yields no packages' => sub {
    my $root = tempdir(CLEANUP => 1);

    my $d = CodingAdventures::BuildTool::Discovery->new(root => $root);
    $d->discover();
    my @pkgs = @{ $d->packages() };

    is(scalar @pkgs, 0, 'empty root has no packages');
};

# ---------------------------------------------------------------------------
# Test 13: Multiple languages discovered
# ---------------------------------------------------------------------------
subtest 'multiple languages discovered' => sub {
    my $root = tempdir(CLEANUP => 1);
    make_pkg($root, 'packages/python/logic-gates');
    make_pkg($root, 'packages/go/logic-gates');
    make_pkg($root, 'packages/perl/logic-gates');

    my $d = CodingAdventures::BuildTool::Discovery->new(root => $root);
    $d->discover();
    my @pkgs = @{ $d->packages() };

    is(scalar @pkgs, 3, '3 packages found');

    my %langs = map { $_->{language} => 1 } @pkgs;
    ok($langs{python},  'python found');
    ok($langs{go},      'go found');
    ok($langs{perl},    'perl found');
};

# ---------------------------------------------------------------------------
# Test 14: Unknown language
# ---------------------------------------------------------------------------
subtest 'unknown language for unrecognised path' => sub {
    my $root = tempdir(CLEANUP => 1);
    make_pkg($root, 'something/unknown-pkg');

    my $d = CodingAdventures::BuildTool::Discovery->new(root => $root);
    $d->discover();
    my @pkgs = @{ $d->packages() };

    is(scalar @pkgs, 1, '1 package found');
    is($pkgs[0]{language}, 'unknown', 'language is unknown');
};

# ---------------------------------------------------------------------------
# Test 15: BUILD file commands extracted correctly
# ---------------------------------------------------------------------------
subtest 'BUILD file content extracted' => sub {
    my $root = tempdir(CLEANUP => 1);
    my $dir  = "$root/perl/logic-gates";
    make_path($dir);
    open(my $fh, '>', "$dir/BUILD") or die;
    print $fh <<'END';
# Install dependencies
cpanm --installdeps --quiet .

# Run tests
prove -l -v t/
END
    close $fh;

    my $d = CodingAdventures::BuildTool::Discovery->new(root => $root);
    $d->discover();
    my @pkgs = @{ $d->packages() };

    is(scalar @pkgs, 1, '1 package found');
    my @cmds = @{ $pkgs[0]{build_commands} };
    is(scalar @cmds, 2, '2 non-blank non-comment lines');
    is($cmds[0], 'cpanm --installdeps --quiet .', 'first command correct');
    is($cmds[1], 'prove -l -v t/', 'second command correct');
};

subtest 'neutral OCaml package and Dune decoy projection' => sub {
    my $root = tempdir(CLEANUP => 1);
    my $fixture = load_registry_fixture();
    my @ocaml = grep { $_->{path} =~ m{^code/packages/ocaml/} }
        @{ $fixture->{workspace}{files} };
    is(scalar @ocaml, 4, 'neutral fixture has package, generated decoy, and both near-case roots');
    for my $file (@ocaml) {
        (my $package_path = $file->{path}) =~ s{/BUILD$}{};
        make_pkg($root, $package_path, $file->{content_utf8});
    }

    my $discovery = CodingAdventures::BuildTool::Discovery->new(root => $root);
    $discovery->discover();
    my @relative = sort map {
        my $path = File::Spec->abs2rel($_->{path}, $root);
        $path =~ s{\\}{/}g;
        $path;
    } @{ $discovery->packages() };
    is(
        \@relative,
        [qw(code/packages/ocaml/demo-ocaml code/packages/ocaml/generated-b/_Build/case-source code/packages/ocaml/generated-c/_build-example/near-source)],
        'only exact lowercase _build is pruned during discovery',
    );
    my ($package) = grep { $_->{path} =~ m{demo-ocaml$} }
        @{ $discovery->packages() };
    is($package->{language}, 'ocaml', 'neutral package is classified as OCaml');
    is($package->{name}, 'ocaml/demo-ocaml', 'neutral package identity is preserved');
};

subtest 'selected BUILD carries explicit source mode and declarations' => sub {
    my $root = tempdir(CLEANUP => 1);
    make_pkg($root, 'code/packages/perl/shell', "prove -l t/\n");
    make_pkg($root, 'code/packages/perl/declared', <<'BUILD');
perl_library(
    name = "declared",
    srcs = glob(["assets/*.bin", "lib/**/*.pm"]),
)
BUILD
    make_pkg($root, 'code/packages/perl/empty', <<'BUILD');
perl_library(name = "empty", srcs = [])
BUILD

    my $discovery = CodingAdventures::BuildTool::Discovery->new(root => $root);
    $discovery->discover();
    my %packages = map { $_->{name} => $_ } @{ $discovery->packages() };

    is($packages{'perl/shell'}{source_mode}, 'extension', 'shell BUILD uses extension mode');
    is($packages{'perl/shell'}{declared_srcs}, [], 'shell BUILD has no declarations');
    is($packages{'perl/shell'}{build_commands}, ['prove -l t/'], 'shell commands unchanged');
    is($packages{'perl/declared'}{source_mode}, 'declared_sources', 'Starlark target uses declared mode');
    is($packages{'perl/declared'}{declared_srcs}, ['assets/*.bin', 'lib/**/*.pm'], 'ordered source globs are carried');
    is($packages{'perl/declared'}{build_commands},
        ['cpanm --installdeps --quiet .', 'prove -l -v t/'],
        'Starlark target generates commands rather than executing raw BUILD lines');
    is($packages{'perl/empty'}{source_mode}, 'declared_sources', 'empty srcs remains declared mode');
    is($packages{'perl/empty'}{declared_srcs}, [], 'empty declaration remains empty');
};

subtest 'detected Starlark without a supported target fails closed' => sub {
    my $root = tempdir(CLEANUP => 1);
    make_pkg($root, 'code/packages/perl/unsupported', 'load("//rules:perl.bzl", "perl_library")');
    my $discovery = CodingAdventures::BuildTool::Discovery->new(root => $root);
    like(
        dies { $discovery->discover() },
        qr/STARLARK_TARGET_INVALID/,
        'invalid declaration is not reinterpreted as shell commands',
    );
};

subtest 'malformed Starlark srcs fail closed' => sub {
    for my $srcs ('glob(["assets/*.bin")', '[unquoted]', '["a.pm"] + ["b.pm"]') {
        my $root = tempdir(CLEANUP => 1);
        make_pkg($root, 'code/packages/perl/malformed',
            "perl_library(name = \"malformed\", srcs = $srcs)\n");
        my $discovery = CodingAdventures::BuildTool::Discovery->new(root => $root);
        like(dies { $discovery->discover() }, qr/STARLARK_TARGET_INVALID/,
            "$srcs rejected rather than treated as an empty declaration");
    }
    my $root = tempdir(CLEANUP => 1);
    make_pkg($root, 'code/packages/perl/partially-valid', <<'BUILD');
perl_library(name = "first", srcs = ["a.pm"])
perl_library(name = "broken", srcs = ["missing.pm"] + ["other.pm"])
BUILD
    my $discovery = CodingAdventures::BuildTool::Discovery->new(root => $root);
    like(dies { $discovery->discover() }, qr/STARLARK_TARGET_INVALID/,
        'a malformed later target invalidates the whole BUILD');
};

done_testing();
