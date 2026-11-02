#!/usr/bin/env perl

# t/17-cargo-link-tables.t -- The Cargo link-table reader, one shape per case.
# ============================================================================
#
# Each case pins one line rule of the Rust contract in
# code/specs/build-tool-conformance.md. The table is the Go reference tool's
# TestCargoLinkDependencyNames, case for case, so a divergence between
# build-tool fronts shows up as the same named case failing. The shared
# resolution-rust-field-aware fixture runs in t/13-resolution-utf8.t.

use strict;
use warnings;
use FindBin qw($Bin);
use lib "$Bin/../lib";

use Test2::V0;
use File::Path qw(make_path);
use File::Temp qw(tempdir);
use File::Spec ();
use IPC::Open3 qw(open3);
use Symbol qw(gensym);
use Time::HiRes qw(time);

use CodingAdventures::BuildTool::Resolver;

# [name, manifest, expected crate names in manifest order]
my @CASES = (
    [
        "top-level inline",
        "[dependencies]\n"
        . "a = { path = \"../a\" }\n",
        ['a'],
    ],
    [
        "build-dependencies",
        "[build-dependencies]\n"
        . "a = { path = \"../a\" }\n",
        ['a'],
    ],
    [
        "legacy build_dependencies",
        "[build_dependencies]\n"
        . "a = { path = \"../a\" }\n",
        ['a'],
    ],
    [
        "quoted cfg target",
        "[target.'cfg(any(target_os = \"linux\", windows))'.dependencies]\n"
        . "a = { path = \"../a\" }\n",
        ['a'],
    ],
    [
        "bare triple target build",
        "[target.x86_64-pc-windows-msvc.build-dependencies]\n"
        . "a = { path = \"../a\" }\n",
        ['a'],
    ],
    [
        "legacy target build",
        "[target.'cfg(unix)'.build_dependencies]\n"
        . "a = { path = \"../a\" }\n",
        ['a'],
    ],
    [
        "spaced header segments",
        "[ target . 'cfg(unix)' . dependencies ]\n"
        . "a = { path = \"../a\" }\n",
        ['a'],
    ],
    [
        "quoted table segment",
        "[\"dependencies\"]\n"
        . "a = { path = \"../a\" }\n",
        ['a'],
    ],
    [
        "sub-table",
        "[dependencies.a]\n"
        . "version = \"1\"\n"
        . "path = \"../a\"\n",
        ['a'],
    ],
    [
        "sub-table rename before path",
        "[build-dependencies.x]\n"
        . "package = \"A\"\n"
        . "path = '../a'\n",
        ['a'],
    ],
    [
        "target sub-table, dotted quoted triple",
        "[target.\"thumbv8m.main-none-eabihf\".dependencies.a]\n"
        . "path = \"../a\"\n",
        ['a'],
    ],
    [
        "quoted sub-table key",
        "[dependencies.\"a\"]\n"
        . "path = \"../a\"\n",
        ['a'],
    ],
    [
        "sub-table without path",
        "[dependencies.a]\n"
        . "version = \"1\"\n",
        [],
    ],
    [
        "sub-table first path wins, ended by header",
        "[dependencies.a]\n"
        . "path = \"../a\"\n"
        . "path = \"../b\"\n"
        . "[package]\n"
        . "name = \"z\"\n",
        ['a'],
    ],
    [
        "sub-table non-string path",
        "[dependencies.a]\n"
        . "path = 1\n",
        [],
    ],
    [
        "sub-table empty path",
        "[dependencies.a]\n"
        . "path = \"\"\n",
        [],
    ],
    [
        "sub-table capitalised Path key",
        "[dependencies.a]\n"
        . "Path = \"../a\"\n",
        [],
    ],
    [
        "rename",
        "[dependencies]\n"
        . "x = { package = \"Logic-Gates\", path = \"../g\" }\n",
        ['logic-gates'],
    ],
    [
        "capitalised Package is not a rename",
        "[dependencies]\n"
        . "x = { Package = \"y\", path = \"../g\" }\n",
        ['x'],
    ],
    [
        "capitalised Path is not a path",
        "[dependencies]\n"
        . "x = { Path = \"../g\" }\n",
        [],
    ],
    [
        "version plus path with trailing comment",
        "[dependencies]\n"
        . "a = { version = \"1\", path = \"../a\" } # b = { path = \"../b\" }\n",
        ['a'],
    ],
    [
        "hash inside a string is not a comment",
        "[dependencies]\n"
        . "a = { path = \"../a#1\" }\n",
        ['a'],
    ],
    [
        "quoted entry key",
        "[dependencies]\n"
        . "\"a\" = { path = \"../a\" }\n",
        ['a'],
    ],
    [
        "registry entry with path feature",
        "[dependencies]\n"
        . "a = { version = \"1\", features = [\"path\", \"x\"] }\n"
        . "b = \"1\"\n",
        [],
    ],
    [
        "nested table path is not the entry's",
        "[dependencies]\n"
        . "a = { x = { path = \"../a\" } }\n",
        [],
    ],
    [
        "features array commas do not split fields",
        "[dependencies]\n"
        . "a = { features = [\"p\", \"q\"], path = \"../a\" }\n",
        ['a'],
    ],
    [
        "empty path",
        "[dependencies]\n"
        . "a = { path = \"\" }\n",
        [],
    ],
    [
        "array continued onto later lines",
        "[dependencies]\n"
        . "a = { path = \"../a\", features = [\n"
        . "    \"x\", # [not-a-header]\n"
        . "] }\n"
        . "b = { path = \"../b\" }\n",
        ['a', 'b'],
    ],
    [
        "continuation lines are not headers",
        "[dependencies]\n"
        . "a = { path = \"../a\", features = [\n"
        . "[\"x\"]\n"
        . "] }\n"
        . "b = { path = \"../b\" }\n",
        ['a', 'b'],
    ],
    [
        "braces continued onto later lines",
        "[dependencies]\n"
        . "a = { version = \"1\",\n"
        . "  path = \"../a\" }\n",
        ['a'],
    ],
    [
        "file ends inside the value",
        "[dependencies]\n"
        . "a = { path = \"../a\", features = [\n"
        . "  \"x\",\n",
        [],
    ],
    [
        "continued value must end with a brace",
        "[dependencies]\n"
        . "a = { path = \"../a\", features = [\n"
        . "] } x\n",
        [],
    ],
    [
        "dotted key is out of scope",
        "[dependencies]\n"
        . "a.path = \"../a\"\n",
        [],
    ],
    [
        "commented-out entry",
        "[dependencies]\n"
        . "# a = { path = \"../a\" }\n",
        [],
    ],
    [
        "dev-dependencies",
        "[dev-dependencies]\n"
        . "a = { path = \"../a\" }\n"
        . "[dev_dependencies]\n"
        . "b = { path = \"../b\" }\n",
        [],
    ],
    [
        "target dev-dependencies",
        "[target.'cfg(unix)'.dev-dependencies]\n"
        . "a = { path = \"../a\" }\n",
        [],
    ],
    [
        "dev-dependencies sub-table",
        "[dev-dependencies.a]\n"
        . "path = \"../a\"\n",
        [],
    ],
    [
        "patch, replace, workspace",
        "[patch.crates-io]\n"
        . "a = { path = \"../a\" }\n"
        . "[workspace.dependencies]\n"
        . "b = { path = \"../b\" }\n"
        . "[replace]\n"
        . "\"c:1.0.0\" = { path = \"../c\" }\n",
        [],
    ],
    [
        "capitalised table name",
        "[Dependencies]\n"
        . "a = { path = \"../a\" }\n",
        [],
    ],
    [
        "features sub-table of a dependency",
        "[dependencies.a.features]\n"
        . "path = \"../a\"\n",
        [],
    ],
    [
        "target table itself",
        "[target.'cfg(unix)']\n"
        . "a = { path = \"../a\" }\n",
        [],
    ],
    [
        "array of tables ends the table",
        "[dependencies]\n"
        . "[[bin]]\n"
        . "a = { path = \"../a\" }\n",
        [],
    ],
    [
        "malformed header ends the table",
        "[dependencies]\n"
        . "[a b]\n"
        . "c = { path = \"../c\" }\n",
        [],
    ],
    [
        "unterminated quoted header ends the table",
        "[dependencies]\n"
        . "[target.'cfg(unix).dependencies]\n"
        . "c = { path = \"../c\" }\n",
        [],
    ],
    [
        "CRLF and BOM",
        "\x{FEFF}[dependencies]\r\n"
        . "a = { path = \"../a\" }\r\n",
        ['a'],
    ],
    [
        "union across tables keeps duplicates for the caller",
        "[dependencies]\n"
        . "a = { path = \"../a\" }\n"
        . "[build-dependencies]\n"
        . "a = { path = \"../a\" }\n",
        ['a', 'a'],
    ],
    [
        "quoted inline path key",
        "[dependencies]\n"
        . "a = { \"path\" = \"../a\" }\n",
        ['a'],
    ],
    [
        "literal-quoted inline path key",
        "[dependencies]\n"
        . "a = { 'path' = \"../a\" }\n",
        ['a'],
    ],
    [
        "quoted inline package key renames",
        "[dependencies]\n"
        . "x = { \"package\" = \"a\", path = \"../a\" }\n",
        ['a'],
    ],
    [
        "quoted capitalised inline Path key",
        "[dependencies]\n"
        . "a = { \"Path\" = \"../a\" }\n",
        [],
    ],
    [
        "escaped inline path key is compared verbatim",
        "[dependencies]\n"
        . "a = { \"pa\\u0074h\" = \"../a\" }\n",
        [],
    ],
    [
        "dotted inline field key",
        "[dependencies]\n"
        . "a = { path.x = \"../a\" }\n",
        [],
    ],
    [
        "quoted sub-table path key",
        "[dependencies.a]\n"
        . "\"path\" = \"../a\"\n",
        ['a'],
    ],
    [
        "quoted sub-table package key",
        "[dependencies.x]\n"
        . "'package' = \"a\"\n"
        . "path = \"../a\"\n",
        ['a'],
    ],
    [
        "quoted capitalised sub-table Path key",
        "[dependencies.a]\n"
        . "\"Path\" = \"../a\"\n",
        [],
    ],
    [
        "open string continues across the join",
        "[dependencies]\n"
        . "a = { path = \"../a\", x = \"q [\n"
        . "\" }\n",
        ['a'],
    ],
    [
        "line-final backslash escapes the joining space",
        "[dependencies]\n"
        . "a = { x = \"q\\\n"
        . "\", path = \"../a\" }\n",
        ['a'],
    ],
);

