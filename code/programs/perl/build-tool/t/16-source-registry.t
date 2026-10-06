#!/usr/bin/env perl

# The same production selector handles inert neutral candidates and the host
# walk. These cases pin the registry projection without granting the fixture
# runner a path into the build tool at runtime.
use strict;
use warnings;
use FindBin qw($Bin);
use lib "$Bin/../lib";
use Test2::V0;
use JSON::PP qw(decode_json);
use Digest::SHA qw(sha256_hex);
use File::Temp qw(tempdir);
use File::Path qw(make_path);

use CodingAdventures::BuildTool::Hasher;

sub raw_file {
    my ($path) = @_;
    open my $fh, '<:raw', $path or die "Cannot read fixture: $!";
    local $/;
    my $bytes = <$fh>;
    close $fh or die "Cannot close fixture: $!";
    return $bytes;
}

my $fixtures = "$Bin/../../../../specs/fixtures/build-tool-v1";
my $h = CodingAdventures::BuildTool::Hasher->new();
my $canonical_bytes = raw_file("$fixtures/language-source-input-registry.json");
my $registry = decode_json($canonical_bytes);

subtest 'packaged registry is a complete and pinned projection' => sub {
    is($h->registry_bytes(), $canonical_bytes, 'runtime package data equals the neutral registry byte for byte');
    is(decode_json($h->registry_bytes()), $registry, 'all roles and owners are present');
    is($h->registry_digest(), '5201a045ea3e2086fd9be316f2692743ca329f1d84f1c0983a0da47e96b3f621',
        'domain-separated canonical registry identity');
};

my @case_names = qw(
    source-collection-extension
    source-collection-declared
    source-collection-registry-roles
    source-collection-engram-wasm-exact-inputs
    source-collection-typescript-blog-exact-inputs
    source-collection-typescript-landing-page-exact-inputs
    source-collection-typescript-site-foreign-package
);

for my $name (@case_names) {
    subtest "$name uses the production source selector" => sub {
        my $fixture = decode_json(raw_file("$fixtures/cases/$name.json"));
        my $options = $fixture->{input}{options};
        is($options->{registry_sha256}, $h->registry_digest(), 'fixture consumes this registry');
        my $pkg = {
            path => '/inert/package',
            language => $options->{language},
            package_root => $options->{package_root},
            source_mode => $options->{mode},
            declared_srcs => $options->{declared_srcs},
        };
        my @paths = $h->select_source_paths($pkg, $options->{candidates});
        my %candidate = map { $_->{path} => $_ } @{ $options->{candidates} };
        my @actual = map {
            +{path => $_, digest => sha256_hex(pack('H*', $candidate{$_}{content_hex}))}
        } @paths;
        is(\@actual, $fixture->{expected}{result}{files}, 'exact selected paths and raw-byte digests');
    };
}

subtest 'unknown language fails before touching candidates' => sub {
    like(dies { $h->select_source_paths(
        {language => 'unknown', package_root => 'code/packages/unknown/demo', source_mode => 'extension'},
        undef,
    ) }, qr/SOURCE_LANGUAGE_INVALID/, 'unknown lane is rejected first');
};

subtest 'portable path and package identity checks fail closed' => sub {
    my $pkg = {language => 'perl', source_mode => 'extension'};
    like(dies { $h->select_source_paths($pkg, [
        {path => 'Foo/a.pm', kind => 'file'},
        {path => 'foo/b.pm', kind => 'file'},
    ]) }, qr/SOURCE_PATH_INVALID/, 'casefold aliases of directory prefixes are rejected');
    like(dies { $h->select_source_paths({%$pkg, package_root => 'code/packages/python/demo'}, []) },
        qr/SOURCE_PACKAGE_ROOT_INVALID/, 'package-exact authority requires matching lane');
    like(dies { $h->select_source_paths({%$pkg, source_mode => 'declared_sources',
        declared_srcs => ['*.pm', 'lib/[z-a].pm']}, []) },
        qr/DECLARED_GLOB_INVALID/, 'a later invalid glob fails before candidates');
    is([$h->select_source_paths({%$pkg, source_mode => 'declared_sources',
        declared_srcs => ['lib/[literal.pm']}, [{path => 'lib/[literal.pm', kind => 'file'}])],
        ['lib/[literal.pm'], 'unmatched left bracket is a literal');
    like(dies { $h->select_source_paths($pkg, [
        {path => 'safe' . chr(0x202e) . '.pm', kind => 'file'},
    ]) }, qr/SOURCE_PATH_INVALID/, 'Unicode format controls are rejected');
    like(dies { $h->select_source_paths($pkg, [
        {path => 'file', kind => 'file'},
        {path => 'file/nested.pm', kind => 'file'},
    ]) }, qr/SOURCE_PATH_INVALID/, 'a file cannot be an ancestor of another candidate');
};

