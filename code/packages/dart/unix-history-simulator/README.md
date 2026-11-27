# Unix history simulator (Dart)

An in-memory, read-only teaching model of paths, inode contents, descriptor
slots, and the kernel calls used by HOS00. `FileSystem`, `DescriptorTable`, and
`Kernel` can be composed without the shell adapter. The CLI accepts one command
and emits the shared JSON `{output, events}` trace.

Run `dart run test/model_test.dart` for component checks. The repository BUILD
also replays the shared nine-case trace through an external fixture adapter.
The package exposes only the in-memory teaching model; it has no shipped CLI
or fixture-file dependency. It requires Dart 3 and does not emulate a historical
CPU or syscall ABI.
