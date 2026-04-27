# Changelog — CodingAdventures::ContentAddressableStorage (Perl)

All notable changes to this package will be documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.01] — 2026-04-12

### Added

- `CodingAdventures::ContentAddressableStorage` — main CAS wrapper. Hashes content with SHA-1,
  delegates storage to a pluggable `BlobStore` backend, verifies integrity on
  every read, and resolves abbreviated hex prefixes to full keys.
- `CodingAdventures::ContentAddressableStorage::BlobStore` — abstract base class defining the four
  required methods (`put`, `get`, `exists`, `keys_with_prefix`). Default
  implementations die with "abstract method" to catch missing overrides early.
- `CodingAdventures::ContentAddressableStorage::LocalDiskStore` — filesystem backend using Git's
  2/38 fanout directory layout. Writes are atomic (write-to-temp + rename).
  Temp files use PID + fractional-time suffix to resist symlink attacks.
- `CodingAdventures::ContentAddressableStorage::Error` — typed exception hierarchy:
  `CasNotFoundError`, `CasCorruptedError`, `CasAmbiguousPrefixError`,
  `CasPrefixNotFoundError`, `CasInvalidPrefixError`. All are blessed hashrefs;
  thrown with `die`, caught with `eval`/`$@`/`isa`.
- SHA-1 hashing uses the in-repo `CodingAdventures::Sha1` package (no CPAN
  `Digest::SHA` dependency).
- Test suite: `t/01-basic.t` (round-trip, idempotency, error paths, prefix
  lookup), `t/02-local-disk-store.t` (2/38 layout, binary data, concurrent
  writes), `t/03-blob-store-abstract.t` (abstract method guards).
