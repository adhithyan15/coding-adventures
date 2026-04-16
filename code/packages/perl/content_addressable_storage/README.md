# CodingAdventures::ContentAddressableStorage — Content-Addressable Storage (Perl)

A generic Content-Addressable Storage (CAS) library in pure Perl.

## What is CAS?

Content-addressable storage maps the *hash of content* to the content itself.
The hash is both the address and the integrity check:

```
Traditional:  name  ──►  content   (name can be reused, content can change)
CAS:          hash  ──►  content   (hash is derived from content, cannot lie)
```

This is the model Git uses for its entire object database. Two identical files
share one object. A renamed file creates zero new storage. Every read is
self-verifying.

## Packages in this directory

| Package | Description |
|---|---|
| `CodingAdventures::ContentAddressableStorage` | CAS wrapper: hashes, stores, verifies, prefix-resolves |
| `CodingAdventures::ContentAddressableStorage::BlobStore` | Abstract base class for storage backends |
| `CodingAdventures::ContentAddressableStorage::LocalDiskStore` | Filesystem backend (Git 2/38 fanout layout) |
| `CodingAdventures::ContentAddressableStorage::Error` | Typed exception hierarchy |

## Quick start

```perl
use CodingAdventures::ContentAddressableStorage;
use CodingAdventures::ContentAddressableStorage::LocalDiskStore;

# Open (or create) a store rooted at /tmp/my-cas
my $store = CodingAdventures::ContentAddressableStorage::LocalDiskStore->new('/tmp/my-cas');
my $cas   = CodingAdventures::ContentAddressableStorage->new($store);

# Store content — get back its SHA-1 key (40 hex chars)
my $key = $cas->put("hello, world");
# $key is now "a0b65939670bc2c010f4d5d6a0b3e4e4590fb92b"

# Retrieve by key — hash is verified automatically
my $data = $cas->get($key);
# $data eq "hello, world"

# Abbreviated lookup (like `git show a0b659`)
my $full_key = $cas->find_by_prefix("a0b659");
# $full_key eq $key

# Idempotent: same content → same key, no duplicate storage
my $key2 = $cas->put("hello, world");
# $key2 eq $key  (one copy stored)
```

## Error handling

```perl
use CodingAdventures::ContentAddressableStorage::Error;

eval { $cas->get('0000000000000000000000000000000000000000') };
if (ref $@ && $@->isa('CodingAdventures::ContentAddressableStorage::Error::CasNotFoundError')) {
    say "not found: ", $@->key;
} elsif (ref $@ && $@->isa('CodingAdventures::ContentAddressableStorage::Error::CasCorruptedError')) {
    say "corrupted: ", $@->key;
} elsif ($@) {
    die $@;   # re-throw unexpected errors
}
```

Error classes:

| Class | Meaning |
|---|---|
| `CasNotFoundError` | Key not in the store |
| `CasCorruptedError` | Stored bytes don't hash to the key |
| `CasAmbiguousPrefixError` | Hex prefix matches 2+ objects |
| `CasPrefixNotFoundError` | Hex prefix matches 0 objects |
| `CasInvalidPrefixError` | Prefix is empty or contains non-hex characters |

## Implementing a custom backend

Subclass `CodingAdventures::ContentAddressableStorage::BlobStore` and override the four methods:

```perl
package MyBackend;
our @ISA = ('CodingAdventures::ContentAddressableStorage::BlobStore');

# $key_hex is a 40-char lowercase hex string
# $data    is a raw byte string
sub put    { my ($self, $key_hex, $data) = @_; ... }
sub get    { my ($self, $key_hex) = @_;        ... }  # die if not found
sub exists { my ($self, $key_hex) = @_;        ... }  # return 1 or 0
sub keys_with_prefix {
    my ($self, $prefix_bytes) = @_;
    ...  # return arrayref of 40-char hex strings
}
```

## LocalDiskStore layout

Objects are stored using Git's 2/38 fanout:

```
<root>/
  a3/
    f4b2c1d0e9f8a7b6c5d4e3f2a1b0c9d8e7f6a5   ← 38-char filename
  fe/
    9a3b...
```

Writes are atomic: data is written to a temp file (with PID + timestamp suffix
to resist symlink attacks), then `rename()`d into place.

## Dependencies

- `CodingAdventures::Sha1` — in-repo pure-Perl SHA-1 (no CPAN Digest::SHA)
- Core Perl modules only: `File::Path`, `File::Basename`, `File::Temp`

## Running tests

```sh
PERL5LIB=../sha1/lib prove -l -v t/
```

## Where this fits

This package implements the CAS layer only. It does **not** define:

- What the stored bytes mean (blob vs tree vs commit — that's a layer above)
- How bytes are compressed at rest (the BlobStore implementation decides)
- Git object headers (`"blob N\0"`) — a separate git-object package
- Ref database (branches, HEAD) — a separate refs package
