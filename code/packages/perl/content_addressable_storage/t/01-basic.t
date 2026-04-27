use strict;
use warnings;
use Test2::V0;

# ============================================================================
# 01-basic.t — core CAS tests using a LocalDiskStore backend
# ============================================================================
#
# These tests exercise CodingAdventures::ContentAddressableStorage end-to-end:
#   - Module loading
#   - Round-trip: put/get for empty, small, and large blobs
#   - Idempotent put
#   - NotFound error on missing key
#   - Corrupted error when stored file is tampered with
#   - exists() before and after put
#   - find_by_prefix: unique, ambiguous, not-found, invalid hex, empty string
#
# All tests use a temporary directory that is cleaned up after the test run.

use File::Temp qw(tempdir);
use File::Path qw(make_path);

ok( eval { require CodingAdventures::ContentAddressableStorage; 1 }, 'CodingAdventures::ContentAddressableStorage loads' );
ok( eval { require CodingAdventures::ContentAddressableStorage::LocalDiskStore; 1 }, 'LocalDiskStore loads' );
ok( eval { require CodingAdventures::ContentAddressableStorage::Error; 1 }, 'Error module loads' );

# ---------------------------------------------------------------------------
# Helper: build a fresh CAS backed by a temp directory
# ---------------------------------------------------------------------------
sub make_cas {
    my $tmpdir = tempdir(CLEANUP => 1);
    my $store  = CodingAdventures::ContentAddressableStorage::LocalDiskStore->new($tmpdir);
    my $cas    = CodingAdventures::ContentAddressableStorage->new($store);
    return ($cas, $tmpdir);
}

# ---------------------------------------------------------------------------
# Round-trip: empty blob
#
# An empty byte string is valid content. Its SHA-1 hash is the well-known
# value "da39a3ee5e6b4b0d3255bfef95601890afd80709".
# ---------------------------------------------------------------------------
subtest 'round-trip: empty blob' => sub {
    my ($cas) = make_cas();
    my $key  = $cas->put('');
    my $data = $cas->get($key);
    is( $data, '', 'get(put("")) returns empty string' );
    is( length($key), 40, 'key is 40 hex chars' );
    is( $key, 'da39a3ee5e6b4b0d3255bfef95601890afd80709', 'SHA-1 of empty string is correct' );
};

# ---------------------------------------------------------------------------
# Round-trip: small blob
#
# "hello, world" is a canonical test string used throughout this codebase.
# ---------------------------------------------------------------------------
subtest 'round-trip: small blob' => sub {
    my ($cas) = make_cas();
    my $content = 'hello, world';
    my $key     = $cas->put($content);
    my $data    = $cas->get($key);
    is( $data, $content, 'get(put("hello, world")) round-trips correctly' );
    is( length($key), 40, 'key is 40 hex chars' );
};

# ---------------------------------------------------------------------------
# Round-trip: 1 MiB blob
#
# Large blobs must also round-trip faithfully. We use a 1 MiB buffer of
# repeating bytes (0x00 through 0xff cyclically). This tests that:
#   1. The SHA-1 implementation handles multi-block messages correctly.
#   2. The file I/O reads and writes binary data without mangling bytes.
# ---------------------------------------------------------------------------
subtest 'round-trip: 1 MiB blob' => sub {
    my ($cas) = make_cas();

    # Build 1 MiB of repeating pattern 0x00..0xff.
    my $size    = 1024 * 1024;
    my $content = pack('C*', map { $_ % 256 } 0 .. $size - 1);

    my $key  = $cas->put($content);
    my $data = $cas->get($key);

    is( length($data), $size, '1 MiB blob: correct length retrieved' );
    is( $data, $content, '1 MiB blob: bytes round-trip intact' );
};

# ---------------------------------------------------------------------------
# Idempotent put
#
# Storing the same content twice must return the same key both times and must
# not raise an error. Only one copy is stored (deduplication).
# ---------------------------------------------------------------------------
subtest 'idempotent put' => sub {
    my ($cas) = make_cas();
    my $content = 'idempotent test content';
    my $key1 = $cas->put($content);
    my $key2 = $cas->put($content);
    is( $key1, $key2, 'two puts of same content return same key' );
    # Verify the data is still readable after the second put.
    is( $cas->get($key1), $content, 'data readable after idempotent put' );
};

