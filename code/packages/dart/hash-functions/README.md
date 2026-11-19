# Dart hash-functions

This package currently contains test-only support for the shared
[DT17 hash-functions profile](../../../specs/DT17b-hash-functions-dart-profile.md).
The support generates a byte-preserving fixture snapshot for Dart VM and Node
tests. Production hash APIs are not included in this fixture-tooling checkpoint.

Run `BUILD` or `BUILD_windows` from this directory. The generator reads the
canonical JSON corpus at `code/specs/fixtures/hash-functions-v1/cases.json`
and writes an ignored Dart constant snapshot under `.dart_tool`. The VM test
compares snapshot bytes with the canonical corpus. VM and Node both decode the
same generated snapshot using `dart:convert`.

Snapshot generation is confined to development tooling. The package declares
no production capabilities or runtime I/O.
