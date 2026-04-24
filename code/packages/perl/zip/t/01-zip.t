use strict;
use warnings;
use Test2::V0;

use CodingAdventures::Zip qw(
    crc32 dos_epoch dos_datetime
    new_writer add_file add_directory finish
    new_reader reader_entries reader_read read_by_name
    zip unzip
);

# ============================================================================
# TC-1: Round-trip single file, Stored (compress=0)
# ============================================================================

subtest 'TC-1: round-trip single file, Stored' => sub {
    my $archive = zip([ ["hello.txt", "Hello, ZIP!"] ], 0);
    my $files   = unzip($archive);
    is $files->{"hello.txt"}, "Hello, ZIP!", 'data matches';

    # Confirm method=0 in archive.
    my $reader  = new_reader($archive);
    my $entries = reader_entries($reader);
    is scalar(@$entries), 1,   'one entry';
    is $entries->[0]{method}, 0, 'method is Stored';
};

# ============================================================================
# TC-2: Round-trip single file, DEFLATE (repetitive text)
# ============================================================================

subtest 'TC-2: round-trip single file, DEFLATE' => sub {
    my $data    = "abcdefgh" x 500;  # 4000 bytes, highly compressible
    my $archive = zip([ ["data.txt", $data] ]);
    my $files   = unzip($archive);
    is $files->{"data.txt"}, $data, 'data round-trips correctly';

    my $reader  = new_reader($archive);
    my $entries = reader_entries($reader);
    is $entries->[0]{method}, 8, 'method is DEFLATE';
    ok length($archive) < length($data) + 200, 'archive is smaller than raw data';
};

# ============================================================================
# TC-3: Multiple files in one archive
# ============================================================================

subtest 'TC-3: multiple files' => sub {
    my $archive = zip([
        ["alpha.txt", "Alpha"],
        ["beta.txt",  "Beta"],
        ["gamma.txt", "Gamma"],
    ]);
    my $files = unzip($archive);
    is $files->{"alpha.txt"}, "Alpha", 'alpha';
    is $files->{"beta.txt"},  "Beta",  'beta';
    is $files->{"gamma.txt"}, "Gamma", 'gamma';
};

# ============================================================================
# TC-4: Directory entry
# ============================================================================

subtest 'TC-4: directory entry' => sub {
    my $w = new_writer();
    add_directory($w, "docs/");
    add_file($w, "docs/readme.txt", "Read me");
    my $archive = finish($w);

    my $reader  = new_reader($archive);
    my $entries = reader_entries($reader);

    my $found_dir = 0;
    for my $e (@$entries) {
        $found_dir = 1 if $e->{name} eq 'docs/' && $e->{is_directory};
    }
    ok $found_dir, 'directory entry docs/ found';

    my $data = read_by_name($reader, "docs/readme.txt");
    is $data, "Read me", 'file inside directory reads correctly';
};

# ============================================================================
# TC-5: CRC-32 mismatch detected (corrupt data)
# ============================================================================

subtest 'TC-5: CRC-32 mismatch detected' => sub {
    my $archive = zip([ ["file.txt", "Hello"] ]);

    # Corrupt a byte in the file data area.
    # Local header is 30 + name_len bytes. "file.txt" = 8 chars.
    # Data starts at offset 30 + 8 = 38.
    my $corrupt = substr($archive, 0, 38) . chr(0xFF) . substr($archive, 39);

    my $died = 0;
    eval { unzip($corrupt) };
    $died = 1 if $@;
    ok $died, 'CRC mismatch raises error';
};

# ============================================================================
# TC-6: Random-access read by name (10 files)
# ============================================================================

subtest 'TC-6: random-access read by name' => sub {
    my @files;
    for my $i (1..10) {
        push @files, ["f${i}.txt", "content_$i"];
    }
    my $archive = zip(\@files);
    my $reader  = new_reader($archive);
    my $data    = read_by_name($reader, "f5.txt");
    is $data, "content_5", 'random-access read correct';
};

# ============================================================================
# TC-7: Incompressible data stored as Stored (method=0)
# ============================================================================

subtest 'TC-7: incompressible data falls back to Stored' => sub {
    # Bytes 144-255 in order: no repeating substrings of length >= 3, so
    # LZSS emits all literals. Fixed Huffman codes for 144-255 cost 9 bits
    # each (vs 8 bits raw), so compressed > original → Stored is chosen.
    my $data = join('', map { chr($_) } 144..255);  # 112 bytes

    my $archive = zip([ ["rand.bin", $data] ]);
    my $reader  = new_reader($archive);
    my $entries = reader_entries($reader);
    is $entries->[0]{method}, 0, 'incompressible data stored as Stored';

    my $files = unzip($archive);
    is $files->{"rand.bin"}, $data, 'data round-trips correctly';
};

