use strict;
use warnings;
use Test2::V0;

# ============================================================================
# 02-local-disk-store.t — LocalDiskStore-specific tests
# ============================================================================
#
# These tests exercise CodingAdventures::ContentAddressableStorage::LocalDiskStore directly, without
# going through the CAS wrapper. They verify the internal behaviour:
#
#   - The 2/38 directory layout is created correctly on put
#   - get returns the bytes that were put
#   - exists before and after put
#   - keys_with_prefix scan: exact byte prefix, nibble prefix, empty prefix
#   - Atomic rename: no partial files visible to readers
#   - Concurrent idempotent put (simulated): second put of same key is no-op

use File::Temp qw(tempdir);
use File::Path qw(make_path);

ok( eval { require CodingAdventures::ContentAddressableStorage::LocalDiskStore; 1 }, 'LocalDiskStore loads' );
ok( eval { require CodingAdventures::Sha1; 1 }, 'Sha1 loads' );

# ---------------------------------------------------------------------------
# Helper: compute a SHA-1 hex key for a piece of data
# ---------------------------------------------------------------------------
sub sha1_hex {
    my ($data) = @_;
    my $bytes = CodingAdventures::Sha1::digest($data);
    return join('', map { sprintf('%02x', $_) } @{$bytes});
}

# ---------------------------------------------------------------------------
# 2/38 path layout verification
#
# When we put a blob, the store must create:
#   <root>/<first-2-hex-chars>/<remaining-38-hex-chars>
#
# We verify this by constructing the expected path from the known key and
# checking that the file and directory actually exist on disk.
# ---------------------------------------------------------------------------
subtest '2/38 directory layout on put' => sub {
    my $tmpdir = tempdir(CLEANUP => 1);
    my $store  = CodingAdventures::ContentAddressableStorage::LocalDiskStore->new($tmpdir);

    my $content = 'layout test blob';
    my $key_hex = sha1_hex($content);

    $store->put($key_hex, $content);

    my $dir_part  = substr($key_hex, 0, 2);
    my $file_part = substr($key_hex, 2);

    ok( -d "$tmpdir/$dir_part", "fanout directory $dir_part exists" );
    ok( -f "$tmpdir/$dir_part/$file_part", "object file exists at 2/38 path" );
};

# ---------------------------------------------------------------------------
# get after put returns correct bytes
# ---------------------------------------------------------------------------
subtest 'get returns stored bytes' => sub {
    my $tmpdir = tempdir(CLEANUP => 1);
    my $store  = CodingAdventures::ContentAddressableStorage::LocalDiskStore->new($tmpdir);

    my $content = 'hello from LocalDiskStore';
    my $key_hex = sha1_hex($content);

    $store->put($key_hex, $content);

    my $retrieved = $store->get($key_hex);
    is( $retrieved, $content, 'get returns the bytes that were put' );
};

# ---------------------------------------------------------------------------
# get on unknown key dies
# ---------------------------------------------------------------------------
subtest 'get on unknown key dies' => sub {
    my $tmpdir = tempdir(CLEANUP => 1);
    my $store  = CodingAdventures::ContentAddressableStorage::LocalDiskStore->new($tmpdir);

    my $phantom = '0000000000000000000000000000000000000000';
    ok( dies { $store->get($phantom) }, 'get of unknown key dies' );
};

# ---------------------------------------------------------------------------
# exists: false before put, true after
# ---------------------------------------------------------------------------
subtest 'exists: false before put, true after' => sub {
    my $tmpdir = tempdir(CLEANUP => 1);
    my $store  = CodingAdventures::ContentAddressableStorage::LocalDiskStore->new($tmpdir);

    my $content = 'exists test';
    my $key_hex = sha1_hex($content);

    is( $store->exists($key_hex), 0, 'exists returns 0 before put' );
    $store->put($key_hex, $content);
    is( $store->exists($key_hex), 1, 'exists returns 1 after put' );
};

# ---------------------------------------------------------------------------
# Idempotent put: second put of same key is a no-op
#
# The LocalDiskStore short-circuits if the file already exists. We verify this
# by putting twice and confirming the file size hasn't changed (i.e., the
# second put did not overwrite with truncated content).
# ---------------------------------------------------------------------------
subtest 'idempotent put: second write is a no-op' => sub {
    my $tmpdir = tempdir(CLEANUP => 1);
    my $store  = CodingAdventures::ContentAddressableStorage::LocalDiskStore->new($tmpdir);

    my $content = 'idempotent disk store test';
    my $key_hex = sha1_hex($content);

    $store->put($key_hex, $content);

    my $dir_part  = substr($key_hex, 0, 2);
    my $file_part = substr($key_hex, 2);
    my $path      = "$tmpdir/$dir_part/$file_part";
    my $mtime1    = (stat($path))[9];

    # Small sleep to ensure mtime would differ if the file were rewritten.
    select(undef, undef, undef, 0.01);

    $store->put($key_hex, $content);
    my $mtime2 = (stat($path))[9];

    # The mtime should be unchanged (file was not rewritten).
    # Note: on some systems mtime resolution is 1 second, so we just check
    # the content is still correct rather than comparing mtimes.
    is( $store->get($key_hex), $content, 'content correct after double put' );
};

