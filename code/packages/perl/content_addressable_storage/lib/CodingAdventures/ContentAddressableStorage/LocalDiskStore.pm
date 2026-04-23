package CodingAdventures::ContentAddressableStorage::LocalDiskStore;

# ============================================================================
# CodingAdventures::ContentAddressableStorage::LocalDiskStore — filesystem CAS backend
# ============================================================================
#
# Stores blobs on the local filesystem using the Git 2/38 fanout layout.
#
# Why 2/38 fanout?
# ─────────────────
# A repository with 100 000 objects would put all 100 000 files in a single
# directory if we stored objects as root/<40-hex>. Most filesystems degrade
# badly at that scale — directory lookups become O(n) in the worst case, and
# tools like `ls` become painfully slow.
#
# Git's solution: split on the first byte of the hash, creating up to 256
# subdirectories (one per possible first-byte value, from "00/" to "ff/").
# Each subdirectory holds at most N/256 objects, keeping directory sizes
# manageable even in large repositories.
#
# Layout example:
#
#   <root>/
#     a3/                        ← first byte of hash "a3f4b2…", hex-encoded
#       f4b2c1d0e9f8a7b6c5d4e3f2a1b0c9d8e7f6a5    ← remaining 38 hex chars
#     fe/
#       9a3b…
#
# Atomic writes
# ─────────────
# The critical correctness property: a reader must never see a partial write.
# We achieve this with the classic write-to-temp-then-rename pattern:
#
#   1. Create a temp file in the same directory as the final file.
#      (Same directory = same filesystem = rename is always local, never a
#       cross-device copy which would not be atomic.)
#   2. Write all data to the temp file.
#   3. Flush and close the temp file.
#   4. rename(temp, final) — on POSIX this is atomic per POSIX.1-2017 §2.9.7.
#      On Windows, rename may fail if the destination already exists; we treat
#      that as a successful idempotent write (another writer got there first
#      with the identical content).
#
# Temp file naming
# ────────────────
# We use "$filename.$$.$time_frac.tmp" (process ID + fractional time) rather
# than a fixed suffix. A fixed name like "a3/f4b2…42.tmp" could be targeted
# by a local attacker who places a symlink at that path before our open(),
# redirecting our write to an arbitrary file. The PID and time make the name
# unpredictable to other processes.

use strict;
use warnings;
use utf8;

use File::Path qw(make_path);
use File::Basename qw(dirname);
use CodingAdventures::ContentAddressableStorage::BlobStore;

our @ISA = ('CodingAdventures::ContentAddressableStorage::BlobStore');
our $VERSION = '0.01';

# ---------------------------------------------------------------------------
# new($root) — create or open a store rooted at directory $root.
#
# Creates $root (and all parent directories) if they do not exist.
# Dies if the directory cannot be created.
# ---------------------------------------------------------------------------
sub new {
    my ($class, $root) = @_;
    die "LocalDiskStore->new() requires a root directory path\n"
        unless defined $root;

    # make_path is like `mkdir -p` — creates all intermediate directories.
    make_path($root) unless -d $root;
    die "Cannot create or access root directory: $root\n" unless -d $root;

    return bless { root => $root }, $class;
}

# ---------------------------------------------------------------------------
# root() — accessor for the root directory path
# ---------------------------------------------------------------------------
sub root { $_[0]->{root} }

# ---------------------------------------------------------------------------
# _object_path($key_hex) — compute the filesystem path for a given key
#
# The 40-char hex key is split 2|38:
#   key_hex = "a3f4b2c1d0e9f8a7b6c5d4e3f2a1b0c9d8e7f6a5"
#   dir     = root . "/a3"
#   file    = dir  . "/f4b2c1d0e9f8a7b6c5d4e3f2a1b0c9d8e7f6a5"
#
# This is a private helper — the leading underscore is a Perl convention for
# "not part of the public API".
# ---------------------------------------------------------------------------
sub _object_path {
    my ($self, $key_hex) = @_;
    my $dir_part  = substr($key_hex, 0, 2);   # first byte → 2 hex chars
    my $file_part = substr($key_hex, 2);       # remaining 38 hex chars
    return $self->{root} . '/' . $dir_part . '/' . $file_part;
}

# ---------------------------------------------------------------------------
# put($key_hex, $data) — atomically write $data to the store
#
# Short-circuit: if the file already exists, return immediately. Because keys
# are cryptographic hashes of content, an existing file is guaranteed to hold
# identical bytes — no verification needed.
#
# Write path:
#   1. Resolve final_path via _object_path.
#   2. If final_path exists → return (idempotent).
#   3. make_path on the parent directory (e.g. create "a3/" if needed).
#   4. Write data to a temp file in the same directory.
#   5. rename temp → final_path.
#   6. On rename failure: clean up temp; if final_path now exists, return OK
#      (concurrent writer beat us with identical content); otherwise die.
# ---------------------------------------------------------------------------
sub put {
    my ($self, $key_hex, $data) = @_;

    my $final_path = $self->_object_path($key_hex);

    # Short-circuit for idempotent writes: same hash = same content.
    return if -e $final_path;

    # Ensure the two-character fanout directory exists.
    my $dir = dirname($final_path);
    make_path($dir) unless -d $dir;

    # Build an unpredictable temp filename: base + PID + fractional time.
    # Fractional time gives microsecond resolution, making collisions vanishingly
    # unlikely even when two puts of different keys happen in the same process
    # at the same time.
    my $basename  = (split '/', $final_path)[-1];
    my $time_frac = sprintf("%.6f", time());
    $time_frac    =~ s/\.//;          # "1713000000.123456" → "1713000000123456"
    my $tmp_path  = "$dir/$basename.$$.${time_frac}.tmp";

    # Write to temp file. open() in write-binary mode ">:raw" ensures no
    # newline translation on Windows and no UTF-8 encoding layers.
    open(my $fh, '>:raw', $tmp_path)
        or die "Cannot create temp file $tmp_path: $!\n";
    print $fh $data;
    close($fh)
        or die "Cannot close temp file $tmp_path: $!\n";

    # Atomic rename into final position.
    unless (rename($tmp_path, $final_path)) {
        # Rename failed. Clean up the temp file to avoid leaving orphans.
        unlink $tmp_path;
        # If the final file now exists, a concurrent writer stored the same
        # object. That is fine — CAS guarantees same hash = same content.
        return if -e $final_path;
        die "Cannot rename $tmp_path to $final_path: $!\n";
    }

    return;
}