# ============================================================================
# TC-8: Empty file
# ============================================================================

subtest 'TC-8: empty file' => sub {
    my $files = unzip(zip([ ["empty.txt", ""] ]));
    is $files->{"empty.txt"}, "", 'empty file round-trips';
};

# ============================================================================
# TC-9: Large file compressed (100 KB repetitive data)
# ============================================================================

subtest 'TC-9: large file (100 KB repetitive)' => sub {
    my $data = "Hello, World! " x 7500;
    $data = substr($data, 0, 102400);  # exactly 100 KB
    my $files = unzip(zip([ ["large.txt", $data] ]));
    is $files->{"large.txt"}, $data, '100 KB compresses and decompresses correctly';
};

# ============================================================================
# TC-10: Unicode filename (UTF-8)
# ============================================================================

subtest 'TC-10: Unicode filename' => sub {
    my $resume = "r\xC3\xA9sum\xC3\xA9.txt";
    my $w = new_writer();
    add_file($w, $resume, "curriculum vitae");
    my $archive = finish($w);

    my $reader = new_reader($archive);
    my $data   = read_by_name($reader, $resume);
    is $data, "curriculum vitae", 'UTF-8 filename read correctly';
};

# ============================================================================
# TC-11: Nested paths
# ============================================================================

subtest 'TC-11: nested paths' => sub {
    my $files = unzip(zip([
        ["a/b/c/deep.txt",  "deeply nested"],
        ["a/b/shallow.txt", "shallow"],
    ]));
    is $files->{"a/b/c/deep.txt"},  "deeply nested", 'deep path';
    is $files->{"a/b/shallow.txt"}, "shallow",        'shallow path';
};

# ============================================================================
# TC-12: Empty archive
# ============================================================================

subtest 'TC-12: empty archive' => sub {
    my $w       = new_writer();
    my $archive = finish($w);

    my $reader  = new_reader($archive);
    my $entries = reader_entries($reader);
    is scalar(@$entries), 0, 'no entries';

    my $files = unzip($archive);
    is $files, {}, 'empty map';
};

# ============================================================================
# CRC-32 known vectors
# ============================================================================

subtest 'crc32 known vectors' => sub {
    is crc32("hello world"),  0x0D4A1185, '"hello world"';
    is crc32(""),             0x00000000, 'empty string';
    is crc32("123456789"),    0xCBF43926, '"123456789"';

    # Chained computation
    my $full = crc32("helloworld");
    my $part = crc32("world", crc32("hello"));
    is $part, $full, 'chained computation matches';
};

# ============================================================================
# DOS datetime
# ============================================================================

subtest 'dos_epoch' => sub {
    is dos_epoch(), 0x00210000, 'DOS_EPOCH is 0x00210000';
};

subtest 'dos_datetime' => sub {
    is dos_datetime(1980, 1, 1, 0, 0, 0), 0x00210000, '1980-01-01 00:00:00';
    # time = (10<<11)|(30<<5)|0 = 20480|960|0 = 21440 = 0x53C0
    # date = ((2024-1980)<<9)|(6<<5)|15 = 22528+192+15 = 22735 = 0x58CF
    is dos_datetime(2024, 6, 15, 10, 30, 0), 0x58CF53C0, '2024-06-15 10:30:00';
};

# ============================================================================
# EOCD scanning errors
# ============================================================================

subtest 'EOCD scanning' => sub {
    my $died;

    $died = 0;
    eval { new_reader("too short") };
    $died = 1 if $@;
    ok $died, 'too-short data raises error';

    $died = 0;
    eval { new_reader("\0" x 100) };
    $died = 1 if $@;
    ok $died, 'no EOCD signature raises error';
};

# ============================================================================
# read_by_name: not found
# ============================================================================

subtest 'read_by_name: not found' => sub {
    my $archive = zip([ ["exists.txt", "yes"] ]);
    my $reader  = new_reader($archive);
    my $died    = 0;
    eval { read_by_name($reader, "missing.txt") };
    $died = 1 if $@;
    ok $died, 'missing entry raises error';
    like $@, qr/not found/, 'error message mentions "not found"';
};

# ============================================================================
# Security: path traversal rejection
# ============================================================================

