# CodingAdventures.ContentAddressableStorage.FSharp

Content-addressable storage helpers for .NET.

The package wraps a pluggable `IBlobStore` with SHA-1 keying, integrity verification on reads, abbreviated hash lookup, and a filesystem-backed `LocalDiskStore` using Git's `<root>/<xx>/<38-hex-chars>` object layout.
