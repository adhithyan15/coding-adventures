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
};

done_testing();
