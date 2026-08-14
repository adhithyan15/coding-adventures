use strict;
use warnings;
use Test2::V0;
use FindBin;
use File::Spec;
use JSON::PP qw(decode_json);
use Compress::Raw::Zlib qw(MAX_WBITS Z_FINISH Z_OK Z_STREAM_END);

use CodingAdventures::Zip qw(
    crc32 raw_deflate raw_inflate raw_inflate_counted
    RAW_INFLATE_MAX_OUTPUT raw_inflate_error_codes
    new_reader reader_entries reader_read
);

sub from_hex { return pack('H*', $_[0] // '') }

sub expected_bytes {
    my ($case) = @_;
    my $output = $case->{expected}{output};
    return from_hex($output->{hex}) if exists $output->{hex};
    return from_hex($output->{repeat_hex}) x $output->{count};
}

sub fixture {
    my $ancestor = $FindBin::Bin;
    my $path;
    for (0 .. 8) {
        my $candidate = File::Spec->catfile(
            $ancestor, 'specs', 'fixtures', 'zip-raw-rfc1951-v1', 'cases.json',
        );
        if (-f $candidate) {
            $path = $candidate;
            last;
        }
        $ancestor = File::Spec->catdir($ancestor, '..');
    }
    die 'cannot locate neutral fixture' unless defined $path;
    open my $handle, '<:raw', $path or die "cannot open neutral fixture: $!";
    local $/;
    return decode_json(<$handle>);
}

sub zlib_raw_inflate {
    my ($compressed) = @_;
    my ($inflater, $create_status) = Compress::Raw::Zlib::Inflate->new(
        -WindowBits => -MAX_WBITS,
        -AppendOutput => 1,
    );
    die "zlib inflate init failed" unless defined $inflater && $create_status == Z_OK;
    my $output = '';
    my $status = $inflater->inflate($compressed, $output);
    die "zlib raw stream did not end" unless $status == Z_STREAM_END;
    return $output;
}

sub zlib_raw_deflate {
    my ($plain) = @_;
    my ($deflater, $create_status) = Compress::Raw::Zlib::Deflate->new(
        -WindowBits => -MAX_WBITS,
        -AppendOutput => 1,
    );
    die "zlib deflate init failed" unless defined $deflater && $create_status == Z_OK;
    my $output = '';
    my $status = $deflater->deflate($plain, $output);
    die "zlib deflate failed" unless $status == Z_OK;
    $status = $deflater->flush($output, Z_FINISH);
    die "zlib deflate finish failed" unless $status == Z_OK;
    return $output;
}

my $fixture = fixture();
is scalar(@{$fixture->{cases}}), 34, 'closed fixture contains 34 cases';
is $fixture->{limits}{default_max_output}, RAW_INFLATE_MAX_OUTPUT,
    'default output limit matches the public constant';
is $fixture->{limits}{hard_max_output}, RAW_INFLATE_MAX_OUTPUT,
    'hard output limit matches the public constant';
is raw_inflate_error_codes(), $fixture->{error_ids},
    'stable error identifiers match the neutral contract';

for my $case (@{$fixture->{cases}}) {
    subtest $case->{id} => sub {
        my $operation = $case->{operation};
        if ($operation eq 'inflate') {
            my $input = from_hex($case->{input_hex});
            my $limit = exists($case->{max_output})
                ? $case->{max_output}
                : RAW_INFLATE_MAX_OUTPUT;
            my $result = raw_inflate_counted($input, $limit);
            is $result->output, expected_bytes($case), 'decoded bytes match';
            is $result->bytes_consumed, $case->{expected}{bytes_consumed},
                'exact byte consumption matches';
            is raw_inflate($input, $limit), expected_bytes($case),
                'uncounted wrapper matches';
        } elsif ($operation eq 'inflate-error') {
            my $limit = exists($case->{max_output})
                ? $case->{max_output}
                : RAW_INFLATE_MAX_OUTPUT;
            my $value = eval {
                raw_inflate_counted(from_hex($case->{input_hex}), $limit);
            };
            my $error = $@;
            ok !defined($value), 'failure exposes no partial result';
            isa_ok $error, ['CodingAdventures::Zip::RawInflateError'];
            is $error->code, $case->{expected}{error_id}, 'typed code matches';
            is "$error", $case->{expected}{error_id}, 'message is payload-blind';
        } elsif ($operation eq 'deflate-interoperability') {
            my $compressed = raw_deflate(from_hex($case->{input_hex}));
            is zlib_raw_inflate($compressed), expected_bytes($case),
                'independent zlib decoder accepts encoder output';
        } elsif ($operation eq 'crc32') {
            my $checksum = exists($case->{initial_crc32_hex})
                ? hex($case->{initial_crc32_hex}) : 0;
            $checksum = crc32(from_hex($_), $checksum) for @{$case->{chunks_hex}};
            is sprintf('%08x', $checksum), $case->{expected}{crc32_hex},
                'incremental CRC-32 matches';
        } else {
            fail "unknown fixture operation $operation";
        }
    };
}

my $dynamic = from_hex(
    '0dc28911c0200c03b0d8f97028ec3f6ed129cab7dd96a0c2445bdb93809663a5d303f6b265e20c2b79ea03379d227e'
);
my $dynamic_output = from_hex(
    '0406030b000e070909010906010a04070007000000000501010908030108050302030401000401000207090009020a0a020605020d060c01020b020302090201'
);

sub raw_zip {
    my ($name, $compressed, $plain, $declared_size) = @_;
    $declared_size //= length($plain);
    my $checksum = crc32($plain);
    my $local = pack(
        'VvvvvvVVVvv', 0x04034B50, 20, 0x0800, 8, 0, 0, $checksum,
        length($compressed), $declared_size, length($name), 0,
    ) . $name . $compressed;
    my $central_offset = length($local);
    my $central = pack(
        'VvvvvvvVVVvvvvvVV', 0x02014B50, 0x031E, 20, 0x0800, 8, 0, 0,
        $checksum, length($compressed), $declared_size, length($name),
        0, 0, 0, 0, 0, 0,
    ) . $name;
    my $eocd = pack(
        'VvvvvVVv', 0x06054B50, 0, 0, 1, 1, length($central),
        $central_offset, 0,
    );
    return $local . $central . $eocd;
}

sub read_only_entry {
    my ($archive) = @_;
    my $reader = new_reader($archive);
    return reader_read($reader, reader_entries($reader)->[0]);
}

is read_only_entry(raw_zip('dynamic.bin', $dynamic, $dynamic_output)),
    $dynamic_output, 'ZIP reader accepts a dynamic raw payload';

for my $boundary (
    [raw_zip('cavity.bin', $dynamic . "\xDE\xAD", $dynamic_output),
        'zip: compressed payload contains trailing bytes'],
    [raw_zip('size.bin', $dynamic, $dynamic_output, length($dynamic_output) + 1),
        'zip: uncompressed size does not match the directory'],
) {
    eval { read_only_entry($boundary->[0]) };
    my $message = $boundary->[1];
    like "$@", qr/^\Q$message\E/, "$message rejected";
}

for my $limit (-1, RAW_INFLATE_MAX_OUTPUT + 1, 1.5) {
    eval { raw_inflate_counted("\x01\x00\x00\xFF\xFF", $limit) };
    my $error = $@;
    isa_ok $error, ['CodingAdventures::Zip::RawInflateError'];
    is $error->code, 'invalid-output-limit', 'invalid caller limit rejected';
}

my $prefix = pack('C*', map { ($_ * 73 + int($_ / 251)) & 0xFF } 0 .. 32_767);
my $full_window_plain = $prefix . $prefix;
my $full_window_stream = zlib_raw_deflate($full_window_plain);
is raw_inflate($full_window_stream, length($full_window_plain)),
    $full_window_plain, 'foreign stream exercises the full 32 KiB window';

done_testing;