# Build a minimal ZIP with a crafted (potentially malicious) entry name.
sub _craft_zip {
    my ($name, $content) = @_;
    my $crc = crc32($content);
    my $nl  = length($name);
    my $dl  = length($content);

    my $lh = join('',
        pack('V', 0x04034B50),      # local sig
        pack('v', 10),              # version needed
        pack('v', 0),               # flags
        pack('v', 0),               # method=stored
        pack('vv', 0, 0),           # time, date
        pack('V', $crc),
        pack('VV', $dl, $dl),       # compressed, uncompressed
        pack('v', $nl), pack('v', 0),
        $name, $content,
    );
    my $cd = join('',
        pack('V', 0x02014B50),
        pack('v', 0x031E),          # version made by
        pack('v', 10),              # version needed
        pack('v', 0),               # flags
        pack('v', 0),               # method=stored
        pack('vv', 0, 0),           # time, date
        pack('V', $crc),
        pack('VV', $dl, $dl),
        pack('v', $nl), pack('v', 0), pack('v', 0), # name, extra, comment
        pack('v', 0), pack('v', 0), # disk, internal attrs
        pack('V', 0),               # external attrs
        pack('V', 0),               # local offset
        $name,
    );
    my $eocd = join('',
        pack('V', 0x06054B50),
        pack('v', 0), pack('v', 0),
        pack('v', 1), pack('v', 1),
        pack('V', length($cd)), pack('V', length($lh)),
        pack('v', 0),
    );
    return $lh . $cd . $eocd;
}

subtest 'security: path traversal (..)' => sub {
    my $archive = _craft_zip("../../evil.txt", "evil");
    my $died = 0;
    eval { new_reader($archive) };
    $died = 1 if $@;
    ok $died, 'path traversal raises error';
    like $@, qr/traversal|\.\./i, 'error message mentions traversal';
};

subtest 'security: absolute path' => sub {
    my $archive = _craft_zip("/etc/passwd", "root");
    my $died = 0;
    eval { new_reader($archive) };
    $died = 1 if $@;
    ok $died, 'absolute path raises error';
    like $@, qr/absolute/i, 'error message mentions absolute';
};

subtest 'security: backslash in name' => sub {
    my $archive = _craft_zip("foo\\bar.txt", "data");
    my $died = 0;
    eval { new_reader($archive) };
    $died = 1 if $@;
    ok $died, 'backslash raises error';
    like $@, qr/backslash/i, 'error message mentions backslash';
};

subtest 'security: path traversal on write path' => sub {
    my $w    = new_writer();
    my $died = 0;
    eval { add_file($w, "../../evil.sh", "evil") };
    $died = 1 if $@;
    ok $died, 'write path rejects path traversal';
};

# ============================================================================
# Security: duplicate entry name rejection
# ============================================================================

subtest 'security: duplicate entry names' => sub {
    # Build a ZIP with two entries named "dup.txt".
    sub _make_entry {
        my ($name, $data, $offset) = @_;
        my $crc = crc32($data);
        my $nl  = length($name);
        my $dl  = length($data);
        my $lh = join('',
            pack('V', 0x04034B50), pack('v', 10), pack('v', 0), pack('v', 0),
            pack('vv', 0, 0), pack('V', $crc), pack('VV', $dl, $dl),
            pack('v', $nl), pack('v', 0), $name, $data,
        );
        my $cd = join('',
            pack('V', 0x02014B50), pack('v', 0x031E), pack('v', 10), pack('v', 0), pack('v', 0),
            pack('vv', 0, 0), pack('V', $crc), pack('VV', $dl, $dl),
            pack('v', $nl), pack('v', 0), pack('v', 0),
            pack('v', 0), pack('v', 0), pack('V', 0), pack('V', $offset),
            $name,
        );
        return ($lh, $cd);
    }

    my ($lh1, $cd1) = _make_entry("dup.txt", "first",  0);
    my ($lh2, $cd2) = _make_entry("dup.txt", "second", length($lh1));

    my $cd_data = $cd1 . $cd2;
    my $cd_off  = length($lh1) + length($lh2);
    my $eocd = join('',
        pack('V', 0x06054B50), pack('v', 0), pack('v', 0),
        pack('v', 2), pack('v', 2), pack('V', length($cd_data)), pack('V', $cd_off),
        pack('v', 0),
    );
    my $archive = $lh1 . $lh2 . $cd_data . $eocd;

    my $died = 0;
    eval { unzip($archive) };
    $died = 1 if $@;
    ok $died, 'duplicate entry names raises error';
    like $@, qr/duplicate/i, 'error mentions duplicate';
};

# ============================================================================
# DEFLATE stored block decode path
# ============================================================================

subtest 'DEFLATE stored block decode' => sub {
    my $files = unzip(zip([ ["empty.bin", ""] ]));
    is $files->{"empty.bin"}, "", 'empty file round-trips via stored block';
};

done_testing;