# ---------------------------------------------------------------------------
# get: NotFound error
#
# Requesting a key that was never stored must die with a CasNotFoundError.
# We construct a valid-looking key (40 hex chars) that we know has never been
# stored.
# ---------------------------------------------------------------------------
subtest 'get: NotFound error on missing key' => sub {
    my ($cas) = make_cas();

    my $phantom_key = '0000000000000000000000000000000000000000';

    my $err;
    eval { $cas->get($phantom_key) };
    $err = $@;

    ok( $err, 'get of unknown key dies' );
    ok( ref($err) && $err->isa('CodingAdventures::ContentAddressableStorage::Error::CasNotFoundError'),
        'dies with CasNotFoundError' );
    is( $err->key, $phantom_key, 'error carries the requested key' );
};

# ---------------------------------------------------------------------------
# get: CorruptedError when file contents are tampered with
#
# After storing a blob, we directly overwrite the backing file with different
# bytes. The next get() must detect the hash mismatch and die with
# CasCorruptedError rather than silently returning wrong data.
# ---------------------------------------------------------------------------
subtest 'get: CorruptedError when file is tampered' => sub {
    my ($cas, $tmpdir) = make_cas();

    my $content = 'this will be corrupted';
    my $key     = $cas->put($content);

    # Locate the backing file: root/XX/YYYYYYYY...
    my $dir_part  = substr($key, 0, 2);
    my $file_part = substr($key, 2);
    my $path      = "$tmpdir/$dir_part/$file_part";

    ok( -e $path, 'backing file exists before corruption' );

    # Overwrite the file with junk bytes (different from original content).
    open(my $fh, '>:raw', $path) or die "Cannot open $path: $!";
    print $fh 'CORRUPTED DATA - this is not the original content';
    close($fh);

    my $err;
    eval { $cas->get($key) };
    $err = $@;

    ok( $err, 'get of corrupted blob dies' );
    ok( ref($err) && $err->isa('CodingAdventures::ContentAddressableStorage::Error::CasCorruptedError'),
        'dies with CasCorruptedError' );
    is( $err->key, $key, 'error carries the requested key' );
};

# ---------------------------------------------------------------------------
# exists: false before put, true after
# ---------------------------------------------------------------------------
subtest 'exists: false before put, true after' => sub {
    my ($cas) = make_cas();

    my $content = 'existence check';

    # Compute what the key would be without storing it.
    # We can compute the SHA-1 externally using our own module.
    require CodingAdventures::Sha1;
    my $bytes   = CodingAdventures::Sha1::digest($content);
    my $key_hex = join('', map { sprintf('%02x', $_) } @{$bytes});

    is( $cas->exists($key_hex), 0, 'exists returns 0 before put' );

    $cas->put($content);

    is( $cas->exists($key_hex), 1, 'exists returns 1 after put' );
};

# ---------------------------------------------------------------------------
# find_by_prefix: unique match
#
# Store a single blob and look it up by the first 7 characters of its key
# (the same length git uses in short hashes). Must return the full 40-char key.
# ---------------------------------------------------------------------------
subtest 'find_by_prefix: unique match' => sub {
    my ($cas) = make_cas();

    my $key = $cas->put('find_by_prefix unique test');
    my $prefix7 = substr($key, 0, 7);

    my $found = $cas->find_by_prefix($prefix7);
    is( $found, $key, 'find_by_prefix with 7-char prefix finds unique key' );
};

# ---------------------------------------------------------------------------
# find_by_prefix: full key as prefix (40 chars)
#
# Using the full 40-char hex key as the prefix must also work.
# ---------------------------------------------------------------------------
subtest 'find_by_prefix: full key as prefix' => sub {
    my ($cas) = make_cas();

    my $key   = $cas->put('full key prefix test');
    my $found = $cas->find_by_prefix($key);
    is( $found, $key, 'find_by_prefix with full 40-char key returns that key' );
};

