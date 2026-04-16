# content-addressable-storage

Content-addressable blob storage for Haskell.

## What it does

- computes SHA-1 keys from blob content
- stores and retrieves blobs through pluggable backends
- supports in-memory and local-disk stores
- resolves abbreviated hex prefixes and verifies content integrity on read

## Status

This Haskell package now includes a self-contained SHA-1 implementation plus tests for hex conversion, corruption detection, prefix lookup, and disk-backed storage.