subtest 'declared globs are bounded and segment-aware' => sub {
    my $pkg = {language => 'perl', source_mode => 'declared_sources'};
    is([$h->select_source_paths({%$pkg, declared_srcs => ['lib[!x]secret.pm']}, [
        {path => 'lib/secret.pm', kind => 'file'},
    ])], [], 'negated class does not cross a slash');
    is([$h->select_source_paths({%$pkg, declared_srcs => ['lib**secret.pm']}, [
        {path => 'lib/x/secret.pm', kind => 'file'},
    ])], [], 'embedded double-star does not cross a slash');
    is([$h->select_source_paths({%$pkg, declared_srcs => ['lib/**/secret.pm']}, [
        {path => 'lib/secret.pm', kind => 'file'},
        {path => 'lib/x/secret.pm', kind => 'file'},
    ])], ['lib/secret.pm', 'lib/x/secret.pm'], 'whole-segment double-star crosses zero or more segments');
    is([$h->select_source_paths({%$pkg, declared_srcs => ['foo--bar.pm', 'lib/***.pm']}, [
        {path => 'foo--bar.pm', kind => 'file'},
        {path => 'lib/a.pm', kind => 'file'},
    ])], ['foo--bar.pm', 'lib/a.pm'], 'literal dashes and coalesced stars are portable');
    my $near_miss = ('x/' x 12) . 'z';
    my $adversarial = ('**/' x 12) . '[ab]';
    is([$h->select_source_paths({%$pkg, declared_srcs => [$adversarial]}, [
        {path => $near_miss, kind => 'file'},
    ])], [], 'many globstars have bounded near-miss matching');
};

subtest 'Hashing v1 frames paths and bytes unambiguously' => sub {
    my $digest = $h->hash_source_records([
        {path => 'b.pm', content => "two\0bytes"},
        {path => 'a.pm', content => "one"},
    ]);
    my $expected = sha256_hex(
        pack('Q>', 4) . 'a.pm' . pack('Q>', 3) . 'one' .
        pack('Q>', 4) . 'b.pm' . pack('Q>', 9) . "two\0bytes"
    );
    is($digest, $expected, 'sorted normalized paths and length-framed raw contents');
    is($h->hash_source_records([]), sha256_hex(''), 'empty package hashes empty stream');
    my $case = decode_json(raw_file("$fixtures/cases/hashing-cache-missing.json"));
    my $input = $case->{workspace}{files}[0];
    is($h->hash_source_records([{path => $input->{path}, content => $input->{content_utf8}}]),
        $case->{expected}{result}{package_digest}, 'one-file neutral Hashing v1 package digest');

    my $dir = tempdir(CLEANUP => 1);
    my $package = "$dir/code/packages/python/demo";
    make_path("$package/src");
    open my $fh, '>:raw', "$package/src/data.bin" or die "Cannot create source: $!";
    print $fh $input->{content_utf8};
    close $fh;
    is($h->hash_package({path => $package, language => 'python', name => 'python/demo',
        source_mode => 'declared_sources', declared_srcs => ['src/data.bin']}),
        $case->{expected}{result}{package_digest},
        'native collector frames the canonical repository-relative path');
};

done_testing();