for my $case (@CASES) {
    my ($name, $manifest, $expected) = @$case;
    is(
        [CodingAdventures::BuildTool::Resolver::cargo_link_dependency_names($manifest)],
        $expected,
        $name,
    );
}

# The table is the Go reference's, row for row: a row added there and not
# here (or the reverse) must fail loudly.
is(scalar(@CASES), 57, "the shape table has the Go reference's 57 rows");

# Rescanning the whole joined value for every appended line is quadratic: a
# 20,000-line continuation (about 500 KB) took tens of seconds. Carrying the
# scanner state makes it one pass; the bound is generous so a loaded CI
# machine cannot flake it.
subtest 'a 20,000-line continuation is joined in linear time' => sub {
    my $manifest = "[dependencies]\na = { path = \"../a\", features = [\n"
        . ("  \"feature-name-padding\",\n" x 20_000) . "] }\n";
    my $started = time;
    is([CodingAdventures::BuildTool::Resolver::cargo_link_dependency_names($manifest)],
        ['a'], 'the entry is read');
    cmp_ok(time - $started, '<', 20, 'and quickly');
};

subtest 'raw-byte BOM and CRLF, as _slurp reads a manifest' => sub {
    is(
        [CodingAdventures::BuildTool::Resolver::cargo_link_dependency_names(
            "\xEF\xBB\xBF[dependencies]\r\na = { path = \"../a\" }\r\n")],
        ['a'],
        'the UTF-8 byte-order mark bytes are ignored',
    );
};