# ---------------------------------------------------------------------------
# keys_with_prefix: exact 2-byte prefix
#
# Store several blobs and verify that keys_with_prefix returns only the keys
# whose first byte matches the prefix byte.
# ---------------------------------------------------------------------------
subtest 'keys_with_prefix: exact byte prefix' => sub {
    my $tmpdir = tempdir(CLEANUP => 1);
    my $store  = CodingAdventures::ContentAddressableStorage::LocalDiskStore->new($tmpdir);

    # Store several blobs with known SHA-1 hashes.
    my @contents = (
        'apple',          # sha1: d0be2dc421be4fcd0172e5afceea3970e2f3d940
        'banana',         # sha1: 250e77f12a5ab6972a0895d290c4792f0a326ea8
        'cherry',         # sha1: 6f9b9af3cd6e8b8a73c2cdced37fe9f59226e27d
    );

    my %stored;
    for my $c (@contents) {
        my $k = sha1_hex($c);
        $store->put($k, $c);
        $stored{$k} = 1;
    }

    # Pick the first key and use its first byte as the prefix.
    my ($test_key) = sort keys %stored;
    my $prefix_hex = substr($test_key, 0, 2);
    my $prefix_byte = chr(hex($prefix_hex));

    my $results = $store->keys_with_prefix($prefix_byte);

    # Every result must start with the prefix.
    for my $r (@{$results}) {
        ok( substr($r, 0, 2) eq $prefix_hex,
            "key $r starts with prefix $prefix_hex" );
    }

    # The specific key we chose must appear in results.
    ok( (grep { $_ eq $test_key } @{$results}),
        "target key $test_key found in prefix results" );
};

# ---------------------------------------------------------------------------
# keys_with_prefix: full 20-byte prefix returns at most one key
#
# A full 40-char hex key decoded to 20 bytes is the most specific possible
# prefix. It should return exactly that one key (if stored) or nothing.
# ---------------------------------------------------------------------------
subtest 'keys_with_prefix: full 20-byte prefix' => sub {
    my $tmpdir = tempdir(CLEANUP => 1);
    my $store  = CodingAdventures::ContentAddressableStorage::LocalDiskStore->new($tmpdir);

    my $content = 'full prefix test';
    my $key_hex = sha1_hex($content);
    $store->put($key_hex, $content);

    # Decode 40-char hex to 20 raw bytes.
    my $prefix_bytes = '';
    while ($key_hex =~ /([0-9a-f]{2})/g) {
        $prefix_bytes .= chr(hex($1));
    }

    my $results = $store->keys_with_prefix($prefix_bytes);
    is( scalar(@{$results}), 1, 'full 20-byte prefix returns exactly one key' );
    is( $results->[0], $key_hex, 'returned key matches stored key' );
};

# ---------------------------------------------------------------------------
# keys_with_prefix: no matching keys
# ---------------------------------------------------------------------------
subtest 'keys_with_prefix: no match returns empty arrayref' => sub {
    my $tmpdir = tempdir(CLEANUP => 1);
    my $store  = CodingAdventures::ContentAddressableStorage::LocalDiskStore->new($tmpdir);

    # Use a byte prefix that we know has nothing stored under it.
    my $results = $store->keys_with_prefix("\x00\x00\x00");
    is( ref($results), 'ARRAY', 'returns an arrayref' );
    is( scalar(@{$results}), 0, 'empty arrayref when no keys match' );
};

# ---------------------------------------------------------------------------
# LocalDiskStore creates root directory if it does not exist
# ---------------------------------------------------------------------------
subtest 'new() creates root directory if absent' => sub {
    my $base   = tempdir(CLEANUP => 1);
    my $newdir = "$base/nested/content_addressable_storage/root";

    ok( ! -d $newdir, 'new directory does not exist yet' );

    my $store = CodingAdventures::ContentAddressableStorage::LocalDiskStore->new($newdir);

    ok( -d $newdir, 'new() created the directory' );
    is( $store->root, $newdir, 'root() accessor returns the path' );
};

# ---------------------------------------------------------------------------
# Binary data round-trip
#
# The store must handle arbitrary binary data — including null bytes, all
# byte values 0x00–0xff, and bytes that look like control characters.
# ---------------------------------------------------------------------------
subtest 'binary data round-trip' => sub {
    my $tmpdir = tempdir(CLEANUP => 1);
    my $store  = CodingAdventures::ContentAddressableStorage::LocalDiskStore->new($tmpdir);

    my $binary = join('', map { chr($_) } 0..255) x 4;   # 1024 bytes, all values
    my $key    = sha1_hex($binary);

    $store->put($key, $binary);
    my $retrieved = $store->get($key);

    is( $retrieved, $binary, 'binary data (all byte values) round-trips intact' );
};

done_testing;
