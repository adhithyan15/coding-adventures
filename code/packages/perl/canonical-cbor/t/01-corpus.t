use strict;
use warnings;
use v5.28;
use Test2::V0;
use JSON::PP;
use File::Basename qw(dirname);
use File::Spec;

use CodingAdventures::CanonicalCbor qw(:all);

my $cases_file = File::Spec->catfile(dirname(__FILE__), '..', '..', '..', '..', 'specs', 'fixtures', 'canonical-cbor-v1', 'cases.json');
open my $fh, '<', $cases_file or die "Cannot open $cases_file: $!";
my $json_text = do { local $/; <$fh> };
close $fh;

my $data = decode_json($json_text);
my $cases = $data->{cases};

sub hex_to_bytes {
    my ($hex) = @_;
    return '' unless defined $hex && length($hex) > 0;
    return pack('H*', $hex);
}

sub bytes_to_hex {
    my ($bytes) = @_;
    return '' unless defined $bytes && length($bytes) > 0;
    return unpack('H*', $bytes);
}

sub build_nested_array {
    my ($n) = @_;
    my $cur = CborArray([]);
    for (1 .. $n) {
        $cur = CborArray([$cur]);
    }
    return $cur;
}

sub build_nested_array_wire {
    my ($n) = @_;
    return ("\x81" x $n) . "\x80";
}

for my $case (@$cases) {
    my $id = $case->{id};
    my $op = $case->{operation};
    my $input = $case->{input};
    my $expected = $case->{expected};

    if ($op eq 'round-trip') {
        my $wire = hex_to_bytes($input);
        my $val = eval { decode($wire) };
        is($@, '', "$id: decode succeeded without exception");
        ok(defined $val, "$id: decoded value is defined");

        my $re_encoded = eval { encode_checked($val) };
        is($@, '', "$id: encode succeeded without exception");
        is(bytes_to_hex($re_encoded), lc($expected), "$id: re-encoded hex matches expected");
        is($re_encoded, hex_to_bytes($expected), "$id: byte-exact round-trip");

        # Verify decode on re-encoded wire returns deeply equal object
        my $re_decoded = decode($re_encoded);
        ok($val == $re_decoded, "$id: deep equality preserved through re-decode");

    } elsif ($op eq 'encode-map') {
        # Format: "6162=f6;1818=f5;60=f4;00=01;6161=02"
        my @pair_strs = split /;/, $input;
        my @entries;
        for my $ps (@pair_strs) {
            my ($k_hex, $v_hex) = split /=/, $ps;
            my $k = decode(hex_to_bytes($k_hex));
            my $v = decode(hex_to_bytes($v_hex));
            push @entries, CborMapEntry($k, $v);
        }
        my $map_val = CborMap(\@entries);
        my $encoded = eval { encode_checked($map_val) };
        is($@, '', "$id: encode-map succeeded");
        is(bytes_to_hex($encoded), lc($expected), "$id: canonical length-first map matches expected hex");

    } elsif ($op eq 'generated-round-trip') {
        if ($input eq 'nested-array:128') {
            my $val = build_nested_array(128);
            my $wire = build_nested_array_wire(128);
            my $enc = eval { encode_checked($val) };
            is($@, '', "$id: depth 128 encoded successfully");
            is($enc, $wire, "$id: depth 128 wire matches expected");
            my $decoded = eval { decode($wire) };
            is($@, '', "$id: depth 128 decoded successfully");
            ok($val == $decoded, "$id: depth 128 round-trips correctly");

        } elsif ($input =~ /^bytes-repeat:(\d+):([0-9a-fA-F]{2})$/) {
            my $count = $1 + 0;
            my $byte = hex_to_bytes($2);
            my $raw = $byte x $count;
            my $val = CborBytes($raw);
            my $wire = eval { encode_checked($val) };
            is($@, '', "$id: 1MB boundary encoded successfully");
            is(length($wire), 1048576, "$id: exactly 1048576 bytes");
            my $decoded = eval { decode($wire) };
            is($@, '', "$id: 1MB boundary decoded successfully");
            ok($val == $decoded, "$id: 1MB payload round-trips correctly");
        } else {
            die "Unknown generated-round-trip input: $input";
        }

    } elsif ($op eq 'decode-error') {
        my $wire;
        if ($input eq 'nested-array-wire:129') {
            $wire = build_nested_array_wire(129);
        } else {
            $wire = hex_to_bytes($input);
        }
        my $val = eval { decode($wire) };
        my $err = $@;
        ok(!defined $val, "$id: decode failed as expected");
        ok(is_cbor_error($err), "$id: error is CborError");
        is($err->id, $expected, "$id: error ID matches expected '$expected'");

    } elsif ($op eq 'encode-error') {
        if ($input eq 'duplicate-map-key') {
            my $e1 = CborMapEntry(CborUnsigned(0), CborBool(0));
            my $e2 = CborMapEntry(CborUnsigned(0), CborBool(1));
            my $map_val = CborMap([ $e1, $e2 ]);
            my $res = eval { encode_checked($map_val) };
            my $err = $@;
            ok(!defined $res, "$id: encode failed as expected");
            ok(is_cbor_error($err), "$id: error is CborError");
            is($err->id, $expected, "$id: error ID is '$expected'");

        } elsif ($input eq 'nested-array:129') {
            my $val = build_nested_array(129);
            my $res = eval { encode_checked($val) };
            my $err = $@;
            ok(!defined $res, "$id: encode failed as expected");
            ok(is_cbor_error($err), "$id: error is CborError");
            is($err->id, $expected, "$id: error ID is '$expected'");

        } elsif ($input =~ /^bytes-repeat:(\d+):([0-9a-fA-F]{2})$/) {
            my $count = $1 + 0;
            my $byte = hex_to_bytes($2);
            my $raw = $byte x $count;
            my $val = CborBytes($raw);
            my $res = eval { encode_checked($val) };
            my $err = $@;
            ok(!defined $res, "$id: encode failed as expected");
            ok(is_cbor_error($err), "$id: error is CborError");
            is($err->id, $expected, "$id: error ID is '$expected'");
        } else {
            die "Unknown encode-error input: $input";
        }
    } else {
        die "Unknown operation: $op";
    }
}

done_testing;
