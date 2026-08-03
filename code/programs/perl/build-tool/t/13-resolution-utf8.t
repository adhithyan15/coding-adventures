#!/usr/bin/env perl

# t/13-resolution-utf8.t -- Shared Lua rockspec UTF-8 conformance tests.

use strict;
use warnings;
use FindBin qw($Bin);
use lib "$Bin/../lib";

use Test2::V0;
use Cwd qw(abs_path);
use Encode qw(encode);
use File::Basename qw(dirname);
use File::Path qw(make_path);
use File::Spec ();
use File::Temp qw(tempdir);
use IPC::Open3 qw(open3);
use JSON::PP ();
use MIME::Base64 qw(decode_base64);
use Scalar::Util qw(blessed);
use Symbol qw(gensym);

use CodingAdventures::BuildTool::Discovery;
use CodingAdventures::BuildTool::Resolver;

my $PACKAGE_ROOT = abs_path(File::Spec->catdir($Bin, '..'));
my $CASES_ROOT = abs_path(
    File::Spec->catdir($PACKAGE_ROOT, '..', '..', '..', 'specs', 'fixtures', 'build-tool-v1', 'cases')
);

sub read_bytes {
    my ($path) = @_;
    open(my $fh, '<:raw', $path) or die "Cannot read $path: $!";
    local $/;
    my $bytes = <$fh>;
    close $fh;
    return $bytes;
}

sub write_bytes {
    my ($path, $bytes) = @_;
    make_path(dirname($path));
    open(my $fh, '>:raw', $path) or die "Cannot write $path: $!";
    print {$fh} $bytes;
    close $fh;
}

sub load_case {
    my ($name) = @_;
    my $path = File::Spec->catfile($CASES_ROOT, $name);
    return JSON::PP::decode_json(read_bytes($path));
}

sub materialize_case {
    my ($case) = @_;
    my $root = tempdir(CLEANUP => 1);
    for my $file (@{ $case->{workspace}{files} }) {
        my $path = File::Spec->catfile($root, split m{/}, $file->{path});
        my $bytes = exists $file->{content_utf8}
            ? encode('UTF-8', $file->{content_utf8})
            : decode_base64($file->{content_base64});
        write_bytes($path, $bytes);
    }
    return $root;
}

sub discover_lua {
    my ($root) = @_;
    my $discovery = CodingAdventures::BuildTool::Discovery->new(root => $root);
    $discovery->discover();
    return [grep { $_->{language} eq 'lua' } @{ $discovery->packages() }];
}

sub resolve_error {
    my ($root) = @_;
    my $error;
    eval {
        CodingAdventures::BuildTool::Resolver->new()->resolve(discover_lua($root));
        1;
    } or $error = $@;
    return $error;
}

sub graph_edges {
    my ($graph) = @_;
    my @edges;
    for my $node (sort $graph->nodes()) {
        push @edges, map { [$node, $_] } sort $graph->successors($node);
    }
    return \@edges;
}

sub run_cli {
    my ($root) = @_;
    my $stderr = gensym();
    my $pid = open3(
        undef,
        my $stdout,
        $stderr,
        $^X,
        File::Spec->catfile($PACKAGE_ROOT, 'bin', 'build-tool'),
        '--root', $root,
        '--language', 'lua',
        '--force',
        '--dry-run',
    );
    my $out = do { local $/; <$stdout> // '' };
    my $err = do { local $/; <$stderr> // '' };
    waitpid($pid, 0);
    return ($? >> 8, $out, $err);
}

sub assert_metadata_error {
    my ($error, $root, $manifest) = @_;
    ok(blessed($error), 'resolver throws a typed error object');
    isa_ok($error, ['CodingAdventures::BuildTool::MetadataEncodingError']);
    is($error->{code}, 'METADATA_INVALID_UTF8', 'diagnostic code is stable');
    is($error->{package}, 'lua/pkg', 'package identity is stable');
    is($error->{manifest}, $manifest, 'manifest path is repository-relative');
    is($error->{encoding}, 'UTF-8', 'encoding is explicit');
    unlike("$error", qr/\Q$root\E/, 'diagnostic does not leak the temporary root');
}

subtest 'shared valid UTF-8 fixture resolves the exact edge' => sub {
    my $root = materialize_case(load_case('resolution-lua-utf8.json'));
    my $graph = CodingAdventures::BuildTool::Resolver->new()->resolve(discover_lua($root));
    is(graph_edges($graph), [['lua/other', 'lua/pkg']], 'fixture produces only the expected edge');
};

subtest 'shared invalid UTF-8 fixture fails closed with typed metadata' => sub {
    my $root = materialize_case(load_case('resolution-lua-invalid-utf8.json'));
    my $manifest = 'code/packages/lua/pkg/coding-adventures-pkg-0.1.0-1.rockspec';
    my $error = resolve_error($root);
    assert_metadata_error($error, $root, $manifest);
    is(
        "$error",
        "METADATA_INVALID_UTF8: package=lua/pkg manifest=$manifest encoding=UTF-8",
        'typed error string is the language-neutral diagnostic',
    );
};

subtest 'a literal replacement character remains valid UTF-8' => sub {
    my $case = load_case('resolution-lua-utf8.json');
    $case->{workspace}{files}[1]{content_utf8} =~ s/UTF-8/UTF-8 \x{FFFD}/;
    my $root = materialize_case($case);
    my $graph = CodingAdventures::BuildTool::Resolver->new()->resolve(discover_lua($root));
    is(
        graph_edges($graph),
        [['lua/other', 'lua/pkg']],
        'U+FFFD is not confused with a decode failure',
    );
};

my @malformed = (
    ['illegal leading byte', "\xFF"],
    ['unexpected continuation byte', "\x80"],
    ['truncated multibyte sequence', "\xE2\x82"],
    ['overlong encoding', "\xC0\xAF"],
    ['UTF-16 surrogate encoding', "\xED\xA0\x80"],
);

for my $malformed (@malformed) {
    my ($label, $bytes) = @{$malformed};
    subtest "rejects $label" => sub {
        my $root = tempdir(CLEANUP => 1);
        my $package_dir = File::Spec->catdir($root, 'code', 'packages', 'lua', 'pkg');
        my $manifest_name = 'coding-adventures-pkg-0.1.0-1.rockspec';
        write_bytes(File::Spec->catfile($package_dir, 'BUILD'), "echo build\n");
        write_bytes(
            File::Spec->catfile($package_dir, $manifest_name),
            qq{package = "coding-adventures-pkg"\n-- malformed: } . $bytes . "\n",
        );
        my $error = resolve_error($root);
        assert_metadata_error($error, $root, "code/packages/lua/pkg/$manifest_name");
    };
}

subtest 'real CLI reports the stable diagnostic on stderr and exits 2' => sub {
    my $root = materialize_case(load_case('resolution-lua-invalid-utf8.json'));
    my ($exit, $stdout, $stderr) = run_cli($root);
    my $normalized_stderr = $stderr;
    $normalized_stderr =~ s/\r\n/\n/g;
    is($exit, 2, 'CLI returns configuration-error exit code 2');
    is($stdout, '', 'CLI emits no stdout on malformed metadata');
    is(
        $normalized_stderr,
        "METADATA_INVALID_UTF8: package=lua/pkg "
            . "manifest=code/packages/lua/pkg/coding-adventures-pkg-0.1.0-1.rockspec encoding=UTF-8\n",
        'CLI emits only the stable diagnostic on stderr',
    );
    unlike($stderr, qr/\Q$root\E/, 'CLI diagnostic does not leak the temporary root');
};

done_testing;