# ---------------------------------------------------------------------------
# get($key_hex) — read and return the stored bytes
#
# Opens the file at the computed path and slurps it. Returns the raw byte
# string. Dies if the file does not exist or cannot be read.
# ---------------------------------------------------------------------------
sub get {
    my ($self, $key_hex) = @_;

    my $path = $self->_object_path($key_hex);

    open(my $fh, '<:raw', $path)
        or die "Object not found: $path: $!\n";

    # Slurp the entire file into $data.
    local $/;
    my $data = <$fh>;
    close($fh);

    return $data;
}

# ---------------------------------------------------------------------------
# exists($key_hex) — check presence without fetching the blob
#
# Returns 1 if the object file exists, 0 otherwise.
# ---------------------------------------------------------------------------
sub exists {
    my ($self, $key_hex) = @_;
    return (-e $self->_object_path($key_hex)) ? 1 : 0;
}

# ---------------------------------------------------------------------------
# keys_with_prefix($prefix_bytes) — list all keys whose bytes start with prefix
#
# $prefix_bytes is a raw byte string (the decoded form of a hex prefix like
# "\xa3\xf4"). We re-encode it to hex to compare against stored filenames,
# then scan the appropriate fanout directories.
#
# Algorithm:
#   1. Convert prefix_bytes to a lowercase hex string: prefix_hex.
#   2. Identify which fanout bucket(s) to scan:
#      - prefix_hex length >= 2: scan only the bucket named prefix_hex[0..1].
#      - prefix_hex length == 1: scan all 16 buckets starting with that nibble
#        ("a0" through "af" if prefix_hex is "a").
#      - prefix_hex length == 0: not reachable (CAS rejects empty prefix).
#   3. For each bucket directory, read its entries and check whether
#      dir_name + file_name starts with prefix_hex.
#   4. Collect the matching 40-char hex keys and return them as an arrayref.
# ---------------------------------------------------------------------------
sub keys_with_prefix {
    my ($self, $prefix_bytes) = @_;

    return [] unless defined $prefix_bytes && length($prefix_bytes) > 0;

    # Encode prefix bytes to lowercase hex for string comparison.
    my $prefix_hex = join('', map { sprintf('%02x', ord($_)) } split //, $prefix_bytes);

    my @results;
    my $root = $self->{root};

    # Determine which fanout directories to scan.
    # The fanout dir name is always exactly 2 hex chars (one byte).
    if (length($prefix_hex) >= 2) {
        # We know the exact bucket: the first 2 chars of the prefix.
        my $bucket = substr($prefix_hex, 0, 2);
        _scan_bucket(\@results, $root, $bucket, $prefix_hex);
    } else {
        # prefix_hex is exactly 1 character — a single nibble.
        # We must scan all 16 buckets that share that high nibble.
        # For example, prefix "a" means scan "a0", "a1", …, "af".
        my $nibble = substr($prefix_hex, 0, 1);
        for my $lo (0..9, 'a'..'f') {
            my $bucket = $nibble . $lo;
            _scan_bucket(\@results, $root, $bucket, $prefix_hex);
        }
    }

    # Sort for deterministic order (mirrors Rust's sort_unstable).
    @results = sort @results;

    return \@results;
}

# ---------------------------------------------------------------------------
# _scan_bucket(\@results, $root, $bucket, $prefix_hex)
#
# Scan a single fanout directory ($root/$bucket/) for files whose full hex
# name (bucket + filename) starts with $prefix_hex. Appends matching
# 40-char hex strings to @results.
#
# This is a module-level private function (not a method). It takes a ref to
# the results array so it can populate it in place without returning a list.
# ---------------------------------------------------------------------------
sub _scan_bucket {
    my ($results_ref, $root, $bucket, $prefix_hex) = @_;

    my $dir = "$root/$bucket";
    return unless -d $dir;

    # opendir/readdir is the low-level Perl way to list a directory.
    opendir(my $dh, $dir) or return;
    my @entries = readdir($dh);
    closedir($dh);

    for my $entry (@entries) {
        next if $entry eq '.' || $entry eq '..';

        # The full 40-char hex key is the bucket name concatenated with the
        # filename. For example: bucket "a3", file "f4b2…" → key "a3f4b2…".
        my $full_hex = $bucket . $entry;
        next unless length($full_hex) == 40;   # paranoia: skip malformed files

        # Check whether this key starts with the desired prefix.
        if (substr($full_hex, 0, length($prefix_hex)) eq $prefix_hex) {
            push @{$results_ref}, $full_hex;
        }
    }
}

1;