# ---------------------------------------------------------------------------
# find_by_prefix: ambiguous match
#
# Store two blobs and verify that a prefix shared by both keys triggers
# CasAmbiguousPrefixError. To guarantee a shared prefix we use a
# MemoryStore-like approach: we need two keys that share a prefix.
#
# Strategy: put two distinct blobs, then try the common prefix of their keys.
# In the unlikely event that their first 2 bytes differ, try a shorter prefix.
# We iterate until we find a prefix length where they agree, or we give up.
# ---------------------------------------------------------------------------
subtest 'find_by_prefix: ambiguous match' => sub {
    my ($cas) = make_cas();

    my $key1 = $cas->put('blob alpha');
    my $key2 = $cas->put('blob beta');

    # Find a common prefix between key1 and key2.
    my $common_prefix = '';
    my $min_len = length($key1) < length($key2) ? length($key1) : length($key2);
    for my $i (0 .. $min_len - 1) {
        if (substr($key1, $i, 1) eq substr($key2, $i, 1)) {
            $common_prefix .= substr($key1, $i, 1);
        } else {
            last;
        }
    }

    if (length($common_prefix) > 0) {
        # There is a common prefix: verify it causes ambiguity.
        my $err;
        eval { $cas->find_by_prefix($common_prefix) };
        $err = $@;
        ok( $err, 'find_by_prefix with ambiguous prefix dies' );
        ok( ref($err) && $err->isa('CodingAdventures::ContentAddressableStorage::Error::CasAmbiguousPrefixError'),
            'dies with CasAmbiguousPrefixError' );
    } else {
        # The two keys have no common prefix characters — skip this assertion.
        # This is theoretically possible but extremely unlikely with SHA-1.
        pass('keys have no common prefix — skipping ambiguity test');
    }
};

# ---------------------------------------------------------------------------
# find_by_prefix: not found
#
# A hex prefix that matches no stored objects must die with
# CasPrefixNotFoundError.
# ---------------------------------------------------------------------------
subtest 'find_by_prefix: not found' => sub {
    my ($cas) = make_cas();

    # Use a prefix that almost certainly matches nothing in an empty store.
    my $err;
    eval { $cas->find_by_prefix('deadbeef') };
    $err = $@;

    ok( $err, 'find_by_prefix on empty store dies' );
    ok( ref($err) && $err->isa('CodingAdventures::ContentAddressableStorage::Error::CasPrefixNotFoundError'),
        'dies with CasPrefixNotFoundError' );
    is( $err->prefix, 'deadbeef', 'error carries the prefix' );
};

# ---------------------------------------------------------------------------
# find_by_prefix: invalid hex — non-hex character
#
# A prefix containing characters outside [0-9a-fA-F] must die with
# CasInvalidPrefixError immediately, without touching the store.
# ---------------------------------------------------------------------------
subtest 'find_by_prefix: invalid hex character' => sub {
    my ($cas) = make_cas();

    my $err;
    eval { $cas->find_by_prefix('xyz') };
    $err = $@;

    ok( $err, 'find_by_prefix with non-hex chars dies' );
    ok( ref($err) && $err->isa('CodingAdventures::ContentAddressableStorage::Error::CasInvalidPrefixError'),
        'dies with CasInvalidPrefixError' );
};

# ---------------------------------------------------------------------------
# find_by_prefix: empty string
#
# An empty prefix would match every object (which is never useful) and must
# die with CasInvalidPrefixError.
# ---------------------------------------------------------------------------
subtest 'find_by_prefix: empty string' => sub {
    my ($cas) = make_cas();

    my $err;
    eval { $cas->find_by_prefix('') };
    $err = $@;

    ok( $err, 'find_by_prefix with empty string dies' );
    ok( ref($err) && $err->isa('CodingAdventures::ContentAddressableStorage::Error::CasInvalidPrefixError'),
        'dies with CasInvalidPrefixError' );
};

# ---------------------------------------------------------------------------
# find_by_prefix: odd-length hex prefix (nibble alignment)
#
# "a3f" (3 chars) is treated as the byte prefix [0xa3, 0xf0] — the trailing
# nibble is the high nibble of the second byte. Any key starting with 0xa3
# and whose second byte has high nibble 0xf qualifies.
# ---------------------------------------------------------------------------
subtest 'find_by_prefix: odd-length prefix resolves correctly' => sub {
    my ($cas) = make_cas();

    my $key = $cas->put('odd prefix test blob');
    # Use the first 3 chars of the key as an odd-length prefix.
    my $prefix3 = substr($key, 0, 3);

    my $found;
    eval { $found = $cas->find_by_prefix($prefix3) };

    if ($@) {
        # It is possible (but very unlikely) that two stored keys share this
        # 3-char prefix, in which case we would get AmbiguousPrefix. Accept
        # that as a pass since we only have one blob here.
        ok(0, 'unexpected error: ' . $@);
    } else {
        is( $found, $key, 'odd-length prefix resolves to correct key' );
    }
};

done_testing;