sub write_file {
    my ($path, $content) = @_;
    open(my $fh, '>:raw', $path) or die "Cannot write $path: $!";
    print {$fh} $content;
    close $fh;
}

subtest 'duplicate links collapse and unknown names drop' => sub {
    my $root = tempdir(CLEANUP => 1);
    make_path("$root/app", "$root/lib");
    write_file("$root/app/Cargo.toml",
        "[package]\nname = \"app\"\n\n[dependencies]\nlib = { path = \"../lib\" }\n"
        . "serde = { path = \"../vendored-serde\" }\n\n[build-dependencies]\nlib = { path = \"../lib\" }\n\n"
        . "[target.'cfg(windows)'.dependencies.lib]\npath = \"../lib\"\n");
    write_file("$root/lib/Cargo.toml", "[package]\nname = \"lib\"\n");
    my @packages = map {
        { name => "rust/$_", path => "$root/$_", language => 'rust', build_commands => [] }
    } qw(app lib);

    my $graph = CodingAdventures::BuildTool::Resolver->new()->resolve(\@packages);

    is([$graph->successors('rust/lib')], ['rust/app'], 'one edge lib -> app');
    is([$graph->successors('rust/app')], [], 'no edge out of app');
};

subtest 'declared [package] name is a known Cargo name' => sub {
    my $root = tempdir(CLEANUP => 1);
    make_path("$root/app", "$root/lib-dir");
    write_file("$root/app/Cargo.toml",
        "[dependencies]\nx = { package = \"Lib-Crate\", path = \"../lib-dir\" }\n");
    write_file("$root/lib-dir/Cargo.toml", "[package]\nname = \"lib-crate\"\n");
    my @packages = map {
        { name => "rust/$_", path => "$root/$_", language => 'rust', build_commands => [] }
    } qw(app lib-dir);

    my $graph = CodingAdventures::BuildTool::Resolver->new()->resolve(\@packages);

    is([$graph->successors('rust/lib-dir')], ['rust/app'], 'rename resolves through the declared name');
};

subtest 'a long run of blank lines before the [package] name' => sub {
    my $root = tempdir(CLEANUP => 1);
    make_path("$root/late-dir");
    write_file("$root/late-dir/Cargo.toml", ("\n" x 100_000) . "[package]\nname = \"Late-Crate\"\n");
    my $started = time;
    my $name = CodingAdventures::BuildTool::Resolver::_read_cargo_package_name(
        { name => 'rust/late-dir', path => "$root/late-dir", language => 'rust' });
    is($name, 'late-crate', 'the name is read');
    cmp_ok(time - $started, '<', 10, 'and quickly');
};

# Step 5: a candidate resolving to the crate itself -- by its directory
# alias, by its [package] name through a renamed sub-table, or from a target
# table -- is DEPENDENCY_SELF_EDGE, neither dropped nor drawn.
my @SELF_EDGES = (
    ['directory alias', "[dependencies]\napp-dir = { path = \".\" }\n"],
    ['cargo name via a renamed sub-table',
        "[build-dependencies.self]\npackage = \"app-crate\"\npath = \".\"\n"],
    ['target table', "[target.'cfg(unix)'.dependencies]\napp-crate = { path = \".\" }\n"],
);
for my $self_edge (@SELF_EDGES) {
    my ($label, $entry) = @$self_edge;
    subtest "self-edge by $label is a diagnostic" => sub {
        my $root = tempdir(CLEANUP => 1);
        my $app = "$root/code/packages/rust/app-dir";
        make_path($app);
        write_file("$app/Cargo.toml", "[package]\nname = \"app-crate\"\n\n$entry");
        my @packages = ({ name => 'rust/app-dir', path => $app, language => 'rust', build_commands => [] });

        my $error;
        eval { CodingAdventures::BuildTool::Resolver->new()->resolve(\@packages); 1 } or $error = $@;

        isa_ok($error, ['CodingAdventures::BuildTool::DependencySelfEdgeError']);
        is($error->{code}, 'DEPENDENCY_SELF_EDGE', 'code');
        is($error->{package}, 'rust/app-dir', 'package');
        is($error->{manifest}, 'code/packages/rust/app-dir/Cargo.toml', 'repository-relative manifest');
        is($error->{dependency}, 'rust/app-dir', 'dependency');
        is("$error",
            'DEPENDENCY_SELF_EDGE: package=rust/app-dir '
            . 'manifest=code/packages/rust/app-dir/Cargo.toml dependency=rust/app-dir',
            'the stable diagnostic');
        unlike("$error", qr/\Q$root\E/, 'no checkout root');
    };
}

subtest 'the real CLI exits 2 for a Cargo self-edge' => sub {
    my $root = tempdir(CLEANUP => 1);
    my $app = "$root/code/packages/rust/app";
    make_path($app);
    write_file("$app/BUILD", "echo build app\n");
    write_file("$app/Cargo.toml", "[package]\nname = \"app\"\n\n[dependencies]\napp = { path = \".\" }\n");
    my $stderr = gensym();
    my $pid = open3(undef, my $stdout, $stderr, $^X, '-I', "$Bin/../lib",
        File::Spec->catfile($Bin, '..', 'bin', 'build-tool'),
        '--root', $root, '--force', '--dry-run');
    my $out = do { local $/; <$stdout> // '' };
    my $err = do { local $/; <$stderr> // '' };
    waitpid($pid, 0);
    is($? >> 8, 2, 'exit status 2');
    like($err, qr/^DEPENDENCY_SELF_EDGE: package=rust\/app manifest=code\/packages\/rust\/app\/Cargo\.toml dependency=rust\/app$/m,
        'the stable diagnostic on stderr');
    unlike($err, qr/\Q$root\E/, 'no checkout root');
};

done_testing;
